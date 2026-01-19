#![forbid(unsafe_code)]

mod app;
mod init;
mod entry;

pub use app::App;
pub use init::{
    app_init_backends,
    app_init_has_completed,
    app_init_mark_completed,
    app_init_reset,
    AppBackends,
    AppInitError,
    AppInitErrorMessage,
    AppInitResult,
    APP_INIT_STORAGE_KEY,
};
