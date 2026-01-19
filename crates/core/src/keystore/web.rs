use async_trait::async_trait;

use crate::backup::RadrootsClientBackupKeystorePayload;
#[cfg(target_arch = "wasm32")]
use crate::crypto::RadrootsClientCryptoError;
use crate::crypto::RadrootsClientLegacyKeyConfig;
use crate::idb::{IDB_CONFIG_KEYSTORE, IDB_STORE_KEYSTORE_CIPHER, RadrootsClientIdbConfig};
#[cfg(target_arch = "wasm32")]
use crate::idb::RadrootsClientIdbStoreError;
use crate::idb::{RadrootsClientWebEncryptedStore, RadrootsClientWebEncryptedStoreConfig};

use super::{
    RadrootsClientKeystore,
    RadrootsClientKeystoreError,
    RadrootsClientKeystoreResult,
};

const DEFAULT_IV_LENGTH: u32 = 12;

pub struct RadrootsClientWebKeystore {
    config: RadrootsClientIdbConfig,
    store_id: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    encrypted_store: RadrootsClientWebEncryptedStore,
}

impl RadrootsClientWebKeystore {
    pub fn new(config: Option<RadrootsClientIdbConfig>) -> Self {
        let config = config.unwrap_or(IDB_CONFIG_KEYSTORE);
        let store_id = format!("keystore:{}:{}", config.database, config.store);
        let legacy_store = IDB_STORE_KEYSTORE_CIPHER;
        let legacy_key_config = RadrootsClientLegacyKeyConfig {
            idb_config: RadrootsClientIdbConfig::new(config.database, legacy_store),
            key_name: format!("radroots.keystore.{}.aes-gcm.key", config.store),
            iv_length: DEFAULT_IV_LENGTH,
            algorithm: "AES-GCM".to_string(),
        };
        let encrypted_store = RadrootsClientWebEncryptedStore::new(
            RadrootsClientWebEncryptedStoreConfig {
                idb_config: config,
                store_id: store_id.clone(),
                legacy_key: Some(legacy_key_config.clone()),
                iv_length: Some(DEFAULT_IV_LENGTH),
                crypto_service: None,
            },
        );
        Self {
            config,
            store_id,
            encrypted_store,
        }
    }

    pub fn get_config(&self) -> RadrootsClientIdbConfig {
        self.config
    }

    pub fn get_store_id(&self) -> &str {
        &self.store_id
    }

    #[cfg(target_arch = "wasm32")]
    async fn store_encrypted(&self, key: &str, bytes: &[u8]) -> RadrootsClientKeystoreResult<()> {
        let value = js_sys::Uint8Array::from(bytes);
        crate::idb::idb_set(
            self.config.database,
            self.config.store,
            key,
            &value.into(),
        )
        .await
        .map_err(map_idb_error)
    }
}

#[async_trait(?Send)]
impl RadrootsClientKeystore for RadrootsClientWebKeystore {
    async fn add(&self, key: &str, value: &str) -> RadrootsClientKeystoreResult<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(value.as_bytes())
                .await
                .map_err(map_crypto_error)?;
            self.store_encrypted(key, &encrypted).await?;
            Ok(key.to_string())
        }
    }

    async fn remove(&self, key: &str) -> RadrootsClientKeystoreResult<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_del(self.config.database, self.config.store, key)
                .await
                .map_err(map_idb_error)?;
            Ok(key.to_string())
        }
    }

    async fn read(&self, key: Option<&str>) -> RadrootsClientKeystoreResult<Option<String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let Some(key) = key else {
                return Err(RadrootsClientKeystoreError::MissingKey);
            };
            let stored = crate::idb::idb_get(self.config.database, self.config.store, key)
                .await
                .map_err(map_idb_error)?;
            let Some(stored) = stored else {
                return Err(RadrootsClientKeystoreError::CorruptData);
            };
            let Some(bytes) = crate::idb::idb_value_as_bytes(&stored) else {
                return Err(RadrootsClientKeystoreError::CorruptData);
            };
            let outcome = self
                .encrypted_store
                .decrypt_record(&bytes)
                .await
                .map_err(map_crypto_error)?;
            if let Some(reencrypted) = outcome.reencrypted {
                self.store_encrypted(key, &reencrypted).await?;
            }
            let plain =
                String::from_utf8(outcome.plaintext).map_err(|_| RadrootsClientKeystoreError::CorruptData)?;
            Ok(Some(plain))
        }
    }

    async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_keys(self.config.database, self.config.store)
                .await
                .map_err(map_idb_error)
        }
    }

    async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_clear(self.config.database, self.config.store)
                .await
                .map_err(map_idb_error)?;
            let index = crate::crypto::crypto_registry_get_store_index(&self.store_id)
                .await
                .map_err(map_crypto_error)?;
            if let Some(index) = index {
                crate::crypto::crypto_registry_clear_store_index(&self.store_id)
                    .await
                    .map_err(map_crypto_error)?;
                for key_id in index.key_ids {
                    crate::crypto::crypto_registry_clear_key_entry(&key_id)
                        .await
                        .map_err(map_crypto_error)?;
                }
            }
            Ok(())
        }
    }

    fn get_store_id(&self) -> &str {
        &self.store_id
    }

    async fn export_backup(
        &self,
    ) -> RadrootsClientKeystoreResult<RadrootsClientBackupKeystorePayload> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let keys = self.keys().await?;
            let mut entries = Vec::new();
            for key in keys {
                let value = self.read(Some(&key)).await?;
                let Some(value) = value else {
                    return Err(RadrootsClientKeystoreError::CorruptData);
                };
                entries.push(crate::backup::RadrootsClientBackupKeystoreEntry { key, value });
            }
            Ok(RadrootsClientBackupKeystorePayload { entries })
        }
    }

    async fn import_backup(
        &self,
        payload: RadrootsClientBackupKeystorePayload,
    ) -> RadrootsClientKeystoreResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = payload;
            return Err(RadrootsClientKeystoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            for entry in payload.entries {
                self.add(&entry.key, &entry.value).await?;
            }
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn map_crypto_error(err: RadrootsClientCryptoError) -> RadrootsClientKeystoreError {
    match err {
        RadrootsClientCryptoError::IdbUndefined | RadrootsClientCryptoError::CryptoUndefined => {
            RadrootsClientKeystoreError::IdbUndefined
        }
        _ => RadrootsClientKeystoreError::CorruptData,
    }
}

#[cfg(target_arch = "wasm32")]
fn map_idb_error(err: RadrootsClientIdbStoreError) -> RadrootsClientKeystoreError {
    match err {
        RadrootsClientIdbStoreError::IdbUndefined => RadrootsClientKeystoreError::IdbUndefined,
        _ => RadrootsClientKeystoreError::CorruptData,
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebKeystore;
    use crate::keystore::RadrootsClientKeystore;

    #[test]
    fn non_wasm_add_errors() {
        let store = RadrootsClientWebKeystore::new(None);
        let err = futures::executor::block_on(store.add("key", "value"))
            .expect_err("idb undefined");
        assert_eq!(err, crate::keystore::RadrootsClientKeystoreError::IdbUndefined);
    }
}
