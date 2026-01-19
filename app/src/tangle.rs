#![forbid(unsafe_code)]

use crate::app_log_debug_emit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTangleError {
    NotImplemented,
}

pub type AppTangleResult<T> = Result<T, AppTangleError>;

impl AppTangleError {
    pub const fn message(self) -> &'static str {
        match self {
            AppTangleError::NotImplemented => "error.app.tangle.not_implemented",
        }
    }
}

impl std::fmt::Display for AppTangleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppTangleError {}

pub trait AppTangleClient {
    fn init(&self) -> AppTangleResult<()>;
}

pub struct AppTangleClientStub;

impl AppTangleClientStub {
    pub fn new() -> Self {
        Self
    }
}

impl AppTangleClient for AppTangleClientStub {
    fn init(&self) -> AppTangleResult<()> {
        let _ = app_log_debug_emit("log.app.tangle.init", "stub", None);
        Err(AppTangleError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppTangleClient, AppTangleClientStub, AppTangleError};

    #[test]
    fn tangle_stub_reports_not_implemented() {
        let client = AppTangleClientStub::new();
        let err = client.init().expect_err("not implemented");
        assert_eq!(err, AppTangleError::NotImplemented);
        assert_eq!(err.to_string(), "error.app.tangle.not_implemented");
    }
}
