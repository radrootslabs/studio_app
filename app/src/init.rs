#![forbid(unsafe_code)]

use std::fmt;

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
    app_datastore_clear_bootstrap,
    app_datastore_has_state,
    app_datastore_has_settings,
    app_datastore_key_nostr_key,
    app_datastore_read_state,
    app_datastore_write_state,
    app_datastore_write_settings,
    app_assets_geocoder_db_url,
    app_assets_sql_wasm_url,
    app_keystore_nostr_ensure_key,
    app_log_debug_emit,
    RadrootsAppState,
    RadrootsAppConfig,
    RadrootsAppSettings,
    RadrootsAppConfigError,
    RadrootsAppKeystoreError,
    RadrootsAppKeyMapConfig,
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
    pub datastore: RadrootsClientWebDatastore,
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
    key_maps: Option<&RadrootsAppKeyMapConfig>,
    keystore: Option<&K>,
) -> RadrootsAppInitResult<()> {
    let _ = app_log_debug_emit("log.app.init.reset", "start", None);
    if let (Some(datastore), Some(key_maps)) = (datastore, key_maps) {
        app_datastore_clear_bootstrap(datastore, key_maps).await?;
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
    let datastore = RadrootsClientWebDatastore::new(Some(config.datastore.idb_config));
    datastore
        .init()
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.init.backends", "datastore_ready", None);
    let has_config = app_datastore_has_settings(&datastore, &config.datastore.key_maps).await?;
    if !has_config {
        let config_data = RadrootsAppSettings::default();
        let _ =
            app_datastore_write_settings(&datastore, &config.datastore.key_maps, &config_data)
                .await?;
    }
    let _ = app_log_debug_emit("log.app.init.backends", "config_ready", None);
    let nostr_keystore = RadrootsClientWebKeystoreNostr::new(Some(config.keystore.nostr_store));
    let key_start = app_init_timer_start();
    let nostr_public_key = app_keystore_nostr_ensure_key(&nostr_keystore)
        .await
        .map_err(|err| match err {
            RadrootsAppKeystoreError::Keystore(inner) => RadrootsAppInitError::Keystore(inner),
        })?;
    let key_ms = app_init_elapsed_ms(key_start);
    let _ = app_log_debug_emit(
        "log.app.init.backends",
        "nostr_key_ready",
        Some(app_init_timing_context("elapsed", key_ms)),
    );
    let nostr_key =
        app_datastore_key_nostr_key(&config.datastore.key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.get(nostr_key).await {
        Ok(existing) => {
            if existing != nostr_public_key {
                let _ = datastore
                    .set(nostr_key, &nostr_public_key)
                    .await
                    .map_err(RadrootsAppInitError::Datastore)?;
            }
        }
        Err(RadrootsClientDatastoreError::NoResult) => {
            let _ = datastore
                .set(nostr_key, &nostr_public_key)
                .await
                .map_err(RadrootsAppInitError::Datastore)?;
        }
        Err(err) => return Err(RadrootsAppInitError::Datastore(err)),
    }
    let _ = app_log_debug_emit("log.app.init.backends", "nostr_key_synced", None);
    let has_app_data = app_datastore_has_state(&datastore, &config.datastore.key_maps).await?;
    let mut app_data = if has_app_data {
        app_datastore_read_state(&datastore, &config.datastore.key_maps).await?
    } else {
        RadrootsAppState::default()
    };
    let should_write = !has_app_data || app_data.active_key != nostr_public_key;
    if should_write {
        app_data.active_key = nostr_public_key;
        let _ =
            app_datastore_write_state(&datastore, &config.datastore.key_maps, &app_data)
                .await?;
    }
    let _ = app_log_debug_emit("log.app.init.backends", "app_data_ready", None);
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
    use crate::{app_config_default, RadrootsAppConfig};
    use radroots_studio_app_core::datastore::RadrootsClientDatastoreError;
    use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
    };
    use async_trait::async_trait;
    use crate::RadrootsAppConfigError;

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
}
