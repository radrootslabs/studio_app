#![forbid(unsafe_code)]

use radroots_types::types::IError;

pub const ERR_PREFIX_APP: &str = "error.app";
pub const ERR_PREFIX_UTILS: &str = "error.app.utils";

pub enum ErrInput {
    Message(String),
    Error(IError<String>),
}

impl From<String> for ErrInput {
    fn from(value: String) -> Self {
        ErrInput::Message(value)
    }
}

impl From<&str> for ErrInput {
    fn from(value: &str) -> Self {
        ErrInput::Message(value.to_string())
    }
}

impl From<IError<String>> for ErrInput {
    fn from(value: IError<String>) -> Self {
        ErrInput::Error(value)
    }
}

pub fn err_msg(err: impl Into<ErrInput>) -> IError<String> {
    match err.into() {
        ErrInput::Message(err) => IError { err },
        ErrInput::Error(err) => err,
    }
}

pub fn throw_err(err: impl Into<ErrInput>) -> ! {
    let err = err_msg(err);
    panic!("{}", err.err);
}

#[cfg(test)]
mod tests {
    use super::{err_msg, throw_err, ERR_PREFIX_APP, ERR_PREFIX_UTILS};
    use radroots_types::types::IError;

    #[test]
    fn err_msg_wraps_string() {
        let err = err_msg("boom");
        assert_eq!(err.err, "boom");
    }

    #[test]
    fn err_msg_accepts_error() {
        let err = err_msg(IError { err: "boom".to_string() });
        assert_eq!(err.err, "boom");
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn throw_err_panics_with_string() {
        throw_err("boom");
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn throw_err_panics_with_error() {
        throw_err(IError { err: "boom".to_string() });
    }

    #[test]
    fn error_prefixes_match_spec() {
        assert_eq!(ERR_PREFIX_APP, "error.app");
        assert_eq!(ERR_PREFIX_UTILS, "error.app.utils");
    }
}
