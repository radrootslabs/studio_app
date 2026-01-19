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
    app_datastore_has_app_data,
    app_datastore_has_config,
    app_datastore_key_nostr_key,
    app_datastore_read_app_data,
    app_datastore_write_app_data,
    app_datastore_write_config,
    app_assets_geocoder_db_url,
    app_assets_sql_wasm_url,
    app_keystore_nostr_ensure_key,
    app_log_debug_emit,
    AppAppData,
    AppConfig,
    AppConfigData,
    AppConfigError,
    AppKeystoreError,
    AppKeyMapConfig,
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
use web_sys::Response;

pub const APP_INIT_STORAGE_KEY: &str = "radroots.app.init.ready";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInitStage {
    Idle,
    Storage,
    DownloadSql,
    DownloadGeo,
    Database,
    Geocoder,
    Ready,
    Error,
}

impl AppInitStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            AppInitStage::Idle => "idle",
            AppInitStage::Storage => "storage",
            AppInitStage::DownloadSql => "download_sql",
            AppInitStage::DownloadGeo => "download_geo",
            AppInitStage::Database => "database",
            AppInitStage::Geocoder => "geocoder",
            AppInitStage::Ready => "ready",
            AppInitStage::Error => "error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "idle" => Some(AppInitStage::Idle),
            "storage" => Some(AppInitStage::Storage),
            "download_sql" => Some(AppInitStage::DownloadSql),
            "download_geo" => Some(AppInitStage::DownloadGeo),
            "database" => Some(AppInitStage::Database),
            "geocoder" => Some(AppInitStage::Geocoder),
            "ready" => Some(AppInitStage::Ready),
            "error" => Some(AppInitStage::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInitState {
    pub stage: AppInitStage,
    pub loaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

pub const fn app_init_state_default() -> AppInitState {
    AppInitState {
        stage: AppInitStage::Idle,
        loaded_bytes: 0,
        total_bytes: Some(0),
    }
}

pub fn app_init_stage_set(state: &mut AppInitState, stage: AppInitStage) {
    state.stage = stage;
}

pub fn app_init_progress_add(state: &mut AppInitState, bytes: u64) {
    if bytes == 0 {
        return;
    }
    state.loaded_bytes = state.loaded_bytes.saturating_add(bytes);
}

pub fn app_init_total_add(state: &mut AppInitState, bytes: u64) {
    if bytes == 0 {
        return;
    }
    let Some(total) = state.total_bytes else {
        return;
    };
    state.total_bytes = Some(total.saturating_add(bytes));
}

pub fn app_init_total_unknown(state: &mut AppInitState) {
    state.total_bytes = None;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppInitAssetProgress {
    pub loaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl AppInitAssetProgress {
    pub const fn empty() -> Self {
        Self {
            loaded_bytes: 0,
            total_bytes: Some(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInitAssetError {
    MissingUrl,
    FetchUnavailable,
    FetchFailed,
}

impl AppInitAssetError {
    pub const fn message(self) -> &'static str {
        match self {
            AppInitAssetError::MissingUrl => "error.app.init.asset_missing_url",
            AppInitAssetError::FetchUnavailable => "error.app.init.asset_unavailable",
            AppInitAssetError::FetchFailed => "error.app.init.asset_fetch_failed",
        }
    }
}

impl fmt::Display for AppInitAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppInitAssetError {}

#[cfg(target_arch = "wasm32")]
pub async fn app_init_fetch_asset<F>(
    url: &str,
    mut on_progress: F,
) -> Result<AppInitAssetProgress, AppInitAssetError>
where
    F: FnMut(u64, Option<u64>),
{
    if url.is_empty() {
        return Err(AppInitAssetError::MissingUrl);
    }
    let response_value = JsFuture::from(window().fetch_with_str(url))
        .await
        .map_err(|_| AppInitAssetError::FetchFailed)?;
    let response: Response = response_value
        .dyn_into()
        .map_err(|_| AppInitAssetError::FetchFailed)?;
    let total_bytes = response
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|value| value.parse::<u64>().ok());
    let buffer_value = JsFuture::from(response.array_buffer().map_err(|_| AppInitAssetError::FetchFailed)?)
        .await
        .map_err(|_| AppInitAssetError::FetchFailed)?;
    let buffer = Uint8Array::new(&buffer_value);
    let loaded_bytes = buffer.length() as u64;
    on_progress(loaded_bytes, total_bytes);
    Ok(AppInitAssetProgress {
        loaded_bytes,
        total_bytes,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn app_init_fetch_asset<F>(
    url: &str,
    _on_progress: F,
) -> Result<AppInitAssetProgress, AppInitAssetError>
where
    F: FnMut(u64, Option<u64>),
{
    if url.is_empty() {
        return Err(AppInitAssetError::MissingUrl);
    }
    Err(AppInitAssetError::FetchUnavailable)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInitError {
    Idb(RadrootsClientIdbStoreError),
    Datastore(RadrootsClientDatastoreError),
    Keystore(RadrootsClientKeystoreError),
    Config(AppConfigError),
    Assets(AppInitAssetError),
}

pub type AppInitErrorMessage = &'static str;

impl AppInitError {
    pub const fn message(&self) -> AppInitErrorMessage {
        match self {
            AppInitError::Idb(_) => "error.app.init.idb",
            AppInitError::Datastore(_) => "error.app.init.datastore",
            AppInitError::Keystore(_) => "error.app.init.keystore",
            AppInitError::Config(_) => "error.app.init.config",
            AppInitError::Assets(_) => "error.app.init.assets",
        }
    }
}

impl fmt::Display for AppInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppInitError {}

pub struct AppBackends {
    pub config: AppConfig,
    pub datastore: RadrootsClientWebDatastore,
    pub nostr_keystore: RadrootsClientWebKeystoreNostr,
}

pub type AppInitResult<T> = Result<T, AppInitError>;

pub async fn app_init_assets<F, G>(
    config: &AppConfig,
    mut on_stage: F,
    mut on_progress: G,
) -> Result<(), AppInitAssetError>
where
    F: FnMut(AppInitStage),
    G: FnMut(u64, Option<u64>),
{
    let _ = app_log_debug_emit("log.app.init.assets", "start", None);
    if let Some(url) = app_assets_sql_wasm_url(config).filter(|value| !value.is_empty()) {
        let _ = app_log_debug_emit("log.app.init.assets.sql", "download_start", Some(url.to_string()));
        on_stage(AppInitStage::DownloadSql);
        app_init_fetch_asset(url, |loaded, total| {
            on_progress(loaded, total);
        })
        .await?;
        let _ = app_log_debug_emit("log.app.init.assets.sql", "download_done", None);
    }
    if let Some(url) = app_assets_geocoder_db_url(config).filter(|value| !value.is_empty()) {
        let _ = app_log_debug_emit("log.app.init.assets.geo", "download_start", Some(url.to_string()));
        on_stage(AppInitStage::DownloadGeo);
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
    key_maps: Option<&AppKeyMapConfig>,
    keystore: Option<&K>,
) -> AppInitResult<()> {
    let _ = app_log_debug_emit("log.app.init.reset", "start", None);
    if let (Some(datastore), Some(key_maps)) = (datastore, key_maps) {
        app_datastore_clear_bootstrap(datastore, key_maps).await?;
    }
    if let Some(keystore) = keystore {
        keystore.reset().await.map_err(AppInitError::Keystore)?;
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

pub async fn app_init_backends(config: AppConfig) -> AppInitResult<AppBackends> {
    let _ = app_log_debug_emit("log.app.init.backends", "start", None);
    config.validate().map_err(AppInitError::Config)?;
    idb_store_bootstrap(RADROOTS_IDB_DATABASE, None)
        .await
        .map_err(AppInitError::Idb)?;
    let _ = app_log_debug_emit("log.app.init.backends", "idb_bootstrap", None);
    let datastore = RadrootsClientWebDatastore::new(Some(config.datastore.idb_config));
    datastore
        .init()
        .await
        .map_err(AppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.init.backends", "datastore_ready", None);
    let has_config = app_datastore_has_config(&datastore, &config.datastore.key_maps).await?;
    if !has_config {
        let config_data = AppConfigData::default();
        let _ =
            app_datastore_write_config(&datastore, &config.datastore.key_maps, &config_data)
                .await?;
    }
    let _ = app_log_debug_emit("log.app.init.backends", "config_ready", None);
    let nostr_keystore = RadrootsClientWebKeystoreNostr::new(Some(config.keystore.nostr_store));
    let nostr_public_key = app_keystore_nostr_ensure_key(&nostr_keystore)
        .await
        .map_err(|err| match err {
            AppKeystoreError::Keystore(inner) => AppInitError::Keystore(inner),
        })?;
    let _ = app_log_debug_emit("log.app.init.backends", "nostr_key_ready", None);
    let nostr_key =
        app_datastore_key_nostr_key(&config.datastore.key_maps).map_err(AppInitError::Config)?;
    match datastore.get(nostr_key).await {
        Ok(existing) => {
            if existing != nostr_public_key {
                let _ = datastore
                    .set(nostr_key, &nostr_public_key)
                    .await
                    .map_err(AppInitError::Datastore)?;
            }
        }
        Err(RadrootsClientDatastoreError::NoResult) => {
            let _ = datastore
                .set(nostr_key, &nostr_public_key)
                .await
                .map_err(AppInitError::Datastore)?;
        }
        Err(err) => return Err(AppInitError::Datastore(err)),
    }
    let _ = app_log_debug_emit("log.app.init.backends", "nostr_key_synced", None);
    let has_app_data = app_datastore_has_app_data(&datastore, &config.datastore.key_maps).await?;
    let mut app_data = if has_app_data {
        app_datastore_read_app_data(&datastore, &config.datastore.key_maps).await?
    } else {
        AppAppData::default()
    };
    let should_write = !has_app_data || app_data.active_key != nostr_public_key;
    if should_write {
        app_data.active_key = nostr_public_key;
        let _ =
            app_datastore_write_app_data(&datastore, &config.datastore.key_maps, &app_data)
                .await?;
    }
    let _ = app_log_debug_emit("log.app.init.backends", "app_data_ready", None);
    Ok(AppBackends {
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
        app_init_progress_add,
        app_init_state_default,
        app_init_stage_set,
        app_init_total_add,
        app_init_total_unknown,
        AppInitError,
        AppInitErrorMessage,
        AppInitStage,
        AppInitAssetError,
    };
    use crate::{app_config_default, AppConfig};
    use radroots_studio_app_core::datastore::RadrootsClientDatastoreError;
    use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
    };
    use async_trait::async_trait;
    use crate::AppConfigError;

    #[test]
    fn app_init_error_messages_match_spec() {
        let cases: &[(AppInitError, AppInitErrorMessage)] = &[
            (
                AppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined),
                "error.app.init.idb",
            ),
            (
                AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined),
                "error.app.init.datastore",
            ),
            (
                AppInitError::Keystore(RadrootsClientKeystoreError::IdbUndefined),
                "error.app.init.keystore",
            ),
            (
                AppInitError::Config(AppConfigError::MissingKeyMap("nostr_key")),
                "error.app.init.config",
            ),
            (
                AppInitError::Assets(AppInitAssetError::FetchUnavailable),
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
            AppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined)
        );
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
            AppInitError::Keystore(RadrootsClientKeystoreError::IdbUndefined)
        );
    }

    #[test]
    fn app_init_stage_roundtrip() {
        let stage = AppInitStage::Ready;
        assert_eq!(stage.as_str(), "ready");
        assert_eq!(AppInitStage::parse("ready"), Some(stage));
        assert_eq!(AppInitStage::parse("unknown"), None);
    }

    #[test]
    fn app_init_state_defaults_match_spec() {
        let state = app_init_state_default();
        assert_eq!(state.stage, AppInitStage::Idle);
        assert_eq!(state.loaded_bytes, 0);
        assert_eq!(state.total_bytes, Some(0));
    }

    #[test]
    fn app_init_progress_helpers_update_state() {
        let mut state = app_init_state_default();
        app_init_stage_set(&mut state, AppInitStage::Storage);
        assert_eq!(state.stage, AppInitStage::Storage);
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
        let mut config = AppConfig::empty();
        config.assets.sql_wasm_url = Some("http://example.com/sql.wasm".to_string());
        let result = futures::executor::block_on(app_init_assets(
            &config,
            |_stage| {},
            |_loaded, _total| {},
        ))
        .expect_err("asset fetch should error on native");
        assert_eq!(result, AppInitAssetError::FetchUnavailable);
    }
}
