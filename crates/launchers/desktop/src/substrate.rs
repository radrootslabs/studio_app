use std::path::PathBuf;

use radroots_studio_app_models::AppMode;
use radroots_studio_app_sqlite::{AppSqliteError, AppSqliteStore, DatabaseTarget};
use radroots_studio_app_state::{
    AppShellProjection, AppStateStore, AppStateStoreError, InMemoryAppStateRepository,
};
use radroots_studio_app_sync::{AppSyncProjection, SyncCheckpointStatus, SyncConflictStatus};
use radroots_runtime_paths::{
    RadrootsPathOverrides, RadrootsPathProfile, RadrootsPathResolver, RadrootsRuntimeNamespace,
    RadrootsRuntimePathsError,
};
use thiserror::Error;

const APP_DATABASE_FILE_NAME: &str = "app.sqlite3";

#[derive(Clone, Debug)]
pub struct DesktopAppSubstrateSummary {
    pub data_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
    pub database_path: Option<PathBuf>,
    pub sqlite_schema_version: Option<u32>,
    pub shell_projection: AppShellProjection,
    pub sync_projection: AppSyncProjection,
    pub startup_issue: Option<String>,
}

impl DesktopAppSubstrateSummary {
    pub fn bootstrap() -> Self {
        match Self::try_bootstrap() {
            Ok(summary) => summary,
            Err(error) => Self::degraded(error),
        }
    }

    fn try_bootstrap() -> Result<Self, DesktopAppSubstrateError> {
        let namespace = RadrootsRuntimeNamespace::app("app")?;
        let roots = RadrootsPathResolver::current()
            .resolve(
                RadrootsPathProfile::InteractiveUser,
                &RadrootsPathOverrides::default(),
            )?
            .namespaced(&namespace);
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
            shell_projection: shell_store.projection().clone(),
            sync_projection,
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppSubstrateError) -> Self {
        Self {
            data_dir: None,
            logs_dir: None,
            database_path: None,
            sqlite_schema_version: None,
            shell_projection: AppShellProjection {
                app_mode: AppMode::Farmer,
                ..AppShellProjection::default()
            },
            sync_projection: AppSyncProjection::default(),
            startup_issue: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Error)]
enum DesktopAppSubstrateError {
    #[error(transparent)]
    RuntimePaths(#[from] RadrootsRuntimePathsError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    State(#[from] AppStateStoreError),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use radroots_runtime_paths::{
        RadrootsHostEnvironment, RadrootsPathOverrides, RadrootsPathProfile, RadrootsPathResolver,
        RadrootsPlatform, RadrootsRuntimeNamespace,
    };

    use super::APP_DATABASE_FILE_NAME;

    #[test]
    fn desktop_namespace_uses_canonical_app_data_root() {
        let namespace = RadrootsRuntimeNamespace::app("app").expect("app namespace should parse");
        let resolver = RadrootsPathResolver::new(
            RadrootsPlatform::Macos,
            RadrootsHostEnvironment {
                home_dir: Some(PathBuf::from("/Users/treesap")),
                ..RadrootsHostEnvironment::default()
            },
        );
        let namespaced = resolver
            .resolve(
                RadrootsPathProfile::InteractiveUser,
                &RadrootsPathOverrides::default(),
            )
            .expect("interactive user roots should resolve")
            .namespaced(&namespace);

        assert_eq!(
            namespaced.data,
            PathBuf::from("/Users/treesap/.radroots/data/apps/app")
        );
        assert_eq!(
            namespaced.logs,
            PathBuf::from("/Users/treesap/.radroots/logs/apps/app")
        );
        assert_eq!(
            namespaced.data.join(APP_DATABASE_FILE_NAME),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app/app.sqlite3")
        );
    }
}
