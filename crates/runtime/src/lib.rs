#![forbid(unsafe_code)]

mod logging;
mod pack_day_export;
mod paths;
mod runtime;
mod sdk;
mod startup;

pub use logging::{
    APP_LOG_PRODUCT, APP_LOG_SCHEMA_VERSION, AppLoggingError, AppLoggingOptions,
    app_runtime_log_dir, bootstrap_logging, init_logging, install_panic_hook,
};
pub use pack_day_export::{
    APP_EXPORTS_DIR_NAME, PACK_DAY_EXPORTS_DIR_NAME, PackDayExportDocument,
    PackDayExportWriteError, PreparedPackDayExportBundle, app_exports_root,
    app_exports_root_from_data_root, prepare_pack_day_export_bundle,
    prepare_pack_day_export_bundle_at_data_root, write_prepared_pack_day_export_bundle,
};
pub use paths::{
    APP_PATHS_PROFILE_ENV, APP_PATHS_REPO_LOCAL_ROOT_ENV, APP_RUNTIME_NAMESPACE,
    APP_RUNTIME_NAMESPACE_KIND, APP_RUNTIME_NAMESPACE_VALUE, AppDesktopRuntimePaths,
    AppRuntimeHostEnvironment, AppRuntimePathsError, AppRuntimePlatform, AppRuntimeRoots,
    AppSharedAccountsPaths, AppSharedIdentityPaths, SHARED_ACCOUNTS_NAMESPACE,
    SHARED_ACCOUNTS_NAMESPACE_KIND, SHARED_ACCOUNTS_NAMESPACE_VALUE,
    SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITIES_NAMESPACE, SHARED_IDENTITIES_NAMESPACE_KIND,
    SHARED_IDENTITIES_NAMESPACE_VALUE, SHARED_IDENTITY_FILE_NAME,
    SHARED_RUNTIME_STORE_DB_FILE_NAME, SHARED_RUNTIME_STORE_NAMESPACE,
    SHARED_RUNTIME_STORE_NAMESPACE_KIND, SHARED_RUNTIME_STORE_NAMESPACE_VALUE,
    shared_runtime_store_database_path_from_shared_accounts,
};
pub use runtime::{
    APP_HOST_PLATFORM, APP_ID, APP_LOCAL_LOG_ROOT_ENV, APP_NAME, APP_NOSTR_RELAY_URLS_ENV,
    APP_PLATFORM_RUNTIME, APP_PROJECTION_SOURCE, APP_RUNTIME_MODE_ENV, APP_RUNTIME_ORIGIN,
    AppBuildIdentity, AppCoreRuntimeMetadata, AppHostRuntimeMetadata, AppRuntimeCapture,
    AppRuntimeConfig, AppRuntimeConfigError, AppRuntimeMode, AppRuntimeSnapshot,
    runtime_mode_label,
};
pub use sdk::{
    DESKTOP_RUNTIME_DEFAULT_EFFECT_QUEUE_CAPACITY, DESKTOP_RUNTIME_STORAGE_DIR_NAME,
    DesktopRuntimeDiagnostics, DesktopRuntimeEffectKind, DesktopRuntimeEffectReceipt,
    DesktopRuntimeEffectState, DesktopRuntimeEffectStatus, DesktopRuntimeEventStoreDiagnostics,
    DesktopRuntimeFarmPublicLocationRequest, DesktopRuntimeFarmPublishRequest,
    DesktopRuntimeIntegrityDiagnostics, DesktopRuntimeIssue, DesktopRuntimeLifecycleState,
    DesktopRuntimeListingPublishRequest, DesktopRuntimeLocalSigner,
    DesktopRuntimeOutboxDiagnostics, DesktopRuntimeProjectionLifecycleState,
    DesktopRuntimeProjectionLifecycleStatus, DesktopRuntimePublicFarmLocation,
    DesktopRuntimeRelayUrlPolicy, DesktopRuntimeRestorePreflightReceipt,
    DesktopRuntimeRestorePreflightRequest, DesktopRuntimeSnapshot,
    DesktopRuntimeSqliteStoreDiagnostics, DesktopRuntimeStartupMilestone,
    DesktopRuntimeStorageDiagnostics, DesktopRuntimeStoragePaths, DesktopRuntimeSupervisor,
    DesktopRuntimeSupervisorConfig, DesktopRuntimeSupervisorError, DesktopRuntimeSyncDiagnostics,
    DesktopRuntimeSyncEventStoreDiagnostics, DesktopRuntimeSyncOutboxDiagnostics,
    DesktopRuntimeSyncTransportTargetDiagnostics, DesktopRuntimeTradeCancellationRequest,
    DesktopRuntimeTradeDecision, DesktopRuntimeTradeDecisionRequest,
    DesktopRuntimeTradeProposeRequest, DesktopRuntimeWorkflowReceipt,
    desktop_runtime_storage_root_from_data_root,
};
pub use startup::{AppStartupEvent, AppStartupEventMetadata, launch_startup_event};
