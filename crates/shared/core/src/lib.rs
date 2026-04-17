#![forbid(unsafe_code)]

mod paths;
mod runtime;
mod startup;

pub use paths::{
    APP_RUNTIME_NAMESPACE, APP_RUNTIME_NAMESPACE_KIND, APP_RUNTIME_NAMESPACE_VALUE,
    AppRuntimeHostEnvironment, AppRuntimePathsError, AppRuntimePlatform, AppRuntimeRoots,
};
pub use runtime::{
    APP_ID, APP_NAME, APP_PLATFORM_RUNTIME, APP_PROJECTION_SOURCE, APP_RUNTIME_ORIGIN,
    AppBuildIdentity, AppCoreRuntimeMetadata, AppHostRuntimeMetadata, AppRuntimeCapture,
    AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label,
};
pub use startup::{AppStartupEvent, AppStartupEventMetadata, launch_startup_event};
