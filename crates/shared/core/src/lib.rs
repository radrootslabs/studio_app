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
    AppRuntimeHostEnvironment, AppRuntimePathsError, AppRuntimePlatform, AppRuntimeRoots,
};
pub use runtime::{
    APP_HOST_PLATFORM, APP_ID, APP_NAME, APP_PLATFORM_RUNTIME, APP_PROJECTION_SOURCE,
    APP_RUNTIME_CONFIG_ENV, APP_RUNTIME_CONFIG_SCHEMA, APP_RUNTIME_ORIGIN, AppBuildIdentity,
    AppCoreRuntimeMetadata, AppHostRuntimeMetadata, AppRuntimeCapture, AppRuntimeConfig,
    AppRuntimeConfigError, AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label,
};
pub use startup::{AppStartupEvent, AppStartupEventMetadata, launch_startup_event};
