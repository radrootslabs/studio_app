use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use radroots_studio_app_core::AppDesktopRuntimePaths;
use radroots_studio_app_models::{AccountCustody, AppIdentityProjection};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerError,
    RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerSessionRecord,
    RadrootsAppRemoteSignerSessionStatus, RadrootsAppRemoteSignerSessionStoreLoadResult,
    RadrootsAppRemoteSignerSessionStoreState,
};
use radroots_identity::{IdentityError, RadrootsIdentityId};
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountRecord, RadrootsNostrAccountsError, RadrootsNostrAccountsManager,
    account_secret_slot,
};
use radroots_protected_store::RadrootsProtectedFileSecretVault;
use radroots_secret_vault::{RadrootsSecretVault, RadrootsSecretVaultAccessError};
use thiserror::Error;

const REMOTE_SIGNER_LABEL: &str = "remote signer";
const REMOTE_SIGNER_SESSIONS_FILE_NAME: &str = "remote-signer-sessions.json";
const REMOTE_SIGNER_SESSIONS_DIR_NAME: &str = "nostr";
const REMOTE_SIGNER_CLIENT_SECRET_DIR_NAME: &str = "remote_signer";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DesktopRemoteSignerPaths {
    pub(crate) sessions_path: PathBuf,
    pub(crate) client_secret_root: PathBuf,
}

impl DesktopRemoteSignerPaths {
    pub(crate) fn from_runtime_paths(paths: &AppDesktopRuntimePaths) -> Self {
        Self {
            sessions_path: paths
                .app
                .data
                .join(REMOTE_SIGNER_SESSIONS_DIR_NAME)
                .join(REMOTE_SIGNER_SESSIONS_FILE_NAME),
            client_secret_root: paths
                .shared_accounts
                .secrets_root
                .join(REMOTE_SIGNER_CLIENT_SECRET_DIR_NAME),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum DesktopRemoteSignerError {
    #[error(transparent)]
    Accounts(#[from] RadrootsNostrAccountsError),
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    SessionStore(#[from] RadrootsAppRemoteSignerError),
    #[error(transparent)]
    SecretVault(#[from] RadrootsSecretVaultAccessError),
    #[error("{0}")]
    State(String),
}

pub(crate) fn reconcile_startup(
    manager: &RadrootsNostrAccountsManager,
    paths: &DesktopRemoteSignerPaths,
) -> Result<(), DesktopRemoteSignerError> {
    let load = load_sessions_with_recovery(paths)?;
    let mut state = load.state;
    let mut dirty = false;
    let accounts = manager.list_accounts()?;
    let account_ids = accounts
        .iter()
        .map(|record| record.account_id.to_string())
        .collect::<HashSet<_>>();
    let active_session_account_ids = state
        .sessions
        .iter()
        .filter(|record| record.status == RadrootsAppRemoteSignerSessionStatus::Active)
        .filter_map(|record| record.account_id().map(ToOwned::to_owned))
        .collect::<HashSet<_>>();

    if load.recovered_from_corruption || state.sessions.is_empty() {
        purge_client_secret_namespace(paths)?;
    }

    for account in remote_signer_public_only_accounts(manager, &accounts)?
        .into_iter()
        .filter(|account| !active_session_account_ids.contains(account.account_id.as_str()))
    {
        manager.remove_account(&account.account_id)?;
    }

    if let Some(record) = state.pending_session().cloned()
        && load_client_secret(paths, record.client_account_id()).is_err()
    {
        state.remove_pending_session();
        dirty = true;
    }

    let stale_active_sessions = state
        .sessions
        .iter()
        .filter(|record| record.status == RadrootsAppRemoteSignerSessionStatus::Active)
        .filter_map(|record| {
            let account_id = record.account_id()?;
            (!account_ids.contains(account_id)).then_some(record.clone())
        })
        .collect::<Vec<_>>();

    for session in stale_active_sessions {
        remove_client_secret(paths, session.client_account_id())?;
        let Some(account_id) = session.account_id() else {
            continue;
        };
        state.remove_active_session_for_account_id(account_id);
        dirty = true;
    }

    if dirty || load.recovered_from_corruption {
        save_sessions(paths, &state)?;
    }

    Ok(())
}

pub(crate) fn store_pending_session(
    paths: &DesktopRemoteSignerPaths,
    pending: &RadrootsAppRemoteSignerPendingSession,
) -> Result<(), DesktopRemoteSignerError> {
    let client_account_id = pending.record.client_account_id().to_owned();
    store_client_secret(
        paths,
        client_account_id.as_str(),
        pending.client_secret_key_hex.as_str(),
    )?;

    let mut state = load_sessions(paths)?;
    if let Err(error) = state.upsert_pending(pending.record.clone()) {
        let _ = remove_client_secret(paths, client_account_id.as_str());
        return Err(error.into());
    }
    if let Err(error) = save_sessions(paths, &state) {
        let _ = remove_client_secret(paths, client_account_id.as_str());
        return Err(error);
    }

    Ok(())
}

pub(crate) fn load_pending_session(
    paths: &DesktopRemoteSignerPaths,
) -> Result<Option<RadrootsAppRemoteSignerPendingSession>, DesktopRemoteSignerError> {
    let state = load_sessions(paths)?;
    let Some(record) = state.pending_session().cloned() else {
        return Ok(None);
    };
    let client_secret_key_hex = load_client_secret(paths, record.client_account_id())?;
    Ok(Some(RadrootsAppRemoteSignerPendingSession {
        record,
        client_secret_key_hex,
    }))
}

pub(crate) fn clear_pending_session(
    paths: &DesktopRemoteSignerPaths,
) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, DesktopRemoteSignerError> {
    let state = load_sessions(paths)?;
    let Some(record) = state.pending_session().cloned() else {
        return Ok(None);
    };
    let mut next_state = state.clone();
    let removed = next_state.remove_pending_session();
    if removed.is_none() {
        return Err(DesktopRemoteSignerError::State(
            "remote signer pending session record cleanup could not complete".to_owned(),
        ));
    }
    save_sessions(paths, &next_state)?;

    if let Err(error) = remove_client_secret(paths, record.client_account_id()) {
        return Err(DesktopRemoteSignerError::State(format!(
            "remote signer pending session record was removed but session secret cleanup needs retry: {error}"
        )));
    }

    Ok(removed)
}

pub(crate) fn activate_pending_session(
    manager: &RadrootsNostrAccountsManager,
    paths: &DesktopRemoteSignerPaths,
    client_account_id: &str,
    approved: &RadrootsAppRemoteSignerApprovedSession,
) -> Result<(), DesktopRemoteSignerError> {
    manager.upsert_public_identity(
        approved.user_identity.clone(),
        Some(REMOTE_SIGNER_LABEL.to_owned()),
        true,
    )?;

    let activation_result = (|| -> Result<(), DesktopRemoteSignerError> {
        let mut state = load_sessions(paths)?;
        state
            .activate_session(
                client_account_id,
                approved.user_identity.clone(),
                approved.relays.clone(),
                approved.approved_permissions.clone(),
            )
            .ok_or_else(|| {
                DesktopRemoteSignerError::State(
                    "pending remote signer session disappeared before activation".to_owned(),
                )
            })?;
        save_sessions(paths, &state)
    })();

    if let Err(error) = activation_result {
        if let Err(rollback_error) = manager.remove_account(&approved.user_identity.id) {
            return Err(DesktopRemoteSignerError::State(format!(
                "{error}. remote signer account rollback needs retry: {rollback_error}"
            )));
        }
        return Err(error);
    }

    Ok(())
}

pub(crate) fn purge_all_state(
    paths: &DesktopRemoteSignerPaths,
) -> Result<(), DesktopRemoteSignerError> {
    let load = load_sessions_with_recovery(paths)?;
    for record in &load.state.sessions {
        remove_client_secret(paths, record.client_account_id())?;
    }
    purge_client_secret_namespace(paths)?;
    remove_sessions_file_if_present(paths.sessions_path.as_path())?;
    Ok(())
}

pub(crate) fn apply_remote_signer_custody(
    projection: AppIdentityProjection,
    paths: &DesktopRemoteSignerPaths,
) -> Result<AppIdentityProjection, DesktopRemoteSignerError> {
    let active_account_ids = active_remote_signer_account_ids(paths)?;
    if active_account_ids.is_empty() {
        return Ok(projection);
    }

    let mut projection = projection;
    for account in &mut projection.roster {
        if active_account_ids.contains(account.account_id.as_str()) {
            account.custody = AccountCustody::RemoteSigner;
        }
    }
    if let Some(selected_account) = projection.selected_account.as_mut()
        && active_account_ids.contains(selected_account.account.account_id.as_str())
    {
        selected_account.account.custody = AccountCustody::RemoteSigner;
    }

    Ok(projection)
}

fn active_remote_signer_account_ids(
    paths: &DesktopRemoteSignerPaths,
) -> Result<HashSet<String>, DesktopRemoteSignerError> {
    Ok(load_sessions(paths)?
        .sessions
        .into_iter()
        .filter(|record| record.status == RadrootsAppRemoteSignerSessionStatus::Active)
        .filter_map(|record| record.account_id().map(ToOwned::to_owned))
        .collect())
}

fn remote_signer_public_only_accounts(
    manager: &RadrootsNostrAccountsManager,
    accounts: &[RadrootsNostrAccountRecord],
) -> Result<Vec<RadrootsNostrAccountRecord>, DesktopRemoteSignerError> {
    let mut stale = Vec::new();
    for account in accounts {
        if account.label.as_deref() != Some(REMOTE_SIGNER_LABEL) {
            continue;
        }
        if manager.get_signing_identity(&account.account_id)?.is_none() {
            stale.push(account.clone());
        }
    }
    Ok(stale)
}

fn client_secret_vault(paths: &DesktopRemoteSignerPaths) -> RadrootsProtectedFileSecretVault {
    RadrootsProtectedFileSecretVault::new(paths.client_secret_root.as_path())
}

fn client_secret_slot(client_account_id: &str) -> Result<String, DesktopRemoteSignerError> {
    let account_id = RadrootsIdentityId::parse(client_account_id)?;
    Ok(account_secret_slot(&account_id))
}

fn store_client_secret(
    paths: &DesktopRemoteSignerPaths,
    client_account_id: &str,
    secret_key_hex: &str,
) -> Result<(), DesktopRemoteSignerError> {
    let slot = client_secret_slot(client_account_id)?;
    client_secret_vault(paths).store_secret(slot.as_str(), secret_key_hex)?;
    Ok(())
}

fn load_client_secret(
    paths: &DesktopRemoteSignerPaths,
    client_account_id: &str,
) -> Result<String, DesktopRemoteSignerError> {
    let slot = client_secret_slot(client_account_id)?;
    client_secret_vault(paths)
        .load_secret(slot.as_str())?
        .ok_or_else(|| {
            DesktopRemoteSignerError::State("remote signer session secret is missing".to_owned())
        })
}

fn remove_client_secret(
    paths: &DesktopRemoteSignerPaths,
    client_account_id: &str,
) -> Result<(), DesktopRemoteSignerError> {
    let slot = client_secret_slot(client_account_id)?;
    client_secret_vault(paths).remove_secret(slot.as_str())?;
    Ok(())
}

fn purge_client_secret_namespace(
    paths: &DesktopRemoteSignerPaths,
) -> Result<(), DesktopRemoteSignerError> {
    match fs::remove_dir_all(paths.client_secret_root.as_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DesktopRemoteSignerError::State(format!(
            "failed to purge remote signer client secret namespace: {error}"
        ))),
    }
}

fn load_sessions(
    paths: &DesktopRemoteSignerPaths,
) -> Result<RadrootsAppRemoteSignerSessionStoreState, DesktopRemoteSignerError> {
    Ok(RadrootsAppRemoteSignerSessionStoreState::load(
        paths.sessions_path.as_path(),
    )?)
}

fn load_sessions_with_recovery(
    paths: &DesktopRemoteSignerPaths,
) -> Result<RadrootsAppRemoteSignerSessionStoreLoadResult, DesktopRemoteSignerError> {
    Ok(
        RadrootsAppRemoteSignerSessionStoreState::load_with_recovery(
            paths.sessions_path.as_path(),
        )?,
    )
}

fn save_sessions(
    paths: &DesktopRemoteSignerPaths,
    state: &RadrootsAppRemoteSignerSessionStoreState,
) -> Result<(), DesktopRemoteSignerError> {
    Ok(state.save(paths.sessions_path.as_path())?)
}

fn remove_sessions_file_if_present(path: &Path) -> Result<(), DesktopRemoteSignerError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DesktopRemoteSignerError::State(format!(
            "failed to remove remote signer session store: {error}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
        RadrootsAppRemoteSignerSessionRecord, radroots_studio_app_remote_signer_requested_permissions,
    };
    use radroots_identity::{RadrootsIdentity, RadrootsIdentityPublic};
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountStatus, RadrootsNostrAccountsManager,
    };

    use super::{
        DesktopRemoteSignerPaths, activate_pending_session, apply_remote_signer_custody,
        clear_pending_session, load_pending_session, purge_all_state, reconcile_startup,
        store_pending_session,
    };

    const CLIENT_SECRET_KEY_HEX: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const SIGNER_SECRET_KEY_HEX: &str =
        "2222222222222222222222222222222222222222222222222222222222222222";
    const USER_SECRET_KEY_HEX: &str =
        "3333333333333333333333333333333333333333333333333333333333333333";

    fn public_identity(secret_key_hex: &str) -> RadrootsIdentityPublic {
        RadrootsIdentity::from_secret_key_str(secret_key_hex)
            .expect("identity")
            .to_public()
    }

    fn temp_paths(label: &str) -> DesktopRemoteSignerPaths {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = env::temp_dir()
            .join("radroots-app-desktop-remote-signer")
            .join(format!("{label}-{unique}"));
        DesktopRemoteSignerPaths {
            sessions_path: root.join("data").join("remote-signer-sessions.json"),
            client_secret_root: root.join("secrets").join("remote_signer"),
        }
    }

    fn pending_session() -> RadrootsAppRemoteSignerPendingSession {
        RadrootsAppRemoteSignerPendingSession {
            record: RadrootsAppRemoteSignerSessionRecord::pending(
                public_identity(CLIENT_SECRET_KEY_HEX),
                public_identity(SIGNER_SECRET_KEY_HEX),
                vec!["ws://127.0.0.1:8080".to_owned()],
            ),
            client_secret_key_hex: CLIENT_SECRET_KEY_HEX.to_owned(),
        }
    }

    #[test]
    fn pending_session_round_trips_with_client_secret() {
        let paths = temp_paths("pending");
        let pending = pending_session();

        store_pending_session(&paths, &pending).expect("store pending");
        let restored = load_pending_session(&paths)
            .expect("load pending")
            .expect("pending session");

        assert_eq!(
            restored.record.client_account_id(),
            pending.record.client_account_id()
        );
        assert_eq!(
            restored.record.signer_identity.id,
            pending.record.signer_identity.id
        );
        assert_eq!(restored.record.relays, pending.record.relays);
        assert_eq!(restored.record.status, pending.record.status);
        assert_eq!(
            restored.client_secret_key_hex,
            pending.client_secret_key_hex
        );

        clear_pending_session(&paths).expect("clear pending");
        assert!(
            load_pending_session(&paths)
                .expect("load after clear")
                .is_none()
        );
    }

    #[test]
    fn activating_pending_session_upserts_selected_remote_signer_account() {
        let paths = temp_paths("activate");
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let pending = pending_session();
        let approved = RadrootsAppRemoteSignerApprovedSession {
            user_identity: public_identity(USER_SECRET_KEY_HEX),
            relays: vec!["ws://127.0.0.1:8080".to_owned()],
            approved_permissions: radroots_studio_app_remote_signer_requested_permissions(),
        };

        store_pending_session(&paths, &pending).expect("store pending");
        activate_pending_session(
            &manager,
            &paths,
            pending.record.client_account_id(),
            &approved,
        )
        .expect("activate pending");

        let selected = match manager
            .default_account_status()
            .expect("selected account status")
        {
            RadrootsNostrAccountStatus::NotConfigured => panic!("configured account"),
            RadrootsNostrAccountStatus::PublicOnly { account }
            | RadrootsNostrAccountStatus::Ready { account } => account,
        };
        assert_eq!(
            selected.account_id.as_str(),
            approved.user_identity.id.as_str()
        );
        assert_eq!(selected.label.as_deref(), Some("remote signer"));

        let projection = apply_remote_signer_custody(
            radroots_studio_app_models::AppIdentityProjection::ready(
                vec![radroots_studio_app_models::AccountSummary {
                    account_id: approved.user_identity.id.to_string(),
                    npub: approved.user_identity.public_key_npub.clone(),
                    label: Some("remote signer".to_owned()),
                    custody: radroots_studio_app_models::AccountCustody::LocalManaged,
                }],
                radroots_studio_app_models::SelectedAccountProjection::new(
                    radroots_studio_app_models::AccountSummary {
                        account_id: approved.user_identity.id.to_string(),
                        npub: approved.user_identity.public_key_npub.clone(),
                        label: Some("remote signer".to_owned()),
                        custody: radroots_studio_app_models::AccountCustody::LocalManaged,
                    },
                    radroots_studio_app_models::SelectedSurfaceProjection::default(),
                    radroots_studio_app_models::FarmerActivationProjection::inactive(),
                ),
            ),
            &paths,
        )
        .expect("decorate projection");
        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .expect("selected")
                .account
                .custody,
            radroots_studio_app_models::AccountCustody::RemoteSigner
        );
    }

    #[test]
    fn reconcile_startup_removes_orphan_remote_signer_account_and_pending_without_secret() {
        let paths = temp_paths("reconcile");
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let pending = pending_session();
        store_pending_session(&paths, &pending).expect("store pending");
        clear_pending_session(&paths).expect("clear pending");
        store_pending_session(&paths, &pending).expect("store pending again");
        manager
            .upsert_public_identity(
                public_identity(USER_SECRET_KEY_HEX),
                Some("remote signer".to_owned()),
                true,
            )
            .expect("upsert remote signer account");

        purge_all_state(&paths).expect("purge all");
        reconcile_startup(&manager, &paths).expect("reconcile startup");

        assert!(manager.list_accounts().expect("accounts").is_empty());
        assert!(
            load_pending_session(&paths)
                .expect("load pending")
                .is_none()
        );
    }
}
