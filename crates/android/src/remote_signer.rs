use crate::security::{ANDROID_NOSTR_SERVICE, resolve_nostr_storage_root};
use crate::vault::RadrootsAndroidKeystoreVault;
use radroots_studio_app_core::{
    IdentityGateState, RadrootsAccountCustody, RadrootsPendingRemoteSignerConnection,
    RadrootsRemoteSignerPreview, SetupActionState,
};
use radroots_studio_app_remote_signer::{
    RADROOTS_APP_REMOTE_SIGNER_SECRET_NAMESPACE, RadrootsAppRemoteSignerController,
    RadrootsAppRemoteSignerControllerHooks, RadrootsAppRemoteSignerPendingSession,
    RadrootsAppRemoteSignerSessionRecord, RadrootsAppRemoteSignerSessionStoreState,
    radroots_studio_app_remote_signer_clear_pending_session,
    radroots_studio_app_remote_signer_disconnect_selected, radroots_studio_app_remote_signer_preview,
    radroots_studio_app_remote_signer_purge_all_custody_state,
    radroots_studio_app_remote_signer_reconcile_startup,
};
use radroots_identity::RadrootsIdentityId;
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrSecretVault, RadrootsNostrSelectedAccountStatus,
};
use std::path::{Path, PathBuf};

const REMOTE_SIGNER_LABEL: &str = "remote signer";

#[derive(Clone, Copy)]
struct AndroidRemoteSignerHooks;

impl RadrootsAppRemoteSignerControllerHooks for AndroidRemoteSignerHooks {
    type ReadyState = IdentityGateState;

    fn reconcile_startup_state(&self) -> Result<(), String> {
        let manager = crate::storage::accounts_manager()?;
        let store_path = sessions_path()?;
        radroots_studio_app_remote_signer_reconcile_startup(
            &manager,
            store_path.as_path(),
            REMOTE_SIGNER_LABEL,
            load_client_secret,
            remove_client_secret,
            purge_client_secret_namespace,
        )
    }

    fn store_pending_session(
        &self,
        pending: &RadrootsAppRemoteSignerPendingSession,
    ) -> Result<(), String> {
        let client_account_id = pending.record.client_account_id().to_owned();
        store_client_secret(
            client_account_id.as_str(),
            pending.client_secret_key_hex.as_str(),
        )?;
        let store_path = sessions_path()?;
        let mut state = load_sessions(store_path.as_path())?;
        if let Err(error) = state.upsert_pending(pending.record.clone()) {
            let _ = remove_client_secret(client_account_id.as_str());
            return Err(error.to_string());
        }
        if let Err(error) = save_sessions(store_path.as_path(), &state) {
            let _ = remove_client_secret(client_account_id.as_str());
            return Err(error);
        }
        Ok(())
    }

    fn pending_session_record(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
        pending_session_record()
    }

    fn load_pending_client_secret(&self, client_account_id: &str) -> Result<String, String> {
        load_client_secret(client_account_id)
    }

    fn activate_pending_session(
        &self,
        client_account_id: &str,
        user_identity: radroots_identity::RadrootsIdentityPublic,
    ) -> Result<Self::ReadyState, String> {
        activate_remote_session(client_account_id, user_identity)
    }

    fn clear_pending_session(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
        let store_path = sessions_path()?;
        radroots_studio_app_remote_signer_clear_pending_session(store_path.as_path(), remove_client_secret)
    }
}

#[derive(Clone)]
pub(crate) struct AndroidRemoteSigner {
    controller: RadrootsAppRemoteSignerController<AndroidRemoteSignerHooks>,
}

impl AndroidRemoteSigner {
    pub(crate) fn new() -> Self {
        Self {
            controller: RadrootsAppRemoteSignerController::new(AndroidRemoteSignerHooks),
        }
    }

    pub(crate) fn take_update(&self) -> Option<Result<Option<IdentityGateState>, String>> {
        self.controller.take_update()
    }

    pub(crate) fn is_connecting(&self) -> bool {
        self.controller.is_connecting()
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
        self.controller.begin_connect(input)
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
    Ok(AndroidRemoteSignerHooks
        .pending_session_record()?
        .map(|record| RadrootsPendingRemoteSignerConnection {
            signer_npub: record.signer_identity.public_key_npub,
            relays: record.relays,
        }))
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
    let store_path = sessions_path()?;
    let status = radroots_studio_app_remote_signer_disconnect_selected(
        manager,
        store_path.as_path(),
        remove_client_secret,
    )?;
    identity_state_from_status(status)
}

pub(crate) fn cancel_pending_connection() -> Result<(), String> {
    let store_path = sessions_path()?;
    let _ = radroots_studio_app_remote_signer_clear_pending_session(
        store_path.as_path(),
        remove_client_secret,
    )?;
    Ok(())
}

pub(crate) fn purge_all_custody_state() -> Result<(), String> {
    let store_path = sessions_path()?;
    radroots_studio_app_remote_signer_purge_all_custody_state(
        store_path.as_path(),
        remove_client_secret,
        purge_client_secret_namespace,
    )
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
    let activation_result = (|| -> Result<(), String> {
        let mut state = load_sessions(store_path.as_path())?;
        state
            .activate_session(client_account_id, user_identity.clone())
            .ok_or_else(|| {
                "pending remote signer session disappeared before activation".to_owned()
            })?;
        save_sessions(store_path.as_path(), &state)
    })();
    if let Err(error) = activation_result {
        if let Err(rollback_error) = manager.remove_account(&user_identity.id) {
            return Err(format!(
                "{error}. remote signer account rollback needs retry: {rollback_error}"
            ));
        }
        return Err(error);
    }
    Ok(IdentityGateState::Ready {
        account_id: user_identity.id.to_string(),
    })
}

fn pending_session_record() -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let state = load_sessions(store_path.as_path())?;
    Ok(state.pending_session().cloned())
}

fn active_session_for_account_id(
    account_id: &str,
) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
    let store_path = sessions_path()?;
    let state = load_sessions(store_path.as_path())?;
    Ok(state.active_session_for_account_id(account_id).cloned())
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
    RadrootsAndroidKeystoreVault::new_with_namespace(
        ANDROID_NOSTR_SERVICE,
        RADROOTS_APP_REMOTE_SIGNER_SECRET_NAMESPACE,
    )
}

fn legacy_client_secret_vault() -> RadrootsAndroidKeystoreVault {
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
    if let Some(secret) = client_secret_vault()
        .load_secret_hex(&account_id)
        .map_err(|source| source.to_string())?
    {
        return Ok(secret);
    }

    let secret = legacy_client_secret_vault()
        .load_secret_hex(&account_id)
        .map_err(|source| source.to_string())?
        .ok_or_else(|| "remote signer session secret is missing".to_owned())?;
    let _ = client_secret_vault().store_secret_hex(&account_id, secret.as_str());
    let _ = legacy_client_secret_vault().remove_secret(&account_id);
    Ok(secret)
}

fn remove_client_secret(client_account_id: &str) -> Result<(), String> {
    let account_id = RadrootsIdentityId::try_from(client_account_id)
        .map_err(|_| "invalid remote signer client account id".to_owned())?;
    client_secret_vault()
        .remove_secret(&account_id)
        .map_err(|source| source.to_string())?;
    legacy_client_secret_vault()
        .remove_secret(&account_id)
        .map_err(|source| source.to_string())
}

fn purge_client_secret_namespace() -> Result<(), String> {
    client_secret_vault()
        .purge_namespace()
        .map_err(|source| source.to_string())
}
