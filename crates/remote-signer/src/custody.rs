use crate::session::{
    RadrootsAppRemoteSignerSessionRecord, RadrootsAppRemoteSignerSessionStatus,
    RadrootsAppRemoteSignerSessionStoreLoadResult, RadrootsAppRemoteSignerSessionStoreState,
};
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountRecord, RadrootsNostrAccountsManager, RadrootsNostrSelectedAccountStatus,
};
use std::collections::HashSet;
use std::path::Path;

pub fn radroots_studio_app_remote_signer_clear_pending_session(
    path: &Path,
    remove_client_secret: impl Fn(&str) -> Result<(), String>,
) -> Result<Option<RadrootsAppRemoteSignerSessionRecord>, String> {
    let mut state = load_sessions(path)?;
    let Some(record) = state.pending_session().cloned() else {
        return Ok(None);
    };
    remove_client_secret(record.client_account_id())?;
    let removed = state.remove_pending_session();
    save_sessions(path, &state)?;
    Ok(removed)
}

pub fn radroots_studio_app_remote_signer_disconnect_selected(
    manager: &RadrootsNostrAccountsManager,
    path: &Path,
    remove_client_secret: impl Fn(&str) -> Result<(), String>,
) -> Result<RadrootsNostrSelectedAccountStatus, String> {
    let Some(account_id) = manager
        .selected_account_id()
        .map_err(|source| source.to_string())?
    else {
        return Ok(RadrootsNostrSelectedAccountStatus::NotConfigured);
    };

    let state = load_sessions(path)?;
    let Some(session) = state
        .active_session_for_account_id(account_id.as_str())
        .cloned()
    else {
        return Ok(RadrootsNostrSelectedAccountStatus::NotConfigured);
    };

    manager
        .remove_account(&account_id)
        .map_err(|source| source.to_string())?;

    if let Err(error) = remove_client_secret(session.client_account_id()) {
        return Err(format!(
            "remote signer account was removed but session secret cleanup failed: {error}"
        ));
    }

    let mut state = load_sessions(path)?;
    let removed = state.remove_active_session_for_account_id(account_id.as_str());
    if removed.is_none() {
        return Err(
            "remote signer account was removed but session record cleanup could not complete"
                .to_owned(),
        );
    }
    save_sessions(path, &state)?;
    manager
        .selected_account_status()
        .map_err(|source| source.to_string())
}

pub fn radroots_studio_app_remote_signer_reconcile_startup(
    manager: &RadrootsNostrAccountsManager,
    path: &Path,
    remote_signer_label: &str,
    load_client_secret: impl Fn(&str) -> Result<String, String>,
    remove_client_secret: impl Fn(&str) -> Result<(), String>,
) -> Result<(), String> {
    let load = load_sessions_with_recovery(path)?;
    let mut state = load.state;
    let mut dirty = false;
    let accounts = manager
        .list_accounts()
        .map_err(|source| source.to_string())?;
    let account_ids = accounts
        .iter()
        .map(|record| record.account_id.to_string())
        .collect::<HashSet<_>>();

    if load.recovered_from_corruption {
        for account in remote_signer_public_only_accounts(manager, &accounts, remote_signer_label)?
        {
            manager
                .remove_account(&account.account_id)
                .map_err(|source| source.to_string())?;
        }
    }

    if let Some(record) = state.pending_session().cloned() {
        if load_client_secret(record.client_account_id()).is_err() {
            state.remove_pending_session();
            dirty = true;
        }
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
        remove_client_secret(session.client_account_id())?;
        let Some(account_id) = session.account_id() else {
            continue;
        };
        state.remove_active_session_for_account_id(account_id);
        dirty = true;
    }

    if dirty || load.recovered_from_corruption {
        save_sessions(path, &state)?;
    }

    Ok(())
}

fn remote_signer_public_only_accounts(
    manager: &RadrootsNostrAccountsManager,
    accounts: &[RadrootsNostrAccountRecord],
    remote_signer_label: &str,
) -> Result<Vec<RadrootsNostrAccountRecord>, String> {
    let mut stale = Vec::new();
    for account in accounts {
        if account.label.as_deref() != Some(remote_signer_label) {
            continue;
        }
        if manager
            .get_signing_identity(&account.account_id)
            .map_err(|source| source.to_string())?
            .is_none()
        {
            stale.push(account.clone());
        }
    }
    Ok(stale)
}

fn load_sessions(path: &Path) -> Result<RadrootsAppRemoteSignerSessionStoreState, String> {
    RadrootsAppRemoteSignerSessionStoreState::load(path).map_err(|error| error.to_string())
}

fn load_sessions_with_recovery(
    path: &Path,
) -> Result<RadrootsAppRemoteSignerSessionStoreLoadResult, String> {
    RadrootsAppRemoteSignerSessionStoreState::load_with_recovery(path)
        .map_err(|error| error.to_string())
}

fn save_sessions(
    path: &Path,
    state: &RadrootsAppRemoteSignerSessionStoreState,
) -> Result<(), String> {
    state.save(path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, FIXTURE_BOB, FIXTURE_CAROL, fixture_identity};
    use radroots_identity::RadrootsIdentityId;
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrSecretVault, RadrootsNostrSecretVaultMemory,
        RadrootsNostrSelectedAccountStatus,
    };

    const REMOTE_SIGNER_LABEL: &str = "remote signer";

    fn fixture_public(
        label: &radroots_studio_app_test_support::RadrootsAppApprovedFixtureIdentity,
    ) -> radroots_identity::RadrootsIdentityPublic {
        fixture_identity(label).expect("identity").to_public()
    }

    fn fixture_account_id(value: &str) -> RadrootsIdentityId {
        RadrootsIdentityId::try_from(value).expect("account id")
    }

    fn secret_store_secret(
        vault: &RadrootsNostrSecretVaultMemory,
        client_account_id: &str,
        secret: &str,
    ) {
        vault
            .store_secret_hex(&fixture_account_id(client_account_id), secret)
            .expect("store secret");
    }

    fn secret_loader(
        vault: RadrootsNostrSecretVaultMemory,
    ) -> impl Fn(&str) -> Result<String, String> {
        move |client_account_id| {
            vault
                .load_secret_hex(&fixture_account_id(client_account_id))
                .map_err(|source| source.to_string())?
                .ok_or_else(|| "missing secret".to_owned())
        }
    }

    fn secret_remover(
        vault: RadrootsNostrSecretVaultMemory,
    ) -> impl Fn(&str) -> Result<(), String> {
        move |client_account_id| {
            vault
                .remove_secret(&fixture_account_id(client_account_id))
                .map_err(|source| source.to_string())
        }
    }

    fn write_pending_state(path: &Path) -> RadrootsAppRemoteSignerSessionRecord {
        let record = RadrootsAppRemoteSignerSessionRecord::pending(
            fixture_public(&FIXTURE_ALICE),
            fixture_public(&FIXTURE_BOB),
            vec!["wss://relay.example.com".to_owned()],
        );
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        state.upsert_pending(record.clone()).expect("pending");
        state.save(path).expect("save");
        record
    }

    fn write_active_state(path: &Path) -> RadrootsAppRemoteSignerSessionRecord {
        let user_identity = fixture_public(&FIXTURE_CAROL);
        let mut record = RadrootsAppRemoteSignerSessionRecord::pending(
            fixture_public(&FIXTURE_ALICE),
            fixture_public(&FIXTURE_BOB),
            vec!["wss://relay.example.com".to_owned()],
        );
        record.user_identity = Some(user_identity);
        record.status = RadrootsAppRemoteSignerSessionStatus::Active;
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        state.sessions.push(record.clone());
        state.save(path).expect("save");
        record
    }

    #[test]
    fn clear_pending_session_removes_secret_and_session_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        let record = write_pending_state(path.as_path());
        let vault = RadrootsNostrSecretVaultMemory::new();
        secret_store_secret(&vault, record.client_account_id(), "deadbeef");

        let removed = radroots_studio_app_remote_signer_clear_pending_session(
            path.as_path(),
            secret_remover(vault.clone()),
        )
        .expect("clear pending");

        assert_eq!(
            removed.expect("removed").client_account_id(),
            record.client_account_id()
        );
        assert!(
            vault
                .load_secret_hex(&fixture_account_id(record.client_account_id()))
                .expect("load")
                .is_none()
        );
        assert!(
            RadrootsAppRemoteSignerSessionStoreState::load(path.as_path())
                .expect("load")
                .sessions
                .is_empty()
        );
    }

    #[test]
    fn disconnect_selected_remote_signer_leaves_session_for_retry_when_secret_cleanup_fails() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        let record = write_active_state(path.as_path());
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        manager
            .upsert_public_identity(
                record.user_identity.clone().expect("user"),
                Some(REMOTE_SIGNER_LABEL.to_owned()),
                true,
            )
            .expect("upsert");

        let error = radroots_studio_app_remote_signer_disconnect_selected(
            &manager,
            path.as_path(),
            |_client_account_id| Err("vault unavailable".to_owned()),
        )
        .expect_err("cleanup failure");

        assert!(error.contains("session secret cleanup failed"));
        assert!(matches!(
            manager.selected_account_status().expect("status"),
            RadrootsNostrSelectedAccountStatus::NotConfigured
        ));
        assert!(
            RadrootsAppRemoteSignerSessionStoreState::load(path.as_path())
                .expect("load")
                .active_session_for_account_id(
                    record
                        .account_id()
                        .expect("account id after disconnect failure")
                )
                .is_some()
        );
    }

    #[test]
    fn reconcile_startup_removes_remote_signer_public_only_accounts_after_store_quarantine() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        std::fs::write(path.as_path(), "{invalid").expect("write invalid");
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let public = fixture_public(&FIXTURE_CAROL);
        let account_id = public.id.clone();
        manager
            .upsert_public_identity(public, Some(REMOTE_SIGNER_LABEL.to_owned()), true)
            .expect("upsert");

        radroots_studio_app_remote_signer_reconcile_startup(
            &manager,
            path.as_path(),
            REMOTE_SIGNER_LABEL,
            secret_loader(RadrootsNostrSecretVaultMemory::new()),
            secret_remover(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("reconcile");

        assert!(
            manager
                .list_accounts()
                .expect("accounts")
                .iter()
                .all(|record| record.account_id != account_id)
        );
        assert!(
            RadrootsAppRemoteSignerSessionStoreState::load(path.as_path())
                .expect("load")
                .sessions
                .is_empty()
        );
    }
}
