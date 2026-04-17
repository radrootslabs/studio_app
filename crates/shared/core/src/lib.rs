#![forbid(unsafe_code)]

mod runtime;
mod startup;

pub use runtime::{
    APP_ID, APP_NAME, APP_PLATFORM_RUNTIME, APP_PROJECTION_SOURCE, APP_RUNTIME_ORIGIN,
    AppBuildIdentity, AppCoreRuntimeMetadata, AppHostRuntimeMetadata, AppRuntimeCapture,
    AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label,
};
pub use startup::{AppStartupEvent, AppStartupEventMetadata, launch_startup_event};
