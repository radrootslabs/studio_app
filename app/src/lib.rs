#![forbid(unsafe_code)]

mod app;
mod context;
mod config;
mod data;
mod init;
mod entry;

pub use app::App;
pub use context::{app_context, AppContext};
pub use data::{AppAppData, AppConfigData, AppConfigRole};
pub use config::{
    app_config_default,
    app_config_from_env,
    app_datastore_param_nostr_profile,
    app_datastore_param_radroots_profile,
    app_keystore_key_maps_default,
    app_key_maps_default,
    app_key_maps_validate,
    AppConfig,
    AppConfigError,
    AppConfigResult,
    AppDatastoreConfig,
    AppDatastoreKeyMap,
    AppDatastoreKeyObjMap,
    AppDatastoreKeyParam,
    AppDatastoreKeyParamMap,
    AppKeystoreConfig,
    AppKeystoreKeyMap,
    AppKeyMapConfig,
    APP_DATASTORE_KEY_EULA_DATE,
    APP_DATASTORE_KEY_NOSTR_KEY,
    APP_DATASTORE_KEY_OBJ_APP_DATA,
    APP_DATASTORE_KEY_OBJ_CFG_DATA,
};
pub use init::{
    app_init_backends,
    app_init_has_completed,
    app_init_mark_completed,
    app_init_reset,
    app_init_state_default,
    AppBackends,
    AppInitError,
    AppInitErrorMessage,
    AppInitResult,
    AppInitStage,
    AppInitState,
    APP_INIT_STORAGE_KEY,
};
