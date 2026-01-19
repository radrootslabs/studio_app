#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

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
static LOG_BUFFER: OnceLock<Mutex<Vec<AppLogEntry>>> = OnceLock::new();

pub const APP_LOG_BUFFER_MAX_ENTRIES: usize = 512;
pub const APP_LOG_MAX_ENTRIES: usize = 2000;

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

pub fn app_log_entry_new(
    level: AppLogLevel,
    code: &str,
    message: &str,
    context: Option<String>,
) -> AppLogEntry {
    AppLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: app_log_timestamp_ms(),
        level,
        code: code.to_string(),
        message: message.to_string(),
        context,
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

pub fn app_log_entry_record(entry: AppLogEntry) -> AppLogEntry {
    app_log_entry_emit(&entry);
    app_log_buffer_push(entry.clone());
    entry
}

pub fn app_log_error_emit<E: AppLoggableError>(err: &E) -> AppLogEntry {
    app_log_entry_record(app_log_entry_error(err))
}

pub fn app_log_debug_emit(code: &str, message: &str, context: Option<String>) -> AppLogEntry {
    app_log_entry_record(app_log_entry_new(
        AppLogLevel::Debug,
        code,
        message,
        context,
    ))
}

pub fn app_log_info_emit(code: &str, message: &str, context: Option<String>) -> AppLogEntry {
    app_log_entry_record(app_log_entry_new(
        AppLogLevel::Info,
        code,
        message,
        context,
    ))
}

pub fn app_log_warn_emit(code: &str, message: &str, context: Option<String>) -> AppLogEntry {
    app_log_entry_record(app_log_entry_new(
        AppLogLevel::Warn,
        code,
        message,
        context,
    ))
}

pub fn app_log_entry_key(
    key_maps: &AppKeyMapConfig,
    entry_id: &str,
) -> AppLogResult<String> {
    let param = app_datastore_param_key(key_maps, "log_entry")?;
    Ok(param(entry_id))
}

pub async fn app_log_entry_store<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    entry: &AppLogEntry,
) -> AppLogResult<AppLogEntry> {
    let key = app_log_entry_key(key_maps, &entry.id)?;
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

pub fn app_log_buffer_push(entry: AppLogEntry) {
    let buffer = LOG_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
    let mut entries = buffer.lock().unwrap_or_else(|err| err.into_inner());
    entries.push(entry);
    if entries.len() > APP_LOG_BUFFER_MAX_ENTRIES {
        let drop = entries.len() - APP_LOG_BUFFER_MAX_ENTRIES;
        entries.drain(0..drop);
    }
}

pub fn app_log_buffer_drain() -> Vec<AppLogEntry> {
    let buffer = LOG_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
    let mut entries = buffer.lock().unwrap_or_else(|err| err.into_inner());
    entries.drain(..).collect()
}

pub async fn app_log_buffer_flush<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppLogResult<usize> {
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

pub fn app_log_entry_prefix(key_maps: &AppKeyMapConfig) -> AppLogResult<String> {
    let param = app_datastore_param_key(key_maps, "log_entry")?;
    Ok(param(""))
}

pub async fn app_log_entries_load<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppLogResult<Vec<AppLogEntry>> {
    let entries = datastore.entries().await.map_err(AppLogError::Datastore)?;
    let prefix = app_log_entry_prefix(key_maps)?;
    let mut out = Vec::new();
    for entry in entries {
        if !entry.key.starts_with(&prefix) {
            continue;
        }
        let Some(value) = entry.value else {
            continue;
        };
        if let Ok(parsed) = serde_json::from_str::<AppLogEntry>(&value) {
            out.push(parsed);
        }
    }
    Ok(out)
}

pub fn app_log_entries_dump(entries: &[AppLogEntry]) -> String {
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
    key_maps: &AppKeyMapConfig,
    max_entries: usize,
) -> AppLogResult<usize> {
    let mut entries = app_log_entries_load(datastore, key_maps).await?;
    if entries.len() <= max_entries {
        return Ok(0);
    }
    entries.sort_by_key(|entry| entry.timestamp_ms);
    let prune_count = entries.len().saturating_sub(max_entries);
    let mut removed = 0;
    for entry in entries.into_iter().take(prune_count) {
        let key = app_log_entry_key(key_maps, &entry.id)?;
        let _ = datastore.del(&key).await.map_err(AppLogError::Datastore)?;
        removed += 1;
    }
    Ok(removed)
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
        app_log_entries_dump,
        app_log_entries_load,
        app_log_entries_prune,
        app_log_entry_error,
        app_log_entry_new,
        app_log_entry_key,
        app_log_entry_prefix,
        app_log_buffer_drain,
        app_log_buffer_flush,
        app_log_buffer_push,
        app_log_metadata,
        app_log_timestamp_ms,
        AppLogLevel,
        AppLogEntry,
        AppLogMetadata,
    };
    use crate::{
        app_key_maps_default,
        AppConfigError,
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
            Ok(self
                .entries
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .clone())
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
        let key = app_log_entry_key(&key_maps, "entry").expect("key");
        assert_eq!(key, format!("{APP_DATASTORE_KEY_LOG_ENTRY}:entry"));
    }

    #[test]
    fn log_entry_new_populates_fields() {
        let entry = app_log_entry_new(
            AppLogLevel::Info,
            "log.code.test",
            "hello",
            Some(String::from("ctx")),
        );
        assert_eq!(entry.level, AppLogLevel::Info);
        assert_eq!(entry.code, "log.code.test");
        assert_eq!(entry.message, "hello");
        assert_eq!(entry.context.as_deref(), Some("ctx"));
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn log_buffer_drains_entries() {
        let _guard = LOG_TEST_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _ = app_log_buffer_drain();
        let entry = app_log_entry_new(AppLogLevel::Debug, "log.code.test", "buf", None);
        app_log_buffer_push(entry.clone());
        let drained = app_log_buffer_drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].id, entry.id);
        assert!(app_log_buffer_drain().is_empty());
    }

    #[test]
    fn log_entry_prefix_uses_log_key() {
        let key_maps = app_key_maps_default();
        let prefix = app_log_entry_prefix(&key_maps).expect("prefix");
        assert_eq!(prefix, format!("{APP_DATASTORE_KEY_LOG_ENTRY}:"));
    }

    #[test]
    fn log_entries_dump_serializes_jsonl() {
        let entries = vec![AppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: AppLogLevel::Info,
            code: String::from("code"),
            message: String::from("hello"),
            context: None,
            metadata: AppLogMetadata::default(),
        }];
        let dump = app_log_entries_dump(&entries);
        assert!(dump.contains("\"code\":\"code\""));
        assert_eq!(dump.lines().count(), 1);
    }

    #[test]
    fn log_entries_load_filters_by_prefix() {
        let key_maps = app_key_maps_default();
        let entry = AppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: AppLogLevel::Info,
            code: String::from("code"),
            message: String::from("hello"),
            context: None,
            metadata: AppLogMetadata::default(),
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
            .map(|idx| AppLogEntry {
                id: format!("id-{idx}"),
                timestamp_ms: idx,
                level: AppLogLevel::Info,
                code: String::from("code"),
                message: String::from("hello"),
                context: None,
                metadata: AppLogMetadata::default(),
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
            AppLogLevel::Info,
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
