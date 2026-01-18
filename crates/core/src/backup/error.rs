use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientBackupError {
    CryptoUndefined,
    InvalidBundle,
    DecodeFailure,
    EncodeFailure,
    ProviderMissing,
}

pub type RadrootsClientBackupErrorMessage = &'static str;

impl RadrootsClientBackupError {
    pub const fn message(self) -> RadrootsClientBackupErrorMessage {
        match self {
            RadrootsClientBackupError::CryptoUndefined => "error.client.backup.crypto_undefined",
            RadrootsClientBackupError::InvalidBundle => "error.client.backup.invalid_bundle",
            RadrootsClientBackupError::DecodeFailure => "error.client.backup.decode_failure",
            RadrootsClientBackupError::EncodeFailure => "error.client.backup.encode_failure",
            RadrootsClientBackupError::ProviderMissing => "error.client.backup.provider_missing",
        }
    }
}

impl fmt::Display for RadrootsClientBackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientBackupError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientBackupError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientBackupError::CryptoUndefined,
                "error.client.backup.crypto_undefined",
            ),
            (
                RadrootsClientBackupError::InvalidBundle,
                "error.client.backup.invalid_bundle",
            ),
            (
                RadrootsClientBackupError::DecodeFailure,
                "error.client.backup.decode_failure",
            ),
            (
                RadrootsClientBackupError::EncodeFailure,
                "error.client.backup.encode_failure",
            ),
            (
                RadrootsClientBackupError::ProviderMissing,
                "error.client.backup.provider_missing",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
