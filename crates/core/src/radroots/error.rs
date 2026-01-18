use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientRadrootsError {
    MissingBaseUrl,
    AccountRegistered,
    RequestFailure,
}

pub type RadrootsClientRadrootsErrorMessage = &'static str;

impl RadrootsClientRadrootsError {
    pub const fn message(self) -> RadrootsClientRadrootsErrorMessage {
        match self {
            RadrootsClientRadrootsError::MissingBaseUrl => {
                "error.client.radroots.missing_base_url"
            }
            RadrootsClientRadrootsError::AccountRegistered => {
                "error.client.radroots.account_registered"
            }
            RadrootsClientRadrootsError::RequestFailure => "error.client.radroots.request_failure",
        }
    }
}

impl fmt::Display for RadrootsClientRadrootsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientRadrootsError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientRadrootsError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientRadrootsError::MissingBaseUrl,
                "error.client.radroots.missing_base_url",
            ),
            (
                RadrootsClientRadrootsError::AccountRegistered,
                "error.client.radroots.account_registered",
            ),
            (
                RadrootsClientRadrootsError::RequestFailure,
                "error.client.radroots.request_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
