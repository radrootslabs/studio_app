#![forbid(unsafe_code)]

use std::fmt;
use std::rc::Rc;

use radroots_studio_app_core::datastore::{
    RadrootsClientDatastore,
    RadrootsClientDatastoreError,
    RadrootsClientWebDatastore,
};
use radroots_studio_app_core::idb::{
    idb_store_bootstrap,
    RadrootsClientIdbStoreError,
    RADROOTS_IDB_DATABASE,
};
use radroots_studio_app_core::keystore::{
    RadrootsClientKeystoreError,
    RadrootsClientKeystoreNostr,
    RadrootsClientWebKeystoreNostr,
};

use crate::{
    app_datastore_has_state,
    app_datastore_read_state,
    app_assets_geocoder_db_url,
    app_assets_sql_wasm_url,
    app_log_debug_emit,
    app_state_is_initialized,
    RadrootsAppStateError,
    RadrootsAppConfig,
    RadrootsAppConfigError,
    RadrootsAppKeyMapConfig,
    RadrootsAppSetupStatus,
    APP_EULA_HASH,
    APP_EULA_VERSION,
};

#[cfg(target_arch = "wasm32")]
use leptos::prelude::window;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
#[cfg(target_arch = "wasm32")]
use web_sys::Response;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub const APP_INIT_STORAGE_KEY: &str = "radroots.app.init.ready";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppInitStage {
    Idle,
    Storage,
    DownloadSql,
    DownloadGeo,
    Database,
    Geocoder,
    Ready,
    Error,
}

impl RadrootsAppInitStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsAppInitStage::Idle => "idle",
            RadrootsAppInitStage::Storage => "storage",
            RadrootsAppInitStage::DownloadSql => "download_sql",
            RadrootsAppInitStage::DownloadGeo => "download_geo",
            RadrootsAppInitStage::Database => "database",
            RadrootsAppInitStage::Geocoder => "geocoder",
            RadrootsAppInitStage::Ready => "ready",
            RadrootsAppInitStage::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "idle" => Some(RadrootsAppInitStage::Idle),
            "storage" => Some(RadrootsAppInitStage::Storage),
            "download_sql" => Some(RadrootsAppInitStage::DownloadSql),
            "download_geo" => Some(RadrootsAppInitStage::DownloadGeo),
            "database" => Some(RadrootsAppInitStage::Database),
            "geocoder" => Some(RadrootsAppInitStage::Geocoder),
            "ready" => Some(RadrootsAppInitStage::Ready),
            "error" => Some(RadrootsAppInitStage::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppInitState {
    pub stage: RadrootsAppInitStage,
    pub loaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub const fn app_init_state_default() -> RadrootsAppInitState {
    RadrootsAppInitState {
        stage: RadrootsAppInitStage::Idle,
        loaded_bytes: 0,
        total_bytes: Some(0),
    }
}

pub fn app_init_stage_set(state: &mut RadrootsAppInitState, stage: RadrootsAppInitStage) {
    state.stage = stage;
}

pub fn app_init_progress_add(state: &mut RadrootsAppInitState, bytes: u64) {
    if bytes == 0 {
        return;
    }
    state.loaded_bytes = state.loaded_bytes.saturating_add(bytes);
}

pub fn app_init_total_add(state: &mut RadrootsAppInitState, bytes: u64) {
    if bytes == 0 {
        return;
    }
    let Some(total) = state.total_bytes else {
        return;
    };
    state.total_bytes = Some(total.saturating_add(bytes));
}

pub fn app_init_total_unknown(state: &mut RadrootsAppInitState) {
    state.total_bytes = None;
}

#[cfg(target_arch = "wasm32")]
fn app_init_timer_start() -> u64 {
    Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn app_init_timer_start() -> Instant {
    Instant::now()
}

#[cfg(target_arch = "wasm32")]
fn app_init_elapsed_ms(start: u64) -> u64 {
    let now = Date::now() as u64;
    now.saturating_sub(start)
}

#[cfg(not(target_arch = "wasm32"))]
fn app_init_elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

fn app_init_timing_context(label: &str, elapsed_ms: u64) -> String {
    format!("{label}_ms={elapsed_ms}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadrootsAppInitAssetProgress {
    pub loaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl RadrootsAppInitAssetProgress {
    pub const fn empty() -> Self {
        Self {
            loaded_bytes: 0,
            total_bytes: Some(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppInitAssetError {
    MissingUrl,
    FetchUnavailable,
    FetchFailed,
}

impl RadrootsAppInitAssetError {
    pub const fn message(self) -> &'static str {
        match self {
            RadrootsAppInitAssetError::MissingUrl => "error.app.init.asset_missing_url",
            RadrootsAppInitAssetError::FetchUnavailable => "error.app.init.asset_unavailable",
            RadrootsAppInitAssetError::FetchFailed => "error.app.init.asset_fetch_failed",
        }
    }
}

impl fmt::Display for RadrootsAppInitAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppInitAssetError {}

#[cfg(target_arch = "wasm32")]
pub async fn app_init_fetch_asset<F>(
    url: &str,
    mut on_progress: F,
) -> Result<RadrootsAppInitAssetProgress, RadrootsAppInitAssetError>
where
    F: FnMut(u64, Option<u64>),
{
    if url.is_empty() {
        return Err(RadrootsAppInitAssetError::MissingUrl);
    }
    let response_value = JsFuture::from(window().fetch_with_str(url))
        .await
        .map_err(|_| RadrootsAppInitAssetError::FetchFailed)?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|_| RadrootsAppInitAssetError::FetchFailed)?;
    let total_bytes = response
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<u64>().ok());
    let buffer_value = JsFuture::from(response.array_buffer().map_err(|_| RadrootsAppInitAssetError::FetchFailed)?)
        .await
        .map_err(|_| RadrootsAppInitAssetError::FetchFailed)?;
    let buffer = Uint8Array::new(&buffer_value);
    let loaded_bytes = buffer.length() as u64;
    on_progress(loaded_bytes, total_bytes);
    Ok(RadrootsAppInitAssetProgress {
        loaded_bytes,
        total_bytes,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn app_init_fetch_asset<F>(
    url: &str,
    _on_progress: F,
) -> Result<RadrootsAppInitAssetProgress, RadrootsAppInitAssetError>
where
    F: FnMut(u64, Option<u64>),
{
    if url.is_empty() {
        return Err(RadrootsAppInitAssetError::MissingUrl);
    }
    Err(RadrootsAppInitAssetError::FetchUnavailable)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppInitError {
    Idb(RadrootsClientIdbStoreError),
    Datastore(RadrootsClientDatastoreError),
    Keystore(RadrootsClientKeystoreError),
    Config(RadrootsAppConfigError),
    Assets(RadrootsAppInitAssetError),
    State(RadrootsAppStateError),
}

pub type RadrootsAppInitErrorMessage = &'static str;

impl RadrootsAppInitError {
    pub const fn message(&self) -> RadrootsAppInitErrorMessage {
        match self {
            RadrootsAppInitError::Idb(_) => "error.app.init.idb",
            RadrootsAppInitError::Datastore(_) => "error.app.init.datastore",
            RadrootsAppInitError::Keystore(_) => "error.app.init.keystore",
            RadrootsAppInitError::Config(_) => "error.app.init.config",
            RadrootsAppInitError::Assets(_) => "error.app.init.assets",
            RadrootsAppInitError::State(err) => err.message(),
        }
    }
}

impl fmt::Display for RadrootsAppInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppInitError {}

pub struct RadrootsAppBackends {
    pub config: RadrootsAppConfig,
    pub datastore: Rc<RadrootsClientWebDatastore>,
    pub nostr_keystore: RadrootsClientWebKeystoreNostr,
}

pub type RadrootsAppInitResult<T> = Result<T, RadrootsAppInitError>;

pub async fn app_init_assets<F, G>(
    config: &RadrootsAppConfig,
    mut on_stage: F,
    mut on_progress: G,
) -> Result<(), RadrootsAppInitAssetError>
where
    F: FnMut(RadrootsAppInitStage),
    G: FnMut(u64, Option<u64>),
{
    let _ = app_log_debug_emit("log.app.init.assets", "start", None);
    if let Some(url) = app_assets_sql_wasm_url(config).filter(|value| !value.is_empty()) {
        let _ = app_log_debug_emit("log.app.init.assets.sql", "download_start", Some(url.to_string()));
        on_stage(RadrootsAppInitStage::DownloadSql);
        app_init_fetch_asset(url, |loaded, total| {
            on_progress(loaded, total);
        })
        .await?;
        let _ = app_log_debug_emit("log.app.init.assets.sql", "download_done", None);
    }
    if let Some(url) = app_assets_geocoder_db_url(config).filter(|value| !value.is_empty()) {
        let _ = app_log_debug_emit("log.app.init.assets.geo", "download_start", Some(url.to_string()));
        on_stage(RadrootsAppInitStage::DownloadGeo);
        app_init_fetch_asset(url, |loaded, total| {
            on_progress(loaded, total);
        })
        .await?;
        let _ = app_log_debug_emit("log.app.init.assets.geo", "download_done", None);
    }
    let _ = app_log_debug_emit("log.app.init.assets", "done", None);
    Ok(())
}

pub fn app_init_has_completed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        match window.local_storage() {
            Ok(Some(storage)) => match storage.get_item(APP_INIT_STORAGE_KEY) {
                Ok(Some(value)) => value == "1",
                _ => false,
            },
            _ => false,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn app_init_mark_completed() {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(APP_INIT_STORAGE_KEY, "1");
        }
    }
}

pub async fn app_init_reset<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr>(
    datastore: Option<&T>,
    _key_maps: Option<&RadrootsAppKeyMapConfig>,
    keystore: Option<&K>,
) -> RadrootsAppInitResult<()> {
    let _ = app_log_debug_emit("log.app.init.reset", "start", None);
    if let Some(datastore) = datastore {
        datastore.reset().await.map_err(RadrootsAppInitError::Datastore)?;
    }
    if let Some(keystore) = keystore {
        keystore.reset().await.map_err(RadrootsAppInitError::Keystore)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(APP_INIT_STORAGE_KEY);
        }
    }
    let _ = app_log_debug_emit("log.app.init.reset", "done", None);
    Ok(())
}

pub async fn app_init_needs_setup<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr>(
    datastore: &T,
    keystore: &K,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<bool> {
    let has_state = app_datastore_has_state(datastore, key_maps).await?;
    if !has_state {
        return Ok(true);
    }
    let state = app_datastore_read_state(datastore, key_maps).await?;
    if !app_state_is_initialized(&state) {
        return Ok(true);
    }
    match keystore.read(&state.active_key).await {
        Ok(_) => Ok(false),
        Err(RadrootsClientKeystoreError::MissingKey) => Ok(true),
        Err(RadrootsClientKeystoreError::NostrNoResults) => Ok(true),
        Err(err) => Err(RadrootsAppInitError::Keystore(err)),
    }
}

pub async fn app_init_setup_status<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr>(
    datastore: &T,
    keystore: &K,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<RadrootsAppSetupStatus> {
    let has_state = app_datastore_has_state(datastore, key_maps).await?;
    if !has_state {
        return Ok(RadrootsAppSetupStatus::Required);
    }
    let state = match app_datastore_read_state(datastore, key_maps).await {
        Ok(state) => state,
        Err(RadrootsAppInitError::State(RadrootsAppStateError::Corrupt))
        | Err(RadrootsAppInitError::State(RadrootsAppStateError::UnsupportedVersion(_))) => {
            return Ok(RadrootsAppSetupStatus::Corrupt);
        }
        Err(err) => return Err(err),
    };
    if !app_state_is_initialized(&state) {
        return Ok(RadrootsAppSetupStatus::Required);
    }
    if state.eula_version != APP_EULA_VERSION || state.eula_hash != APP_EULA_HASH {
        return Ok(RadrootsAppSetupStatus::Corrupt);
    }
    match keystore.read(&state.active_key).await {
        Ok(_) => Ok(RadrootsAppSetupStatus::Configured),
        Err(RadrootsClientKeystoreError::MissingKey)
        | Err(RadrootsClientKeystoreError::NostrNoResults) => {
            Ok(RadrootsAppSetupStatus::Corrupt)
        }
        Err(err) => Err(RadrootsAppInitError::Keystore(err)),
    }
}

pub async fn app_init_backends(config: RadrootsAppConfig) -> RadrootsAppInitResult<RadrootsAppBackends> {
    let _ = app_log_debug_emit("log.app.init.backends", "start", None);
    config.validate().map_err(RadrootsAppInitError::Config)?;
    let idb_start = app_init_timer_start();
    idb_store_bootstrap(RADROOTS_IDB_DATABASE, None)
        .await
        .map_err(RadrootsAppInitError::Idb)?;
    let idb_ms = app_init_elapsed_ms(idb_start);
    let _ = app_log_debug_emit(
        "log.app.init.backends",
        "idb_bootstrap",
        Some(app_init_timing_context("elapsed", idb_ms)),
    );
    let datastore = Rc::new(RadrootsClientWebDatastore::new(Some(config.datastore.idb_config)));
    datastore
        .init()
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.init.backends", "datastore_ready", None);
    let nostr_keystore = RadrootsClientWebKeystoreNostr::new(Some(config.keystore.nostr_store));
    Ok(RadrootsAppBackends {
        config,
        datastore,
        nostr_keystore,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        app_init_backends,
        app_init_assets,
        app_init_needs_setup,
        app_init_setup_status,
        app_init_timing_context,
        app_init_progress_add,
        app_init_state_default,
        app_init_stage_set,
        app_init_total_add,
        app_init_total_unknown,
        RadrootsAppInitError,
        RadrootsAppInitErrorMessage,
        RadrootsAppInitStage,
        RadrootsAppInitAssetError,
    };
    use crate::{
        app_config_default,
        app_key_maps_default,
        RadrootsAppConfig,
        RadrootsAppState,
        RadrootsAppStateError,
        RadrootsAppStateRecord,
        RadrootsAppSetupStatus,
        APP_EULA_HASH,
        APP_EULA_VERSION,
    };
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
    };
    use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
    };
    use async_trait::async_trait;
    use crate::RadrootsAppConfigError;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_DATASTORE};
    use serde::{de::DeserializeOwned, Serialize};

    #[test]
    fn app_init_error_messages_match_spec() {
        let cases: &[(RadrootsAppInitError, RadrootsAppInitErrorMessage)] = &[
            (
                RadrootsAppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined),
                "error.app.init.idb",
            ),
            (
                RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined),
                "error.app.init.datastore",
            ),
            (
                RadrootsAppInitError::Keystore(RadrootsClientKeystoreError::IdbUndefined),
                "error.app.init.keystore",
            ),
            (
                RadrootsAppInitError::Config(RadrootsAppConfigError::MissingKeyMap("nostr_key")),
                "error.app.init.config",
            ),
            (
                RadrootsAppInitError::Assets(RadrootsAppInitAssetError::FetchUnavailable),
                "error.app.init.assets",
            ),
            (
                RadrootsAppInitError::State(RadrootsAppStateError::Missing),
                "error.app.state.missing",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), *expected);
            assert_eq!(err.to_string(), *expected);
        }
    }

    #[test]
    fn app_init_backends_maps_idb_errors() {
        let err = match futures::executor::block_on(app_init_backends(app_config_default())) {
            Ok(_) => panic!("idb bootstrap should error on non-wasm"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            RadrootsAppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined)
        );
    }

    #[test]
    fn app_init_timing_context_formats_elapsed() {
        let context = app_init_timing_context("idb", 123);
        assert_eq!(context, "idb_ms=123");
    }

    #[test]
    fn app_init_has_completed_is_false_on_native() {
        assert!(!super::app_init_has_completed());
    }

    #[test]
    fn app_init_reset_is_noop_on_native() {
        super::app_init_mark_completed();
        let result = futures::executor::block_on(super::app_init_reset::<
            radroots_studio_app_core::datastore::RadrootsClientWebDatastore,
            TestKeystore,
        >(None, None, None));
        assert!(result.is_ok());
    }

    #[test]
    fn app_init_reset_maps_datastore_errors() {
        let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(None);
        let err = futures::executor::block_on(super::app_init_reset::<
            radroots_studio_app_core::datastore::RadrootsClientWebDatastore,
            TestKeystore,
        >(Some(&datastore), None, None))
        .expect_err("datastore reset should error on native");
        assert_eq!(
            err,
            RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined)
        );
    }

    struct TestKeystore;

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
    fn app_init_reset_maps_keystore_errors() {
        let keystore = TestKeystore;
        let err = futures::executor::block_on(super::app_init_reset::<
            radroots_studio_app_core::datastore::RadrootsClientWebDatastore,
            TestKeystore,
        >(None, None, Some(&keystore)))
        .expect_err("keystore reset should error on native");
        assert_eq!(
            err,
            RadrootsAppInitError::Keystore(RadrootsClientKeystoreError::IdbUndefined)
        );
    }

    #[test]
    fn app_init_stage_roundtrip() {
        let stage = RadrootsAppInitStage::Ready;
        assert_eq!(stage.as_str(), "ready");
        assert_eq!(RadrootsAppInitStage::parse("ready"), Some(stage));
        assert_eq!(RadrootsAppInitStage::parse("unknown"), None);
    }

    #[test]
    fn app_init_state_defaults_match_spec() {
        let state = app_init_state_default();
        assert_eq!(state.stage, RadrootsAppInitStage::Idle);
        assert_eq!(state.loaded_bytes, 0);
        assert_eq!(state.total_bytes, Some(0));
    }

    #[test]
    fn app_init_progress_helpers_update_state() {
        let mut state = app_init_state_default();
        app_init_stage_set(&mut state, RadrootsAppInitStage::Storage);
        assert_eq!(state.stage, RadrootsAppInitStage::Storage);
        app_init_progress_add(&mut state, 0);
        assert_eq!(state.loaded_bytes, 0);
        app_init_progress_add(&mut state, 5);
        assert_eq!(state.loaded_bytes, 5);
        app_init_total_add(&mut state, 10);
        assert_eq!(state.total_bytes, Some(10));
        app_init_total_unknown(&mut state);
        assert_eq!(state.total_bytes, None);
        app_init_total_add(&mut state, 5);
        assert_eq!(state.total_bytes, None);
    }

    #[test]
    fn app_init_assets_skips_when_empty() {
        let config = app_config_default();
        let mut stages = Vec::new();
        let mut progress = Vec::new();
        let result = futures::executor::block_on(app_init_assets(
            &config,
            |stage| stages.push(stage),
            |loaded, total| progress.push((loaded, total)),
        ));
        assert!(result.is_ok());
        assert!(stages.is_empty());
        assert!(progress.is_empty());
    }

    #[test]
    fn app_init_assets_reports_unavailable_on_native() {
        let mut config = RadrootsAppConfig::empty();
        config.assets.sql_wasm_url = Some("http://example.com/sql.wasm".to_string());
        let result = futures::executor::block_on(app_init_assets(
            &config,
            |_stage| {},
            |_loaded, _total| {},
        ))
        .expect_err("asset fetch should error on native");
        assert_eq!(result, RadrootsAppInitAssetError::FetchUnavailable);
    }

    use std::cell::RefCell;

    struct SetupDatastore {
        state: Option<RadrootsAppState>,
        record: RefCell<Option<RadrootsAppStateRecord>>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for SetupDatastore {
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
            _key: &str,
            value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
            let encoded = serde_json::to_string(value)
                .map_err(|_| RadrootsClientDatastoreError::IdbUndefined)?;
            if let Ok(parsed) = serde_json::from_str::<RadrootsAppStateRecord>(&encoded) {
                *self.record.borrow_mut() = Some(parsed);
                return Ok(value.clone());
            }
            Err(RadrootsClientDatastoreError::IdbUndefined)
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
            if let Some(record) = self.record.borrow().as_ref() {
                let encoded = serde_json::to_string(record)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
                if let Ok(parsed) = serde_json::from_str(&encoded) {
                    return Ok(parsed);
                }
            };
            let Some(state) = self.state.as_ref() else {
                return Err(RadrootsClientDatastoreError::NoResult);
            };
            let encoded = serde_json::to_string(state)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            serde_json::from_str(&encoded).map_err(|_| RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
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
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn entries_pref(
            &self,
            _key_prefix: &str,
        ) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
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

    struct SetupKeystore {
        read_result: RadrootsClientKeystoreResult<String>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientKeystoreNostr for SetupKeystore {
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

    fn ready_state() -> RadrootsAppState {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        state.eula_version = APP_EULA_VERSION.to_string();
        state.eula_hash = APP_EULA_HASH.to_string();
        state
    }

    #[test]
    fn app_init_needs_setup_when_state_missing() {
        let datastore = SetupDatastore {
            state: None,
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let needs_setup = futures::executor::block_on(app_init_needs_setup(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("needs setup");
        assert!(needs_setup);
    }

    #[test]
    fn app_init_needs_setup_when_state_incomplete() {
        let datastore = SetupDatastore {
            state: Some(RadrootsAppState::default()),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let needs_setup = futures::executor::block_on(app_init_needs_setup(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("needs setup");
        assert!(needs_setup);
    }

    #[test]
    fn app_init_needs_setup_when_keystore_missing() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        let datastore = SetupDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Err(RadrootsClientKeystoreError::MissingKey),
        };
        let key_maps = app_key_maps_default();
        let needs_setup = futures::executor::block_on(app_init_needs_setup(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("needs setup");
        assert!(needs_setup);
    }

    #[test]
    fn app_init_needs_setup_is_false_when_ready() {
        let state = ready_state();
        let datastore = SetupDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let needs_setup = futures::executor::block_on(app_init_needs_setup(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("needs setup");
        assert!(!needs_setup);
    }

    #[test]
    fn app_init_setup_status_required_when_state_missing() {
        let datastore = SetupDatastore {
            state: None,
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let status = futures::executor::block_on(app_init_setup_status(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("setup status");
        assert_eq!(status, RadrootsAppSetupStatus::Required);
    }

    #[test]
    fn app_init_setup_status_corrupt_when_eula_mismatch() {
        let mut state = ready_state();
        state.eula_version = "0.0.0".to_string();
        let datastore = SetupDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let status = futures::executor::block_on(app_init_setup_status(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("setup status");
        assert_eq!(status, RadrootsAppSetupStatus::Corrupt);
    }

    #[test]
    fn app_init_setup_status_corrupt_when_keystore_missing() {
        let state = ready_state();
        let datastore = SetupDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Err(RadrootsClientKeystoreError::MissingKey),
        };
        let key_maps = app_key_maps_default();
        let status = futures::executor::block_on(app_init_setup_status(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("setup status");
        assert_eq!(status, RadrootsAppSetupStatus::Corrupt);
    }

    #[test]
    fn app_init_setup_status_configured_when_ready() {
        let state = ready_state();
        let datastore = SetupDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let keystore = SetupKeystore {
            read_result: Ok("secret".to_string()),
        };
        let key_maps = app_key_maps_default();
        let status = futures::executor::block_on(app_init_setup_status(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("setup status");
        assert_eq!(status, RadrootsAppSetupStatus::Configured);
    }
}
