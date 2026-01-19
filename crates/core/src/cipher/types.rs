use async_trait::async_trait;

use crate::idb::RadrootsClientIdbConfig;

use super::RadrootsClientCipherError;

pub type RadrootsClientCipherEncryptResult = Result<Vec<u8>, RadrootsClientCipherError>;
pub type RadrootsClientCipherDecryptResult = Result<Vec<u8>, RadrootsClientCipherError>;
pub type RadrootsClientCipherResetResult = Result<(), RadrootsClientCipherError>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RadrootsClientCipherConfig {
    pub idb_config: Option<RadrootsClientIdbConfig>,
    pub key_name: Option<String>,
    pub key_length: Option<u32>,
    pub iv_length: Option<u32>,
    pub algorithm: Option<String>,
}

#[async_trait(?Send)]
pub trait RadrootsClientCipher {
    fn get_config(&self) -> RadrootsClientIdbConfig;

    async fn reset(&self) -> RadrootsClientCipherResetResult;
    async fn encrypt(&self, data: &[u8]) -> RadrootsClientCipherEncryptResult;
    async fn decrypt(&self, blob: &[u8]) -> RadrootsClientCipherDecryptResult;
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientCipherConfig;

    #[test]
    fn default_config_is_empty() {
        let config = RadrootsClientCipherConfig::default();
        assert!(config.idb_config.is_none());
        assert!(config.key_name.is_none());
        assert!(config.key_length.is_none());
        assert!(config.iv_length.is_none());
        assert!(config.algorithm.is_none());
    }
}
