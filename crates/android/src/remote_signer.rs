use crate::security::{ANDROID_NOSTR_SERVICE, resolve_nostr_storage_root};
use crate::vault::RadrootsAndroidKeystoreVault;
use radroots_studio_app_core::{
    IdentityGateState, RadrootsAccountCustody, RadrootsPendingRemoteSignerConnection,
    RadrootsRemoteSignerPreview, SetupActionState,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerSessionStoreState,
    radroots_studio_app_remote_signer_connect_pending, radroots_studio_app_remote_signer_poll_pending_session,
    radroots_studio_app_remote_signer_preview,
};
use radroots_identity::RadrootsIdentityId;
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrSecretVault, RadrootsNostrSelectedAccountStatus,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const REMOTE_SIGNER_LABEL: &str = "remote signer";

#[derive(Clone, Default)]
pub(crate) struct AndroidRemoteSigner {
    update: Arc<Mutex<Option<Result<Option<IdentityGateState>, String>>>>,
    changed: Arc<AtomicBool>,
    connecting: Arc<AtomicBool>,
    polling: Arc<AtomicBool>,
}

impl AndroidRemoteSigner {
    pub(crate) fn new() -> Self {
        let tracker = Self::default();
        if let Err(error) = tracker.resume_pending() {
            tracker.push_update(Err(error));
        }
        tracker
    }

    pub(crate) fn take_update(&self) -> Option<Result<Option<IdentityGateState>, String>> {
        if !self.changed.swap(false, Ordering::AcqRel) {
            return None;
        }

        self.update.lock().ok().and_then(|mut slot| slot.take())
    }

    pub(crate) fn is_connecting(&self) -> bool {
        self.connecting.load(Ordering::Acquire)
    }

    pub(crate) fn action_state(&self) -> Result<SetupActionState, String> {
        if self.is_connecting() {
            return Ok(SetupActionState {
                label: "Connecting Remote Signer...".to_owned(),
                enabled: false,
                pending: true,
            });
        }

        if pending_connection()?.is_some() {
            return Ok(SetupActionState {
                label: "Remote Signer Waiting for Approval".to_owned(),
                enabled: false,
                pending: false,
            });
        }

        Ok(SetupActionState {
            label: "Connect Remote Signer".to_owned(),
            enabled: true,
            pending: false,
        })
    }

    pub(crate) fn begin_connect(&self, input: &str) -> Result<(), String> {
        if self.connecting.swap(true, Ordering::AcqRel) {
            return Err("remote signer connection is already starting".to_owned());
        }

        if pending_connection()?.is_some() {
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
                store_client_secret(
                    client_account_id.as_str(),
                    pending.client_secret_key_hex.as_str(),
                )?;
                let store_path = sessions_path()?;
                let mut state = load_sessions(store_path.as_path())?;
                state
                    .upsert_pending(pending.record.clone())
                    .map_err(|error| error.to_string())?;
                save_sessions(store_path.as_path(), &state)?;
                tracker.start_polling(
                    pending.record.client_account_id().to_owned(),
                    pending.client_secret_key_hex,
                );
                Ok(())
            })();

            if let Err(error) = outcome {
                tracker.push_update(Err(error));
            }
            tracker.connecting.store(false, Ordering::Release);
        });

        Ok(())
    }

    pub(crate) fn resume_pending(&self) -> Result<(), String> {
        let Some(record) = pending_session_record()? else {
            return Ok(());
        };
        let client_secret_key_hex = load_client_secret(record.client_account_id())?;
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
                let pending_record = match pending_session_record() {
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
                        let ready_state = match activate_remote_session(
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
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message }) => {
                        let _ = remove_pending_session();
                        let _ = remove_client_secret(client_account_id.as_str());
                        tracker.push_update(Err(message));
                        tracker.polling.store(false, Ordering::Release);
                        return;
                    }
                    Ok(RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message }) => {
                        let _ = remove_pending_session();
                        let _ = remove_client_secret(client_account_id.as_str());
                        tracker.push_update(Err(message));
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

    fn push_update(&self, result: Result<Option<IdentityGateState>, String>) {
        if let Ok(mut slot) = self.update.lock() {
            *slot = Some(result);
            self.changed.store(true, Ordering::Release);
        }
    }
}

pub(crate) fn preview_connection(input: &str) -> Result<RadrootsRemoteSignerPreview, String> {
    let preview = radroots_studio_app_remote_signer_preview(input).map_err(|error| error.to_string())?;
    Ok(RadrootsRemoteSignerPreview {
        source_label: preview.source_label().to_owned(),
        signer_npub: preview.signer_identity.public_key_npub,
        relays: preview.relays,
        requested_permissions: Vec::new(),
    })
}

pub(crate) fn pending_connection() -> Result<Option<RadrootsPendingRemoteSignerConnection>, String>
{
    Ok(
        pending_session_record()?.map(|record| RadrootsPendingRemoteSignerConnection {
            signer_npub: record.signer_identity.public_key_npub,
            relays: record.relays,
        }),
    )
}

pub(crate) fn identity_state_from_status(
    status: RadrootsNostrSelectedAccountStatus,
) -> Result<IdentityGateState, String> {
    match status {
        RadrootsNostrSelectedAccountStatus::NotConfigured => Ok(IdentityGateState::Missing),
        RadrootsNostrSelectedAccountStatus::Ready { account } => Ok(IdentityGateState::Ready {
            account_id: account.account_id.to_string(),
        }),
        RadrootsNostrSelectedAccountStatus::PublicOnly { account } => {
            if active_session_for_account_id(account.account_id.as_str())?.is_some() {
                Ok(IdentityGateState::Ready {
                    account_id: account.account_id.to_string(),
                })
            } else {
                Ok(IdentityGateState::Missing)
            }
        }
    }
}

pub(crate) fn custody_for_account_id(account_id: &str) -> Result<RadrootsAccountCustody, String> {
    if active_session_for_account_id(account_id)?.is_some() {
        Ok(RadrootsAccountCustody::RemoteSigner)
    } else {
        Ok(RadrootsAccountCustody::LocalManaged)
    }
}

pub(crate) fn disconnect_selected_remote_signer(
    manager: &RadrootsNostrAccountsManager,
) -> Result<IdentityGateState, String> {
    let Some(account_id) = manager
        .selected_account_id()
        .map_err(|source| source.to_string())?
    else {
        return Ok(IdentityGateState::Missing);
    };
    let Some(session) = remove_active_session(account_id.as_str())? else {
        return Ok(IdentityGateState::Missing);
    };
    remove_client_secret(session.client_account_id())?;
    manager
        .remove_account(&account_id)
        .map_err(|source| source.to_string())?;
    let status = manager
        .selected_account_status()
        .map_err(|source| source.to_string())?;
    identity_state_from_status(status)
}

pub(crate) fn cancel_pending_connection() -> Result<(), String> {
    if let Some(session) = remove_pending_session()? {
        remove_client_secret(session.client_account_id())?;
    }
    Ok(())
}

fn activate_remote_session(
    client_account_id: &str,
    user_identity: radroots_identity::RadrootsIdentityPublic,
) -> Result<IdentityGateState, String> {
    let manager = crate::storage::accounts_manager()?;
    manager
        .upsert_public_identity(
            user_identity.clone(),
            Some(REMOTE_SIGNER_LABEL.to_owned()),
            true,
        )
        .map_err(|source| source.to_string())?;
    let store_path = sessions_path()?;
    let mut state = load_sessions(store_path.as_path())?;
    state
        .activate_session(client_account_id, user_identity.clone())
        .ok_or_else(|| "pending remote signer session disappeared before activation".to_owned())?;
    save_sessions(store_path.as_path(), &state)?;
    Ok(IdentityGateState::Ready {
        account_id: user_identity.id.to_string(),
    })
}

fn pending_session_record()
-> Result<Option<radroots_studio_app_remote_signer::RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let state = load_sessions(store_path.as_path())?;
    Ok(state.pending_session().cloned())
}

fn active_session_for_account_id(
    account_id: &str,
) -> Result<Option<radroots_studio_app_remote_signer::RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let state = load_sessions(store_path.as_path())?;
    Ok(state.active_session_for_account_id(account_id).cloned())
}

fn remove_pending_session()
-> Result<Option<radroots_studio_app_remote_signer::RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let mut state = load_sessions(store_path.as_path())?;
    let removed = state.remove_pending_session();
    save_sessions(store_path.as_path(), &state)?;
    Ok(removed)
}

fn remove_active_session(
    account_id: &str,
) -> Result<Option<radroots_studio_app_remote_signer::RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let mut state = load_sessions(store_path.as_path())?;
    let removed = state.remove_active_session_for_account_id(account_id);
    save_sessions(store_path.as_path(), &state)?;
    Ok(removed)
}

fn load_sessions(path: &Path) -> Result<RadrootsAppRemoteSignerSessionStoreState, String> {
    RadrootsAppRemoteSignerSessionStoreState::load(path).map_err(|error| error.to_string())
}

fn save_sessions(
    path: &Path,
    state: &RadrootsAppRemoteSignerSessionStoreState,
) -> Result<(), String> {
    state.save(path).map_err(|error| error.to_string())
}

fn sessions_path() -> Result<PathBuf, String> {
    let root = resolve_nostr_storage_root().map_err(|source| source.to_string())?;
    Ok(root.join("remote-signer-sessions.json"))
}

fn client_secret_vault() -> RadrootsAndroidKeystoreVault {
    RadrootsAndroidKeystoreVault::new(ANDROID_NOSTR_SERVICE)
}

fn store_client_secret(client_account_id: &str, secret_key_hex: &str) -> Result<(), String> {
    let account_id = RadrootsIdentityId::try_from(client_account_id)
        .map_err(|_| "invalid remote signer client account id".to_owned())?;
    client_secret_vault()
        .store_secret_hex(&account_id, secret_key_hex)
        .map_err(|source| source.to_string())
}

fn load_client_secret(client_account_id: &str) -> Result<String, String> {
    let account_id = RadrootsIdentityId::try_from(client_account_id)
        .map_err(|_| "invalid remote signer client account id".to_owned())?;
    client_secret_vault()
        .load_secret_hex(&account_id)
        .map_err(|source| source.to_string())?
        .ok_or_else(|| "remote signer session secret is missing".to_owned())
}

fn remove_client_secret(client_account_id: &str) -> Result<(), String> {
    let account_id = RadrootsIdentityId::try_from(client_account_id)
        .map_err(|_| "invalid remote signer client account id".to_owned())?;
    client_secret_vault()
        .remove_secret(&account_id)
        .map_err(|source| source.to_string())
}
