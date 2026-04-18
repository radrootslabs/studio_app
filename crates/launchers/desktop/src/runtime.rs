use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use radroots_studio_app_core::{AppRuntimePathsError, AppRuntimeRoots};
use radroots_studio_app_models::{
    AppActivityContext, AppActivityKind, AppStartupGate, SettingsAccountProjection,
    SettingsPreference, SettingsSection, TodayAgendaProjection,
};
use radroots_studio_app_sqlite::{
    APP_ACTIVITY_CONTEXT_LIMIT, AppSqliteError, AppSqliteStore, DatabaseTarget,
};
use radroots_studio_app_state::{
    AppShellProjection, AppStateCommand, AppStateStore, AppStateStoreError,
    InMemoryAppStateRepository,
};
use thiserror::Error;
use tracing::error;

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
    sqlite_store: Option<AppSqliteStore>,
    startup_issue: Option<String>,
}

impl fmt::Debug for DesktopAppRuntimeState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DesktopAppRuntimeState")
            .field("state_store", &self.state_store)
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
        let roots = AppRuntimeRoots::current_desktop()?;
        let database_path = roots.data.join(APP_DATABASE_FILE_NAME);
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::Path(database_path.clone()))?;
        let mut state_store = AppStateStore::load(InMemoryAppStateRepository::default())?;
        let today_projection = sqlite_store.load_today_agenda(None)?;
        let _ =
            state_store.apply_in_memory(AppStateCommand::replace_today_agenda(today_projection));

        Ok(Self {
            state_store,
            sqlite_store: Some(sqlite_store),
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self {
            state_store: AppStateStore::in_memory(AppShellProjection::default()),
            sqlite_store: None,
            startup_issue: Some(error.to_string()),
        }
    }

    fn record_activity(&self, kind: AppActivityKind) -> Result<(), AppSqliteError> {
        match self.sqlite_store.as_ref() {
            Some(store) => store.record_activity_event(&kind),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Error)]
enum DesktopAppRuntimeBootstrapError {
    #[error(transparent)]
    RuntimePaths(#[from] AppRuntimePathsError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    State(#[from] AppStateStoreError),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use radroots_studio_app_core::{AppRuntimeHostEnvironment, AppRuntimePlatform, AppRuntimeRoots};
    use radroots_studio_app_models::{
        AppActivityKind, AppStartupGate, FarmReadiness, FarmSummary, SettingsPreference,
        SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
        TodaySummary,
    };
    use radroots_studio_app_sqlite::{AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_state::{
        AppStateRepositoryError, AppStateStore, AppStateStoreError, InMemoryAppStateRepository,
    };

    use super::{APP_DATABASE_FILE_NAME, DesktopAppRuntime, DesktopAppRuntimeState};

    #[test]
    fn desktop_namespace_uses_canonical_app_data_root() {
        let roots = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(PathBuf::from("/Users/treesap")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("interactive user roots should resolve");

        assert_eq!(
            roots.data,
            PathBuf::from("/Users/treesap/.radroots/data/apps/app")
        );
        assert_eq!(
            roots.logs,
            PathBuf::from("/Users/treesap/.radroots/logs/apps/app")
        );
        assert_eq!(
            roots.data.join(APP_DATABASE_FILE_NAME),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app/app.sqlite3")
        );
    }

    #[test]
    fn cloned_runtime_handles_shared_settings_state() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
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
}
