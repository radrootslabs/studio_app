use crate::crypto::{
    RadrootsClientCryptoDecryptOutcome,
    RadrootsClientCryptoError,
    RadrootsClientCryptoStoreConfig,
    RadrootsClientLegacyKeyConfig,
    RadrootsClientWebCryptoService,
    RadrootsClientWebCryptoServiceImpl,
};
use crate::idb::{idb_store_ensure, RadrootsClientIdbConfig, RadrootsClientIdbStoreError};

pub struct RadrootsClientWebEncryptedStoreConfig {
    pub idb_config: RadrootsClientIdbConfig,
    pub store_id: String,
    pub legacy_key: Option<RadrootsClientLegacyKeyConfig>,
    pub iv_length: Option<u32>,
    pub crypto_service: Option<Box<dyn RadrootsClientWebCryptoService>>,
}

pub struct RadrootsClientWebEncryptedStore {
    config: RadrootsClientIdbConfig,
    store_id: String,
    crypto: Box<dyn RadrootsClientWebCryptoService>,
}

impl RadrootsClientWebEncryptedStore {
    pub fn new(config: RadrootsClientWebEncryptedStoreConfig) -> Self {
        let mut crypto = config
            .crypto_service
            .unwrap_or_else(|| Box::new(RadrootsClientWebCryptoServiceImpl::default()));
        let store_config = RadrootsClientCryptoStoreConfig {
            store_id: config.store_id.clone(),
            legacy_key: config.legacy_key,
            iv_length: config.iv_length,
        };
        crypto.register_store_config(store_config);
        Self {
            config: config.idb_config,
            store_id: config.store_id,
            crypto,
        }
    }

    pub fn get_config(&self) -> RadrootsClientIdbConfig {
        self.config
    }

    pub fn get_store_id(&self) -> &str {
        &self.store_id
    }

    pub async fn ensure_store(&self) -> Result<(), RadrootsClientCryptoError> {
        idb_store_ensure(self.config.database, self.config.store)
            .await
            .map_err(map_idb_error)
    }

    pub async fn encrypt_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<u8>, RadrootsClientCryptoError> {
        self.crypto.encrypt(&self.store_id, bytes).await
    }

    pub async fn decrypt_record(
        &self,
        blob: &[u8],
    ) -> Result<RadrootsClientCryptoDecryptOutcome, RadrootsClientCryptoError> {
        self.crypto.decrypt_record(&self.store_id, blob).await
    }
}

fn map_idb_error(err: RadrootsClientIdbStoreError) -> RadrootsClientCryptoError {
    match err {
        RadrootsClientIdbStoreError::IdbUndefined => RadrootsClientCryptoError::IdbUndefined,
        _ => RadrootsClientCryptoError::RegistryFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::{RadrootsClientWebEncryptedStore, RadrootsClientWebEncryptedStoreConfig};
    use crate::crypto::RadrootsClientCryptoError;
    use crate::idb::RadrootsClientIdbConfig;

    #[test]
    fn encrypted_store_exposes_ids() {
        let config = RadrootsClientWebEncryptedStoreConfig {
            idb_config: RadrootsClientIdbConfig::new("db", "store"),
            store_id: "store-id".to_string(),
            legacy_key: None,
            iv_length: None,
            crypto_service: None,
        };
        let store = RadrootsClientWebEncryptedStore::new(config);
        let idb_config = store.get_config();
        assert_eq!(idb_config.database, "db");
        assert_eq!(idb_config.store, "store");
        assert_eq!(store.get_store_id(), "store-id");
    }

    #[test]
    fn non_wasm_store_ensure_errors() {
        let config = RadrootsClientWebEncryptedStoreConfig {
            idb_config: RadrootsClientIdbConfig::new("db", "store"),
            store_id: "store-id".to_string(),
            legacy_key: None,
            iv_length: None,
            crypto_service: None,
        };
        let store = RadrootsClientWebEncryptedStore::new(config);
        let err = futures::executor::block_on(store.ensure_store())
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientCryptoError::IdbUndefined);
    }
}
