pub mod error;
pub mod types;
pub mod envelope;
pub mod random;
pub mod keys;
pub mod kdf;
pub mod registry;
pub mod provider;

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
pub use provider::RadrootsClientDeviceKeyMaterialProvider;
pub use envelope::{crypto_envelope_decode, crypto_envelope_encode};
pub use keys::crypto_key_id_create;
pub use kdf::{crypto_kdf_iterations_default, crypto_kdf_salt_create};
pub use registry::{
    crypto_registry_clear_key_entry,
    crypto_registry_clear_store_index,
    crypto_registry_export,
    crypto_registry_get_device_material,
    crypto_registry_get_key_entry,
    crypto_registry_get_store_index,
    crypto_registry_import,
    crypto_registry_list_key_entries,
    crypto_registry_list_store_indices,
    crypto_registry_set_device_material,
    crypto_registry_set_key_entry,
    crypto_registry_set_store_index,
};
#[cfg(target_arch = "wasm32")]
pub use keys::{
    crypto_key_export_raw,
    crypto_key_generate,
    crypto_key_import_raw,
    crypto_key_unwrap,
    crypto_key_wrap,
};
#[cfg(target_arch = "wasm32")]
pub use kdf::crypto_kdf_derive_kek;
