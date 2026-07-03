use std::{
    fs,
    path::{Path, PathBuf},
};

use radroots_identity::{IdentityError, RadrootsIdentity, RadrootsIdentityId};
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountRecord, RadrootsNostrAccountStatus, RadrootsNostrAccountsError,
    RadrootsNostrAccountsManager,
};
use radroots_secret_vault::{
    RadrootsHostVaultCapabilities, RadrootsSecretBackend, RadrootsSecretBackendAvailability,
    RadrootsSecretBackendSelection,
};
use radroots_studio_app_core::AppSharedAccountsPaths;
use radroots_studio_app_sqlite::{AppSqliteError, AppSqliteStore};
use radroots_studio_app_view::{
    AccountSummary, AccountSurfaceActivationProjection, ActiveSurface, AppIdentityProjection,
    FarmId, FarmerActivationProjection, SelectedAccountProjection, SelectedSurfaceProjection,
};
use thiserror::Error;

pub struct DesktopAccountsBootstrap {
    pub accounts_manager: Option<RadrootsNostrAccountsManager>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopLocalIdentityImportMode {
    RawSecretKey,
    EncryptedSecretKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopLocalIdentityImportRequest {
    pub mode: DesktopLocalIdentityImportMode,
    pub secret_text: String,
    pub password: Option<String>,
}

impl DesktopLocalIdentityImportRequest {
    pub fn new(
        mode: DesktopLocalIdentityImportMode,
        secret_text: impl Into<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            mode,
            secret_text: secret_text.into(),
            password,
        }
    }

    pub fn raw_secret_key(secret_text: impl Into<String>) -> Self {
        Self::new(
            DesktopLocalIdentityImportMode::RawSecretKey,
            secret_text,
            None,
        )
    }

    pub fn encrypted_secret_key(
        secret_text: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self::new(
            DesktopLocalIdentityImportMode::EncryptedSecretKey,
            secret_text,
            Some(password.into()),
        )
    }
}

pub fn bootstrap_desktop_accounts(
    paths: &AppSharedAccountsPaths,
    _sqlite_store: &AppSqliteStore,
) -> Result<DesktopAccountsBootstrap, DesktopAccountsBootstrapError> {
    bootstrap_desktop_accounts_with_availability(paths, secret_backend_availability()?)
}

pub fn generate_local_account(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
    label: Option<String>,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    manager.generate_identity(label, true)?;
    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

pub fn import_local_account(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
    request: &DesktopLocalIdentityImportRequest,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    let identity = import_identity(request)?;
    manager.upsert_identity(&identity, None, true)?;
    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

pub fn select_local_account(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
    account_id: &str,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    let account_id = RadrootsIdentityId::parse(account_id.trim())?;
    manager.set_default_account(&account_id)?;
    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

pub fn select_active_surface(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
    active_surface: ActiveSurface,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    let Some(selected_account) = selected_account_record(manager)? else {
        return Ok(identity_projection_from_manager(manager, sqlite_store)?);
    };
    let selected_projection =
        selected_account_projection_from_record(&selected_account, sqlite_store)?;
    let activation = AccountSurfaceActivationProjection::new(
        selected_projection.account.account_id.clone(),
        SelectedSurfaceProjection::new(active_surface),
        selected_projection.farmer_activation.clone(),
    );

    sqlite_store.save_surface_activation(&activation)?;
    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

pub fn remove_selected_local_key(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    let Some(selected_account) = selected_account_record(manager)? else {
        return Ok(identity_projection_from_manager(manager, sqlite_store)?);
    };
    let account_id = selected_account.account_id.to_string();

    sqlite_store.clear_surface_activation(account_id.as_str())?;
    manager.remove_account(&selected_account.account_id)?;
    if let Some(next_account) = manager.list_accounts()?.into_iter().next() {
        manager.set_default_account(&next_account.account_id)?;
    }

    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

pub fn reset_local_device_state(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
    accounts_paths: &AppSharedAccountsPaths,
) -> Result<AppIdentityProjection, DesktopAccountsCommandError> {
    let account_ids = manager
        .list_accounts()?
        .into_iter()
        .map(|record| record.account_id)
        .collect::<Vec<_>>();

    for account_id in &account_ids {
        sqlite_store.clear_surface_activation(account_id.as_str())?;
    }
    for account_id in account_ids {
        manager.remove_account(&account_id)?;
    }

    remove_accounts_file_if_present(accounts_paths.store_path.as_path())?;
    Ok(identity_projection_from_manager(manager, sqlite_store)?)
}

fn bootstrap_desktop_accounts_with_availability(
    paths: &AppSharedAccountsPaths,
    availability: RadrootsSecretBackendAvailability,
) -> Result<DesktopAccountsBootstrap, DesktopAccountsBootstrapError> {
    ensure_directory(paths.data_root.as_path())?;
    ensure_directory(paths.secrets_root.as_path())?;

    let selection = local_account_secret_backend_selection();
    let (accounts_manager, _) = RadrootsNostrAccountsManager::new_local_file_backed(
        paths.store_path.as_path(),
        paths.secrets_root.as_path(),
        selection,
        availability,
        "radroots_studio_app_encrypted_file",
    )?;
    Ok(DesktopAccountsBootstrap {
        accounts_manager: Some(accounts_manager),
    })
}

fn ensure_directory(path: &Path) -> Result<(), DesktopAccountsBootstrapError> {
    fs::create_dir_all(path).map_err(|source| DesktopAccountsBootstrapError::CreateDirectory {
        path: path.to_path_buf(),
        source,
    })
}

fn local_account_secret_backend_selection() -> RadrootsSecretBackendSelection {
    RadrootsSecretBackendSelection {
        primary: RadrootsSecretBackend::EncryptedFile,
    }
}

fn secret_backend_availability()
-> Result<RadrootsSecretBackendAvailability, DesktopAccountsBootstrapError> {
    Ok(RadrootsSecretBackendAvailability {
        host_vault: RadrootsHostVaultCapabilities::unavailable(),
        encrypted_file: true,
        external_command: false,
        memory: false,
    })
}

fn import_identity(
    request: &DesktopLocalIdentityImportRequest,
) -> Result<RadrootsIdentity, DesktopAccountsCommandError> {
    match request.mode {
        DesktopLocalIdentityImportMode::RawSecretKey => Ok(RadrootsIdentity::from_secret_key_str(
            request.secret_text.trim(),
        )?),
        DesktopLocalIdentityImportMode::EncryptedSecretKey => {
            let Some(password) = request.password.as_deref() else {
                return Err(DesktopAccountsCommandError::EncryptedImportPasswordRequired);
            };
            Ok(RadrootsIdentity::from_encrypted_secret_key_str(
                request.secret_text.trim(),
                password,
            )?)
        }
    }
}

fn remove_accounts_file_if_present(path: &Path) -> Result<(), DesktopAccountsCommandError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(DesktopAccountsCommandError::RemoveAccountStore {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub(crate) fn identity_projection_from_manager(
    manager: &RadrootsNostrAccountsManager,
    sqlite_store: &AppSqliteStore,
) -> Result<AppIdentityProjection, DesktopAccountsProjectionError> {
    let roster_records = manager.list_accounts()?;
    let roster = account_roster_from_records(roster_records.as_slice());

    match manager.default_account_status()? {
        RadrootsNostrAccountStatus::NotConfigured => {
            Ok(AppIdentityProjection::missing_with_roster(roster))
        }
        RadrootsNostrAccountStatus::PublicOnly { account }
        | RadrootsNostrAccountStatus::Ready { account } => Ok(AppIdentityProjection::ready(
            roster,
            selected_account_projection_from_record(&account, sqlite_store)?,
        )),
    }
}

fn selected_account_projection_from_record(
    record: &RadrootsNostrAccountRecord,
    sqlite_store: &AppSqliteStore,
) -> Result<SelectedAccountProjection, DesktopAccountsProjectionError> {
    let account = account_summary_from_record(record);

    Ok(
        match sqlite_store.load_surface_activation(account.account_id.as_str())? {
            Some(activation) => {
                SelectedAccountProjection::from_surface_activation(account, activation)
            }
            None => {
                let activation = default_farmer_surface_activation(account.account_id.as_str());
                sqlite_store.save_surface_activation(&activation)?;
                SelectedAccountProjection::from_surface_activation(account, activation)
            }
        },
    )
}

fn selected_account_record(
    manager: &RadrootsNostrAccountsManager,
) -> Result<Option<RadrootsNostrAccountRecord>, RadrootsNostrAccountsError> {
    match manager.default_account_status()? {
        RadrootsNostrAccountStatus::NotConfigured => Ok(None),
        RadrootsNostrAccountStatus::PublicOnly { account }
        | RadrootsNostrAccountStatus::Ready { account } => Ok(Some(account)),
    }
}

fn default_farmer_surface_activation(account_id: &str) -> AccountSurfaceActivationProjection {
    AccountSurfaceActivationProjection::new(
        account_id,
        SelectedSurfaceProjection::new(ActiveSurface::Farmer),
        FarmerActivationProjection::active(FarmId::new()),
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
        custody: radroots_studio_app_view::AccountCustody::LocalManaged,
    }
}

#[derive(Debug, Error)]
pub enum DesktopAccountsProjectionError {
    #[error(transparent)]
    Accounts(#[from] RadrootsNostrAccountsError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
}

#[derive(Debug, Error)]
pub enum DesktopAccountsCommandError {
    #[error(transparent)]
    Accounts(#[from] RadrootsNostrAccountsError),
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    Projection(#[from] DesktopAccountsProjectionError),
    #[error("encrypted secret key import requires a password")]
    EncryptedImportPasswordRequired,
    #[error("failed to remove account store {path}: {source}")]
    RemoveAccountStore {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum DesktopAccountsBootstrapError {
    #[error("failed to create runtime directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Accounts(#[from] RadrootsNostrAccountsError),
    #[error(transparent)]
    Projection(#[from] DesktopAccountsProjectionError),
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use radroots_identity::RadrootsIdentity;
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory,
    };
    use radroots_secret_vault::RadrootsHostVaultCapabilities;
    use radroots_studio_app_core::AppSharedAccountsPaths;
    use radroots_studio_app_sqlite::{AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_view::{
        AccountSurfaceActivationProjection, ActiveSurface, AppStartupGate, IdentityReadiness,
        SelectedSurfaceProjection,
    };

    use super::{
        DesktopLocalIdentityImportRequest, account_summary_from_record,
        bootstrap_desktop_accounts_with_availability, generate_local_account,
        identity_projection_from_manager, import_local_account, remove_selected_local_key,
        reset_local_device_state, select_local_account, selected_account_projection_from_record,
        selected_account_record,
    };

    fn temp_shared_accounts_paths(label: &str) -> AppSharedAccountsPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("radroots_studio_app_accounts_{label}_{suffix}"));

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
    fn bootstrap_fails_when_encrypted_file_backend_is_unavailable() {
        let paths = temp_shared_accounts_paths("blocked");
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");
        match bootstrap_desktop_accounts_with_availability(
            &paths,
            unavailable_secret_backend_availability(),
        ) {
            Err(super::DesktopAccountsBootstrapError::Accounts(_)) => {}
            Err(other) => panic!("unexpected bootstrap error: {other}"),
            Ok(_) => panic!("bootstrap should fail when encrypted file backend is unavailable"),
        }

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
        let selected_account = selected_account_record(&manager)
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
            SelectedSurfaceProjection::new(ActiveSurface::Farmer)
        );
        assert!(selected_account_projection.farmer_activation.is_active());

        let activation = AccountSurfaceActivationProjection::new(
            account_id.as_str(),
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            radroots_studio_app_view::FarmerActivationProjection::active(
                radroots_studio_app_view::FarmId::new(),
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
    fn command_generate_and_select_support_multiple_local_accounts() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");

        let first_projection =
            generate_local_account(&manager, &sqlite_store, Some("First".to_owned()))
                .expect("first account should generate");
        let first_account_id = first_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();

        let second_projection =
            generate_local_account(&manager, &sqlite_store, Some("Second".to_owned()))
                .expect("second account should generate");
        let second_account_id = second_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();

        assert_eq!(first_projection.roster.len(), 1);
        assert_eq!(second_projection.roster.len(), 2);
        assert_ne!(first_account_id, second_account_id);
        assert_eq!(
            second_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.label.as_deref()),
            Some(Some("Second"))
        );
        assert_eq!(second_projection.startup_gate(), AppStartupGate::Farmer);
    }

    #[test]
    fn command_import_supports_raw_and_encrypted_secret_keys() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");
        let raw_identity = RadrootsIdentity::generate();
        let encrypted_identity = RadrootsIdentity::generate();
        let encrypted_secret = encrypted_identity
            .encrypt_secret_key_ncryptsec("radroots-password")
            .expect("encrypted secret should export");

        let raw_projection = import_local_account(
            &manager,
            &sqlite_store,
            &DesktopLocalIdentityImportRequest::raw_secret_key(raw_identity.nsec()),
        )
        .expect("raw import should succeed");
        let encrypted_projection = import_local_account(
            &manager,
            &sqlite_store,
            &DesktopLocalIdentityImportRequest::encrypted_secret_key(
                encrypted_secret,
                "radroots-password",
            ),
        )
        .expect("encrypted import should succeed");

        assert_eq!(raw_projection.roster.len(), 1);
        assert_eq!(encrypted_projection.roster.len(), 2);
        assert_eq!(
            encrypted_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(encrypted_identity.id().as_str())
        );
    }

    #[test]
    fn command_select_refreshes_selected_account_activation() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");
        let first_account_id = manager
            .generate_identity(Some("First".to_owned()), true)
            .expect("first account should generate");
        let second_account_id = manager
            .generate_identity(Some("Second".to_owned()), false)
            .expect("second account should generate");
        let activation = AccountSurfaceActivationProjection::new(
            second_account_id.as_str(),
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            radroots_studio_app_view::FarmerActivationProjection::active(
                radroots_studio_app_view::FarmId::new(),
            ),
        );
        sqlite_store
            .save_surface_activation(&activation)
            .expect("surface activation should save");

        let projection = select_local_account(&manager, &sqlite_store, second_account_id.as_str())
            .expect("selection should refresh");

        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(second_account_id.as_str())
        );
        assert_eq!(projection.startup_gate(), AppStartupGate::Farmer);
        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );
        assert_eq!(
            selected_account_record(&manager)
                .expect("selected account")
                .map(|account| account.account_id),
            Some(second_account_id.clone())
        );
        assert_ne!(
            first_account_id,
            selected_account_record(&manager)
                .expect("selected account")
                .expect("selected")
                .account_id
        );
    }

    #[test]
    fn command_remove_selected_local_key_clears_activation_and_selects_next_account() {
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrMemoryAccountStore::new()),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("memory manager should build");
        let first_account_id = manager
            .generate_identity(Some("First".to_owned()), true)
            .expect("first account should generate");
        let second_account_id = manager
            .generate_identity(Some("Second".to_owned()), false)
            .expect("second account should generate");
        manager
            .set_default_account(&first_account_id)
            .expect("first account should remain selected");
        let activation = AccountSurfaceActivationProjection::new(
            first_account_id.as_str(),
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            radroots_studio_app_view::FarmerActivationProjection::active(
                radroots_studio_app_view::FarmId::new(),
            ),
        );
        sqlite_store
            .save_surface_activation(&activation)
            .expect("surface activation should save");

        let projection = remove_selected_local_key(&manager, &sqlite_store)
            .expect("selected local key should remove");

        assert_eq!(projection.roster.len(), 1);
        assert_eq!(
            projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(second_account_id.as_str())
        );
        assert_eq!(
            sqlite_store
                .load_surface_activation(first_account_id.as_str())
                .expect("removed activation should load"),
            None
        );
    }

    #[test]
    fn command_reset_local_device_state_removes_store_file_and_all_activations() {
        let paths = temp_shared_accounts_paths("reset");
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");
        let manager = RadrootsNostrAccountsManager::new(
            Arc::new(RadrootsNostrFileAccountStore::new(
                paths.store_path.as_path(),
            )),
            Arc::new(RadrootsNostrSecretVaultMemory::new()),
        )
        .expect("file-backed manager should build");
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("sqlite store");
        let first_account_id = manager
            .generate_identity(Some("First".to_owned()), true)
            .expect("first account should generate");
        let second_account_id = manager
            .generate_identity(Some("Second".to_owned()), false)
            .expect("second account should generate");
        sqlite_store
            .save_surface_activation(&AccountSurfaceActivationProjection::new(
                first_account_id.as_str(),
                SelectedSurfaceProjection::new(ActiveSurface::Farmer),
                radroots_studio_app_view::FarmerActivationProjection::active(
                    radroots_studio_app_view::FarmId::new(),
                ),
            ))
            .expect("first activation should save");
        sqlite_store
            .save_surface_activation(&AccountSurfaceActivationProjection::new(
                second_account_id.as_str(),
                SelectedSurfaceProjection::new(ActiveSurface::Farmer),
                radroots_studio_app_view::FarmerActivationProjection::active(
                    radroots_studio_app_view::FarmId::new(),
                ),
            ))
            .expect("second activation should save");
        assert!(paths.store_path.exists());

        let projection = reset_local_device_state(&manager, &sqlite_store, &paths)
            .expect("device state should reset");

        assert_eq!(projection.readiness, IdentityReadiness::MissingAccount);
        assert_eq!(projection.startup_gate(), AppStartupGate::SetupRequired);
        assert!(projection.roster.is_empty());
        assert!(projection.selected_account.is_none());
        assert!(!paths.store_path.exists());
        assert_eq!(
            sqlite_store
                .load_surface_activation(first_account_id.as_str())
                .expect("first activation should load"),
            None
        );
        assert_eq!(
            sqlite_store
                .load_surface_activation(second_account_id.as_str())
                .expect("second activation should load"),
            None
        );

        cleanup_paths(&paths);
    }

    fn cleanup_paths(paths: &AppSharedAccountsPaths) {
        let Some(base) = paths.data_root.ancestors().nth(3).map(PathBuf::from) else {
            return;
        };
        let _ = fs::remove_dir_all(base);
    }
}
