use crate::protocol::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingPollOutcome,
    RadrootsAppRemoteSignerPendingPoller, RadrootsAppRemoteSignerPendingSession,
    RadrootsAppRemoteSignerProgressUpdate, radroots_studio_app_remote_signer_connect_pending,
    radroots_studio_app_remote_signer_open_pending_poller,
    radroots_studio_app_remote_signer_poll_pending_poller_with_progress,
};
use crate::session::RadrootsAppRemoteSignerSessionRecord;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type RadrootsAppRemoteSignerConnectPendingFn =
    Arc<dyn Fn(&str) -> Result<RadrootsAppRemoteSignerPendingSession, String> + Send + Sync>;
type RadrootsAppRemoteSignerPollPendingFn = Arc<
    dyn Fn(
            &RadrootsAppRemoteSignerSessionRecord,
            &str,
            Arc<dyn Fn(RadrootsAppRemoteSignerProgressUpdate) + Send + Sync>,
        ) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, String>
        + Send
        + Sync,
>;
type RadrootsAppRemoteSignerSleepFn = Arc<dyn Fn(Duration) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppRemoteSignerPendingState {
    Idle,
    WaitingApproval,
    AwaitingAuthorization { url: String },
    TransportFailure { message: String },
}

pub trait RadrootsAppRemoteSignerControllerHooks: Clone + Send + Sync + 'static {
    type ReadyState: Send + Sync + 'static;

    fn reconcile_startup_state(&self) -> Result<(), String> {
        Ok(())
    }

    fn store_pending_session(
        &self,
        pending: &RadrootsAppRemoteSignerPendingSession,
    ) -> Result<(), String>;

    fn pending_session_record(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String>;

    fn load_pending_client_secret(&self, client_account_id: &str) -> Result<String, String>;

    fn activate_pending_session(
        &self,
        client_account_id: &str,
        approved: RadrootsAppRemoteSignerApprovedSession,
    ) -> Result<Self::ReadyState, String>;

    fn clear_pending_session(&self)
    -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String>;
}

pub struct RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    hooks: H,
    connect_pending: RadrootsAppRemoteSignerConnectPendingFn,
    poll_pending: RadrootsAppRemoteSignerPollPendingFn,
    sleep: RadrootsAppRemoteSignerSleepFn,
    update: Arc<Mutex<Option<Result<Option<H::ReadyState>, String>>>>,
    changed: Arc<AtomicBool>,
    connecting: Arc<AtomicBool>,
    polling: Arc<AtomicBool>,
    poll_generation: Arc<AtomicU64>,
    pending_state: Arc<Mutex<RadrootsAppRemoteSignerPendingState>>,
    _ready_state: PhantomData<H::ReadyState>,
}

impl<H> Clone for RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
            connect_pending: Arc::clone(&self.connect_pending),
            poll_pending: Arc::clone(&self.poll_pending),
            sleep: Arc::clone(&self.sleep),
            update: Arc::clone(&self.update),
            changed: Arc::clone(&self.changed),
            connecting: Arc::clone(&self.connecting),
            polling: Arc::clone(&self.polling),
            poll_generation: Arc::clone(&self.poll_generation),
            pending_state: Arc::clone(&self.pending_state),
            _ready_state: PhantomData,
        }
    }
}

impl<H> RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    pub fn new(hooks: H) -> Self {
        Self::new_with_ops(
            hooks,
            Arc::new(default_connect_pending),
            default_poll_pending(),
            Arc::new(std::thread::sleep),
        )
    }

    fn new_with_ops(
        hooks: H,
        connect_pending: RadrootsAppRemoteSignerConnectPendingFn,
        poll_pending: RadrootsAppRemoteSignerPollPendingFn,
        sleep: RadrootsAppRemoteSignerSleepFn,
    ) -> Self {
        let controller = Self {
            hooks,
            connect_pending,
            poll_pending,
            sleep,
            update: Arc::new(Mutex::new(None)),
            changed: Arc::new(AtomicBool::new(false)),
            connecting: Arc::new(AtomicBool::new(false)),
            polling: Arc::new(AtomicBool::new(false)),
            poll_generation: Arc::new(AtomicU64::new(0)),
            pending_state: Arc::new(Mutex::new(RadrootsAppRemoteSignerPendingState::Idle)),
            _ready_state: PhantomData,
        };
        if let Err(error) = controller.reconcile_startup_state() {
            controller.push_update(Err(error));
        } else if let Err(error) = controller.resume_pending() {
            controller.push_update(Err(error));
        }
        controller
    }

    pub fn take_update(&self) -> Option<Result<Option<H::ReadyState>, String>> {
        if !self.changed.swap(false, Ordering::AcqRel) {
            return None;
        }

        self.update.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn is_connecting(&self) -> bool {
        self.connecting.load(Ordering::Acquire)
    }

    pub fn pending_state(&self) -> RadrootsAppRemoteSignerPendingState {
        self.pending_state
            .lock()
            .map(|state| state.clone())
            .unwrap_or(RadrootsAppRemoteSignerPendingState::Idle)
    }

    pub fn begin_connect(&self, input: &str) -> Result<(), String> {
        if self.connecting.swap(true, Ordering::AcqRel) {
            return Err("remote signer connection is already starting".to_owned());
        }

        if self.pending_session_record()?.is_some() {
            self.connecting.store(false, Ordering::Release);
            return Err("a remote signer connection is already pending approval".to_owned());
        }

        if let Ok(mut slot) = self.update.lock() {
            *slot = None;
        }
        self.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);

        let tracker = self.clone();
        let input = input.to_owned();
        std::thread::spawn(move || {
            let outcome = (|| -> Result<(), String> {
                let pending = (tracker.connect_pending)(input.as_str())?;
                tracker.hooks.store_pending_session(&pending)?;
                tracker.start_polling();
                Ok(())
            })();

            if let Err(error) = outcome {
                tracker.push_update(Err(error));
            }
            tracker.connecting.store(false, Ordering::Release);
        });

        Ok(())
    }

    pub fn pending_session_record(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
        self.hooks.pending_session_record()
    }

    fn reconcile_startup_state(&self) -> Result<(), String> {
        self.hooks.reconcile_startup_state()
    }

    fn resume_pending(&self) -> Result<(), String> {
        let Some(record) = self.pending_session_record()? else {
            return Ok(());
        };
        self.hooks
            .load_pending_client_secret(record.client_account_id())?;
        self.start_polling();
        Ok(())
    }

    fn start_polling(&self) {
        let request_generation = self.poll_generation.fetch_add(1, Ordering::AcqRel) + 1;
        if self.polling.swap(true, Ordering::AcqRel) {
            return;
        }

        let tracker = self.clone();
        std::thread::spawn(move || {
            loop {
                let pending_record = match tracker.hooks.pending_session_record() {
                    Ok(Some(record)) => record,
                    Ok(None) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        tracker.finish_polling(request_generation);
                        return;
                    }
                    Err(error) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        tracker.push_update(Err(error));
                        tracker.finish_polling(request_generation);
                        return;
                    }
                };
                let client_secret_key_hex = match tracker
                    .hooks
                    .load_pending_client_secret(pending_record.client_account_id())
                {
                    Ok(secret) => secret,
                    Err(error) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        tracker.push_update(Err(error));
                        tracker.finish_polling(request_generation);
                        return;
                    }
                };

                let progress_tracker = tracker.clone();
                let progress: Arc<dyn Fn(RadrootsAppRemoteSignerProgressUpdate) + Send + Sync> =
                    Arc::new(move |update| progress_tracker.apply_progress(update));

                match (tracker.poll_pending)(
                    &pending_record,
                    client_secret_key_hex.as_str(),
                    progress,
                ) {
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval) => {
                        tracker.set_pending_state(
                            RadrootsAppRemoteSignerPendingState::WaitingApproval,
                        );
                        (tracker.sleep)(Duration::from_secs(1));
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { message }) => {
                        let changed = tracker.set_pending_state(
                            RadrootsAppRemoteSignerPendingState::TransportFailure {
                                message: message.clone(),
                            },
                        );
                        if changed {
                            tracker.push_update(Err(format!(
                                "remote signer approval check failed: {message}"
                            )));
                        }
                        (tracker.sleep)(Duration::from_secs(1));
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::Approved(approved)) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        let ready_state = match tracker
                            .hooks
                            .activate_pending_session(pending_record.client_account_id(), approved)
                        {
                            Ok(state) => state,
                            Err(error) => {
                                tracker
                                    .set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                                tracker.push_update(Err(error));
                                tracker.finish_polling(request_generation);
                                return;
                            }
                        };
                        tracker.push_update(Ok(Some(ready_state)));
                        tracker.finish_polling(request_generation);
                        return;
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message })
                    | Ok(RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message }) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        let outcome = tracker
                            .hooks
                            .clear_pending_session()
                            .map(|_| None)
                            .unwrap_or_else(|cleanup_error| Some(cleanup_error));
                        let error = match outcome {
                            Some(cleanup_error) => format!(
                                "{message}. remote signer cleanup needs retry: {cleanup_error}"
                            ),
                            None => message,
                        };
                        tracker.push_update(Err(error));
                        tracker.finish_polling(request_generation);
                        return;
                    }
                    Err(error) => {
                        tracker.set_pending_state(RadrootsAppRemoteSignerPendingState::Idle);
                        tracker.push_update(Err(error));
                        tracker.finish_polling(request_generation);
                        return;
                    }
                }
            }
        });
    }

    fn push_update(&self, result: Result<Option<H::ReadyState>, String>) {
        if let Ok(mut slot) = self.update.lock() {
            *slot = Some(result);
            self.changed.store(true, Ordering::Release);
        }
    }

    fn apply_progress(&self, update: RadrootsAppRemoteSignerProgressUpdate) {
        match update {
            RadrootsAppRemoteSignerProgressUpdate::AuthChallenge { url } => {
                let next =
                    RadrootsAppRemoteSignerPendingState::AwaitingAuthorization { url: url.clone() };
                if self.set_pending_state(next) {
                    self.push_update(Err(format!(
                        "authorize the remote signer to continue: {url}"
                    )));
                }
            }
        }
    }

    fn set_pending_state(&self, next: RadrootsAppRemoteSignerPendingState) -> bool {
        if let Ok(mut state) = self.pending_state.lock() {
            if *state == next {
                return false;
            }
            *state = next;
            return true;
        }
        false
    }

    fn finish_polling(&self, worker_generation: u64) {
        self.polling.store(false, Ordering::Release);
        if self.poll_generation.load(Ordering::Acquire) != worker_generation {
            self.start_polling();
        }
    }
}

fn default_connect_pending(input: &str) -> Result<RadrootsAppRemoteSignerPendingSession, String> {
    radroots_studio_app_remote_signer_connect_pending(input).map_err(|error| error.to_string())
}

fn default_poll_pending() -> RadrootsAppRemoteSignerPollPendingFn {
    let cache: Arc<Mutex<Option<(String, RadrootsAppRemoteSignerPendingPoller)>>> =
        Arc::new(Mutex::new(None));
    Arc::new(
        move |record, client_secret_key_hex, progress| -> Result<_, String> {
            let client_account_id = record.client_account_id().to_owned();
            let mut cache = cache
                .lock()
                .map_err(|_| "pending poller cache lock poisoned".to_owned())?;
            let poller = match cache.as_mut() {
                Some((cached_account_id, poller)) if *cached_account_id == client_account_id => {
                    poller
                }
                _ => {
                    let poller = radroots_studio_app_remote_signer_open_pending_poller(
                        record,
                        client_secret_key_hex,
                    )
                    .map_err(|error| error.to_string())?;
                    *cache = Some((client_account_id.clone(), poller));
                    &mut cache.as_mut().expect("cache initialized").1
                }
            };
            let outcome = radroots_studio_app_remote_signer_poll_pending_poller_with_progress(
                poller,
                &mut |update| progress(update),
            )
            .map_err(|error| error.to_string())?;
            if !matches!(
                outcome,
                RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval
                    | RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { .. }
            ) {
                *cache = None;
            }
            Ok(outcome)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, FIXTURE_BOB, FIXTURE_CAROL, fixture_identity};
    use radroots_identity::RadrootsIdentityPublic;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Condvar;
    use std::sync::mpsc::{self, Receiver, Sender};
    use std::time::Instant;

    #[derive(Clone, Debug)]
    enum TestPendingBehavior {
        PendingApproval,
        TransportFailure(&'static str),
        Rejected(&'static str),
    }

    #[derive(Default)]
    struct TestHooksState {
        pending: Option<RadrootsAppRemoteSignerSessionRecord>,
        active: HashMap<String, String>,
        secrets: HashMap<String, String>,
        pending_record_gate: Option<PendingRecordGate>,
        clear_pending_gate: Option<ClearPendingGate>,
    }

    #[derive(Clone)]
    struct PendingRecordGate {
        entered: Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
    }

    #[derive(Clone)]
    struct ClearPendingGate {
        entered: Sender<()>,
        release: Arc<(Mutex<bool>, Condvar)>,
    }

    #[derive(Clone, Default)]
    struct TestHooks {
        state: Arc<Mutex<TestHooksState>>,
    }

    impl TestHooks {
        fn set_pending(&self, record: Option<RadrootsAppRemoteSignerSessionRecord>) {
            self.state.lock().expect("hooks lock").pending = record;
        }

        fn set_secret(&self, client_account_id: &str, secret: &str) {
            self.state
                .lock()
                .expect("hooks lock")
                .secrets
                .insert(client_account_id.to_owned(), secret.to_owned());
        }

        fn install_pending_record_gate(
            &self,
            entered: Sender<()>,
            release: Arc<(Mutex<bool>, Condvar)>,
        ) {
            self.state.lock().expect("hooks lock").pending_record_gate =
                Some(PendingRecordGate { entered, release });
        }

        fn install_clear_pending_gate(
            &self,
            entered: Sender<()>,
            release: Arc<(Mutex<bool>, Condvar)>,
        ) {
            self.state.lock().expect("hooks lock").clear_pending_gate =
                Some(ClearPendingGate { entered, release });
        }
    }

    impl RadrootsAppRemoteSignerControllerHooks for TestHooks {
        type ReadyState = String;

        fn store_pending_session(
            &self,
            pending: &RadrootsAppRemoteSignerPendingSession,
        ) -> Result<(), String> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "hooks lock poisoned".to_owned())?;
            state.pending = Some(pending.record.clone());
            state.secrets.insert(
                pending.record.client_account_id().to_owned(),
                pending.client_secret_key_hex.clone(),
            );
            Ok(())
        }

        fn pending_session_record(
            &self,
        ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
            let gate = {
                self.state
                    .lock()
                    .map_err(|_| "hooks lock poisoned".to_owned())?
                    .pending_record_gate
                    .take()
            };
            if let Some(gate) = gate {
                let _ = gate.entered.send(());
                wait_for_gate(&gate.release);
            }
            self.state
                .lock()
                .map_err(|_| "hooks lock poisoned".to_owned())
                .map(|state| state.pending.clone())
        }

        fn load_pending_client_secret(&self, client_account_id: &str) -> Result<String, String> {
            self.state
                .lock()
                .map_err(|_| "hooks lock poisoned".to_owned())?
                .secrets
                .get(client_account_id)
                .cloned()
                .ok_or_else(|| "missing pending client secret".to_owned())
        }

        fn activate_pending_session(
            &self,
            client_account_id: &str,
            approved: RadrootsAppRemoteSignerApprovedSession,
        ) -> Result<Self::ReadyState, String> {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "hooks lock poisoned".to_owned())?;
            state.pending = None;
            state.active.insert(
                client_account_id.to_owned(),
                approved.user_identity.id.to_string(),
            );
            Ok(approved.user_identity.id.to_string())
        }

        fn clear_pending_session(
            &self,
        ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
            let (removed, gate) = {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| "hooks lock poisoned".to_owned())?;
                (state.pending.take(), state.clear_pending_gate.take())
            };
            if let Some(gate) = gate {
                let _ = gate.entered.send(());
                wait_for_gate(&gate.release);
            }
            Ok(removed)
        }
    }

    fn wait_for_gate(gate: &Arc<(Mutex<bool>, Condvar)>) {
        let (ready_lock, ready_cvar) = &**gate;
        let mut ready = ready_lock.lock().expect("gate lock");
        while !*ready {
            ready = ready_cvar.wait(ready).expect("gate wait");
        }
    }

    fn open_gate(gate: &Arc<(Mutex<bool>, Condvar)>) {
        let (ready_lock, ready_cvar) = &**gate;
        let mut ready = ready_lock.lock().expect("gate lock");
        *ready = true;
        ready_cvar.notify_all();
    }

    fn fixture_public(
        fixture: &radroots_studio_app_test_support::RadrootsAppApprovedFixtureIdentity,
    ) -> RadrootsIdentityPublic {
        fixture_identity(fixture).expect("identity").to_public()
    }

    fn pending_record(client: &str, signer: &str) -> RadrootsAppRemoteSignerSessionRecord {
        RadrootsAppRemoteSignerSessionRecord::pending(
            fixture_public(match client {
                "alice-client" => &FIXTURE_ALICE,
                "bob-client" => &FIXTURE_BOB,
                _ => &FIXTURE_CAROL,
            }),
            fixture_public(match signer {
                "alice-signer" => &FIXTURE_ALICE,
                "bob-signer" => &FIXTURE_BOB,
                _ => &FIXTURE_CAROL,
            }),
            vec!["wss://relay.example".to_owned()],
        )
    }

    fn pending_session_for_input(
        input: &str,
    ) -> Result<RadrootsAppRemoteSignerPendingSession, String> {
        let record = match input {
            "next" => pending_record("bob-client", "bob-signer"),
            "reject-next" => pending_record("carol-client", "carol-signer"),
            other => return Err(format!("unexpected connect input: {other}")),
        };
        Ok(RadrootsAppRemoteSignerPendingSession {
            client_secret_key_hex: format!("secret-for-{}", record.client_account_id()),
            record,
        })
    }

    fn no_sleep(_: Duration) {}

    fn wait_for_message(receiver: &Receiver<String>) -> String {
        receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("timed out waiting for poll message")
    }

    fn wait_for_update(
        controller: &RadrootsAppRemoteSignerController<TestHooks>,
    ) -> Result<Option<String>, String> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(update) = controller.take_update() {
                return update;
            }
            if Instant::now() >= deadline {
                panic!("timed out waiting for controller update");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn restart_request_during_empty_exit_window_respawns_poller() {
        let hooks = TestHooks::default();
        let (poll_tx, poll_rx) = mpsc::channel();

        let controller = RadrootsAppRemoteSignerController::new_with_ops(
            hooks.clone(),
            Arc::new(pending_session_for_input),
            Arc::new(move |record, _, _progress| {
                poll_tx
                    .send(record.client_account_id().to_owned())
                    .expect("send poll id");
                Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected {
                    message: "rejected".to_owned(),
                })
            }),
            Arc::new(no_sleep),
        );

        let initial = pending_record("alice-client", "alice-signer");
        hooks.set_secret(initial.client_account_id(), "secret-for-initial");
        hooks.set_pending(Some(initial.clone()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        hooks.install_pending_record_gate(entered_tx, Arc::clone(&release));
        controller.start_polling();

        hooks.set_pending(None);
        entered_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("pending record gate was not entered");

        let next = pending_session_for_input("next").expect("next pending");
        hooks.set_secret(next.record.client_account_id(), "secret-for-next");
        hooks.set_pending(Some(next.record.clone()));
        controller.start_polling();
        open_gate(&release);

        assert_eq!(wait_for_message(&poll_rx), next.record.client_account_id());
    }

    #[test]
    fn begin_connect_after_pending_clear_restarts_polling() {
        let hooks = TestHooks::default();
        let (poll_tx, poll_rx) = mpsc::channel();

        let controller = RadrootsAppRemoteSignerController::new_with_ops(
            hooks.clone(),
            Arc::new(pending_session_for_input),
            Arc::new(move |record, _, _progress| {
                poll_tx
                    .send(record.client_account_id().to_owned())
                    .expect("send poll id");
                Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected {
                    message: "rejected".to_owned(),
                })
            }),
            Arc::new(no_sleep),
        );

        let initial = pending_record("alice-client", "alice-signer");
        hooks.set_secret(initial.client_account_id(), "secret-for-initial");
        hooks.set_pending(Some(initial));
        let (entered_tx, entered_rx) = mpsc::channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        hooks.install_pending_record_gate(entered_tx, Arc::clone(&release));
        controller.start_polling();

        hooks.set_pending(None);
        entered_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("pending record gate was not entered");

        controller.begin_connect("next").expect("begin connect");
        open_gate(&release);

        let expected = pending_session_for_input("next")
            .expect("next pending")
            .record
            .client_account_id()
            .to_owned();
        assert_eq!(wait_for_message(&poll_rx), expected);
    }

    #[test]
    fn begin_connect_after_rejection_cleanup_restarts_polling() {
        let hooks = TestHooks::default();
        let (poll_tx, poll_rx) = mpsc::channel();

        let controller = RadrootsAppRemoteSignerController::new_with_ops(
            hooks.clone(),
            Arc::new(pending_session_for_input),
            Arc::new(move |record, _, _progress| {
                poll_tx
                    .send(record.client_account_id().to_owned())
                    .expect("send poll id");
                Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected {
                    message: "rejected".to_owned(),
                })
            }),
            Arc::new(no_sleep),
        );

        let initial = pending_record("alice-client", "alice-signer");
        hooks.set_secret(initial.client_account_id(), "secret-for-initial");
        hooks.set_pending(Some(initial));
        let (clear_tx, clear_rx) = mpsc::channel();
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        hooks.install_clear_pending_gate(clear_tx, Arc::clone(&release));
        controller.start_polling();

        assert_eq!(
            wait_for_message(&poll_rx),
            fixture_public(&FIXTURE_ALICE).id.to_string()
        );
        clear_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("clear pending gate was not entered");

        controller
            .begin_connect("reject-next")
            .expect("begin connect after rejection");
        open_gate(&release);

        let expected = pending_session_for_input("reject-next")
            .expect("reject-next pending")
            .record
            .client_account_id()
            .to_owned();
        assert_eq!(wait_for_message(&poll_rx), expected);
    }

    #[test]
    fn transport_failure_recovers_back_to_waiting_approval() {
        let hooks = TestHooks::default();
        let pending = pending_record("alice-client", "alice-signer");
        hooks.set_secret(pending.client_account_id(), "secret-for-initial");
        hooks.set_pending(Some(pending.clone()));
        let outcomes = Arc::new(Mutex::new(VecDeque::from([
            TestPendingBehavior::TransportFailure("relay down"),
            TestPendingBehavior::PendingApproval,
            TestPendingBehavior::Rejected("done"),
        ])));
        let (sleep_enter_tx, sleep_enter_rx) = mpsc::channel();
        let first_sleep_release = Arc::new((Mutex::new(false), Condvar::new()));
        let second_sleep_release = Arc::new((Mutex::new(false), Condvar::new()));
        let first_sleep_release_for_closure = Arc::clone(&first_sleep_release);
        let second_sleep_release_for_closure = Arc::clone(&second_sleep_release);
        let sleep_tick = Arc::new(AtomicU64::new(0));
        let sleep_tick_for_closure = Arc::clone(&sleep_tick);

        let controller = RadrootsAppRemoteSignerController::new_with_ops(
            hooks.clone(),
            Arc::new(pending_session_for_input),
            Arc::new(move |_, _, _progress| {
                let next = outcomes
                    .lock()
                    .expect("outcomes lock")
                    .pop_front()
                    .expect("missing test outcome");
                match next {
                    TestPendingBehavior::PendingApproval => {
                        Ok(RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval)
                    }
                    TestPendingBehavior::TransportFailure(message) => Ok(
                        RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure {
                            message: message.to_owned(),
                        },
                    ),
                    TestPendingBehavior::Rejected(message) => {
                        Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected {
                            message: message.to_owned(),
                        })
                    }
                }
            }),
            Arc::new(move |_| {
                let tick = sleep_tick_for_closure.fetch_add(1, Ordering::AcqRel) + 1;
                let _ = sleep_enter_tx.send(tick);
                match tick {
                    1 => wait_for_gate(&first_sleep_release_for_closure),
                    2 => wait_for_gate(&second_sleep_release_for_closure),
                    _ => {}
                }
            }),
        );

        let update = wait_for_update(&controller).expect_err("transport failure update");
        assert_eq!(update, "remote signer approval check failed: relay down");
        assert_eq!(
            sleep_enter_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("transport retry sleep"),
            1
        );
        open_gate(&first_sleep_release);

        assert_eq!(
            sleep_enter_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("pending approval sleep"),
            2
        );
        assert_eq!(
            controller.pending_state(),
            RadrootsAppRemoteSignerPendingState::WaitingApproval
        );
        open_gate(&second_sleep_release);
    }
}
