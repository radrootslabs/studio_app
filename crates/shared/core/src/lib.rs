#![forbid(unsafe_code)]

mod logging;
mod paths;
mod runtime;
mod startup;

pub use logging::{
    APP_LOG_PRODUCT, APP_LOG_SCHEMA_VERSION, AppLoggingError, AppLoggingOptions,
    app_runtime_log_dir, bootstrap_logging, init_logging, install_panic_hook,
};
pub use paths::{
    APP_RUNTIME_NAMESPACE, APP_RUNTIME_NAMESPACE_KIND, APP_RUNTIME_NAMESPACE_VALUE,
    AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePathsError, AppRuntimePlatform,
    AppRuntimeRoots, AppSharedAccountsPaths, AppSharedIdentityPaths, SHARED_ACCOUNTS_NAMESPACE,
    SHARED_ACCOUNTS_NAMESPACE_KIND, SHARED_ACCOUNTS_NAMESPACE_VALUE,
    SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITIES_NAMESPACE, SHARED_IDENTITIES_NAMESPACE_KIND,
    SHARED_IDENTITIES_NAMESPACE_VALUE, SHARED_IDENTITY_FILE_NAME,
};
pub use runtime::{
    APP_HOST_PLATFORM, APP_ID, APP_NAME, APP_PLATFORM_RUNTIME, APP_PROJECTION_SOURCE,
    APP_RUNTIME_CONFIG_ENV, APP_RUNTIME_CONFIG_SCHEMA, APP_RUNTIME_ORIGIN, AppBuildIdentity,
    AppCoreRuntimeMetadata, AppHostRuntimeMetadata, AppRuntimeCapture, AppRuntimeConfig,
    AppRuntimeConfigError, AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label,
};
pub use startup::{AppStartupEvent, AppStartupEventMetadata, launch_startup_event};
