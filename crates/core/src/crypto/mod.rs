pub mod error;
pub mod types;
pub mod envelope;
pub mod random;
pub mod keys;

pub use error::{RadrootsClientCryptoError, RadrootsClientCryptoErrorMessage};
pub use types::{
    RadrootsClientCryptoAlgorithm,
    RadrootsClientCryptoDecryptOutcome,
    RadrootsClientCryptoEnvelope,
    RadrootsClientCryptoKeyEntry,
    RadrootsClientCryptoKeyStatus,
    RadrootsClientCryptoRegistryExport,
    RadrootsClientCryptoStoreConfig,
    RadrootsClientCryptoStoreIndex,
    RadrootsClientKeyMaterialProvider,
    RadrootsClientLegacyKeyConfig,
    RadrootsClientWebCryptoService,
};
pub use envelope::{crypto_envelope_decode, crypto_envelope_encode};
pub use keys::crypto_key_id_create;
#[cfg(target_arch = "wasm32")]
pub use keys::{
    crypto_key_export_raw,
    crypto_key_generate,
    crypto_key_import_raw,
    crypto_key_unwrap,
    crypto_key_wrap,
};
