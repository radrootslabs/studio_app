use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::idb::RadrootsClientIdbConfig;

use super::RadrootsClientCryptoError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RadrootsClientCryptoKeyStatus {
    Active,
    Rotated,
}

impl RadrootsClientCryptoKeyStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsClientCryptoKeyStatus::Active => "active",
            RadrootsClientCryptoKeyStatus::Rotated => "rotated",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(RadrootsClientCryptoKeyStatus::Active),
            "rotated" => Some(RadrootsClientCryptoKeyStatus::Rotated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadrootsClientCryptoAlgorithm {
    #[serde(rename = "AES-GCM")]
    AesGcm,
}

impl RadrootsClientCryptoAlgorithm {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsClientCryptoAlgorithm::AesGcm => "AES-GCM",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "AES-GCM" => Some(RadrootsClientCryptoAlgorithm::AesGcm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsClientCryptoEnvelope {
    pub version: u8,
    pub key_id: String,
    pub iv: Vec<u8>,
    pub created_at: u64,
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsClientCryptoKeyEntry {
    pub key_id: String,
    pub store_id: String,
    pub created_at: u64,
    pub status: RadrootsClientCryptoKeyStatus,
    pub wrapped_key: Vec<u8>,
    pub wrap_iv: Vec<u8>,
    pub kdf_salt: Vec<u8>,
    pub kdf_iterations: u32,
    pub iv_length: u32,
    pub algorithm: RadrootsClientCryptoAlgorithm,
    pub provider_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsClientCryptoStoreIndex {
    pub store_id: String,
    pub active_key_id: String,
    pub key_ids: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsClientCryptoRegistryExport {
    pub stores: Vec<RadrootsClientCryptoStoreIndex>,
    pub keys: Vec<RadrootsClientCryptoKeyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsClientCryptoDecryptOutcome {
    pub plaintext: Vec<u8>,
    pub needs_reencrypt: bool,
    pub reencrypted: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientLegacyKeyConfig {
    pub idb_config: RadrootsClientIdbConfig,
    pub key_name: String,
    pub iv_length: u32,
    pub algorithm: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientCryptoStoreConfig {
    pub store_id: String,
    pub legacy_key: Option<RadrootsClientLegacyKeyConfig>,
    pub iv_length: Option<u32>,
}

#[async_trait(?Send)]
pub trait RadrootsClientKeyMaterialProvider {
    async fn get_key_material(&self) -> Result<Vec<u8>, RadrootsClientCryptoError>;
    async fn get_provider_id(&self) -> Result<String, RadrootsClientCryptoError>;
}

#[async_trait(?Send)]
pub trait RadrootsClientWebCryptoService {
    fn register_store_config(&mut self, config: RadrootsClientCryptoStoreConfig);

    async fn encrypt(
        &self,
        store_id: &str,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, RadrootsClientCryptoError>;

    async fn decrypt(
        &self,
        store_id: &str,
        blob: &[u8],
    ) -> Result<Vec<u8>, RadrootsClientCryptoError>;

    async fn decrypt_record(
        &self,
        store_id: &str,
        blob: &[u8],
    ) -> Result<RadrootsClientCryptoDecryptOutcome, RadrootsClientCryptoError>;

    async fn rotate_store_key(&self, store_id: &str) -> Result<String, RadrootsClientCryptoError>;

    async fn export_registry(
        &self,
    ) -> Result<RadrootsClientCryptoRegistryExport, RadrootsClientCryptoError>;

    async fn import_registry(
        &self,
        registry: RadrootsClientCryptoRegistryExport,
    ) -> Result<(), RadrootsClientCryptoError>;
}

#[cfg(test)]
mod tests {
    use super::{RadrootsClientCryptoAlgorithm, RadrootsClientCryptoKeyStatus};

    #[test]
    fn key_status_roundtrip() {
        let active = RadrootsClientCryptoKeyStatus::Active;
        let rotated = RadrootsClientCryptoKeyStatus::Rotated;

        assert_eq!(active.as_str(), "active");
        assert_eq!(rotated.as_str(), "rotated");
        assert_eq!(RadrootsClientCryptoKeyStatus::parse("active"), Some(active));
        assert_eq!(RadrootsClientCryptoKeyStatus::parse("rotated"), Some(rotated));
        assert_eq!(RadrootsClientCryptoKeyStatus::parse("unknown"), None);
    }

    #[test]
    fn algorithm_roundtrip() {
        let algo = RadrootsClientCryptoAlgorithm::AesGcm;

        assert_eq!(algo.as_str(), "AES-GCM");
        assert_eq!(
            RadrootsClientCryptoAlgorithm::parse("AES-GCM"),
            Some(algo)
        );
        assert_eq!(RadrootsClientCryptoAlgorithm::parse("AES-CBC"), None);
    }
}
