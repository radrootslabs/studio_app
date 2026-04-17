use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use radroots_studio_app_core::{AppRuntimePathsError, AppRuntimeRoots};
use radroots_studio_app_models::{AppMode, SettingsSection, TodayAgendaProjection};
use radroots_studio_app_sqlite::{AppSqliteError, AppSqliteStore, DatabaseTarget};
use radroots_studio_app_state::{
    AppShellProjection, AppStateCommand, AppStateStore, AppStateStoreError,
    InMemoryAppStateRepository, SettingsPreference,
};
use thiserror::Error;

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
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::select_settings_section(section))
    }

    pub fn set_settings_preference(&self, preference: SettingsPreference, enabled: bool) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::SetSettingsPreference {
                preference,
                enabled,
            })
    }

    #[allow(dead_code)]
    pub fn replace_today_agenda(&self, projection: TodayAgendaProjection) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::replace_today_agenda(projection))
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
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntimeSummary {
    pub shell_projection: AppShellProjection,
    pub today_projection: TodayAgendaProjection,
    pub startup_issue: Option<String>,
}

#[derive(Debug)]
struct DesktopAppRuntimeState {
    state_store: AppStateStore<InMemoryAppStateRepository>,
    startup_issue: Option<String>,
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
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self {
            state_store: AppStateStore::in_memory(AppShellProjection {
                app_mode: AppMode::Farmer,
                ..AppShellProjection::default()
            }),
            startup_issue: Some(error.to_string()),
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
        AppMode, FarmReadiness, FarmSummary, SettingsSection, ShellSection, TodayAgendaProjection,
        TodaySetupTask, TodaySetupTaskKind, TodaySummary,
    };
    use radroots_studio_app_state::{
        AppStateRepositoryError, AppStateStore, AppStateStoreError, InMemoryAppStateRepository,
        SettingsPreference,
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
    }

    #[test]
    fn replacing_today_agenda_is_shared_without_clobbering_home_shell() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
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
        assert_eq!(summary.shell_projection.app_mode, AppMode::Farmer);
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

        assert_eq!(summary.shell_projection.app_mode, AppMode::Farmer);
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::Account
        );
        assert_eq!(summary.today_projection, TodayAgendaProjection::default());
        assert_eq!(
            summary.startup_issue.as_deref(),
            Some("app state repository load failed: state unavailable")
        );
    }
}
