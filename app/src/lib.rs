#![forbid(unsafe_code)]

mod app;
mod bootstrap;
mod context;
mod config;
mod data;
mod health;
mod init;
mod keystore;
mod logging;
mod logs;
mod notifications;
mod tangle;
mod entry;

pub use app::App;
pub use bootstrap::{
    app_datastore_clear_bootstrap,
    app_datastore_has_app_data,
    app_datastore_has_config,
    app_datastore_read_app_data,
    app_datastore_set_notifications_permission,
    app_datastore_write_app_data,
    app_datastore_write_config,
};
pub use context::{app_context, AppContext};
pub use data::{RadrootsAppState, RadrootsAppSettings, RadrootsAppRole};
pub use health::{
    app_health_check_all,
    app_health_check_all_logged,
    app_health_check_app_data_active_key,
    app_health_check_bootstrap_app_data,
    app_health_check_bootstrap_config,
    app_health_check_datastore_roundtrip,
    app_health_check_keystore_access,
    app_health_check_notifications,
    app_health_check_tangle,
    app_health_check_key_maps,
    AppHealthCheckResult,
    AppHealthCheckStatus,
    AppHealthReport,
};
pub use keystore::{
    app_keystore_nostr_ensure_key,
    app_keystore_nostr_keys,
    app_keystore_nostr_public_key,
    AppKeystoreError,
    AppKeystoreResult,
};
pub use logs::LogsPage;
pub use logging::{
    app_log_entry_error,
    app_log_entry_emit,
    app_log_entry_new,
    app_log_entry_record,
    app_log_entry_store,
    app_log_buffer_drain,
    app_log_buffer_flush_critical,
    app_log_buffer_flush,
    app_log_buffer_push,
    app_log_entries_dump,
    app_log_entries_clear,
    app_log_entries_load,
    app_log_entries_prune,
    app_log_error_emit,
    app_log_error_store,
    app_log_entry_key,
    app_log_entry_prefix,
    app_log_debug_emit,
    app_log_info_emit,
    app_log_metadata,
    app_log_timestamp_ms,
    app_log_warn_emit,
    app_logging_init,
    AppLogEntry,
    AppLogError,
    AppLogLevel,
    AppLogResult,
    AppLoggableError,
    AppLogMetadata,
    AppLoggingError,
    AppLoggingResult,
    APP_LOG_BUFFER_MAX_ENTRIES,
    APP_LOG_MAX_ENTRIES,
};
pub use notifications::{AppNotifications, AppNotificationsError, AppNotificationsResult};
pub use tangle::{AppTangleClient, AppTangleClientStub, AppTangleError, AppTangleResult};
pub use config::{
    app_config_default,
    app_config_from_env,
    app_datastore_key,
    app_datastore_key_eula_date,
    app_datastore_key_nostr_key,
    app_datastore_param_nostr_profile,
    app_datastore_param_log_entry,
    app_datastore_param_radroots_profile,
    app_datastore_param_key,
    app_datastore_obj_key,
    app_datastore_obj_key_app_data,
    app_datastore_obj_key_cfg_data,
    app_assets_geocoder_db_url,
    app_assets_sql_wasm_url,
    app_keystore_key_maps_default,
    app_keystore_key_maps_validate,
    app_keystore_key,
    app_keystore_key_nostr_default,
    app_key_maps_default,
    app_key_maps_validate,
    RadrootsAppConfig,
    RadrootsAppConfigError,
    RadrootsAppConfigResult,
    RadrootsAppAssetConfig,
    RadrootsAppDatastoreConfig,
    RadrootsAppDatastoreKeyMap,
    RadrootsAppDatastoreKeyObjMap,
    RadrootsAppDatastoreKeyParam,
    RadrootsAppDatastoreKeyParamMap,
    RadrootsAppKeystoreConfig,
    RadrootsAppKeystoreKeyMap,
    RadrootsAppKeyMapConfig,
    APP_DATASTORE_KEY_EULA_DATE,
    APP_DATASTORE_KEY_LOG_ENTRY,
    APP_DATASTORE_KEY_NOSTR_KEY,
    APP_DATASTORE_KEY_OBJ_APP_DATA,
    APP_DATASTORE_KEY_OBJ_CFG_DATA,
    APP_KEYSTORE_KEY_NOSTR_DEFAULT,
};
pub use init::{
    app_init_assets,
    app_init_backends,
    app_init_fetch_asset,
    app_init_has_completed,
    app_init_mark_completed,
    app_init_progress_add,
    app_init_reset,
    app_init_state_default,
    app_init_stage_set,
    app_init_total_add,
    app_init_total_unknown,
    AppBackends,
    AppInitAssetError,
    AppInitAssetProgress,
    AppInitError,
    AppInitErrorMessage,
    AppInitResult,
    AppInitStage,
    AppInitState,
    APP_INIT_STORAGE_KEY,
};
