#![forbid(unsafe_code)]

use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};

use crate::{
    app_datastore_param_key,
    AppConfigError,
    AppInitAssetError,
    AppInitError,
    AppKeystoreError,
    AppKeyMapConfig,
    AppNotificationsError,
    AppTangleError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppLogMetadata {
    pub app_name: String,
    pub app_version: String,
    pub app_hash: String,
    pub target: String,
}

impl Default for AppLogMetadata {
    fn default() -> Self {
        let app_name = String::from(env!("CARGO_PKG_NAME"));
        let app_version = String::from(env!("CARGO_PKG_VERSION"));
        let app_hash = String::from(option_env!("RADROOTS_GIT_HASH").unwrap_or("unknown"));
        let target = if cfg!(target_arch = "wasm32") {
            String::from("wasm32")
        } else {
            String::from("native")
        };
        Self {
            app_name,
            app_version,
            app_hash,
            target,
        }
    }
}

static LOG_META: OnceLock<AppLogMetadata> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl AppLogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            AppLogLevel::Debug => "debug",
            AppLogLevel::Info => "info",
            AppLogLevel::Warn => "warn",
            AppLogLevel::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppLogEntry {
    pub id: String,
    pub timestamp_ms: i64,
    pub level: AppLogLevel,
    pub code: String,
    pub message: String,
    pub context: Option<String>,
    pub metadata: AppLogMetadata,
}

pub trait AppLoggableError: std::fmt::Display {
    fn log_code(&self) -> &'static str;
    fn log_context(&self) -> Option<String> {
        None
    }
}

impl AppLoggableError for AppInitAssetError {
    fn log_code(&self) -> &'static str {
        self.message()
    }
}

impl AppLoggableError for AppConfigError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            AppConfigError::MissingKeyMap(key) => Some(format!("key_map={key}")),
            AppConfigError::MissingParamMap(key) => Some(format!("param_map={key}")),
            AppConfigError::MissingObjMap(key) => Some(format!("obj_map={key}")),
            AppConfigError::MissingKeystoreKeyMap(key) => Some(format!("keystore_map={key}")),
        }
    }
}

impl AppLoggableError for AppInitError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            AppInitError::Idb(err) => Some(err.to_string()),
            AppInitError::Datastore(err) => Some(err.to_string()),
            AppInitError::Keystore(err) => Some(err.to_string()),
            AppInitError::Config(err) => err.log_context().or_else(|| Some(err.message().to_string())),
            AppInitError::Assets(err) => Some(err.message().to_string()),
        }
    }
}

impl AppLoggableError for AppKeystoreError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            AppKeystoreError::Keystore(err) => Some(err.to_string()),
        }
    }
}

impl AppLoggableError for AppNotificationsError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            AppNotificationsError::Notifications(err) => Some(err.message().to_string()),
        }
    }
}

impl AppLoggableError for AppTangleError {
    fn log_code(&self) -> &'static str {
        self.message()
    }
}

#[derive(Debug)]
pub enum AppLogError {
    Config(AppConfigError),
    Datastore(RadrootsClientDatastoreError),
}

pub type AppLogResult<T> = Result<T, AppLogError>;

impl std::fmt::Display for AppLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppLogError::Config(err) => write!(f, "{err}"),
            AppLogError::Datastore(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AppLogError {}

impl From<AppConfigError> for AppLogError {
    fn from(err: AppConfigError) -> Self {
        AppLogError::Config(err)
    }
}

impl From<RadrootsClientDatastoreError> for AppLogError {
    fn from(err: RadrootsClientDatastoreError) -> Self {
        AppLogError::Datastore(err)
    }
}

pub fn app_log_timestamp_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis() as i64)
            .unwrap_or(0)
    }
}

pub fn app_log_entry_error<E: AppLoggableError>(err: &E) -> AppLogEntry {
    AppLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: app_log_timestamp_ms(),
        level: AppLogLevel::Error,
        code: err.log_code().to_string(),
        message: err.to_string(),
        context: err.log_context(),
        metadata: app_log_metadata().clone(),
    }
}

pub fn app_log_entry_emit(entry: &AppLogEntry) {
    let payload = serde_json::to_string(entry)
        .unwrap_or_else(|_| format!("{}: {}", entry.code, entry.message));
    match entry.level {
        AppLogLevel::Error => radroots_log::log_error(payload),
        AppLogLevel::Warn => radroots_log::log_info(payload),
        AppLogLevel::Info => radroots_log::log_info(payload),
        AppLogLevel::Debug => radroots_log::log_debug(payload),
    }
}

pub fn app_log_error_emit<E: AppLoggableError>(err: &E) -> AppLogEntry {
    let entry = app_log_entry_error(err);
    app_log_entry_emit(&entry);
    entry
}

pub fn app_log_error_key(
    key_maps: &AppKeyMapConfig,
    entry_id: &str,
) -> AppLogResult<String> {
    let param = app_datastore_param_key(key_maps, "log_error")?;
    Ok(param(entry_id))
}

pub async fn app_log_entry_store<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    entry: &AppLogEntry,
) -> AppLogResult<AppLogEntry> {
    let key = app_log_error_key(key_maps, &entry.id)?;
    datastore
        .set_obj(&key, entry)
        .await
        .map_err(AppLogError::Datastore)
}

pub async fn app_log_error_store<T: RadrootsClientDatastore, E: AppLoggableError>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    err: &E,
) -> AppLogResult<AppLogEntry> {
    let entry = app_log_error_emit(err);
    app_log_entry_store(datastore, key_maps, &entry).await
}

#[derive(Debug)]
pub enum AppLoggingError {
    Logging(radroots_log::Error),
}

pub type AppLoggingResult<T> = Result<T, AppLoggingError>;

impl std::fmt::Display for AppLoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppLoggingError::Logging(err) => write!(f, "{err:?}"),
        }
    }
}

impl std::error::Error for AppLoggingError {}

pub fn app_log_metadata() -> &'static AppLogMetadata {
    LOG_META.get_or_init(AppLogMetadata::default)
}

pub fn app_logging_init(meta: Option<AppLogMetadata>) -> AppLoggingResult<()> {
    if LOG_META.get().is_none() {
        let _ = LOG_META.set(meta.unwrap_or_default());
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = tracing_wasm::set_as_global_default();
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let opts = radroots_log::LoggingOptions {
            dir: Some(PathBuf::from("logs")),
            file_name: "radroots-app.log".into(),
            stdout: true,
            default_level: Some(String::from("info")),
        };
        match radroots_log::init_logging(opts) {
            Ok(()) => Ok(()),
            Err(err) => {
                radroots_log::init_stdout().map_err(AppLoggingError::Logging)?;
                radroots_log::log_error(format!("logging_init_failed: {err}"));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_log_entry_error,
        app_log_error_key,
        app_log_metadata,
        app_log_timestamp_ms,
        AppLogLevel,
        AppLogMetadata,
    };
    use crate::{
        app_key_maps_default,
        AppConfigError,
        APP_DATASTORE_KEY_LOG_ERROR,
    };

    #[test]
    fn log_metadata_defaults_populated() {
        let meta = AppLogMetadata::default();
        assert!(!meta.app_name.is_empty());
        assert!(!meta.app_version.is_empty());
        assert!(!meta.app_hash.is_empty());
        assert!(!meta.target.is_empty());
    }

    #[test]
    fn log_metadata_once_lock_returns_default() {
        let meta = app_log_metadata();
        assert!(!meta.app_name.is_empty());
    }

    #[test]
    fn log_entry_error_includes_context() {
        let err = AppConfigError::MissingKeyMap("nostr_key");
        let entry = app_log_entry_error(&err);
        assert_eq!(entry.level, AppLogLevel::Error);
        assert_eq!(entry.code, err.message());
        assert_eq!(entry.message, err.to_string());
        assert_eq!(entry.context.as_deref(), Some("key_map=nostr_key"));
        assert!(entry.timestamp_ms >= app_log_timestamp_ms() - 10_000);
    }

    #[test]
    fn log_error_key_uses_param_map() {
        let key_maps = app_key_maps_default();
        let key = app_log_error_key(&key_maps, "entry").expect("key");
        assert_eq!(key, format!("{APP_DATASTORE_KEY_LOG_ERROR}:entry"));
    }
}
