use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::backup::RadrootsClientBackupDatastorePayload;
#[cfg(target_arch = "wasm32")]
use crate::crypto::RadrootsClientCryptoError;
use crate::idb::{IDB_CONFIG_DATASTORE, RadrootsClientIdbConfig};
#[cfg(target_arch = "wasm32")]
use crate::idb::RadrootsClientIdbStoreError;
use crate::idb::{RadrootsClientWebEncryptedStore, RadrootsClientWebEncryptedStoreConfig};

use super::{
    RadrootsClientDatastore,
    RadrootsClientDatastoreEntries,
    RadrootsClientDatastoreError,
    RadrootsClientDatastoreResult,
};

#[cfg(target_arch = "wasm32")]
use super::RadrootsClientDatastoreEntry;

const DATASTORE_STORE_PREFIX: &str = "datastore";
const DEFAULT_IV_LENGTH: u32 = 12;

pub struct RadrootsClientWebDatastore {
    encrypted_store: RadrootsClientWebEncryptedStore,
}

impl RadrootsClientWebDatastore {
    pub fn new(config: Option<RadrootsClientIdbConfig>) -> Self {
        let idb_config = config.unwrap_or(IDB_CONFIG_DATASTORE);
        let store_id = format!(
            "{}:{}:{}",
            DATASTORE_STORE_PREFIX, idb_config.database, idb_config.store
        );
        let encrypted_store = RadrootsClientWebEncryptedStore::new(
            RadrootsClientWebEncryptedStoreConfig {
                idb_config,
                store_id,
                legacy_key: None,
                iv_length: Some(DEFAULT_IV_LENGTH),
                crypto_service: None,
            },
        );
        Self { encrypted_store }
    }

    #[cfg(target_arch = "wasm32")]
    async fn decrypt_value(
        &self,
        store_key: &str,
        stored: crate::idb::RadrootsClientIdbValue,
    ) -> RadrootsClientDatastoreResult<String> {
        if let Some(text) = stored.as_string() {
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(text.as_bytes())
                .await
                .map_err(map_crypto_error)?;
            self.store_encrypted(store_key, &encrypted).await?;
            return Ok(text);
        }
        let Some(bytes) = crate::idb::idb_value_as_bytes(&stored) else {
            return Err(RadrootsClientDatastoreError::NoResult);
        };
        let outcome = self
            .encrypted_store
            .decrypt_record(&bytes)
            .await
            .map_err(map_crypto_error)?;
        if let Some(reencrypted) = outcome.reencrypted {
            self.store_encrypted(store_key, &reencrypted).await?;
        }
        String::from_utf8(outcome.plaintext)
            .map_err(|_| RadrootsClientDatastoreError::NoResult)
    }

    #[cfg(target_arch = "wasm32")]
    async fn store_encrypted(
        &self,
        store_key: &str,
        bytes: &[u8],
    ) -> RadrootsClientDatastoreResult<()> {
        let value = js_sys::Uint8Array::from(bytes);
        crate::idb::idb_set(
            self.encrypted_store.get_config().database,
            self.encrypted_store.get_config().store,
            store_key,
            &value.into(),
        )
        .await
        .map_err(map_idb_error)
    }

    fn param_key(key: &str, key_param: &str) -> String {
        format!("{key}:{key_param}")
    }
}

#[async_trait(?Send)]
impl RadrootsClientDatastore for RadrootsClientWebDatastore {
    fn get_config(&self) -> RadrootsClientIdbConfig {
        self.encrypted_store.get_config()
    }

    fn get_store_id(&self) -> &str {
        self.encrypted_store.get_store_id()
    }

    async fn init(&self) -> RadrootsClientDatastoreResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.encrypted_store
                .ensure_store()
                .await
                .map_err(map_crypto_error)?;
            Ok(())
        }
    }

    async fn set(&self, key: &str, value: &str) -> RadrootsClientDatastoreResult<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(value.as_bytes())
                .await
                .map_err(map_crypto_error)?;
            self.store_encrypted(key, &encrypted).await?;
            Ok(value.to_string())
        }
    }

    async fn get(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let stored = crate::idb::idb_get(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                key,
            )
            .await
            .map_err(map_idb_error)?;
            let Some(stored) = stored else {
                return Err(RadrootsClientDatastoreError::NoResult);
            };
            self.decrypt_value(key, stored).await
        }
    }

    async fn set_obj<T>(&self, key: &str, value: &T) -> RadrootsClientDatastoreResult<T>
    where
        T: Serialize + DeserializeOwned + Clone,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let serialized = serde_json::to_string(value)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(serialized.as_bytes())
                .await
                .map_err(map_crypto_error)?;
            self.store_encrypted(key, &encrypted).await?;
            Ok(value.clone())
        }
    }

    async fn update_obj<T>(&self, key: &str, value: &T) -> RadrootsClientDatastoreResult<T>
    where
        T: Serialize + DeserializeOwned + Clone,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (key, value);
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let stored = crate::idb::idb_get(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                key,
            )
            .await
            .map_err(map_idb_error)?;
            let mut base = if let Some(stored) = stored {
                let decrypted = self.decrypt_value(key, stored).await?;
                serde_json::from_str(&decrypted)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult)?
            } else {
                serde_json::Value::Object(Default::default())
            };
            let update = serde_json::to_value(value)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            merge_json(&mut base, update);
            let updated: T = serde_json::from_value(base)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            let serialized = serde_json::to_string(&updated)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(serialized.as_bytes())
                .await
                .map_err(map_crypto_error)?;
            self.store_encrypted(key, &encrypted).await?;
            Ok(updated)
        }
    }

    async fn get_obj<T>(&self, key: &str) -> RadrootsClientDatastoreResult<T>
    where
        T: DeserializeOwned,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let stored = crate::idb::idb_get(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                key,
            )
            .await
            .map_err(map_idb_error)?;
            let Some(stored) = stored else {
                return Err(RadrootsClientDatastoreError::NoResult);
            };
            let decrypted = self.decrypt_value(key, stored).await?;
            serde_json::from_str(&decrypted)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)
        }
    }

    async fn del_obj(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_del(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                key,
            )
            .await
            .map_err(map_idb_error)?;
            Ok(key.to_string())
        }
    }

    async fn del(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
        self.del_obj(key).await
    }

    async fn del_pref(&self, key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key_prefix;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let keys = crate::idb::idb_keys(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
            )
            .await
            .map_err(map_idb_error)?;
            let prefixed: Vec<String> = keys
                .into_iter()
                .filter(|key| key.starts_with(key_prefix))
                .collect();
            for key in &prefixed {
                crate::idb::idb_del(
                    self.encrypted_store.get_config().database,
                    self.encrypted_store.get_config().store,
                    key,
                )
                .await
                .map_err(map_idb_error)?;
            }
            Ok(prefixed)
        }
    }

    async fn set_param(
        &self,
        key: &str,
        key_param: &str,
        value: &str,
    ) -> RadrootsClientDatastoreResult<String> {
        let store_key = Self::param_key(key, key_param);
        self.set(&store_key, value).await
    }

    async fn get_param(
        &self,
        key: &str,
        key_param: &str,
    ) -> RadrootsClientDatastoreResult<String> {
        let store_key = Self::param_key(key, key_param);
        self.get(&store_key).await
    }

    async fn keys(&self) -> RadrootsClientDatastoreResult<Vec<String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_keys(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
            )
            .await
            .map_err(map_idb_error)
        }
    }

    async fn entries(&self) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
        self.entries_pref("").await
    }

    async fn entries_pref(
        &self,
        key_prefix: &str,
    ) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = key_prefix;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let keys = crate::idb::idb_keys(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
            )
            .await
            .map_err(map_idb_error)?;
            let prefixed: Vec<String> = keys
                .into_iter()
                .filter(|key| key.starts_with(key_prefix))
                .collect();
            let mut out = Vec::with_capacity(prefixed.len());
            for key in prefixed {
                let stored = crate::idb::idb_get(
                    self.encrypted_store.get_config().database,
                    self.encrypted_store.get_config().store,
                    &key,
                )
                .await
                .map_err(map_idb_error)?;
                let value = if let Some(stored) = stored {
                    Some(self.decrypt_value(&key, stored).await?)
                } else {
                    None
                };
                out.push(RadrootsClientDatastoreEntry::new(key, value));
            }
            Ok(out)
        }
    }

    async fn reset(&self) -> RadrootsClientDatastoreResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_clear(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
            )
            .await
            .map_err(map_idb_error)?;
            let index = crate::crypto::crypto_registry_get_store_index(
                self.encrypted_store.get_store_id(),
            )
            .await
            .map_err(map_crypto_error)?;
            if let Some(index) = index {
                crate::crypto::crypto_registry_clear_store_index(
                    self.encrypted_store.get_store_id(),
                )
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

    async fn export_backup(
        &self,
    ) -> RadrootsClientDatastoreResult<RadrootsClientBackupDatastorePayload> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let keys = self.keys().await?;
            let mut entries = Vec::new();
            for key in keys {
                let stored = crate::idb::idb_get(
                    self.encrypted_store.get_config().database,
                    self.encrypted_store.get_config().store,
                    &key,
                )
                .await
                .map_err(map_idb_error)?;
                let Some(stored) = stored else {
                    return Err(RadrootsClientDatastoreError::NoResult);
                };
                let value = self.decrypt_value(&key, stored).await?;
                entries.push(crate::backup::RadrootsClientBackupDatastoreEntry {
                    key,
                    value,
                });
            }
            Ok(RadrootsClientBackupDatastorePayload { entries })
        }
    }

    async fn import_backup(
        &self,
        payload: RadrootsClientBackupDatastorePayload,
    ) -> RadrootsClientDatastoreResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = payload;
            return Err(RadrootsClientDatastoreError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            for entry in payload.entries {
                let encrypted = self
                    .encrypted_store
                    .encrypt_bytes(entry.value.as_bytes())
                    .await
                    .map_err(map_crypto_error)?;
                self.store_encrypted(&entry.key, &encrypted).await?;
            }
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn map_crypto_error(err: RadrootsClientCryptoError) -> RadrootsClientDatastoreError {
    match err {
        RadrootsClientCryptoError::IdbUndefined | RadrootsClientCryptoError::CryptoUndefined => {
            RadrootsClientDatastoreError::IdbUndefined
        }
        _ => RadrootsClientDatastoreError::NoResult,
    }
}

#[cfg(target_arch = "wasm32")]
fn map_idb_error(err: RadrootsClientIdbStoreError) -> RadrootsClientDatastoreError {
    match err {
        RadrootsClientIdbStoreError::IdbUndefined => RadrootsClientDatastoreError::IdbUndefined,
        _ => RadrootsClientDatastoreError::NoResult,
    }
}

#[cfg(target_arch = "wasm32")]
fn merge_json(base: &mut serde_json::Value, update: serde_json::Value) {
    match (base, update) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(update_map)) => {
            for (key, value) in update_map {
                base_map.insert(key, value);
            }
        }
        (base_value, update_value) => {
            *base_value = update_value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientWebDatastore;
    use crate::datastore::RadrootsClientDatastore;

    #[test]
    fn param_key_uses_colon_separator() {
        let key = RadrootsClientWebDatastore::param_key("alpha", "beta");
        assert_eq!(key, "alpha:beta");
    }

    #[test]
    fn non_wasm_get_errors() {
        let store = RadrootsClientWebDatastore::new(None);
        let err = futures::executor::block_on(store.get("key"))
            .expect_err("idb undefined");
        assert_eq!(err, crate::datastore::RadrootsClientDatastoreError::IdbUndefined);
    }

    #[test]
    fn non_wasm_entries_pref_errors() {
        let store = RadrootsClientWebDatastore::new(None);
        let err = futures::executor::block_on(store.entries_pref("log:"))
            .expect_err("idb undefined");
        assert_eq!(err, crate::datastore::RadrootsClientDatastoreError::IdbUndefined);
    }
}
