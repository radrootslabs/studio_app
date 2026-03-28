use crate::protocol::{
    RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerPendingSession,
    radroots_studio_app_remote_signer_connect_pending, radroots_studio_app_remote_signer_poll_pending_session,
};
use crate::session::RadrootsAppRemoteSignerSessionRecord;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub trait RadrootsAppRemoteSignerControllerHooks: Clone + Send + Sync + 'static {
    type ReadyState: Send + 'static;

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
        user_identity: radroots_identity::RadrootsIdentityPublic,
    ) -> Result<Self::ReadyState, String>;

    fn clear_pending_session(&self)
    -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String>;
}

pub struct RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    hooks: H,
    update: Arc<Mutex<Option<Result<Option<H::ReadyState>, String>>>>,
    changed: Arc<AtomicBool>,
    connecting: Arc<AtomicBool>,
    polling: Arc<AtomicBool>,
    _ready_state: PhantomData<H::ReadyState>,
}

impl<H> Clone for RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    fn clone(&self) -> Self {
        Self {
            hooks: self.hooks.clone(),
            update: Arc::clone(&self.update),
            changed: Arc::clone(&self.changed),
            connecting: Arc::clone(&self.connecting),
            polling: Arc::clone(&self.polling),
            _ready_state: PhantomData,
        }
    }
}

impl<H> RadrootsAppRemoteSignerController<H>
where
    H: RadrootsAppRemoteSignerControllerHooks,
{
    pub fn new(hooks: H) -> Self {
        let controller = Self {
            hooks,
            update: Arc::new(Mutex::new(None)),
            changed: Arc::new(AtomicBool::new(false)),
            connecting: Arc::new(AtomicBool::new(false)),
            polling: Arc::new(AtomicBool::new(false)),
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

        let tracker = self.clone();
        let input = input.to_owned();
        std::thread::spawn(move || {
            let outcome = (|| -> Result<(), String> {
                let pending = radroots_studio_app_remote_signer_connect_pending(input.as_str())
                    .map_err(|error| error.to_string())?;
                let client_account_id = pending.record.client_account_id().to_owned();
                let client_secret_key_hex = pending.client_secret_key_hex.clone();
                tracker.hooks.store_pending_session(&pending)?;
                tracker.start_polling(client_account_id, client_secret_key_hex);
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
        let client_secret_key_hex = self
            .hooks
            .load_pending_client_secret(record.client_account_id())?;
        self.start_polling(record.client_account_id().to_owned(), client_secret_key_hex);
        Ok(())
    }

    fn start_polling(&self, client_account_id: String, client_secret_key_hex: String) {
        if self.polling.swap(true, Ordering::AcqRel) {
            return;
        }

        let tracker = self.clone();
        std::thread::spawn(move || {
            loop {
                let pending_record = match tracker.hooks.pending_session_record() {
                    Ok(Some(record)) if record.client_account_id() == client_account_id => record,
                    Ok(Some(_)) | Ok(None) => {
                        tracker.polling.store(false, Ordering::Release);
                        return;
                    }
                    Err(error) => {
                        tracker.push_update(Err(error));
                        tracker.polling.store(false, Ordering::Release);
                        return;
                    }
                };

                match radroots_studio_app_remote_signer_poll_pending_session(
                    &pending_record,
                    client_secret_key_hex.as_str(),
                )
                .map_err(|error| error.to_string())
                {
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval)
                    | Ok(RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { .. }) => {
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::Approved(user_identity)) => {
                        let ready_state = match tracker.hooks.activate_pending_session(
                            pending_record.client_account_id(),
                            user_identity,
                        ) {
                            Ok(state) => state,
                            Err(error) => {
                                tracker.push_update(Err(error));
                                tracker.polling.store(false, Ordering::Release);
                                return;
                            }
                        };
                        tracker.push_update(Ok(Some(ready_state)));
                        tracker.polling.store(false, Ordering::Release);
                        return;
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message })
                    | Ok(RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message }) => {
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
                        tracker.polling.store(false, Ordering::Release);
                        return;
                    }
                    Err(error) => {
                        tracker.push_update(Err(error));
                        tracker.polling.store(false, Ordering::Release);
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
}
