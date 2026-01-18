use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientCipherError {
    IdbUndefined,
    CryptoUndefined,
    InvalidCiphertext,
    DecryptFailure,
}

pub type RadrootsClientCipherErrorMessage = &'static str;

impl RadrootsClientCipherError {
    pub const fn message(self) -> RadrootsClientCipherErrorMessage {
        match self {
            RadrootsClientCipherError::IdbUndefined => "error.client.cipher.idb_undefined",
            RadrootsClientCipherError::CryptoUndefined => "error.client.cipher.crypto_undefined",
            RadrootsClientCipherError::InvalidCiphertext => "error.client.cipher.invalid_ciphertext",
            RadrootsClientCipherError::DecryptFailure => "error.client.cipher.decrypt_failure",
        }
    }
}

impl fmt::Display for RadrootsClientCipherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientCipherError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientCipherError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientCipherError::IdbUndefined,
                "error.client.cipher.idb_undefined",
            ),
            (
                RadrootsClientCipherError::CryptoUndefined,
                "error.client.cipher.crypto_undefined",
            ),
            (
                RadrootsClientCipherError::InvalidCiphertext,
                "error.client.cipher.invalid_ciphertext",
            ),
            (
                RadrootsClientCipherError::DecryptFailure,
                "error.client.cipher.decrypt_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
