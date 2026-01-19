#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;

pub type ResolveError<T> = Result<T, RadrootsAppUtilsError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultPass {
    pub pass: bool,
}

impl ResultPass {
    pub const fn ok() -> Self {
        Self { pass: true }
    }
}

pub fn resolve_ok<T>(value: T) -> ResolveError<T> {
    Ok(value)
}

pub fn resolve_err<T>(err: RadrootsAppUtilsError) -> ResolveError<T> {
    Err(err)
}

#[cfg(test)]
mod tests {
    use super::{resolve_err, resolve_ok, ResultPass};
    use crate::error::RadrootsAppUtilsError;

    #[test]
    fn result_pass_is_true() {
        let pass = ResultPass::ok();
        assert!(pass.pass);
    }

    #[test]
    fn resolve_ok_returns_value() {
        let value = resolve_ok(5).expect("value");
        assert_eq!(value, 5);
    }

    #[test]
    fn resolve_err_returns_error() {
        let err = resolve_err::<()>(RadrootsAppUtilsError::Unavailable)
            .expect_err("err");
        assert_eq!(err, RadrootsAppUtilsError::Unavailable);
    }
}
