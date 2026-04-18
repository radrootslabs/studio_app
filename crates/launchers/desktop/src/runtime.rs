use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use radroots_studio_app_core::{AppDesktopRuntimePaths, AppRuntimePathsError, AppSharedAccountsPaths};
use radroots_studio_app_models::{
    ActiveSurface, AppActivityContext, AppActivityKind, AppStartupGate, SettingsAccountProjection,
    SettingsPreference, SettingsSection, TodayAgendaProjection,
};
use radroots_studio_app_sqlite::{
    APP_ACTIVITY_CONTEXT_LIMIT, AppSqliteError, AppSqliteStore, DatabaseTarget,
};
use radroots_studio_app_state::{
    AppShellProjection, AppStateCommand, AppStateStore, AppStateStoreError,
    InMemoryAppStateRepository,
};
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
use thiserror::Error;
use tracing::error;

use crate::accounts::{
    DesktopAccountsBootstrapError, DesktopAccountsCommandError, DesktopLocalIdentityImportRequest,
    bootstrap_desktop_accounts, generate_local_account, import_local_account,
    remove_selected_local_key, reset_local_device_state, select_active_surface,
    select_local_account,
};

const APP_DATABASE_FILE_NAME: &str = "app.sqlite3";

#[derive(Clone, Debug)]
pub struct DesktopAppRuntime {
    state: Arc<Mutex<DesktopAppRuntimeState>>,
}

impl DesktopAppRuntime {
    pub fn bootstrap() -> Self {
        let state = match DesktopAppRuntimeState::try_bootstrap() {
            Ok(state) => state,
            Err(error) => DesktopAppRuntimeState::degraded(error),
        };

        Self::from_state(state)
    }

    pub fn summary(&self) -> DesktopAppRuntimeSummary {
        let state = self.lock_state();

        DesktopAppRuntimeSummary {
            shell_projection: state.state_store.shell_projection().clone(),
            settings_account_projection: state.state_store.settings_account_projection(),
            startup_gate: state.state_store.startup_gate(),
            today_projection: state.state_store.today_projection().clone(),
            startup_issue: state.startup_issue.clone(),
        }
    }

    pub fn selected_settings_section(&self) -> SettingsSection {
        self.lock_state()
            .state_store
            .shell_projection()
            .settings
            .selected_section
    }

    pub fn select_settings_section(&self, section: SettingsSection) -> bool {
        let changed = self
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::select_settings_section(section));

        if changed {
            let _ = self.record_activity(AppActivityKind::SettingsSectionSelected { section });
        }

        changed
    }

    pub fn set_settings_preference(&self, preference: SettingsPreference, enabled: bool) -> bool {
        let changed = self.lock_state_mut().state_store.apply_in_memory(
            AppStateCommand::SetSettingsPreference {
                preference,
                enabled,
            },
        );

        if changed {
            let _ = self.record_activity(AppActivityKind::SettingsPreferenceUpdated {
                preference,
                enabled,
            });
        }

        changed
    }

    pub fn generate_local_account(
        &self,
        label: Option<String>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().generate_local_account(label)
    }

    pub fn import_local_account(
        &self,
        request: DesktopLocalIdentityImportRequest,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().import_local_account(&request)
    }

    pub fn select_local_account(
        &self,
        account_id: &str,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().select_local_account(account_id)
    }

    pub fn select_active_surface(
        &self,
        active_surface: ActiveSurface,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().select_active_surface(active_surface)
    }

    pub fn remove_selected_local_key(&self) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().remove_selected_local_key()
    }

    pub fn reset_local_device_state(&self) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().reset_local_device_state()
    }

    #[allow(dead_code)]
    pub fn replace_today_agenda(&self, projection: TodayAgendaProjection) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::replace_today_agenda(projection))
    }

    pub fn record_home_opened(&self) -> bool {
        self.record_activity(AppActivityKind::HomeOpened)
    }

    pub fn record_settings_opened(&self, section: SettingsSection) -> bool {
        self.record_activity(AppActivityKind::SettingsOpened { section })
    }

    #[allow(dead_code)]
    pub fn activity_context(&self, limit: Option<usize>) -> Option<AppActivityContext> {
        self.lock_state().sqlite_store.as_ref().and_then(|store| {
            store
                .load_activity_context(limit.unwrap_or(APP_ACTIVITY_CONTEXT_LIMIT))
                .ok()
        })
    }

    fn from_state(state: DesktopAppRuntimeState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, DesktopAppRuntimeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_state_mut(&self) -> MutexGuard<'_, DesktopAppRuntimeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn record_activity(&self, kind: AppActivityKind) -> bool {
        let result = self.lock_state().record_activity(kind.clone());
        if let Err(error) = result {
            error!(
                target: "activity",
                event = "activity.record_failed",
                activity_kind = kind.storage_key(),
                error = %error,
                "failed to record activity event"
            );
            return false;
        }

        true
    }
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntimeSummary {
    pub shell_projection: AppShellProjection,
    pub settings_account_projection: SettingsAccountProjection,
    pub startup_gate: AppStartupGate,
    pub today_projection: TodayAgendaProjection,
    pub startup_issue: Option<String>,
}

struct DesktopAppRuntimeState {
    state_store: AppStateStore<InMemoryAppStateRepository>,
    shared_accounts_paths: Option<AppSharedAccountsPaths>,
    accounts_manager: Option<RadrootsNostrAccountsManager>,
    sqlite_store: Option<AppSqliteStore>,
    startup_issue: Option<String>,
}

impl fmt::Debug for DesktopAppRuntimeState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DesktopAppRuntimeState")
            .field("state_store", &self.state_store)
            .field(
                "shared_accounts_paths",
                &self.shared_accounts_paths.as_ref().map(|_| "available"),
            )
            .field(
                "accounts_manager",
                &self.accounts_manager.as_ref().map(|_| "available"),
            )
            .field(
                "sqlite_store",
                &self.sqlite_store.as_ref().map(|_| "available"),
            )
            .field("startup_issue", &self.startup_issue)
            .finish()
    }
}

impl DesktopAppRuntimeState {
    fn try_bootstrap() -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let paths = AppDesktopRuntimePaths::current_desktop()?;
        let database_path = paths.app.data.join(APP_DATABASE_FILE_NAME);
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::Path(database_path.clone()))?;
        let mut state_store = AppStateStore::load(InMemoryAppStateRepository::default())?;
        let accounts_bootstrap = bootstrap_desktop_accounts(&paths.shared_accounts, &sqlite_store)?;
        let today_projection = sqlite_store.load_today_agenda(None)?;
        let _ =
            state_store.apply_in_memory(AppStateCommand::replace_today_agenda(today_projection));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_identity_projection(
            accounts_bootstrap.identity_projection,
        ));

        Ok(Self {
            state_store,
            shared_accounts_paths: Some(paths.shared_accounts.clone()),
            accounts_manager: accounts_bootstrap.accounts_manager,
            sqlite_store: Some(sqlite_store),
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self {
            state_store: AppStateStore::in_memory(AppShellProjection::default()),
            shared_accounts_paths: None,
            accounts_manager: None,
            sqlite_store: None,
            startup_issue: Some(error.to_string()),
        }
    }

    fn generate_local_account(
        &mut self,
        label: Option<String>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            generate_local_account(accounts_manager, sqlite_store, label)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn import_local_account(
        &mut self,
        request: &DesktopLocalIdentityImportRequest,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            import_local_account(accounts_manager, sqlite_store, request)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn select_local_account(
        &mut self,
        account_id: &str,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            select_local_account(accounts_manager, sqlite_store, account_id)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn select_active_surface(
        &mut self,
        active_surface: ActiveSurface,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            select_active_surface(accounts_manager, sqlite_store, active_surface)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn remove_selected_local_key(&mut self) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            remove_selected_local_key(accounts_manager, sqlite_store)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn reset_local_device_state(&mut self) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            let shared_accounts_paths = self.shared_accounts_paths()?;
            reset_local_device_state(accounts_manager, sqlite_store, shared_accounts_paths)?
        };

        Ok(self.replace_identity_projection(projection))
    }

    fn record_activity(&self, kind: AppActivityKind) -> Result<(), AppSqliteError> {
        match self.sqlite_store.as_ref() {
            Some(store) => store.record_activity_event(&kind),
            None => Ok(()),
        }
    }

    fn replace_identity_projection(
        &mut self,
        projection: radroots_studio_app_models::AppIdentityProjection,
    ) -> bool {
        self.state_store
            .apply_in_memory(AppStateCommand::replace_identity_projection(projection))
    }

    fn accounts_manager(
        &self,
    ) -> Result<&RadrootsNostrAccountsManager, DesktopAppRuntimeCommandError> {
        self.accounts_manager
            .as_ref()
            .ok_or_else(|| self.command_unavailable_error())
    }

    fn sqlite_store(&self) -> Result<&AppSqliteStore, DesktopAppRuntimeCommandError> {
        self.sqlite_store
            .as_ref()
            .ok_or(DesktopAppRuntimeCommandError::RuntimeUnavailable)
    }

    fn shared_accounts_paths(
        &self,
    ) -> Result<&AppSharedAccountsPaths, DesktopAppRuntimeCommandError> {
        self.shared_accounts_paths
            .as_ref()
            .ok_or(DesktopAppRuntimeCommandError::RuntimeUnavailable)
    }

    fn command_unavailable_error(&self) -> DesktopAppRuntimeCommandError {
        if self.startup_issue.is_some() || self.sqlite_store.is_none() {
            DesktopAppRuntimeCommandError::RuntimeUnavailable
        } else {
            DesktopAppRuntimeCommandError::HostVaultUnavailable
        }
    }
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeCommandError {
    #[error("desktop runtime commands are unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error("desktop runtime commands require an available host vault")]
    HostVaultUnavailable,
    #[error(transparent)]
    Accounts(#[from] DesktopAccountsCommandError),
}

#[derive(Debug, Error)]
enum DesktopAppRuntimeBootstrapError {
    #[error(transparent)]
    RuntimePaths(#[from] AppRuntimePathsError),
    #[error(transparent)]
    Accounts(#[from] DesktopAccountsBootstrapError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    State(#[from] AppStateStoreError),
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use radroots_studio_app_core::{
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePlatform,
        AppSharedAccountsPaths, SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITY_FILE_NAME,
    };
    use radroots_studio_app_models::{
        AccountSurfaceActivationProjection, ActiveSurface, AppActivityKind, AppStartupGate, FarmId,
        FarmReadiness, FarmSummary, FarmerActivationProjection, SelectedSurfaceProjection,
        SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TodaySummary,
    };
    use radroots_studio_app_sqlite::{AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_state::{
        AppStateRepositoryError, AppStateStore, AppStateStoreError, InMemoryAppStateRepository,
    };
    use radroots_identity::RadrootsIdentity;
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory,
    };

    use crate::accounts::DesktopLocalIdentityImportRequest;

    use super::{
        APP_DATABASE_FILE_NAME, DesktopAppRuntime, DesktopAppRuntimeCommandError,
        DesktopAppRuntimeState,
    };

    #[test]
    fn desktop_namespace_uses_canonical_app_and_shared_runtime_roots() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(PathBuf::from("/Users/treesap")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("interactive user roots should resolve");

        assert_eq!(
            paths.app.data,
            PathBuf::from("/Users/treesap/.radroots/data/apps/app")
        );
        assert_eq!(
            paths.app.logs,
            PathBuf::from("/Users/treesap/.radroots/logs/apps/app")
        );
        assert_eq!(
            paths.app.data.join(APP_DATABASE_FILE_NAME),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app/app.sqlite3")
        );
        assert_eq!(
            paths.shared_accounts.data_root,
            PathBuf::from("/Users/treesap/.radroots/data/shared/accounts")
        );
        assert_eq!(
            paths.shared_accounts.secrets_root,
            PathBuf::from("/Users/treesap/.radroots/secrets/shared/accounts")
        );
        assert_eq!(
            paths.shared_accounts.store_path,
            PathBuf::from("/Users/treesap/.radroots/data/shared/accounts")
                .join(SHARED_ACCOUNTS_STORE_FILE_NAME)
        );
        assert_eq!(
            paths.shared_identity.default_identity_path,
            PathBuf::from("/Users/treesap/.radroots/secrets/shared/identities")
                .join(SHARED_IDENTITY_FILE_NAME)
        );
    }

    #[test]
    fn cloned_runtime_handles_shared_settings_state() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            shared_accounts_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();

        assert!(runtime.select_settings_section(SettingsSection::About));
        assert!(cloned_runtime.set_settings_preference(SettingsPreference::LaunchAtLogin, true));

        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::About
        );
        assert!(summary.shell_projection.settings.general.launch_at_login);
        assert_eq!(
            cloned_runtime.selected_settings_section(),
            SettingsSection::About
        );
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert!(
            summary
                .settings_account_projection
                .selected_account
                .is_none()
        );
    }

    #[test]
    fn replacing_today_agenda_is_shared_without_clobbering_home_shell() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            shared_accounts_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();
        let today_agenda = TodayAgendaProjection {
            farm: Some(FarmSummary {
                farm_id: radroots_studio_app_models::FarmId::new(),
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            }),
            summary: Some(TodaySummary {
                farm_id: radroots_studio_app_models::FarmId::new(),
                orders_needing_action: 2,
                low_stock_products: 1,
                draft_products: 3,
            }),
            setup_checklist: vec![TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: false,
            }],
            ..TodayAgendaProjection::default()
        };

        assert!(runtime.select_settings_section(SettingsSection::About));
        assert!(cloned_runtime.replace_today_agenda(today_agenda.clone()));

        let summary = runtime.summary();

        assert_eq!(summary.today_projection, today_agenda);
        assert_eq!(
            summary.shell_projection.active_surface,
            radroots_studio_app_models::ActiveSurface::Personal
        );
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::About
        );
        assert!(summary.today_projection.needs_setup());
    }

    #[test]
    fn degraded_runtime_surfaces_startup_issue_with_default_today_projection() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState::degraded(
            super::DesktopAppRuntimeBootstrapError::State(AppStateStoreError::Repository(
                AppStateRepositoryError::load("state unavailable"),
            )),
        ));

        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.active_surface,
            radroots_studio_app_models::ActiveSurface::Personal
        );
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::Account
        );
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert_eq!(summary.today_projection, TodayAgendaProjection::default());
        assert_eq!(
            summary.startup_issue.as_deref(),
            Some("app state repository load failed: state unavailable")
        );
    }

    #[test]
    fn runtime_records_activity_context_for_user_visible_actions() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            shared_accounts_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        });

        assert!(runtime.record_home_opened());
        assert!(runtime.record_settings_opened(SettingsSection::About));
        assert!(runtime.select_settings_section(SettingsSection::Settings));
        assert!(runtime.set_settings_preference(SettingsPreference::LaunchAtLogin, true));

        let context = runtime
            .activity_context(Some(8))
            .expect("activity context should load");

        assert_eq!(context.recent_events.len(), 4);
        assert_eq!(
            context.recent_events[0].kind,
            AppActivityKind::SettingsPreferenceUpdated {
                preference: SettingsPreference::LaunchAtLogin,
                enabled: true,
            }
        );
        assert_eq!(
            context.recent_events[1].kind,
            AppActivityKind::SettingsSectionSelected {
                section: SettingsSection::Settings,
            }
        );
        assert_eq!(
            context.recent_events[2].kind,
            AppActivityKind::SettingsOpened {
                section: SettingsSection::About,
            }
        );
        assert_eq!(context.recent_events[3].kind, AppActivityKind::HomeOpened);
    }

    #[test]
    fn runtime_account_commands_refresh_identity_projection() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("First".to_owned()))
                .expect("first account should generate")
        );
        let first_summary = runtime.summary();
        let first_account_id = first_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();

        assert!(
            runtime
                .generate_local_account(Some("Second".to_owned()))
                .expect("second account should generate")
        );
        let second_summary = runtime.summary();
        let second_account_id = second_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();
        assert_eq!(second_summary.settings_account_projection.roster.len(), 2);
        assert_eq!(
            second_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .and_then(|account| account.account.label.as_deref()),
            Some("Second")
        );

        save_surface_activation(
            &runtime,
            second_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        assert!(
            runtime
                .select_local_account(second_account_id.as_str())
                .expect("selection should succeed")
        );
        let selected_summary = runtime.summary();
        assert_eq!(selected_summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(
            selected_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );

        assert!(
            runtime
                .remove_selected_local_key()
                .expect("selected local key should remove")
        );
        let removed_summary = runtime.summary();
        assert_eq!(removed_summary.settings_account_projection.roster.len(), 1);
        assert_eq!(
            removed_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(first_account_id.as_str())
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(second_account_id.as_str())
                .expect("removed activation should load"),
            None
        );

        let imported_identity = RadrootsIdentity::generate();
        assert!(
            runtime
                .import_local_account(DesktopLocalIdentityImportRequest::raw_secret_key(
                    imported_identity.nsec(),
                ))
                .expect("raw import should succeed")
        );
        let imported_summary = runtime.summary();
        assert_eq!(imported_summary.settings_account_projection.roster.len(), 2);
        assert_eq!(
            imported_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(imported_identity.id().as_str())
        );
    }

    #[test]
    fn runtime_select_active_surface_persists_selected_surface() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        save_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer, true);
        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert_eq!(runtime.summary().startup_gate, AppStartupGate::Farmer);

        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should select")
        );
        let personal_summary = runtime.summary();
        assert_eq!(personal_summary.startup_gate, AppStartupGate::Personal);
        assert_eq!(
            personal_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Personal)
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(account_id.as_str())
                .expect("surface activation should load")
                .expect("surface activation should exist")
                .active_surface(),
            ActiveSurface::Personal
        );

        assert!(
            runtime
                .select_active_surface(ActiveSurface::Farmer)
                .expect("surface should reselect")
        );
        let farmer_summary = runtime.summary();
        assert_eq!(farmer_summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(
            farmer_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(account_id.as_str())
                .expect("surface activation should load")
                .expect("surface activation should exist")
                .active_surface(),
            ActiveSurface::Farmer
        );
    }

    #[test]
    fn runtime_reset_local_device_state_clears_store_file_and_projection() {
        let (runtime, paths) = file_backed_runtime("reset");

        assert!(
            runtime
                .generate_local_account(Some("First".to_owned()))
                .expect("first account should generate")
        );
        let first_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();
        assert!(
            runtime
                .generate_local_account(Some("Second".to_owned()))
                .expect("second account should generate")
        );
        let second_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();
        save_surface_activation(
            &runtime,
            first_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        save_surface_activation(
            &runtime,
            second_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        assert!(paths.store_path.exists());

        assert!(
            runtime
                .reset_local_device_state()
                .expect("device state should reset")
        );
        let summary = runtime.summary();

        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert!(
            summary
                .settings_account_projection
                .selected_account
                .is_none()
        );
        assert!(!paths.store_path.exists());
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(first_account_id.as_str())
                .expect("first activation should load"),
            None
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(second_account_id.as_str())
                .expect("second activation should load"),
            None
        );

        cleanup_paths(&paths);
    }

    #[test]
    fn runtime_account_commands_fail_closed_without_host_vault_manager() {
        let paths = temp_shared_accounts_paths("blocked");
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            shared_accounts_paths: Some(paths),
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        });

        let error = runtime
            .generate_local_account(Some("Blocked".to_owned()))
            .expect_err("blocked runtime should fail closed");

        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::HostVaultUnavailable
        ));
    }

    fn memory_runtime() -> DesktopAppRuntime {
        DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            shared_accounts_paths: None,
            accounts_manager: Some(
                RadrootsNostrAccountsManager::new(
                    Arc::new(RadrootsNostrMemoryAccountStore::new()),
                    Arc::new(RadrootsNostrSecretVaultMemory::new()),
                )
                .expect("memory manager should build"),
            ),
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        })
    }

    fn file_backed_runtime(label: &str) -> (DesktopAppRuntime, AppSharedAccountsPaths) {
        let paths = temp_shared_accounts_paths(label);
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");

        (
            DesktopAppRuntime::from_state(DesktopAppRuntimeState {
                state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                    .expect("in-memory state store should load"),
                shared_accounts_paths: Some(paths.clone()),
                accounts_manager: Some(
                    RadrootsNostrAccountsManager::new(
                        Arc::new(RadrootsNostrFileAccountStore::new(
                            paths.store_path.as_path(),
                        )),
                        Arc::new(RadrootsNostrSecretVaultMemory::new()),
                    )
                    .expect("file-backed manager should build"),
                ),
                sqlite_store: Some(
                    AppSqliteStore::open(DatabaseTarget::InMemory)
                        .expect("in-memory sqlite store should open"),
                ),
                startup_issue: None,
            }),
            paths,
        )
    }

    fn temp_shared_accounts_paths(label: &str) -> AppSharedAccountsPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("radroots_runtime_accounts_{label}_{suffix}"));

        AppSharedAccountsPaths {
            data_root: base.join("data/shared/accounts"),
            secrets_root: base.join("secrets/shared/accounts"),
            store_path: base.join("data/shared/accounts/store.json"),
        }
    }

    fn save_surface_activation(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        active_surface: ActiveSurface,
        farmer_active: bool,
    ) {
        let activation = AccountSurfaceActivationProjection::new(
            account_id,
            SelectedSurfaceProjection::new(active_surface),
            if farmer_active {
                FarmerActivationProjection::active(FarmId::new())
            } else {
                FarmerActivationProjection::inactive()
            },
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_surface_activation(&activation)
            .expect("surface activation should save");
    }

    fn cleanup_paths(paths: &AppSharedAccountsPaths) {
        let Some(base) = paths.data_root.ancestors().nth(3).map(PathBuf::from) else {
            return;
        };
        let _ = fs::remove_dir_all(base);
    }
}
