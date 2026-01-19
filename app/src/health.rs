#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppHealthCheckStatus {
    Ok,
    Error,
    Skipped,
}

impl AppHealthCheckStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            AppHealthCheckStatus::Ok => "ok",
            AppHealthCheckStatus::Error => "error",
            AppHealthCheckStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHealthCheckResult {
    pub status: AppHealthCheckStatus,
    pub message: Option<String>,
}

impl AppHealthCheckResult {
    pub fn ok() -> Self {
        Self {
            status: AppHealthCheckStatus::Ok,
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: AppHealthCheckStatus::Error,
            message: Some(message.into()),
        }
    }

    pub fn skipped() -> Self {
        Self {
            status: AppHealthCheckStatus::Skipped,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHealthReport {
    pub key_maps: AppHealthCheckResult,
    pub bootstrap_config: AppHealthCheckResult,
    pub bootstrap_app_data: AppHealthCheckResult,
    pub datastore_roundtrip: AppHealthCheckResult,
    pub keystore: AppHealthCheckResult,
}

impl Default for AppHealthReport {
    fn default() -> Self {
        Self {
            key_maps: AppHealthCheckResult::skipped(),
            bootstrap_config: AppHealthCheckResult::skipped(),
            bootstrap_app_data: AppHealthCheckResult::skipped(),
            datastore_roundtrip: AppHealthCheckResult::skipped(),
            keystore: AppHealthCheckResult::skipped(),
        }
    }
}

impl AppHealthReport {
    pub fn empty() -> Self {
        Self::default()
    }
}

use crate::{
    app_datastore_has_app_data,
    app_datastore_has_config,
    app_key_maps_validate,
    AppKeyMapConfig,
};
use radroots_studio_app_core::datastore::RadrootsClientDatastore;
use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

pub fn app_health_check_key_maps(key_maps: &AppKeyMapConfig) -> AppHealthCheckResult {
    match app_key_maps_validate(key_maps) {
        Ok(()) => AppHealthCheckResult::ok(),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

pub async fn app_health_check_bootstrap_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppHealthCheckResult {
    match app_datastore_has_config(datastore, key_maps).await {
        Ok(true) => AppHealthCheckResult::ok(),
        Ok(false) => AppHealthCheckResult::error("missing"),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

pub async fn app_health_check_bootstrap_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppHealthCheckResult {
    match app_datastore_has_app_data(datastore, key_maps).await {
        Ok(true) => AppHealthCheckResult::ok(),
        Ok(false) => AppHealthCheckResult::error("missing"),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

const APP_HEALTH_TEMP_KEY: &str = "radroots.health.temp";

pub async fn app_health_check_datastore_roundtrip<T: RadrootsClientDatastore>(
    datastore: &T,
) -> AppHealthCheckResult {
    let value = "ok";
    if let Err(err) = datastore.set(APP_HEALTH_TEMP_KEY, value).await {
        return AppHealthCheckResult::error(err.to_string());
    }
    match datastore.get(APP_HEALTH_TEMP_KEY).await {
        Ok(read) => {
            if read != value {
                return AppHealthCheckResult::error("mismatch");
            }
        }
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    }
    if let Err(err) = datastore.del(APP_HEALTH_TEMP_KEY).await {
        return AppHealthCheckResult::error(err.to_string());
    }
    AppHealthCheckResult::ok()
}

pub async fn app_health_check_keystore_access<T: RadrootsClientKeystoreNostr>(
    keystore: &T,
) -> AppHealthCheckResult {
    match keystore.keys().await {
        Ok(_) => AppHealthCheckResult::ok(),
        Err(RadrootsClientKeystoreError::NostrNoResults) => AppHealthCheckResult::ok(),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_health_check_key_maps,
        app_health_check_bootstrap_app_data,
        app_health_check_bootstrap_config,
        app_health_check_datastore_roundtrip,
        app_health_check_keystore_access,
        AppHealthCheckResult,
        AppHealthCheckStatus,
        AppHealthReport,
    };
    use crate::AppKeyMapConfig;
    use async_trait::async_trait;
    use radroots_studio_app_core::datastore::RadrootsClientWebDatastore;
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
        RadrootsClientWebKeystoreNostr,
    };

    #[test]
    fn health_status_as_str() {
        assert_eq!(AppHealthCheckStatus::Ok.as_str(), "ok");
        assert_eq!(AppHealthCheckStatus::Error.as_str(), "error");
        assert_eq!(AppHealthCheckStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn health_result_constructors() {
        let ok = AppHealthCheckResult::ok();
        assert_eq!(ok.status, AppHealthCheckStatus::Ok);
        assert!(ok.message.is_none());

        let err = AppHealthCheckResult::error("boom");
        assert_eq!(err.status, AppHealthCheckStatus::Error);
        assert_eq!(err.message.as_deref(), Some("boom"));
    }

    #[test]
    fn health_report_defaults_skipped() {
        let report = AppHealthReport::default();
        assert_eq!(report.key_maps.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_config.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_app_data.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.datastore_roundtrip.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.keystore.status, AppHealthCheckStatus::Skipped);
    }

    #[test]
    fn health_check_key_maps_reports_errors() {
        let empty = AppKeyMapConfig::empty();
        let result = app_health_check_key_maps(&empty);
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(
            result.message.as_deref(),
            Some("error.app.config.key_map_missing")
        );
    }

    #[test]
    fn health_check_bootstrap_reports_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_bootstrap_config(
            &datastore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        let result = futures::executor::block_on(app_health_check_bootstrap_app_data(
            &datastore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
    }

    #[test]
    fn health_check_roundtrip_reports_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let result =
            futures::executor::block_on(app_health_check_datastore_roundtrip(&datastore));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
    }

    struct TestKeystore {
        result: RadrootsClientKeystoreResult<Vec<String>>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientKeystoreNostr for TestKeystore {
        async fn generate(&self) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn add(&self, _secret_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn read(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
            self.result.clone()
        }

        async fn remove(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }
    }

    #[test]
    fn health_check_keystore_maps_empty_ok() {
        let keystore = TestKeystore {
            result: Err(RadrootsClientKeystoreError::NostrNoResults),
        };
        let result =
            futures::executor::block_on(app_health_check_keystore_access(&keystore));
        assert_eq!(result.status, AppHealthCheckStatus::Ok);
    }

    #[test]
    fn health_check_keystore_maps_idb_errors() {
        let keystore = RadrootsClientWebKeystoreNostr::new(None);
        let result =
            futures::executor::block_on(app_health_check_keystore_access(&keystore));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
    }
}
