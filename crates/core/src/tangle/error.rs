use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientTangleError {
    InitFailure,
    ParseFailure,
    InvalidResponse,
    RuntimeUnavailable,
    CryptoUnavailable,
}

pub type RadrootsClientTangleErrorMessage = &'static str;

impl RadrootsClientTangleError {
    pub const fn message(self) -> RadrootsClientTangleErrorMessage {
        match self {
            RadrootsClientTangleError::InitFailure => "error.client.tangle.init_failure",
            RadrootsClientTangleError::ParseFailure => "error.client.tangle.parse_failure",
            RadrootsClientTangleError::InvalidResponse => {
                "error.client.tangle.invalid_response"
            }
            RadrootsClientTangleError::RuntimeUnavailable => {
                "error.client.tangle.runtime_unavailable"
            }
            RadrootsClientTangleError::CryptoUnavailable => {
                "error.client.tangle.crypto_unavailable"
            }
        }
    }
}

impl fmt::Display for RadrootsClientTangleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientTangleError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientTangleError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientTangleError::InitFailure,
                "error.client.tangle.init_failure",
            ),
            (
                RadrootsClientTangleError::ParseFailure,
                "error.client.tangle.parse_failure",
            ),
            (
                RadrootsClientTangleError::InvalidResponse,
                "error.client.tangle.invalid_response",
            ),
            (
                RadrootsClientTangleError::RuntimeUnavailable,
                "error.client.tangle.runtime_unavailable",
            ),
            (
                RadrootsClientTangleError::CryptoUnavailable,
                "error.client.tangle.crypto_unavailable",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
