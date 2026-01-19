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
use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientWebKeystoreNostr};

use crate::{
    app_datastore_write_app_data,
    app_datastore_write_config,
    AppAppData,
    AppConfig,
    AppConfigData,
    AppConfigError,
};

#[cfg(target_arch = "wasm32")]
use leptos::prelude::window;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInitError {
    Idb(RadrootsClientIdbStoreError),
    Datastore(RadrootsClientDatastoreError),
    Keystore(RadrootsClientKeystoreError),
    Config(AppConfigError),
}

pub type AppInitErrorMessage = &'static str;

impl AppInitError {
    pub const fn message(&self) -> AppInitErrorMessage {
        match self {
            AppInitError::Idb(_) => "error.app.init.idb",
            AppInitError::Datastore(_) => "error.app.init.datastore",
            AppInitError::Keystore(_) => "error.app.init.keystore",
            AppInitError::Config(_) => "error.app.init.config",
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

pub fn app_init_reset() {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(APP_INIT_STORAGE_KEY);
        }
    }
}

pub async fn app_init_backends(config: AppConfig) -> AppInitResult<AppBackends> {
    config.validate().map_err(AppInitError::Config)?;
    idb_store_bootstrap(RADROOTS_IDB_DATABASE, None)
        .await
        .map_err(AppInitError::Idb)?;
    let datastore = RadrootsClientWebDatastore::new(Some(config.datastore.idb_config));
    datastore
        .init()
        .await
        .map_err(AppInitError::Datastore)?;
    let config_data = AppConfigData::default();
    let _ = app_datastore_write_config(&datastore, &config.datastore.key_maps, &config_data).await?;
    let app_data = AppAppData::default();
    let _ = app_datastore_write_app_data(&datastore, &config.datastore.key_maps, &app_data).await?;
    let nostr_keystore = RadrootsClientWebKeystoreNostr::new(Some(config.keystore.nostr_store));
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
        app_init_state_default,
        AppInitError,
        AppInitErrorMessage,
        AppInitStage,
    };
    use crate::app_config_default;
    use radroots_studio_app_core::datastore::RadrootsClientDatastoreError;
    use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
    use radroots_studio_app_core::keystore::RadrootsClientKeystoreError;
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
        super::app_init_reset();
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
}
