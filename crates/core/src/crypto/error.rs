use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientCryptoError {
    IdbUndefined,
    CryptoUndefined,
    InvalidEnvelope,
    InvalidKeyId,
    KeyNotFound,
    UnwrapFailure,
    WrapFailure,
    LegacyKeyMissing,
    EncryptFailure,
    DecryptFailure,
    KdfFailure,
    RegistryFailure,
}

pub type RadrootsClientCryptoErrorMessage = &'static str;

impl RadrootsClientCryptoError {
    pub const fn message(self) -> RadrootsClientCryptoErrorMessage {
        match self {
            RadrootsClientCryptoError::IdbUndefined => "error.client.crypto.idb_undefined",
            RadrootsClientCryptoError::CryptoUndefined => "error.client.crypto.crypto_undefined",
            RadrootsClientCryptoError::InvalidEnvelope => "error.client.crypto.invalid_envelope",
            RadrootsClientCryptoError::InvalidKeyId => "error.client.crypto.invalid_key_id",
            RadrootsClientCryptoError::KeyNotFound => "error.client.crypto.key_not_found",
            RadrootsClientCryptoError::UnwrapFailure => "error.client.crypto.unwrap_failure",
            RadrootsClientCryptoError::WrapFailure => "error.client.crypto.wrap_failure",
            RadrootsClientCryptoError::LegacyKeyMissing => "error.client.crypto.legacy_key_missing",
            RadrootsClientCryptoError::EncryptFailure => "error.client.crypto.encrypt_failure",
            RadrootsClientCryptoError::DecryptFailure => "error.client.crypto.decrypt_failure",
            RadrootsClientCryptoError::KdfFailure => "error.client.crypto.kdf_failure",
            RadrootsClientCryptoError::RegistryFailure => "error.client.crypto.registry_failure",
        }
    }
}

impl fmt::Display for RadrootsClientCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientCryptoError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientCryptoError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientCryptoError::IdbUndefined,
                "error.client.crypto.idb_undefined",
            ),
            (
                RadrootsClientCryptoError::CryptoUndefined,
                "error.client.crypto.crypto_undefined",
            ),
            (
                RadrootsClientCryptoError::InvalidEnvelope,
                "error.client.crypto.invalid_envelope",
            ),
            (
                RadrootsClientCryptoError::InvalidKeyId,
                "error.client.crypto.invalid_key_id",
            ),
            (
                RadrootsClientCryptoError::KeyNotFound,
                "error.client.crypto.key_not_found",
            ),
            (
                RadrootsClientCryptoError::UnwrapFailure,
                "error.client.crypto.unwrap_failure",
            ),
            (
                RadrootsClientCryptoError::WrapFailure,
                "error.client.crypto.wrap_failure",
            ),
            (
                RadrootsClientCryptoError::LegacyKeyMissing,
                "error.client.crypto.legacy_key_missing",
            ),
            (
                RadrootsClientCryptoError::EncryptFailure,
                "error.client.crypto.encrypt_failure",
            ),
            (
                RadrootsClientCryptoError::DecryptFailure,
                "error.client.crypto.decrypt_failure",
            ),
            (
                RadrootsClientCryptoError::KdfFailure,
                "error.client.crypto.kdf_failure",
            ),
            (
                RadrootsClientCryptoError::RegistryFailure,
                "error.client.crypto.registry_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
