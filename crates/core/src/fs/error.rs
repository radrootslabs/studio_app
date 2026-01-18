use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientFsError {
    NotFound,
    RequestFailure,
}

pub type RadrootsClientFsErrorMessage = &'static str;

impl RadrootsClientFsError {
    pub const fn message(self) -> RadrootsClientFsErrorMessage {
        match self {
            RadrootsClientFsError::NotFound => "error.client.fs.not_found",
            RadrootsClientFsError::RequestFailure => "error.client.fs.request_failure",
        }
    }
}

impl fmt::Display for RadrootsClientFsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientFsError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientFsError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientFsError::NotFound,
                "error.client.fs.not_found",
            ),
            (
                RadrootsClientFsError::RequestFailure,
                "error.client.fs.request_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
