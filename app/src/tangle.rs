#![forbid(unsafe_code)]

use crate::app_log_debug_emit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppTangleError {
    NotImplemented,
}

pub type RadrootsAppTangleResult<T> = Result<T, RadrootsAppTangleError>;

impl RadrootsAppTangleError {
    pub const fn message(self) -> &'static str {
        match self {
            RadrootsAppTangleError::NotImplemented => "error.app.tangle.not_implemented",
        }
    }
}

impl std::fmt::Display for RadrootsAppTangleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppTangleError {}

pub trait RadrootsAppTangleClient {
    fn init(&self) -> RadrootsAppTangleResult<()>;
}

pub struct RadrootsAppTangleClientStub;

impl RadrootsAppTangleClientStub {
    pub fn new() -> Self {
        Self
    }
}

impl RadrootsAppTangleClient for RadrootsAppTangleClientStub {
    fn init(&self) -> RadrootsAppTangleResult<()> {
        let _ = app_log_debug_emit("log.app.tangle.init", "stub", None);
        Err(RadrootsAppTangleError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::{RadrootsAppTangleClient, RadrootsAppTangleClientStub, RadrootsAppTangleError};

    #[test]
    fn tangle_stub_reports_not_implemented() {
        let client = RadrootsAppTangleClientStub::new();
        let err = client.init().expect_err("not implemented");
        assert_eq!(err, RadrootsAppTangleError::NotImplemented);
        assert_eq!(err.to_string(), "error.app.tangle.not_implemented");
    }
}
