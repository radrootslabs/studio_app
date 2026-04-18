use std::{env, fs, path::PathBuf};

use radroots_studio_app_core::AppSharedAccountsPaths;
use radroots_studio_app_models::{
    AccountSummary, AppIdentityProjection, FarmerActivationProjection, IdentityBlockedReason,
    SelectedAccountProjection, SelectedSurfaceProjection,
};
use radroots_studio_app_sqlite::{AppSqliteError, AppSqliteStore};
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountRecord, RadrootsNostrAccountStore, RadrootsNostrAccountStoreState,
    RadrootsNostrAccountsError, RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
    RadrootsNostrSelectedAccountStatus,
};
use radroots_secret_vault::{
    RadrootsHostVaultCapabilities, RadrootsHostVaultPolicy, RadrootsSecretBackend,
    RadrootsSecretBackendAvailability, RadrootsSecretBackendSelection, RadrootsSecretVault,
    RadrootsSecretVaultError, RadrootsSecretVaultOsKeyring,
};
use thiserror::Error;

const HOST_VAULT_AVAILABILITY_OVERRIDE_ENV: &str = "RADROOTS_APP_HOST_VAULT_AVAILABLE";
const HOST_VAULT_SERVICE_NAME: &str = "org.radroots.app.local-account";
const HOST_VAULT_PROBE_SLOT: &str = "__radroots_studio_app_host_vault_probe__";

pub struct DesktopAccountsBootstrap {
    pub accounts_manager: Option<RadrootsNostrAccountsManager>,
    pub identity_projection: AppIdentityProjection,
}

pub fn bootstrap_desktop_accounts(
    paths: &AppSharedAccountsPaths,
    sqlite_store: &AppSqliteStore,
) -> Result<DesktopAccountsBootstrap, DesktopAccountsBootstrapError> {
    bootstrap_desktop_accounts_with_availability(
        paths,
        sqlite_store,
        secret_backend_availability()?,
    )
}

fn bootstrap_desktop_accounts_with_availability(
    paths: &AppSharedAccountsPaths,
    sqlite_store: &AppSqliteStore,
    availability: RadrootsSecretBackendAvailability,
) -> Result<DesktopAccountsBootstrap, DesktopAccountsBootstrapError> {
    ensure_directory(paths.data_root.as_path())?;
    ensure_directory(paths.secrets_root.as_path())?;

    let selection = local_account_secret_backend_selection();
    let store = RadrootsNostrFileAccountStore::new(paths.store_path.as_path());

    match RadrootsNostrAccountsManager::resolve_local_backend(selection, availability) {
        Ok(_) => {
            let (accounts_manager, _) = RadrootsNostrAccountsManager::new_local_file_backed(
                paths.store_path.as_path(),
                paths.secrets_root.as_path(),
                selection,
                availability,
                HOST_VAULT_SERVICE_NAME,
            )?;
            let identity_projection =
                identity_projection_from_manager(&accounts_manager, sqlite_store)?;

            Ok(DesktopAccountsBootstrap {
                accounts_manager: Some(accounts_manager),
                identity_projection,
            })
        }
        Err(RadrootsSecretVaultError::BackendUnavailable { .. })
        | Err(RadrootsSecretVaultError::FallbackUnavailable { .. }) => {
            let state = store.load()?;
            let identity_projection =
                blocked_identity_projection_from_store_state(state, sqlite_store)?;

            Ok(DesktopAccountsBootstrap {
                accounts_manager: None,
                identity_projection,
            })
        }
        Err(error) => Err(error.into()),
    }
}

fn ensure_directory(path: &std::path::Path) -> Result<(), DesktopAccountsBootstrapError> {
    fs::create_dir_all(path).map_err(|source| DesktopAccountsBootstrapError::CreateDirectory {
        path: path.to_path_buf(),
        source,
    })
}

fn local_account_secret_backend_selection() -> RadrootsSecretBackendSelection {
    RadrootsSecretBackendSelection {
        primary: RadrootsSecretBackend::HostVault(RadrootsHostVaultPolicy::desktop()),
        fallback: None,
    }
}

fn secret_backend_availability()
-> Result<RadrootsSecretBackendAvailability, DesktopAccountsBootstrapError> {
    Ok(RadrootsSecretBackendAvailability {
        host_vault: host_vault_capabilities()?,
        encrypted_file: false,
        external_command: false,
        memory: false,
    })
}

fn host_vault_capabilities() -> Result<RadrootsHostVaultCapabilities, DesktopAccountsBootstrapError>
{
    if let Some(available) = host_vault_availability_override()? {
        return Ok(match available {
            true => RadrootsHostVaultCapabilities::desktop_keyring(),
            false => RadrootsHostVaultCapabilities::unavailable(),
        });
    }

    let keyring = RadrootsSecretVaultOsKeyring::new(HOST_VAULT_SERVICE_NAME);
    match keyring.load_secret(HOST_VAULT_PROBE_SLOT) {
        Ok(_) => Ok(RadrootsHostVaultCapabilities::desktop_keyring()),
        Err(_) => Ok(RadrootsHostVaultCapabilities::unavailable()),
    }
}

fn host_vault_availability_override() -> Result<Option<bool>, DesktopAccountsBootstrapError> {
    let Ok(value) = env::var(HOST_VAULT_AVAILABILITY_OVERRIDE_ENV) else {
        return Ok(None);
    };

    parse_bool_value(HOST_VAULT_AVAILABILITY_OVERRIDE_ENV, value.trim()).map(Some)
}

fn parse_bool_value(key: &str, value: &str) -> Result<bool, DesktopAccountsBootstrapError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(DesktopAccountsBootstrapError::Configuration(format!(
            "{key} must be a boolean value, got `{other}`"
        ))),
    }
}

fn blocked_identity_projection_from_store_state(
    state: RadrootsNostrAccountStoreState,
    sqlite_store: &AppSqliteStore,
) -> Result<AppIdentityProjection, DesktopAccountsBootstrapError> {
    let selected_account = selected_account_from_store_state(&state, sqlite_store)?;

    Ok(AppIdentityProjection::blocked_with_selection(
        IdentityBlockedReason::HostVaultUnavailable,
        account_roster_from_records(state.accounts.as_slice()),
        selected_account,
    ))
}

fn identity_projection_from_manager(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
) -> Result<AppIdentityProjection, DesktopAccountsBootstrapError> {
    let roster_records = manager.list_accounts()?;
    let roster = account_roster_from_records(roster_records.as_slice());

    match manager.selected_account_status()? {
        RadrootsNostrSelectedAccountStatus::NotConfigured => {
            Ok(AppIdentityProjection::missing_with_roster(roster))
        }
        RadrootsNostrSelectedAccountStatus::PublicOnly { account }
        | RadrootsNostrSelectedAccountStatus::Ready { account } => {
            Ok(AppIdentityProjection::ready(
                roster,
                selected_account_projection_from_record(&account, sqlite_store)?,
            ))
        }
    }
}

fn selected_account_from_store_state(
    state: &RadrootsNostrAccountStoreState,
    sqlite_store: &AppSqliteStore,
) -> Result<Option<SelectedAccountProjection>, DesktopAccountsBootstrapError> {
    let Some(selected_account_id) = state.selected_account_id.as_ref() else {
        return Ok(None);
    };
    let Some(record) = state
        .accounts
        .iter()
        .find(|record| &record.account_id == selected_account_id)
    else {
        return Ok(None);
    };

    selected_account_projection_from_record(record, sqlite_store).map(Some)
}

fn selected_account_projection_from_record(
    record: &RadrootsNostrAccountRecord,
    sqlite_store: &AppSqliteStore,
) -> Result<SelectedAccountProjection, DesktopAccountsBootstrapError> {
    let account = account_summary_from_record(record);

    Ok(
        match sqlite_store.load_surface_activation(account.account_id.as_str())? {
            Some(activation) => {
                SelectedAccountProjection::from_surface_activation(account, activation)
            }
            None => SelectedAccountProjection::new(
                account,
                SelectedSurfaceProjection::default(),
                FarmerActivationProjection::inactive(),
            ),
        },
    )
}

fn account_roster_from_records(records: &[RadrootsNostrAccountRecord]) -> Vec<AccountSummary> {
    records.iter().map(account_summary_from_record).collect()
}

fn account_summary_from_record(record: &RadrootsNostrAccountRecord) -> AccountSummary {
    AccountSummary {
        account_id: record.account_id.to_string(),
        npub: record.public_identity.public_key_npub.clone(),
        label: record.label.clone(),
        custody: radroots_studio_app_models::AccountCustody::LocalManaged,
    }
}

#[derive(Debug, Error)]
pub enum DesktopAccountsBootstrapError {
    #[error("failed to create runtime directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    Accounts(#[from] RadrootsNostrAccountsError),
    #[error(transparent)]
    SecretVault(#[from] RadrootsSecretVaultError),
    #[error("{0}")]
    Configuration(String),
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use radroots_studio_app_core::AppSharedAccountsPaths;
    use radroots_studio_app_models::{
        ActiveSurface, AppStartupGate, IdentityBlockedReason, IdentityReadiness,
        SelectedSurfaceProjection,
    };
    use radroots_studio_app_sqlite::{AppSqliteStore, DatabaseTarget};
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory,
    };
    use radroots_secret_vault::RadrootsHostVaultCapabilities;

    use super::{
        account_summary_from_record, blocked_identity_projection_from_store_state,
        bootstrap_desktop_accounts_with_availability, identity_projection_from_manager,
        selected_account_projection_from_record,
    };

    fn temp_shared_accounts_paths(label: &str) -> AppSharedAccountsPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("radroots_studio_app_accounts_{label}_{suffix}"));

        AppSharedAccountsPaths {
            data_root: base.join("data/shared/accounts"),
            secrets_root: base.join("secrets/shared/accounts"),
            store_path: base.join("data/shared/accounts/store.json"),
        }
    }

    fn unavailable_secret_backend_availability()
    -> radroots_secret_vault::RadrootsSecretBackendAvailability {
        radroots_secret_vault::RadrootsSecretBackendAvailability {
            host_vault: RadrootsHostVaultCapabilities::unavailable(),
            encrypted_file: false,
            external_command: false,
            memory: false,
        }
    }

    #[test]
    fn blocked_bootstrap_keeps_roster_and_selected_account_when_host_vault_is_unavailable() {
        let paths = temp_shared_accounts_paths("blocked");
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");
        let store = Arc::new(RadrootsNostrFileAccountStore::new(
            paths.store_path.as_path(),
        ));
        let manager = RadrootsNostrAccountsManager::new(
            store,
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("file-backed memory manager should build");
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");

        let first_account_id = manager
            .generate_identity(Some("North field".to_owned()), true)
            .expect("first account should generate");
        let second_account_id = manager
            .generate_identity(Some("South field".to_owned()), false)
            .expect("second account should generate");
        manager
            .select_account(&first_account_id)
            .expect("first account should remain selected");

        let bootstrap = bootstrap_desktop_accounts_with_availability(
            &paths,
            &sqlite_store,
            unavailable_secret_backend_availability(),
        )
        .expect("blocked bootstrap should succeed");

        assert!(bootstrap.accounts_manager.is_none());
        assert_eq!(
            bootstrap.identity_projection.readiness,
            IdentityReadiness::Blocked(IdentityBlockedReason::HostVaultUnavailable)
        );
        assert_eq!(
            bootstrap.identity_projection.startup_gate(),
            AppStartupGate::Blocked
        );
        assert_eq!(bootstrap.identity_projection.roster.len(), 2);
        assert_eq!(
            bootstrap
                .identity_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(first_account_id.as_str())
        );
        assert!(
            bootstrap
                .identity_projection
                .roster
                .iter()
                .any(|account| account.account_id == second_account_id.as_str())
        );

        cleanup_paths(&paths);
    }

    #[test]
    fn manager_projection_uses_selected_account_and_activation_state() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");
        let account_id = manager
            .generate_identity(Some("North field".to_owned()), true)
            .expect("account should generate");
        let selected_account = manager
            .selected_account()
            .expect("selected account should load")
            .expect("selected account should exist");
        let selected_account_summary = account_summary_from_record(&selected_account);
        let selected_account_projection =
            selected_account_projection_from_record(&selected_account, &sqlite_store)
                .expect("selected account projection");

        assert_eq!(
            selected_account_projection.account,
            selected_account_summary
        );
        assert_eq!(
            selected_account_projection.selected_surface,
            SelectedSurfaceProjection::default()
        );

        let activation = radroots_studio_app_models::AccountSurfaceActivationProjection::new(
            account_id.as_str(),
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            radroots_studio_app_models::FarmerActivationProjection::active(
                radroots_studio_app_models::FarmId::new(),
            ),
        );
        sqlite_store
            .save_surface_activation(&activation)
            .expect("surface activation should save");

        let projection =
            identity_projection_from_manager(&manager, &sqlite_store).expect("projection");

        assert_eq!(projection.readiness, IdentityReadiness::Ready);
        assert_eq!(projection.startup_gate(), AppStartupGate::Farmer);
        assert_eq!(projection.roster.len(), 1);
        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(account_id.as_str())
        );
        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );
        assert!(
            projection
                .selected_account
                .as_ref()
                .is_some_and(|account| account.farmer_activation.is_active())
        );
    }

    #[test]
    fn blocked_projection_from_store_state_ignores_stale_selected_account_ids() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");
        let account_id = manager
            .generate_identity(Some("North field".to_owned()), true)
            .expect("account should generate");
        let stale_selected_account_id = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("secondary memory manager should build")
        .generate_identity(Some("South field".to_owned()), true)
        .expect("secondary account should generate");
        let record = manager
            .selected_account()
            .expect("selected account should load")
            .expect("selected account should exist");
        let state = radroots_nostr_accounts::prelude::RadrootsNostrAccountStoreState {
            version: radroots_nostr_accounts::prelude::RADROOTS_NOSTR_ACCOUNTS_STORE_VERSION,
            selected_account_id: Some(stale_selected_account_id),
            accounts: vec![record],
        };

        let projection =
            blocked_identity_projection_from_store_state(state, &sqlite_store).expect("projection");

        assert_eq!(
            projection.readiness,
            IdentityReadiness::Blocked(IdentityBlockedReason::HostVaultUnavailable)
        );
        assert!(projection.selected_account.is_none());
        assert_eq!(projection.roster.len(), 1);
        assert_eq!(projection.roster[0].account_id, account_id.as_str());
    }

    fn cleanup_paths(paths: &AppSharedAccountsPaths) {
        let Some(base) = paths.data_root.ancestors().nth(3).map(PathBuf::from) else {
            return;
        };
        let _ = fs::remove_dir_all(base);
    }
}
