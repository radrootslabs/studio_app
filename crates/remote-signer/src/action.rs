use crate::protocol::{
    RadrootsAppRemoteSignerProgressUpdate, RadrootsAppRemoteSignerSignedEvent,
    radroots_studio_app_remote_signer_sign_kind1_note_with_progress,
};
use crate::session::RadrootsAppRemoteSignerSessionRecord;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type RadrootsAppRemoteSignerSignNoteFn = Arc<
    dyn Fn(
            &RadrootsAppRemoteSignerSessionRecord,
            &str,
            &str,
            Arc<dyn Fn(RadrootsAppRemoteSignerProgressUpdate) + Send + Sync>,
        ) -> Result<RadrootsAppRemoteSignerSignedEvent, String>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppRemoteSignerActionState {
    Idle,
    Signing,
    AwaitingAuthorization { url: String },
}

pub trait RadrootsAppRemoteSignerActionControllerHooks: Clone + Send + Sync + 'static {
    type ReadyState: Send + Sync + 'static;

    fn selected_active_session(
        &self,
    ) -> Result<Option<(RadrootsAppRemoteSignerSessionRecord, String)>, String>;

    fn complete_sign_event(
        &self,
        signed_event: RadrootsAppRemoteSignerSignedEvent,
    ) -> Result<Self::ReadyState, String>;
}

pub struct RadrootsAppRemoteSignerActionController<H>
where
    H: RadrootsAppRemoteSignerActionControllerHooks,
{
    hooks: H,
    sign_kind1_note: RadrootsAppRemoteSignerSignNoteFn,
    update: Arc<Mutex<Option<Result<Option<H::ReadyState>, String>>>>,
    changed: Arc<AtomicBool>,
    signing: Arc<AtomicBool>,
    state: Arc<Mutex<RadrootsAppRemoteSignerActionState>>,
    _ready_state: PhantomData<H::ReadyState>,
}

impl<H> Clone for RadrootsAppRemoteSignerActionController<H>
where
    H: RadrootsAppRemoteSignerActionControllerHooks,
{
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
            sign_kind1_note: Arc::clone(&self.sign_kind1_note),
            update: Arc::clone(&self.update),
            changed: Arc::clone(&self.changed),
            signing: Arc::clone(&self.signing),
            state: Arc::clone(&self.state),
            _ready_state: PhantomData,
        }
    }
}

impl<H> RadrootsAppRemoteSignerActionController<H>
where
    H: RadrootsAppRemoteSignerActionControllerHooks,
{
    pub fn new(hooks: H) -> Self {
        Self {
            hooks,
            sign_kind1_note: Arc::new(default_sign_kind1_note),
            update: Arc::new(Mutex::new(None)),
            changed: Arc::new(AtomicBool::new(false)),
            signing: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(RadrootsAppRemoteSignerActionState::Idle)),
            _ready_state: PhantomData,
        }
    }

    #[cfg(test)]
    fn new_with_ops(hooks: H, sign_kind1_note: RadrootsAppRemoteSignerSignNoteFn) -> Self {
        Self {
            hooks,
            sign_kind1_note,
            update: Arc::new(Mutex::new(None)),
            changed: Arc::new(AtomicBool::new(false)),
            signing: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(RadrootsAppRemoteSignerActionState::Idle)),
            _ready_state: PhantomData,
        }
    }

    pub fn take_update(&self) -> Option<Result<Option<H::ReadyState>, String>> {
        if !self.changed.swap(false, Ordering::AcqRel) {
            return None;
        }
        self.update.lock().ok().and_then(|mut slot| slot.take())
    }

    pub fn is_signing(&self) -> bool {
        self.signing.load(Ordering::Acquire)
    }

    pub fn state(&self) -> RadrootsAppRemoteSignerActionState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or(RadrootsAppRemoteSignerActionState::Idle)
    }

    pub fn begin_sign_kind1_note(&self, content: &str) -> Result<(), String> {
        if self.signing.swap(true, Ordering::AcqRel) {
            return Err("remote signer note signing is already running".to_owned());
        }

        let Some((record, client_secret_key_hex)) = self.hooks.selected_active_session()? else {
            self.signing.store(false, Ordering::Release);
            return Err("select a remote signer account before signing a note".to_owned());
        };
        let note_content = content.trim().to_owned();
        if note_content.is_empty() {
            self.signing.store(false, Ordering::Release);
            return Err("enter a note before requesting a remote signature".to_owned());
        }

        self.set_state(RadrootsAppRemoteSignerActionState::Signing);
        if let Ok(mut slot) = self.update.lock() {
            *slot = None;
        }

        let tracker = self.clone();
        std::thread::spawn(move || {
            let progress_tracker = tracker.clone();
            let progress: Arc<dyn Fn(RadrootsAppRemoteSignerProgressUpdate) + Send + Sync> =
                Arc::new(move |update| progress_tracker.apply_progress(update));
            let outcome = (tracker.sign_kind1_note)(
                &record,
                client_secret_key_hex.as_str(),
                note_content.as_str(),
                progress,
            )
            .and_then(|signed_event| tracker.hooks.complete_sign_event(signed_event));

            tracker.set_state(RadrootsAppRemoteSignerActionState::Idle);
            tracker.signing.store(false, Ordering::Release);
            match outcome {
                Ok(result) => tracker.push_update(Ok(Some(result))),
                Err(error) => tracker.push_update(Err(error)),
            }
        });

        Ok(())
    }

    fn apply_progress(&self, update: RadrootsAppRemoteSignerProgressUpdate) {
        match update {
            RadrootsAppRemoteSignerProgressUpdate::AuthChallenge { url } => {
                let next =
                    RadrootsAppRemoteSignerActionState::AwaitingAuthorization { url: url.clone() };
                if self.set_state(next) {
                    self.push_update(Err(format!(
                        "authorize the remote signer to continue: {url}"
                    )));
                }
            }
        }
    }

    fn push_update(&self, result: Result<Option<H::ReadyState>, String>) {
        if let Ok(mut slot) = self.update.lock() {
            *slot = Some(result);
            self.changed.store(true, Ordering::Release);
        }
    }

    fn set_state(&self, next: RadrootsAppRemoteSignerActionState) -> bool {
        if let Ok(mut state) = self.state.lock() {
            if *state == next {
                return false;
            }
            *state = next;
            return true;
        }
        false
    }
}

fn default_sign_kind1_note(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    content: &str,
    progress: Arc<dyn Fn(RadrootsAppRemoteSignerProgressUpdate) + Send + Sync>,
) -> Result<RadrootsAppRemoteSignerSignedEvent, String> {
    radroots_studio_app_remote_signer_sign_kind1_note_with_progress(
        record,
        client_secret_key_hex,
        content,
        move |update| progress(update),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::RadrootsAppRemoteSignerSessionRecord;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, FIXTURE_BOB, fixture_identity};
    use std::sync::mpsc;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    #[derive(Clone)]
    struct TestHooks {
        session: Option<(RadrootsAppRemoteSignerSessionRecord, String)>,
    }

    impl RadrootsAppRemoteSignerActionControllerHooks for TestHooks {
        type ReadyState = String;

        fn selected_active_session(
            &self,
        ) -> Result<Option<(RadrootsAppRemoteSignerSessionRecord, String)>, String> {
            Ok(self.session.clone())
        }

        fn complete_sign_event(
            &self,
            signed_event: RadrootsAppRemoteSignerSignedEvent,
        ) -> Result<Self::ReadyState, String> {
            Ok(signed_event.event_id_hex)
        }
    }

    fn fixture_session() -> RadrootsAppRemoteSignerSessionRecord {
        let client = fixture_identity(&FIXTURE_ALICE)
            .expect("client")
            .to_public();
        let signer = fixture_identity(&FIXTURE_BOB).expect("signer").to_public();
        let mut record = RadrootsAppRemoteSignerSessionRecord::pending(
            client,
            signer.clone(),
            vec!["ws://localhost:8080".to_owned()],
        );
        record.user_identity = Some(signer);
        record.status = crate::session::RadrootsAppRemoteSignerSessionStatus::Active;
        record
    }

    fn wait_for_update(
        controller: &RadrootsAppRemoteSignerActionController<TestHooks>,
    ) -> Result<Option<String>, String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(update) = controller.take_update() {
                return update;
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out waiting for action update");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn sign_controller_reports_auth_challenge_then_success() {
        let hooks = TestHooks {
            session: Some((fixture_session(), "client-secret".to_owned())),
        };
        let (challenge_seen_tx, challenge_seen_rx) = mpsc::channel();
        let release_gate = Arc::new((Mutex::new(false), Condvar::new()));
        let controller = RadrootsAppRemoteSignerActionController::new_with_ops(
            hooks,
            Arc::new({
                let release_gate = Arc::clone(&release_gate);
                move |_, _, _, progress| {
                    progress(RadrootsAppRemoteSignerProgressUpdate::AuthChallenge {
                        url: "http://localhost/auth".to_owned(),
                    });
                    challenge_seen_tx.send(()).expect("challenge seen");
                    let (released, condvar) = &*release_gate;
                    let mut released = released.lock().expect("release gate lock");
                    while !*released {
                        released = condvar.wait(released).expect("release gate wait");
                    }
                    Ok(RadrootsAppRemoteSignerSignedEvent {
                        event_id_hex: "deadbeef".to_owned(),
                        event_json: "{\"id\":\"deadbeef\"}".to_owned(),
                        relays: vec!["ws://localhost:8080".to_owned()],
                    })
                }
            }),
        );

        controller
            .begin_sign_kind1_note("hello from remote signer")
            .expect("begin signing");

        challenge_seen_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("challenge notification");
        let first = wait_for_update(&controller).expect_err("auth challenge status");
        assert_eq!(
            first,
            "authorize the remote signer to continue: http://localhost/auth"
        );
        assert_eq!(
            controller.state(),
            RadrootsAppRemoteSignerActionState::AwaitingAuthorization {
                url: "http://localhost/auth".to_owned()
            }
        );

        let (released, condvar) = &*release_gate;
        *released.lock().expect("release gate lock") = true;
        condvar.notify_one();
        let second = wait_for_update(&controller).expect("signed");
        assert_eq!(second, Some("deadbeef".to_owned()));
        assert_eq!(controller.state(), RadrootsAppRemoteSignerActionState::Idle);
    }
}
