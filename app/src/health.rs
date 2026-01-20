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
    pub app_data_active_key: AppHealthCheckResult,
    pub notifications: AppHealthCheckResult,
    pub tangle: AppHealthCheckResult,
    pub datastore_roundtrip: AppHealthCheckResult,
    pub keystore: AppHealthCheckResult,
}

impl Default for AppHealthReport {
    fn default() -> Self {
        Self {
            key_maps: AppHealthCheckResult::skipped(),
            bootstrap_config: AppHealthCheckResult::skipped(),
            bootstrap_app_data: AppHealthCheckResult::skipped(),
            app_data_active_key: AppHealthCheckResult::skipped(),
            notifications: AppHealthCheckResult::skipped(),
            tangle: AppHealthCheckResult::skipped(),
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
    app_datastore_key_nostr_key,
    app_datastore_read_app_data,
    app_log_buffer_flush_critical,
    app_log_debug_emit,
    app_log_entry_new,
    app_log_entry_record,
    app_key_maps_validate,
    AppNotifications,
    AppLogLevel,
    AppTangleClient,
    AppKeyMapConfig,
};
use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;
use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};
use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

fn log_health_context(result: &AppHealthCheckResult) -> Option<String> {
    match result.message.as_deref() {
        Some(message) => Some(format!("status={},detail={message}", result.status.as_str())),
        None => Some(format!("status={}", result.status.as_str())),
    }
}

fn log_health_start(name: &str) {
    let _ = app_log_debug_emit("log.app.health.start", name, None);
}

fn log_health_end(name: &str, result: &AppHealthCheckResult) {
    let context = log_health_context(result);
    if result.status == AppHealthCheckStatus::Error {
        let entry = app_log_entry_new(AppLogLevel::Error, "log.app.health.end", name, context);
        let _ = app_log_entry_record(entry);
    } else {
        let _ = app_log_debug_emit("log.app.health.end", name, context);
    }
}

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

pub async fn app_health_check_app_data_active_key<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppHealthCheckResult {
    let app_data = match app_datastore_read_app_data(datastore, key_maps).await {
        Ok(value) => value,
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    };
    if app_data.active_key.is_empty() {
        return AppHealthCheckResult::error("missing");
    }
    let key_name = match app_datastore_key_nostr_key(key_maps) {
        Ok(value) => value,
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    };
    let stored = match datastore.get(key_name).await {
        Ok(value) => value,
        Err(RadrootsClientDatastoreError::NoResult) => return AppHealthCheckResult::error("missing"),
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    };
    if stored != app_data.active_key {
        return AppHealthCheckResult::error("mismatch");
    }
    AppHealthCheckResult::ok()
}

pub async fn app_health_check_notifications(
    notifications: &AppNotifications,
) -> AppHealthCheckResult {
    match notifications.permission().await {
        Ok(permission) => app_health_check_notifications_permission(permission),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

fn app_health_check_notifications_permission(
    permission: RadrootsClientNotificationsPermission,
) -> AppHealthCheckResult {
    match permission {
        RadrootsClientNotificationsPermission::Granted => AppHealthCheckResult::ok(),
        RadrootsClientNotificationsPermission::Denied
        | RadrootsClientNotificationsPermission::Default => AppHealthCheckResult::skipped(),
        RadrootsClientNotificationsPermission::Unavailable => {
            AppHealthCheckResult::error(permission.as_str())
        }
    }
}

pub async fn app_health_check_notifications_with_state(
    notifications: &AppNotifications,
    stored_permission: Option<&str>,
) -> AppHealthCheckResult {
    if let Some(value) = stored_permission {
        if let Some(permission) = RadrootsClientNotificationsPermission::parse(value) {
            return app_health_check_notifications_permission(permission);
        }
    }
    app_health_check_notifications(notifications).await
}

pub fn app_health_check_tangle<T: AppTangleClient>(tangle: &T) -> AppHealthCheckResult {
    match tangle.init() {
        Ok(()) => AppHealthCheckResult::ok(),
        Err(crate::AppTangleError::NotImplemented) => AppHealthCheckResult::skipped(),
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

pub async fn app_health_check_keystore_access<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr>(
    datastore: &T,
    keystore: &K,
    key_maps: &AppKeyMapConfig,
) -> AppHealthCheckResult {
    let key_name = match app_datastore_key_nostr_key(key_maps) {
        Ok(value) => value,
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    };
    let public_key = match datastore.get(key_name).await {
        Ok(value) if !value.is_empty() => value,
        Ok(_) => return AppHealthCheckResult::error("missing"),
        Err(RadrootsClientDatastoreError::NoResult) => return AppHealthCheckResult::error("missing"),
        Err(err) => return AppHealthCheckResult::error(err.to_string()),
    };
    match keystore.read(&public_key).await {
        Ok(_) => AppHealthCheckResult::ok(),
        Err(RadrootsClientKeystoreError::MissingKey) => AppHealthCheckResult::error("missing"),
        Err(RadrootsClientKeystoreError::NostrNoResults) => AppHealthCheckResult::error("missing"),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

pub async fn app_health_check_all<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr, G: AppTangleClient>(
    datastore: &T,
    keystore: &K,
    notifications: &AppNotifications,
    tangle: &G,
    key_maps: &AppKeyMapConfig,
) -> AppHealthReport {
    log_health_start("key_maps");
    let key_maps_result = app_health_check_key_maps(key_maps);
    log_health_end("key_maps", &key_maps_result);
    log_health_start("bootstrap_config");
    let bootstrap_config = app_health_check_bootstrap_config(datastore, key_maps).await;
    log_health_end("bootstrap_config", &bootstrap_config);
    log_health_start("bootstrap_app_data");
    let bootstrap_app_data = app_health_check_bootstrap_app_data(datastore, key_maps).await;
    log_health_end("bootstrap_app_data", &bootstrap_app_data);
    log_health_start("app_data_active_key");
    let app_data_active_key = app_health_check_app_data_active_key(datastore, key_maps).await;
    log_health_end("app_data_active_key", &app_data_active_key);
    log_health_start("notifications");
    let stored_permission = app_datastore_read_app_data(datastore, key_maps)
        .await
        .ok()
        .and_then(|data| data.notifications_permission);
    let notifications_result =
        app_health_check_notifications_with_state(notifications, stored_permission.as_deref())
            .await;
    log_health_end("notifications", &notifications_result);
    log_health_start("tangle");
    let tangle_result = app_health_check_tangle(tangle);
    log_health_end("tangle", &tangle_result);
    log_health_start("datastore_roundtrip");
    let datastore_roundtrip = app_health_check_datastore_roundtrip(datastore).await;
    log_health_end("datastore_roundtrip", &datastore_roundtrip);
    log_health_start("keystore");
    let keystore_result = app_health_check_keystore_access(datastore, keystore, key_maps).await;
    log_health_end("keystore", &keystore_result);
    AppHealthReport {
        key_maps: key_maps_result,
        bootstrap_config,
        bootstrap_app_data,
        app_data_active_key,
        notifications: notifications_result,
        tangle: tangle_result,
        datastore_roundtrip,
        keystore: keystore_result,
    }
}

pub async fn app_health_check_all_logged<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr, G: AppTangleClient>(
    datastore: &T,
    keystore: &K,
    notifications: &AppNotifications,
    tangle: &G,
    key_maps: &AppKeyMapConfig,
) -> AppHealthReport {
    let report = app_health_check_all(datastore, keystore, notifications, tangle, key_maps).await;
    let _ = app_log_buffer_flush_critical(datastore, key_maps).await;
    report
}

#[cfg(test)]
mod tests {
    use super::{
        app_health_check_app_data_active_key,
        app_health_check_all,
        app_health_check_all_logged,
        app_health_check_key_maps,
        app_health_check_bootstrap_app_data,
        app_health_check_bootstrap_config,
        app_health_check_datastore_roundtrip,
        app_health_check_keystore_access,
        app_health_check_notifications,
        app_health_check_notifications_with_state,
        app_health_check_notifications_permission,
        app_health_check_tangle,
        log_health_context,
        AppHealthCheckResult,
        AppHealthCheckStatus,
        AppHealthReport,
    };
    use crate::app_log_buffer_drain;
    use crate::AppKeyMapConfig;
    use async_trait::async_trait;
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreEntry,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
        RadrootsClientWebDatastore,
    };
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
        RadrootsClientWebKeystoreNostr,
    };
    use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;
    use radroots_studio_app_core::idb::IDB_CONFIG_DATASTORE;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::idb::RadrootsClientIdbConfig;
    use std::sync::Mutex;

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
    fn health_log_context_formats_error_detail() {
        let result = AppHealthCheckResult::error("missing");
        let context = log_health_context(&result);
        assert_eq!(context.as_deref(), Some("status=error,detail=missing"));
    }

    #[test]
    fn health_report_defaults_skipped() {
        let report = AppHealthReport::default();
        assert_eq!(report.key_maps.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_config.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_app_data.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.app_data_active_key.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.notifications.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.tangle.status, AppHealthCheckStatus::Skipped);
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

    struct TestDatastore {
        get_result: RadrootsClientDatastoreResult<String>,
        app_data: Option<crate::AppAppData>,
    }

    fn datastore_err<T>() -> RadrootsClientDatastoreResult<T> {
        Err(RadrootsClientDatastoreError::IdbUndefined)
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for TestDatastore {
        fn get_config(&self) -> RadrootsClientIdbConfig {
            IDB_CONFIG_DATASTORE
        }

        fn get_store_id(&self) -> &str {
            "test"
        }

        async fn init(&self) -> RadrootsClientDatastoreResult<()> {
            datastore_err()
        }

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            datastore_err()
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            self.get_result.clone()
        }

        async fn set_obj<T>(&self, _key: &str, _value: &T) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::Serialize + serde::de::DeserializeOwned + Clone,
        {
            datastore_err()
        }

        async fn update_obj<T>(&self, _key: &str, _value: &T) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::Serialize + serde::de::DeserializeOwned + Clone,
        {
            datastore_err()
        }

        async fn get_obj<T>(&self, _key: &str) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::de::DeserializeOwned,
        {
            let Some(data) = self.app_data.as_ref() else {
                return Err(RadrootsClientDatastoreError::NoResult);
            };
            let serialized =
                serde_json::to_string(data).map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            serde_json::from_str(&serialized)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            datastore_err()
        }

        async fn del(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            datastore_err()
        }

        async fn del_pref(&self, _key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>> {
            datastore_err()
        }

        async fn set_param(
            &self,
            _key: &str,
            _key_param: &str,
            _value: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            datastore_err()
        }

        async fn get_param(
            &self,
            _key: &str,
            _key_param: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            datastore_err()
        }

        async fn keys(&self) -> RadrootsClientDatastoreResult<Vec<String>> {
            datastore_err()
        }

        async fn entries(&self) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            datastore_err()
        }

        async fn reset(&self) -> RadrootsClientDatastoreResult<()> {
            datastore_err()
        }

        async fn export_backup(
            &self,
        ) -> RadrootsClientDatastoreResult<RadrootsClientBackupDatastorePayload> {
            datastore_err()
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupDatastorePayload,
        ) -> RadrootsClientDatastoreResult<()> {
            datastore_err()
        }
    }

    struct TestKeystore {
        read_result: RadrootsClientKeystoreResult<String>,
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
            self.read_result.clone()
        }

        async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn remove(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }
    }

    #[test]
    fn health_check_keystore_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let keystore = RadrootsClientWebKeystoreNostr::new(None);
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_keystore_access(
            &datastore,
            &keystore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
    }

    #[test]
    fn health_check_keystore_reports_missing_datastore_key() {
        let datastore = TestDatastore {
            get_result: Err(RadrootsClientDatastoreError::NoResult),
            app_data: None,
        };
        let keystore = TestKeystore {
            read_result: Err(RadrootsClientKeystoreError::MissingKey),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_keystore_access(
            &datastore,
            &keystore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(result.message.as_deref(), Some("missing"));
    }

    #[test]
    fn health_check_keystore_reports_missing_keystore_key() {
        let datastore = TestDatastore {
            get_result: Ok("pub".to_string()),
            app_data: None,
        };
        let keystore = TestKeystore {
            read_result: Err(RadrootsClientKeystoreError::MissingKey),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_keystore_access(
            &datastore,
            &keystore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(result.message.as_deref(), Some("missing"));
    }

    #[test]
    fn health_check_keystore_accepts_matching_key() {
        let datastore = TestDatastore {
            get_result: Ok("pub".to_string()),
            app_data: None,
        };
        let keystore = TestKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_keystore_access(
            &datastore,
            &keystore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Ok);
    }

    #[test]
    fn health_check_app_data_requires_active_key() {
        let datastore = TestDatastore {
            get_result: Ok("pub".to_string()),
            app_data: Some(crate::AppAppData::default()),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_app_data_active_key(
            &datastore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(result.message.as_deref(), Some("missing"));
    }

    #[test]
    fn health_check_app_data_detects_mismatch() {
        let mut app_data = crate::AppAppData::default();
        app_data.active_key = "other".to_string();
        let datastore = TestDatastore {
            get_result: Ok("pub".to_string()),
            app_data: Some(app_data),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_app_data_active_key(
            &datastore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(result.message.as_deref(), Some("mismatch"));
    }

    #[test]
    fn health_check_app_data_accepts_match() {
        let mut app_data = crate::AppAppData::default();
        app_data.active_key = "pub".to_string();
        let datastore = TestDatastore {
            get_result: Ok("pub".to_string()),
            app_data: Some(app_data),
        };
        let key_maps = crate::app_key_maps_default();
        let result = futures::executor::block_on(app_health_check_app_data_active_key(
            &datastore,
            &key_maps,
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Ok);
    }

    #[test]
    fn health_check_all_reports_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let keystore = RadrootsClientWebKeystoreNostr::new(None);
        let notifications = crate::AppNotifications::new(None);
        let tangle = crate::AppTangleClientStub::new();
        let key_maps = crate::app_key_maps_default();
        let report = futures::executor::block_on(app_health_check_all(
            &datastore,
            &keystore,
            &notifications,
            &tangle,
            &key_maps,
        ));
        assert_eq!(report.key_maps.status, AppHealthCheckStatus::Ok);
        assert_eq!(report.bootstrap_config.status, AppHealthCheckStatus::Error);
        assert_eq!(report.bootstrap_app_data.status, AppHealthCheckStatus::Error);
        assert_eq!(report.app_data_active_key.status, AppHealthCheckStatus::Error);
        assert_eq!(report.notifications.status, AppHealthCheckStatus::Error);
        assert_eq!(report.tangle.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.datastore_roundtrip.status, AppHealthCheckStatus::Error);
        assert_eq!(report.keystore.status, AppHealthCheckStatus::Error);
    }

    #[test]
    fn health_check_notifications_reports_unavailable() {
        let notifications = crate::AppNotifications::new(None);
        let result =
            futures::executor::block_on(app_health_check_notifications(&notifications));
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(result.message.as_deref(), Some("unavailable"));
    }

    #[test]
    fn health_check_notifications_skips_default_and_denied() {
        let default_result =
            app_health_check_notifications_permission(RadrootsClientNotificationsPermission::Default);
        assert_eq!(default_result.status, AppHealthCheckStatus::Skipped);
        let denied_result =
            app_health_check_notifications_permission(RadrootsClientNotificationsPermission::Denied);
        assert_eq!(denied_result.status, AppHealthCheckStatus::Skipped);
    }

    #[test]
    fn health_check_notifications_uses_stored_permission() {
        let notifications = crate::AppNotifications::new(None);
        let result = futures::executor::block_on(app_health_check_notifications_with_state(
            &notifications,
            Some("granted"),
        ));
        assert_eq!(result.status, AppHealthCheckStatus::Ok);
    }

    #[test]
    fn health_check_tangle_reports_not_implemented() {
        let tangle = crate::AppTangleClientStub::new();
        let result = app_health_check_tangle(&tangle);
        assert_eq!(result.status, AppHealthCheckStatus::Skipped);
        assert!(result.message.is_none());
    }

    struct FlushDatastore {
        entries: Mutex<Vec<RadrootsClientDatastoreEntry>>,
    }

    impl FlushDatastore {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }

        fn entry_len(&self) -> usize {
            self.entries.lock().unwrap_or_else(|err| err.into_inner()).len()
        }
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for FlushDatastore {
        fn get_config(&self) -> RadrootsClientIdbConfig {
            IDB_CONFIG_DATASTORE
        }

        fn get_store_id(&self) -> &str {
            "test"
        }

        async fn init(&self) -> RadrootsClientDatastoreResult<()> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn set_obj<T>(&self, key: &str, value: &T) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::Serialize + serde::de::DeserializeOwned + Clone,
        {
            let serialized =
                serde_json::to_string(value).map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            let mut entries = self.entries.lock().unwrap_or_else(|err| err.into_inner());
            entries.push(RadrootsClientDatastoreEntry::new(
                key.to_string(),
                Some(serialized),
            ));
            Ok(value.clone())
        }

        async fn update_obj<T>(&self, _key: &str, _value: &T) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::Serialize + serde::de::DeserializeOwned + Clone,
        {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get_obj<T>(&self, _key: &str) -> RadrootsClientDatastoreResult<T>
        where
            T: serde::de::DeserializeOwned,
        {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
            let mut entries = self.entries.lock().unwrap_or_else(|err| err.into_inner());
            entries.retain(|entry| entry.key != key);
            Ok(key.to_string())
        }

        async fn del_pref(&self, _key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn set_param(
            &self,
            _key: &str,
            _key_param: &str,
            _value: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get_param(
            &self,
            _key: &str,
            _key_param: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn keys(&self) -> RadrootsClientDatastoreResult<Vec<String>> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn entries(&self) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            let entries = self.entries.lock().unwrap_or_else(|err| err.into_inner());
            Ok(entries.clone())
        }

        async fn reset(&self) -> RadrootsClientDatastoreResult<()> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn export_backup(
            &self,
        ) -> RadrootsClientDatastoreResult<RadrootsClientBackupDatastorePayload> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupDatastorePayload,
        ) -> RadrootsClientDatastoreResult<()> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }
    }

    #[test]
    fn health_check_all_logged_flushes_buffer() {
        let _ = app_log_buffer_drain();
        let datastore = FlushDatastore::new();
        let keystore = TestKeystore {
            read_result: Err(RadrootsClientKeystoreError::MissingKey),
        };
        let notifications = crate::AppNotifications::new(None);
        let tangle = crate::AppTangleClientStub::new();
        let key_maps = crate::app_key_maps_default();
        let report = futures::executor::block_on(app_health_check_all_logged(
            &datastore,
            &keystore,
            &notifications,
            &tangle,
            &key_maps,
        ));
        assert_eq!(report.key_maps.status, AppHealthCheckStatus::Ok);
        assert!(datastore.entry_len() > 0);
    }
}
