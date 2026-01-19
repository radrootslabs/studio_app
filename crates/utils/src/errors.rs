#![forbid(unsafe_code)]

use radroots_types::types::IError;

pub const ERR_PREFIX_APP: &str = "error.app";
pub const ERR_PREFIX_UTILS: &str = "error.app.utils";

pub fn err_msg(err: impl Into<String>) -> IError<String> {
    IError { err: err.into() }
}

#[cfg(test)]
mod tests {
    use super::{err_msg, ERR_PREFIX_APP, ERR_PREFIX_UTILS};

    #[test]
    fn err_msg_wraps_string() {
        let err = err_msg("boom");
        assert_eq!(err.err, "boom");
    }

    #[test]
    fn error_prefixes_match_spec() {
        assert_eq!(ERR_PREFIX_APP, "error.app");
        assert_eq!(ERR_PREFIX_UTILS, "error.app.utils");
    }
}
