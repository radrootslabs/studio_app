#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUtilsError {
    InvalidInput,
    Unavailable,
}

pub type RadrootsAppUtilsErrorMessage = &'static str;

impl RadrootsAppUtilsError {
    pub const fn message(self) -> RadrootsAppUtilsErrorMessage {
        match self {
            RadrootsAppUtilsError::InvalidInput => "error.app.utils.invalid_input",
            RadrootsAppUtilsError::Unavailable => "error.app.utils.unavailable",
        }
    }
}

impl fmt::Display for RadrootsAppUtilsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppUtilsError {}

#[cfg(test)]
mod tests {
    use super::RadrootsAppUtilsError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsAppUtilsError::InvalidInput,
                "error.app.utils.invalid_input",
            ),
            (
                RadrootsAppUtilsError::Unavailable,
                "error.app.utils.unavailable",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
