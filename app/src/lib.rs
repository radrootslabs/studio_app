#![forbid(unsafe_code)]

mod app;
mod bootstrap;
mod context;
mod config;
mod configuration;
mod data;
mod health;
mod health_ui;
mod init;
mod i18n;
mod keystore;
mod logging;
mod logs;
mod notifications;
mod settings;
mod settings_status;
mod setup;
mod setup_flow;
mod setup_lock;
mod setup_status;
mod theme;
mod tangle;
mod ui_demo;
mod entry;

pub use app::RadrootsApp;
pub use bootstrap::{
    app_datastore_clear_bootstrap,
    app_datastore_create_state,
    app_datastore_has_state,
    app_datastore_read_state,
    app_datastore_read_setup_draft,
    app_datastore_write_setup_draft,
    app_datastore_clear_setup_draft,
    app_datastore_write_profile_seed,
    app_datastore_update_state,
    app_state_set_notifications_permission,
    app_state_set_notifications_permission_value,
    app_state_notifications_permission_value,
    app_datastore_write_state,
};
pub use context::{app_context, RadrootsAppContext};
pub use data::{
    app_state_is_initialized,
    app_state_record_new,
    app_state_record_validate,
    app_state_timestamp_ms,
    RadrootsAppProfileSeed,
    RadrootsAppRole,
    RadrootsAppState,
    RadrootsAppSetupDraft,
    RadrootsAppStateError,
    RadrootsAppStateRecord,
    APP_EULA_HASH,
    APP_EULA_VERSION,
    APP_STATE_SCHEMA_VERSION,
};
pub use configuration::{
    app_config_record_new,
    app_config_record_validate,
    app_datastore_clear_config,
    app_datastore_create_config,
    app_datastore_has_config,
    app_datastore_read_config,
    app_datastore_read_config_record,
    app_datastore_update_config,
    app_datastore_write_config_record,
    RadrootsAppConfigBusiness,
    RadrootsAppConfigData,
    RadrootsAppConfigFarmer,
    RadrootsAppConfigIndividual,
    RadrootsAppConfigPreferences,
    RadrootsAppConfigProfile,
    RadrootsAppConfigRecord,
    RadrootsAppConfigRecordError,
    RadrootsAppConfigStoreError,
    RadrootsAppConfigStoreResult,
    APP_CONFIG_SCHEMA_VERSION,
};
pub use health::{
    app_health_check_all,
    app_health_check_all_logged,
    app_health_check_state_active_key,
    app_health_check_bootstrap_state,
    app_health_check_datastore_roundtrip,
    app_health_check_keystore_access,
    app_health_check_notifications,
    app_health_check_tangle,
    app_health_check_key_maps,
    RadrootsAppHealthCheckResult,
    RadrootsAppHealthCheckStatus,
    RadrootsAppHealthReport,
};
pub use health_ui::{
    active_key_label,
    app_health_check_delay_ms,
    health_message_label,
    health_report_summary,
    health_result_label,
    health_status_class,
    health_status_label,
    spawn_health_checks,
};
pub use keystore::{
    app_keystore_nostr_ensure_key,
    app_keystore_nostr_keys,
    app_keystore_nostr_public_key,
    app_keystore_nostr_verify_key,
    RadrootsAppKeystoreError,
    RadrootsAppKeystoreResult,
};
pub use logs::RadrootsAppLogsPage;
pub use settings::RadrootsAppSettingsPage;
pub use settings_status::RadrootsAppSettingsStatusPage;
pub use setup_status::{
    app_setup_gate_from_status,
    RadrootsAppSetupGate,
    RadrootsAppSetupStatus,
};
pub use ui_demo::RadrootsAppUiDemoPage;
pub use theme::{
    app_theme_apply_mode,
    app_theme_init,
    app_theme_read_mode,
    app_theme_store_mode,
    app_theme_mode_from_value,
    app_theme_mode_to_name,
    RadrootsAppThemeError,
    RadrootsAppThemeMode,
    RadrootsAppThemeResult,
    APP_THEME_STORAGE_KEY,
};
pub use logging::{
    app_log_entry_error,
    app_log_entry_emit,
    app_log_entry_new,
    app_log_entry_record,
    app_log_entry_store,
    app_log_buffer_drain,
    app_log_buffer_flush_critical,
    app_log_buffer_flush_deferred,
    app_log_buffer_flush,
    app_log_buffer_flush_no_prune,
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
    app_log_dump_header,
    app_log_info_emit,
    app_log_metadata,
    app_log_timestamp_ms,
    app_log_warn_emit,
    app_logging_init,
    RadrootsAppLogDumpMeta,
    RadrootsAppLogEntry,
    RadrootsAppLogError,
    RadrootsAppLogLevel,
    RadrootsAppLogResult,
    RadrootsAppLoggableError,
    RadrootsAppLogMetadata,
    RadrootsAppLoggingError,
    RadrootsAppLoggingResult,
    APP_LOG_BUFFER_MAX_ENTRIES,
    APP_LOG_MAX_ENTRIES,
};
pub use notifications::{RadrootsAppNotifications, RadrootsAppNotificationsError, RadrootsAppNotificationsResult};
pub use setup::{
    app_setup_eula_date,
    app_setup_commit,
    app_setup_finalize_with_key,
    app_setup_initialize,
    app_setup_state_new,
    app_setup_step_default,
    RadrootsAppSetupStep,
};
pub use setup_flow::{
    app_setup_flow_next_step,
    app_setup_flow_prev_step,
    app_setup_flow_role_from_choices,
    app_setup_flow_validate,
    RadrootsAppSetupBusinessChoice,
    RadrootsAppSetupFarmerChoice,
    RadrootsAppSetupFlowDraft,
    RadrootsAppSetupFlowValidation,
    RadrootsAppSetupKeyChoice,
};
pub use setup_lock::{
    app_setup_lock_acquire,
    app_setup_lock_enabled,
    app_setup_lock_is_expired,
    app_setup_lock_release,
    app_setup_lock_ttl_ms,
    RadrootsAppSetupLock,
    RadrootsAppSetupLockStatus,
    APP_SETUP_LOCK_TTL_MS,
};
pub use tangle::{RadrootsAppTangleClient, RadrootsAppTangleClientStub, RadrootsAppTangleError, RadrootsAppTangleResult};
pub use config::{
    app_config_default,
    app_config_from_env,
    app_default_relays,
    app_datastore_key,
    app_datastore_key_eula_date,
    app_datastore_key_nostr_key,
    app_datastore_param_nostr_profile,
    app_datastore_param_log_entry,
    app_datastore_param_radroots_profile,
    app_datastore_param_key,
    app_datastore_obj_key,
    app_datastore_obj_key_state,
    app_datastore_obj_key_setup_draft,
    app_datastore_obj_key_config,
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
    APP_DATASTORE_KEY_OBJ_STATE,
    APP_DATASTORE_KEY_OBJ_SETUP_DRAFT,
    APP_DATASTORE_KEY_OBJ_CONFIG,
    APP_DATASTORE_KEY_SETUP_LOCK,
    APP_KEYSTORE_KEY_NOSTR_DEFAULT,
    app_datastore_key_setup_lock,
};
pub use init::{
    app_init_assets,
    app_init_backends,
    app_init_fetch_asset,
    app_init_has_completed,
    app_init_needs_setup,
    app_init_setup_status,
    app_init_mark_completed,
    app_init_progress_add,
    app_init_reset,
    app_init_state_default,
    app_init_stage_set,
    app_init_total_add,
    app_init_total_unknown,
    RadrootsAppBackends,
    RadrootsAppInitAssetError,
    RadrootsAppInitAssetProgress,
    RadrootsAppInitError,
    RadrootsAppInitErrorMessage,
    RadrootsAppInitResult,
    RadrootsAppInitStage,
    RadrootsAppInitState,
    APP_INIT_STORAGE_KEY,
};
pub use i18n::{app_i18n, app_i18n_init, RadrootsAppI18nContext};
