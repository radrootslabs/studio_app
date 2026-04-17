use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use radroots_studio_app_core::{AppRuntimePathsError, AppRuntimeRoots};
use radroots_studio_app_models::{AppMode, SettingsSection};
use radroots_studio_app_sqlite::{AppSqliteError, AppSqliteStore, DatabaseTarget};
use radroots_studio_app_state::{
    AppShellCommand, AppShellProjection, AppStateStore, AppStateStoreError,
    InMemoryAppStateRepository, SettingsPreference,
};
use radroots_studio_app_sync::{AppSyncProjection, SyncCheckpointStatus, SyncConflictStatus};
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
            data_dir: state.data_dir.clone(),
            logs_dir: state.logs_dir.clone(),
            database_path: state.database_path.clone(),
            sqlite_schema_version: state.sqlite_schema_version,
            shell_projection: state.shell_store.projection().clone(),
            sync_projection: state.sync_projection.clone(),
            startup_issue: state.startup_issue.clone(),
        }
    }

    pub fn selected_settings_section(&self) -> SettingsSection {
        self.lock_state()
            .shell_store
            .projection()
            .settings
            .selected_section
    }

    pub fn select_settings_section(&self, section: SettingsSection) -> bool {
        self.lock_state_mut()
            .shell_store
            .apply_in_memory(AppShellCommand::select_settings_section(section))
    }

    pub fn set_settings_preference(&self, preference: SettingsPreference, enabled: bool) -> bool {
        self.lock_state_mut()
            .shell_store
            .apply_in_memory(AppShellCommand::SetSettingsPreference {
                preference,
                enabled,
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
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntimeSummary {
    pub data_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
    pub database_path: Option<PathBuf>,
    pub sqlite_schema_version: Option<u32>,
    pub shell_projection: AppShellProjection,
    pub sync_projection: AppSyncProjection,
    pub startup_issue: Option<String>,
}

#[derive(Debug)]
struct DesktopAppRuntimeState {
    data_dir: Option<PathBuf>,
    logs_dir: Option<PathBuf>,
    database_path: Option<PathBuf>,
    sqlite_schema_version: Option<u32>,
    shell_store: AppStateStore<InMemoryAppStateRepository>,
    sync_projection: AppSyncProjection,
    startup_issue: Option<String>,
}

impl DesktopAppRuntimeState {
    fn try_bootstrap() -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let roots = AppRuntimeRoots::current_desktop()?;
        let database_path = roots.data.join(APP_DATABASE_FILE_NAME);
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::Path(database_path.clone()))?;
        let shell_store = AppStateStore::load(InMemoryAppStateRepository::default())?;
        let sync_projection = AppSyncProjection {
            checkpoint: SyncCheckpointStatus::never_synced(),
            conflict_status: SyncConflictStatus::clear(),
            ..AppSyncProjection::default()
        };

        Ok(Self {
            data_dir: Some(roots.data),
            logs_dir: Some(roots.logs),
            database_path: Some(database_path),
            sqlite_schema_version: Some(sqlite_store.schema_version()?),
            shell_store,
            sync_projection,
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self {
            data_dir: None,
            logs_dir: None,
            database_path: None,
            sqlite_schema_version: None,
            shell_store: AppStateStore::in_memory(AppShellProjection {
                app_mode: AppMode::Farmer,
                ..AppShellProjection::default()
            }),
            sync_projection: AppSyncProjection::default(),
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
    use radroots_studio_app_state::{AppStateStore, InMemoryAppStateRepository, SettingsPreference};
    use radroots_studio_app_sync::AppSyncProjection;

    use super::{
        APP_DATABASE_FILE_NAME, DesktopAppRuntime, DesktopAppRuntimeState, SettingsSection,
    };

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
    fn cloned_runtime_handles_share_shell_state() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            data_dir: None,
            logs_dir: None,
            database_path: None,
            sqlite_schema_version: None,
            shell_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            sync_projection: AppSyncProjection::default(),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();

        assert!(runtime.select_settings_section(SettingsSection::About));
        assert!(cloned_runtime.set_settings_preference(SettingsPreference::LaunchAtLogin, true));

        let summary = runtime.summary();
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
}
