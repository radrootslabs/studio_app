#![forbid(unsafe_code)]

mod app;
mod context;
mod init;
mod entry;

pub use app::App;
pub use context::{app_context, AppContext};
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
