use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientKeystoreError {
    IdbUndefined,
    MissingKey,
    CorruptData,
    NostrInvalidSecretKey,
    NostrNoResults,
}

pub type RadrootsClientKeystoreErrorMessage = &'static str;

impl RadrootsClientKeystoreError {
    pub const fn message(self) -> RadrootsClientKeystoreErrorMessage {
        match self {
            RadrootsClientKeystoreError::IdbUndefined => "error.client.keystore.idb_undefined",
            RadrootsClientKeystoreError::MissingKey => "error.client.keystore.missing_key",
            RadrootsClientKeystoreError::CorruptData => "error.client.keystore.corrupt_data",
            RadrootsClientKeystoreError::NostrInvalidSecretKey => {
                "error.client.keystore.nostr_invalid_secret_key"
            }
            RadrootsClientKeystoreError::NostrNoResults => "error.client.keystore.nostr_no_results",
        }
    }
}

impl fmt::Display for RadrootsClientKeystoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientKeystoreError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientKeystoreError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientKeystoreError::IdbUndefined,
                "error.client.keystore.idb_undefined",
            ),
            (
                RadrootsClientKeystoreError::MissingKey,
                "error.client.keystore.missing_key",
            ),
            (
                RadrootsClientKeystoreError::CorruptData,
                "error.client.keystore.corrupt_data",
            ),
            (
                RadrootsClientKeystoreError::NostrInvalidSecretKey,
                "error.client.keystore.nostr_invalid_secret_key",
            ),
            (
                RadrootsClientKeystoreError::NostrNoResults,
                "error.client.keystore.nostr_no_results",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
