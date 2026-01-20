#![forbid(unsafe_code)]

use std::sync::OnceLock;

#[cfg(not(test))]
use std::sync::Mutex;

#[cfg(test)]
use std::cell::RefCell;

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
    RadrootsAppConfigError,
    RadrootsAppInitAssetError,
    RadrootsAppInitError,
    RadrootsAppKeystoreError,
    RadrootsAppKeyMapConfig,
    RadrootsAppNotificationsError,
    RadrootsAppTangleError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppLogMetadata {
    pub app_name: String,
    pub app_version: String,
    pub app_hash: String,
    pub target: String,
}

impl Default for RadrootsAppLogMetadata {
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

static LOG_META: OnceLock<RadrootsAppLogMetadata> = OnceLock::new();

#[cfg(not(test))]
static LOG_BUFFER: OnceLock<Mutex<Vec<RadrootsAppLogEntry>>> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static LOG_BUFFER: RefCell<Vec<RadrootsAppLogEntry>> = RefCell::new(Vec::new());
}

pub const APP_LOG_BUFFER_MAX_ENTRIES: usize = 512;
pub const APP_LOG_MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadrootsAppLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl RadrootsAppLogLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsAppLogLevel::Debug => "debug",
            RadrootsAppLogLevel::Info => "info",
            RadrootsAppLogLevel::Warn => "warn",
            RadrootsAppLogLevel::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppLogEntry {
    pub id: String,
    pub timestamp_ms: i64,
    pub level: RadrootsAppLogLevel,
    pub code: String,
    pub message: String,
    pub context: Option<String>,
    pub metadata: RadrootsAppLogMetadata,
}

pub trait RadrootsAppLoggableError: std::fmt::Display {
    fn log_code(&self) -> &'static str;
    fn log_context(&self) -> Option<String> {
        None
    }
}

impl RadrootsAppLoggableError for RadrootsAppInitAssetError {
    fn log_code(&self) -> &'static str {
        self.message()
    }
}

impl RadrootsAppLoggableError for RadrootsAppConfigError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            RadrootsAppConfigError::MissingKeyMap(key) => Some(format!("key_map={key}")),
            RadrootsAppConfigError::MissingParamMap(key) => Some(format!("param_map={key}")),
            RadrootsAppConfigError::MissingObjMap(key) => Some(format!("obj_map={key}")),
            RadrootsAppConfigError::MissingKeystoreKeyMap(key) => Some(format!("keystore_map={key}")),
        }
    }
}

impl RadrootsAppLoggableError for RadrootsAppInitError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            RadrootsAppInitError::Idb(err) => Some(err.to_string()),
            RadrootsAppInitError::Datastore(err) => Some(err.to_string()),
            RadrootsAppInitError::Keystore(err) => Some(err.to_string()),
            RadrootsAppInitError::Config(err) => err.log_context().or_else(|| Some(err.message().to_string())),
            RadrootsAppInitError::Assets(err) => Some(err.message().to_string()),
        }
    }
}

impl RadrootsAppLoggableError for RadrootsAppKeystoreError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            RadrootsAppKeystoreError::Keystore(err) => Some(err.to_string()),
        }
    }
}

impl RadrootsAppLoggableError for RadrootsAppNotificationsError {
    fn log_code(&self) -> &'static str {
        self.message()
    }

    fn log_context(&self) -> Option<String> {
        match self {
            RadrootsAppNotificationsError::Notifications(err) => Some(err.message().to_string()),
        }
    }
}

impl RadrootsAppLoggableError for RadrootsAppTangleError {
    fn log_code(&self) -> &'static str {
        self.message()
    }
}

#[derive(Debug)]
pub enum RadrootsAppLogError {
    Config(RadrootsAppConfigError),
    Datastore(RadrootsClientDatastoreError),
}

pub type RadrootsAppLogResult<T> = Result<T, RadrootsAppLogError>;

impl std::fmt::Display for RadrootsAppLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RadrootsAppLogError::Config(err) => write!(f, "{err}"),
            RadrootsAppLogError::Datastore(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RadrootsAppLogError {}

impl From<RadrootsAppConfigError> for RadrootsAppLogError {
    fn from(err: RadrootsAppConfigError) -> Self {
        RadrootsAppLogError::Config(err)
    }
}

impl From<RadrootsClientDatastoreError> for RadrootsAppLogError {
    fn from(err: RadrootsClientDatastoreError) -> Self {
        RadrootsAppLogError::Datastore(err)
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

pub fn app_log_entry_error<E: RadrootsAppLoggableError>(err: &E) -> RadrootsAppLogEntry {
    RadrootsAppLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: app_log_timestamp_ms(),
        level: RadrootsAppLogLevel::Error,
        code: err.log_code().to_string(),
        message: err.to_string(),
        context: err.log_context(),
        metadata: app_log_metadata().clone(),
    }
}

pub fn app_log_entry_new(
    level: RadrootsAppLogLevel,
    code: &str,
    message: &str,
    context: Option<String>,
) -> RadrootsAppLogEntry {
    RadrootsAppLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: app_log_timestamp_ms(),
        level,
        code: code.to_string(),
        message: message.to_string(),
        context,
        metadata: app_log_metadata().clone(),
    }
}

pub fn app_log_entry_emit(entry: &RadrootsAppLogEntry) {
    let payload = serde_json::to_string(entry)
        .unwrap_or_else(|_| format!("{}: {}", entry.code, entry.message));
    match entry.level {
        RadrootsAppLogLevel::Error => radroots_log::log_error(payload),
        RadrootsAppLogLevel::Warn => radroots_log::log_info(payload),
        RadrootsAppLogLevel::Info => radroots_log::log_info(payload),
        RadrootsAppLogLevel::Debug => radroots_log::log_debug(payload),
    }
}

pub fn app_log_entry_record(entry: RadrootsAppLogEntry) -> RadrootsAppLogEntry {
    app_log_entry_emit(&entry);
    app_log_buffer_push(entry.clone());
    entry
}

pub fn app_log_error_emit<E: RadrootsAppLoggableError>(err: &E) -> RadrootsAppLogEntry {
    app_log_entry_record(app_log_entry_error(err))
}

pub fn app_log_debug_emit(code: &str, message: &str, context: Option<String>) -> RadrootsAppLogEntry {
    app_log_entry_record(app_log_entry_new(
        RadrootsAppLogLevel::Debug,
        code,
        message,
        context,
    ))
}

pub fn app_log_info_emit(code: &str, message: &str, context: Option<String>) -> RadrootsAppLogEntry {
    app_log_entry_record(app_log_entry_new(
        RadrootsAppLogLevel::Info,
        code,
        message,
        context,
    ))
}

pub fn app_log_warn_emit(code: &str, message: &str, context: Option<String>) -> RadrootsAppLogEntry {
    app_log_entry_record(app_log_entry_new(
        RadrootsAppLogLevel::Warn,
        code,
        message,
        context,
    ))
}

pub fn app_log_entry_key(
    key_maps: &RadrootsAppKeyMapConfig,
    entry_id: &str,
) -> RadrootsAppLogResult<String> {
    let param = app_datastore_param_key(key_maps, "log_entry")?;
    Ok(param(entry_id))
}

pub async fn app_log_entry_store<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    entry: &RadrootsAppLogEntry,
) -> RadrootsAppLogResult<RadrootsAppLogEntry> {
    let key = app_log_entry_key(key_maps, &entry.id)?;
    datastore
        .set_obj(&key, entry)
        .await
        .map_err(RadrootsAppLogError::Datastore)
}

pub async fn app_log_error_store<T: RadrootsClientDatastore, E: RadrootsAppLoggableError>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    err: &E,
) -> RadrootsAppLogResult<RadrootsAppLogEntry> {
    let entry = app_log_error_emit(err);
    app_log_entry_store(datastore, key_maps, &entry).await
}

pub fn app_log_buffer_push(entry: RadrootsAppLogEntry) {
    #[cfg(test)]
    {
        LOG_BUFFER.with(|buffer| {
            let mut entries = buffer.borrow_mut();
            entries.push(entry);
            if entries.len() > APP_LOG_BUFFER_MAX_ENTRIES {
                let drop = entries.len() - APP_LOG_BUFFER_MAX_ENTRIES;
                entries.drain(0..drop);
            }
        });
    }
    #[cfg(not(test))]
    {
        let buffer = LOG_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
        let mut entries = buffer.lock().unwrap_or_else(|err| err.into_inner());
        entries.push(entry);
        if entries.len() > APP_LOG_BUFFER_MAX_ENTRIES {
            let drop = entries.len() - APP_LOG_BUFFER_MAX_ENTRIES;
            entries.drain(0..drop);
        }
    }
}

pub fn app_log_buffer_drain() -> Vec<RadrootsAppLogEntry> {
    #[cfg(test)]
    {
        LOG_BUFFER.with(|buffer| buffer.borrow_mut().drain(..).collect())
    }
    #[cfg(not(test))]
    {
        let buffer = LOG_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
        let mut entries = buffer.lock().unwrap_or_else(|err| err.into_inner());
        entries.drain(..).collect()
    }
}

fn app_log_entry_should_persist(level: RadrootsAppLogLevel) -> bool {
    matches!(level, RadrootsAppLogLevel::Warn | RadrootsAppLogLevel::Error)
}

pub async fn app_log_buffer_flush<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppLogResult<usize> {
    let entries = app_log_buffer_drain();
    let mut stored = 0;
    let mut iter = entries.into_iter();
    while let Some(entry) = iter.next() {
        if let Err(err) = app_log_entry_store(datastore, key_maps, &entry).await {
            app_log_buffer_push(entry);
            for remaining in iter {
                app_log_buffer_push(remaining);
            }
            return Err(err);
        }
        stored += 1;
    }
    let _ = app_log_entries_prune(datastore, key_maps, APP_LOG_MAX_ENTRIES).await?;
    Ok(stored)
}

pub async fn app_log_buffer_flush_critical<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppLogResult<usize> {
    let entries = app_log_buffer_drain();
    let mut keep = Vec::new();
    let mut persist = Vec::new();
    for entry in entries {
        if app_log_entry_should_persist(entry.level) {
            persist.push(entry);
        } else {
            keep.push(entry);
        }
    }
    let mut stored = 0;
    let mut iter = persist.into_iter();
    while let Some(entry) = iter.next() {
        if let Err(err) = app_log_entry_store(datastore, key_maps, &entry).await {
            app_log_buffer_push(entry);
            for remaining in iter {
                app_log_buffer_push(remaining);
            }
            for remaining in keep {
                app_log_buffer_push(remaining);
            }
            return Err(err);
        }
        stored += 1;
    }
    for entry in keep {
        app_log_buffer_push(entry);
    }
    let _ = app_log_entries_prune(datastore, key_maps, APP_LOG_MAX_ENTRIES).await?;
    Ok(stored)
}

pub fn app_log_entry_prefix(key_maps: &RadrootsAppKeyMapConfig) -> RadrootsAppLogResult<String> {
    let param = app_datastore_param_key(key_maps, "log_entry")?;
    Ok(param(""))
}

pub async fn app_log_entries_load<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppLogResult<Vec<RadrootsAppLogEntry>> {
    let entries = datastore.entries().await.map_err(RadrootsAppLogError::Datastore)?;
    let prefix = app_log_entry_prefix(key_maps)?;
    let mut out = Vec::new();
    for entry in entries {
        if !entry.key.starts_with(&prefix) {
            continue;
        }
        let Some(value) = entry.value else {
            continue;
        };
        if let Ok(parsed) = serde_json::from_str::<RadrootsAppLogEntry>(&value) {
            out.push(parsed);
        }
    }
    Ok(out)
}

pub async fn app_log_entries_clear<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppLogResult<usize> {
    let prefix = app_log_entry_prefix(key_maps)?;
    let removed = datastore
        .del_pref(&prefix)
        .await
        .map_err(RadrootsAppLogError::Datastore)?;
    Ok(removed.len())
}

pub fn app_log_entries_dump(entries: &[RadrootsAppLogEntry]) -> String {
    let mut out = String::new();
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        match serde_json::to_string(entry) {
            Ok(line) => out.push_str(&line),
            Err(_) => out.push_str("{\"error\":\"log_entry_encode_failed\"}"),
        }
    }
    out
}

pub async fn app_log_entries_prune<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    max_entries: usize,
) -> RadrootsAppLogResult<usize> {
    let mut entries = app_log_entries_load(datastore, key_maps).await?;
    if entries.len() <= max_entries {
        return Ok(0);
    }
    entries.sort_by_key(|entry| entry.timestamp_ms);
    let prune_count = entries.len().saturating_sub(max_entries);
    let mut removed = 0;
    for entry in entries.into_iter().take(prune_count) {
        let key = app_log_entry_key(key_maps, &entry.id)?;
        let _ = datastore.del(&key).await.map_err(RadrootsAppLogError::Datastore)?;
        removed += 1;
    }
    Ok(removed)
}

#[derive(Debug)]
pub enum RadrootsAppLoggingError {
    Logging(radroots_log::Error),
}

pub type RadrootsAppLoggingResult<T> = Result<T, RadrootsAppLoggingError>;

impl std::fmt::Display for RadrootsAppLoggingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RadrootsAppLoggingError::Logging(err) => write!(f, "{err:?}"),
        }
    }
}

impl std::error::Error for RadrootsAppLoggingError {}

pub fn app_log_metadata() -> &'static RadrootsAppLogMetadata {
    LOG_META.get_or_init(RadrootsAppLogMetadata::default)
}

pub fn app_logging_init(meta: Option<RadrootsAppLogMetadata>) -> RadrootsAppLoggingResult<()> {
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
                radroots_log::init_stdout().map_err(RadrootsAppLoggingError::Logging)?;
                radroots_log::log_error(format!("logging_init_failed: {err}"));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_log_entries_clear,
        app_log_entries_dump,
        app_log_entries_load,
        app_log_entries_prune,
        app_log_entry_error,
        app_log_entry_new,
        app_log_entry_key,
        app_log_entry_prefix,
        app_log_buffer_drain,
        app_log_buffer_flush_critical,
        app_log_buffer_flush,
        app_log_buffer_push,
        app_log_metadata,
        app_log_timestamp_ms,
        RadrootsAppLogLevel,
        RadrootsAppLogEntry,
        RadrootsAppLogMetadata,
    };
    use crate::{
        app_key_maps_default,
        RadrootsAppConfigError,
        APP_DATASTORE_KEY_LOG_ENTRY,
    };
    use async_trait::async_trait;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreEntry,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
    };
    use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_DATASTORE};
    use serde::{de::DeserializeOwned, Serialize};
    use std::sync::Mutex;

    static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct TestDatastore {
        entries: Mutex<Vec<RadrootsClientDatastoreEntry>>,
    }

    impl TestDatastore {
        fn new(entries: Vec<RadrootsClientDatastoreEntry>) -> Self {
            Self {
                entries: Mutex::new(entries),
            }
        }

        fn len(&self) -> usize {
            self.entries.lock().unwrap_or_else(|err| err.into_inner()).len()
        }
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
            Ok(())
        }

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn set_obj<T>(
            &self,
            key: &str,
            value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
            let encoded = serde_json::to_string(value)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            let mut entries = self.entries.lock().unwrap_or_else(|err| err.into_inner());
            entries.retain(|entry| entry.key != key);
            entries.push(RadrootsClientDatastoreEntry::new(
                key.to_string(),
                Some(encoded),
            ));
            Ok(value.clone())
        }

        async fn update_obj<T>(
            &self,
            _key: &str,
            _value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get_obj<T>(&self, _key: &str) -> RadrootsClientDatastoreResult<T>
        where
            T: DeserializeOwned,
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

        async fn del_pref(&self, key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>> {
            let mut entries = self.entries.lock().unwrap_or_else(|err| err.into_inner());
            let mut removed = Vec::new();
            let mut kept = Vec::new();
            for entry in entries.drain(..) {
                if entry.key.starts_with(key_prefix) {
                    removed.push(entry.key);
                } else {
                    kept.push(entry);
                }
            }
            *entries = kept;
            Ok(removed)
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
            Ok(self
                .entries
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .clone())
        }

        async fn entries_pref(
            &self,
            key_prefix: &str,
        ) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            Ok(self
                .entries
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .iter()
                .filter(|entry| entry.key.starts_with(key_prefix))
                .cloned()
                .collect())
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
    fn log_metadata_defaults_populated() {
        let meta = RadrootsAppLogMetadata::default();
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
        let err = RadrootsAppConfigError::MissingKeyMap("nostr_key");
        let entry = app_log_entry_error(&err);
        assert_eq!(entry.level, RadrootsAppLogLevel::Error);
        assert_eq!(entry.code, err.message());
        assert_eq!(entry.message, err.to_string());
        assert_eq!(entry.context.as_deref(), Some("key_map=nostr_key"));
        assert!(entry.timestamp_ms >= app_log_timestamp_ms() - 10_000);
    }

    #[test]
    fn log_error_key_uses_param_map() {
        let key_maps = app_key_maps_default();
        let key = app_log_entry_key(&key_maps, "entry").expect("key");
        assert_eq!(key, format!("{APP_DATASTORE_KEY_LOG_ENTRY}:entry"));
    }

    #[test]
    fn log_entry_new_populates_fields() {
        let entry = app_log_entry_new(
            RadrootsAppLogLevel::Info,
            "log.code.test",
            "hello",
            Some(String::from("ctx")),
        );
        assert_eq!(entry.level, RadrootsAppLogLevel::Info);
        assert_eq!(entry.code, "log.code.test");
        assert_eq!(entry.message, "hello");
        assert_eq!(entry.context.as_deref(), Some("ctx"));
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn log_buffer_drains_entries() {
        let _guard = LOG_TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _ = app_log_buffer_drain();
        let entry = app_log_entry_new(RadrootsAppLogLevel::Debug, "log.code.test", "buf", None);
        app_log_buffer_push(entry.clone());
        let drained = app_log_buffer_drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].id, entry.id);
        assert!(app_log_buffer_drain().is_empty());
    }

    #[test]
    fn log_buffer_flush_critical_keeps_debug_entries() {
        let _guard = LOG_TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _ = app_log_buffer_drain();
        let debug = app_log_entry_new(RadrootsAppLogLevel::Debug, "log.code.debug", "debug", None);
        let error = app_log_entry_new(RadrootsAppLogLevel::Error, "log.code.error", "error", None);
        app_log_buffer_push(debug.clone());
        app_log_buffer_push(error.clone());
        let datastore = TestDatastore::new(Vec::new());
        let key_maps = app_key_maps_default();
        let stored = futures::executor::block_on(app_log_buffer_flush_critical(
            &datastore,
            &key_maps,
        ))
        .expect("flush");
        assert_eq!(stored, 1);
        let remaining = app_log_buffer_drain();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, debug.id);
    }

    #[test]
    fn log_entry_prefix_uses_log_key() {
        let key_maps = app_key_maps_default();
        let prefix = app_log_entry_prefix(&key_maps).expect("prefix");
        assert_eq!(prefix, format!("{APP_DATASTORE_KEY_LOG_ENTRY}:"));
    }

    #[test]
    fn log_entries_clear_removes_prefixed_keys() {
        let key_maps = app_key_maps_default();
        let key_a = app_log_entry_key(&key_maps, "a").expect("key");
        let entries = vec![
            RadrootsClientDatastoreEntry::new(key_a, Some(String::from("{}"))),
            RadrootsClientDatastoreEntry::new(String::from("other:1"), Some(String::from("{}"))),
        ];
        let datastore = TestDatastore::new(entries);
        let removed = futures::executor::block_on(app_log_entries_clear(&datastore, &key_maps))
            .expect("clear");
        assert_eq!(removed, 1);
        assert_eq!(datastore.len(), 1);
    }

    #[test]
    fn log_entries_dump_serializes_jsonl() {
        let entries = vec![RadrootsAppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: RadrootsAppLogLevel::Info,
            code: String::from("code"),
            message: String::from("hello"),
            context: None,
            metadata: RadrootsAppLogMetadata::default(),
        }];
        let dump = app_log_entries_dump(&entries);
        assert!(dump.contains("\"code\":\"code\""));
        assert_eq!(dump.lines().count(), 1);
    }

    #[test]
    fn log_entries_load_filters_by_prefix() {
        let key_maps = app_key_maps_default();
        let entry = RadrootsAppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: RadrootsAppLogLevel::Info,
            code: String::from("code"),
            message: String::from("hello"),
            context: None,
            metadata: RadrootsAppLogMetadata::default(),
        };
        let key = app_log_entry_key(&key_maps, &entry.id).expect("key");
        let entries = vec![
            RadrootsClientDatastoreEntry::new(
                key,
                Some(serde_json::to_string(&entry).expect("json")),
            ),
            RadrootsClientDatastoreEntry::new(String::from("other"), Some(String::from("{}"))),
        ];
        let datastore = TestDatastore::new(entries);
        let loaded = futures::executor::block_on(app_log_entries_load(&datastore, &key_maps))
            .expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "a");
    }

    #[test]
    fn log_entries_prune_enforces_limit() {
        let key_maps = app_key_maps_default();
        let entries = (0..3)
            .map(|idx| RadrootsAppLogEntry {
                id: format!("id-{idx}"),
                timestamp_ms: idx,
                level: RadrootsAppLogLevel::Info,
                code: String::from("code"),
                message: String::from("hello"),
                context: None,
                metadata: RadrootsAppLogMetadata::default(),
            })
            .collect::<Vec<_>>();
        let mut stored = Vec::new();
        for entry in entries {
            let key = app_log_entry_key(&key_maps, &entry.id).expect("key");
            stored.push(RadrootsClientDatastoreEntry::new(
                key,
                Some(serde_json::to_string(&entry).expect("json")),
            ));
        }
        let datastore = TestDatastore::new(stored);
        let removed =
            futures::executor::block_on(app_log_entries_prune(&datastore, &key_maps, 2))
        .expect("prune");
        assert_eq!(removed, 1);
        assert_eq!(datastore.len(), 2);
    }

    #[test]
    fn log_buffer_flush_stores_entries() {
        let _guard = LOG_TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _ = app_log_buffer_drain();
        let key_maps = app_key_maps_default();
        let datastore = TestDatastore::new(Vec::new());
        app_log_buffer_push(app_log_entry_new(
            RadrootsAppLogLevel::Info,
            "log.code.flush",
            "flush",
            None,
        ));
        let stored = futures::executor::block_on(app_log_buffer_flush(&datastore, &key_maps))
            .expect("flush");
        assert_eq!(stored, 1);
        assert_eq!(datastore.len(), 1);
    }
}
