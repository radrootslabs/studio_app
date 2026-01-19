use std::cell::RefCell;
use std::collections::HashMap;

use async_trait::async_trait;

use crate::crypto::{crypto_registry_export, crypto_registry_import};

#[cfg(target_arch = "wasm32")]
use crate::crypto::{
    crypto_envelope_decode,
    crypto_envelope_encode,
    crypto_kdf_derive_kek,
    crypto_kdf_iterations_default,
    crypto_kdf_salt_create,
    crypto_key_export_raw,
    crypto_key_generate,
    crypto_key_id_create,
    crypto_key_import_raw,
    crypto_key_unwrap,
    crypto_key_wrap,
    crypto_registry_get_key_entry,
    crypto_registry_get_store_index,
    crypto_registry_set_key_entry,
    crypto_registry_set_store_index,
};
#[cfg(target_arch = "wasm32")]
use crate::crypto::random::fill_random;
#[cfg(target_arch = "wasm32")]
use crate::idb::{idb_get, idb_store_ensure, idb_store_exists, idb_value_as_bytes};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use super::{
    RadrootsClientCryptoDecryptOutcome,
    RadrootsClientCryptoError,
    RadrootsClientCryptoRegistryExport,
    RadrootsClientCryptoStoreConfig,
    RadrootsClientKeyMaterialProvider,
    RadrootsClientWebCryptoService,
};
use super::provider::RadrootsClientDeviceKeyMaterialProvider;

#[cfg(target_arch = "wasm32")]
use super::{
    RadrootsClientCryptoAlgorithm,
    RadrootsClientCryptoEnvelope,
    RadrootsClientCryptoKeyEntry,
    RadrootsClientCryptoKeyStatus,
    RadrootsClientCryptoStoreIndex,
    RadrootsClientLegacyKeyConfig,
};

const DEFAULT_IV_LENGTH: u32 = 12;
#[cfg(target_arch = "wasm32")]
const DEFAULT_KDF_SALT_BYTES: usize = 16;

pub struct RadrootsClientWebCryptoServiceConfig {
    pub key_material_provider: Option<Box<dyn RadrootsClientKeyMaterialProvider>>,
}

pub struct RadrootsClientWebCryptoServiceImpl {
    store_configs: RefCell<HashMap<String, RadrootsClientCryptoStoreConfig>>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    key_material_provider: Box<dyn RadrootsClientKeyMaterialProvider>,
}

impl RadrootsClientWebCryptoServiceImpl {
    pub fn new(config: Option<RadrootsClientWebCryptoServiceConfig>) -> Self {
        let provider = config
            .and_then(|config| config.key_material_provider)
            .unwrap_or_else(|| Box::new(RadrootsClientDeviceKeyMaterialProvider));
        Self {
            store_configs: RefCell::new(HashMap::new()),
            key_material_provider: provider,
        }
    }

    #[cfg(any(test, target_arch = "wasm32"))]
    fn resolve_store_config(&self, store_id: &str) -> RadrootsClientCryptoStoreConfig {
        if let Some(existing) = self.store_configs.borrow().get(store_id) {
            return existing.clone();
        }
        let config = RadrootsClientCryptoStoreConfig {
            store_id: store_id.to_string(),
            legacy_key: None,
            iv_length: Some(DEFAULT_IV_LENGTH),
        };
        self.store_configs
            .borrow_mut()
            .insert(store_id.to_string(), config.clone());
        config
    }

    #[cfg(target_arch = "wasm32")]
    async fn resolve_active_key(
        &self,
        store_id: &str,
    ) -> Result<ResolvedKey, RadrootsClientCryptoError> {
        let index = crypto_registry_get_store_index(store_id).await?;
        let config = self.resolve_store_config(store_id);
        let Some(index) = index else {
            return self.create_store_key(store_id, &config).await;
        };
        let entry = crypto_registry_get_key_entry(&index.active_key_id).await?;
        let Some(entry) = entry else {
            return self.create_store_key(store_id, &config).await;
        };
        let key = self.unwrap_key_entry(&entry).await?;
        Ok(ResolvedKey { key, entry, index })
    }

    #[cfg(target_arch = "wasm32")]
    async fn resolve_key_by_id(
        &self,
        store_id: &str,
        key_id: &str,
    ) -> Result<ResolvedKey, RadrootsClientCryptoError> {
        let entry = crypto_registry_get_key_entry(key_id).await?;
        let Some(entry) = entry else {
            return Err(RadrootsClientCryptoError::KeyNotFound);
        };
        let index = match crypto_registry_get_store_index(store_id).await? {
            Some(index) => index,
            None => {
                let next_index = RadrootsClientCryptoStoreIndex {
                    store_id: store_id.to_string(),
                    active_key_id: entry.key_id.clone(),
                    key_ids: vec![entry.key_id.clone()],
                    created_at: entry.created_at,
                };
                crypto_registry_set_store_index(next_index.clone()).await?;
                next_index
            }
        };
        let key = self.unwrap_key_entry(&entry).await?;
        Ok(ResolvedKey {
            key,
            entry,
            index,
        })
    }

    #[cfg(target_arch = "wasm32")]
    async fn create_store_key(
        &self,
        store_id: &str,
        config: &RadrootsClientCryptoStoreConfig,
    ) -> Result<ResolvedKey, RadrootsClientCryptoError> {
        let created = self.create_key_entry(store_id, config).await?;
        let index = RadrootsClientCryptoStoreIndex {
            store_id: store_id.to_string(),
            active_key_id: created.entry.key_id.clone(),
            key_ids: vec![created.entry.key_id.clone()],
            created_at: created.entry.created_at,
        };
        crypto_registry_set_store_index(index.clone()).await?;
        Ok(ResolvedKey {
            key: created.key,
            entry: created.entry,
            index,
        })
    }

    #[cfg(target_arch = "wasm32")]
    async fn create_key_entry(
        &self,
        store_id: &str,
        config: &RadrootsClientCryptoStoreConfig,
    ) -> Result<CreatedKey, RadrootsClientCryptoError> {
        let key_id = crypto_key_id_create()?;
        let created_at = js_sys::Date::now() as u64;
        let kdf_salt = crypto_kdf_salt_create(DEFAULT_KDF_SALT_BYTES)?;
        let kdf_iterations = crypto_kdf_iterations_default();
        let mut material = self.key_material_provider.get_key_material().await?;
        let provider_id = self.key_material_provider.get_provider_id().await?;
        let kek = crypto_kdf_derive_kek(&material, &kdf_salt, kdf_iterations).await?;
        material.fill(0);
        let key = crypto_key_generate().await?;
        let mut raw_key = crypto_key_export_raw(&key).await?;
        let (wrapped_key, wrap_iv) = crypto_key_wrap(&kek, &mut raw_key).await?;
        let iv_length = config.iv_length.unwrap_or(DEFAULT_IV_LENGTH);
        let entry = RadrootsClientCryptoKeyEntry {
            key_id,
            store_id: store_id.to_string(),
            created_at,
            status: RadrootsClientCryptoKeyStatus::Active,
            wrapped_key,
            wrap_iv,
            kdf_salt,
            kdf_iterations,
            iv_length,
            algorithm: RadrootsClientCryptoAlgorithm::AesGcm,
            provider_id,
        };
        crypto_registry_set_key_entry(entry.clone()).await?;
        Ok(CreatedKey { key, entry })
    }

    #[cfg(target_arch = "wasm32")]
    async fn unwrap_key_entry(
        &self,
        entry: &RadrootsClientCryptoKeyEntry,
    ) -> Result<web_sys::CryptoKey, RadrootsClientCryptoError> {
        let mut material = self.key_material_provider.get_key_material().await?;
        let kek = crypto_kdf_derive_kek(&material, &entry.kdf_salt, entry.kdf_iterations).await?;
        material.fill(0);
        crypto_key_unwrap(&kek, &entry.wrapped_key, &entry.wrap_iv).await
    }

    #[cfg(target_arch = "wasm32")]
    async fn decrypt_envelope(
        &self,
        store_id: &str,
        envelope: RadrootsClientCryptoEnvelope,
    ) -> Result<RadrootsClientCryptoDecryptOutcome, RadrootsClientCryptoError> {
        let resolved = self.resolve_key_by_id(store_id, &envelope.key_id).await?;
        let plaintext =
            decrypt_bytes(&resolved.key, &envelope.iv, &envelope.ciphertext).await?;
        let needs_reencrypt = resolved.index.active_key_id != envelope.key_id;
        if !needs_reencrypt {
            return Ok(RadrootsClientCryptoDecryptOutcome {
                plaintext,
                needs_reencrypt,
                reencrypted: None,
            });
        }
        let reencrypted = self.encrypt(store_id, &plaintext).await?;
        Ok(RadrootsClientCryptoDecryptOutcome {
            plaintext,
            needs_reencrypt,
            reencrypted: Some(reencrypted),
        })
    }

    #[cfg(target_arch = "wasm32")]
    async fn decrypt_legacy(
        &self,
        store_id: &str,
        blob: &[u8],
        legacy_key: Option<RadrootsClientLegacyKeyConfig>,
        iv_length: u32,
    ) -> Result<RadrootsClientCryptoDecryptOutcome, RadrootsClientCryptoError> {
        let Some(legacy_key) = legacy_key else {
            return Err(RadrootsClientCryptoError::LegacyKeyMissing);
        };
        let legacy_crypto_key = self.load_legacy_key(&legacy_key).await?;
        let Some(legacy_crypto_key) = legacy_crypto_key else {
            return Err(RadrootsClientCryptoError::LegacyKeyMissing);
        };
        let iv_len = iv_length as usize;
        if blob.len() <= iv_len {
            return Err(RadrootsClientCryptoError::InvalidEnvelope);
        }
        let iv = &blob[..iv_len];
        let ciphertext = &blob[iv_len..];
        let plaintext = decrypt_bytes_with_algorithm(
            &legacy_crypto_key,
            &legacy_key.algorithm,
            iv,
            ciphertext,
        )
        .await?;
        let reencrypted = self.encrypt(store_id, &plaintext).await?;
        Ok(RadrootsClientCryptoDecryptOutcome {
            plaintext,
            needs_reencrypt: true,
            reencrypted: Some(reencrypted),
        })
    }

    #[cfg(target_arch = "wasm32")]
    async fn load_legacy_key(
        &self,
        legacy: &RadrootsClientLegacyKeyConfig,
    ) -> Result<Option<web_sys::CryptoKey>, RadrootsClientCryptoError> {
        let exists = idb_store_exists(legacy.idb_config.database, legacy.idb_config.store)
            .await
            .map_err(map_idb_error)?;
        if !exists {
            return Ok(None);
        }
        idb_store_ensure(legacy.idb_config.database, legacy.idb_config.store)
            .await
            .map_err(map_idb_error)?;
        let stored = idb_get(
            legacy.idb_config.database,
            legacy.idb_config.store,
            &legacy.key_name,
        )
        .await
        .map_err(map_idb_error)?;
        let Some(stored) = stored else {
            return Ok(None);
        };
        if let Ok(key) = stored.clone().dyn_into::<web_sys::CryptoKey>() {
            return Ok(Some(key));
        }
        let Some(bytes) = idb_value_as_bytes(&stored) else {
            return Ok(None);
        };
        crypto_key_import_raw(&bytes).await.map(Some)
    }
}

impl Default for RadrootsClientWebCryptoServiceImpl {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait(?Send)]
impl RadrootsClientWebCryptoService for RadrootsClientWebCryptoServiceImpl {
    fn register_store_config(&mut self, config: RadrootsClientCryptoStoreConfig) {
        let store_id = config.store_id.clone();
        let mut configs = self.store_configs.borrow_mut();
        if let Some(existing) = configs.get(&store_id).cloned() {
            configs.insert(
                store_id,
                RadrootsClientCryptoStoreConfig {
                    store_id: config.store_id,
                    iv_length: config.iv_length.or(existing.iv_length),
                    legacy_key: config.legacy_key.or_else(|| existing.legacy_key.clone()),
                },
            );
            return;
        }
        configs.insert(
            store_id,
            RadrootsClientCryptoStoreConfig {
                store_id: config.store_id,
                iv_length: Some(config.iv_length.unwrap_or(DEFAULT_IV_LENGTH)),
                legacy_key: config.legacy_key,
            },
        );
    }

    async fn encrypt(
        &self,
        store_id: &str,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, RadrootsClientCryptoError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store_id, plaintext);
            return Err(RadrootsClientCryptoError::CryptoUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let resolved = self.resolve_active_key(store_id).await?;
            let iv_length = if resolved.entry.iv_length == 0 {
                DEFAULT_IV_LENGTH
            } else {
                resolved.entry.iv_length
            };
            let mut iv = vec![0u8; iv_length as usize];
            fill_random(&mut iv)?;
            let ciphertext = encrypt_bytes(&resolved.key, &iv, plaintext).await?;
            let envelope = RadrootsClientCryptoEnvelope {
                version: 1,
                key_id: resolved.entry.key_id.clone(),
                iv,
                created_at: js_sys::Date::now() as u64,
                ciphertext,
            };
            crypto_envelope_encode(&envelope)
        }
    }

    async fn decrypt(
        &self,
        store_id: &str,
        blob: &[u8],
    ) -> Result<Vec<u8>, RadrootsClientCryptoError> {
        let outcome = self.decrypt_record(store_id, blob).await?;
        Ok(outcome.plaintext)
    }

    async fn decrypt_record(
        &self,
        store_id: &str,
        blob: &[u8],
    ) -> Result<RadrootsClientCryptoDecryptOutcome, RadrootsClientCryptoError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store_id, blob);
            return Err(RadrootsClientCryptoError::CryptoUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let config = self.resolve_store_config(store_id);
            let envelope = crypto_envelope_decode(blob)?;
            if let Some(envelope) = envelope {
                return self.decrypt_envelope(store_id, envelope).await;
            }
            let iv_length = config.iv_length.unwrap_or(DEFAULT_IV_LENGTH);
            return self
                .decrypt_legacy(store_id, blob, config.legacy_key, iv_length)
                .await;
        }
    }

    async fn rotate_store_key(
        &self,
        store_id: &str,
    ) -> Result<String, RadrootsClientCryptoError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store_id;
            return Err(RadrootsClientCryptoError::CryptoUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let config = self.resolve_store_config(store_id);
            let index = match crypto_registry_get_store_index(store_id).await? {
                Some(index) => index,
                None => {
                    let created = self.create_store_key(store_id, &config).await?;
                    return Ok(created.entry.key_id);
                }
            };
            if let Some(entry) = crypto_registry_get_key_entry(&index.active_key_id).await? {
                let rotated = RadrootsClientCryptoKeyEntry {
                    status: RadrootsClientCryptoKeyStatus::Rotated,
                    ..entry
                };
                crypto_registry_set_key_entry(rotated).await?;
            }
            let created = self.create_key_entry(store_id, &config).await?;
            let next_index = RadrootsClientCryptoStoreIndex {
                store_id: index.store_id,
                active_key_id: created.entry.key_id.clone(),
                key_ids: merge_key_ids(&index.key_ids, &created.entry.key_id),
                created_at: index.created_at,
            };
            crypto_registry_set_store_index(next_index).await?;
            Ok(created.entry.key_id)
        }
    }

    async fn export_registry(
        &self,
    ) -> Result<RadrootsClientCryptoRegistryExport, RadrootsClientCryptoError> {
        crypto_registry_export().await
    }

    async fn import_registry(
        &self,
        registry: RadrootsClientCryptoRegistryExport,
    ) -> Result<(), RadrootsClientCryptoError> {
        crypto_registry_import(registry).await
    }
}

#[cfg(target_arch = "wasm32")]
struct ResolvedKey {
    key: web_sys::CryptoKey,
    entry: RadrootsClientCryptoKeyEntry,
    index: RadrootsClientCryptoStoreIndex,
}

#[cfg(target_arch = "wasm32")]
struct CreatedKey {
    key: web_sys::CryptoKey,
    entry: RadrootsClientCryptoKeyEntry,
}

#[cfg(target_arch = "wasm32")]
fn merge_key_ids(current: &[String], next_key_id: &str) -> Vec<String> {
    if current.iter().any(|key_id| key_id == next_key_id) {
        return current.to_vec();
    }
    let mut merged = current.to_vec();
    merged.push(next_key_id.to_string());
    merged
}

#[cfg(target_arch = "wasm32")]
fn map_idb_error(err: crate::idb::RadrootsClientIdbStoreError) -> RadrootsClientCryptoError {
    match err {
        crate::idb::RadrootsClientIdbStoreError::IdbUndefined => {
            RadrootsClientCryptoError::IdbUndefined
        }
        _ => RadrootsClientCryptoError::RegistryFailure,
    }
}

#[cfg(target_arch = "wasm32")]
fn subtle_crypto() -> Result<web_sys::SubtleCrypto, RadrootsClientCryptoError> {
    let window = web_sys::window().ok_or(RadrootsClientCryptoError::CryptoUndefined)?;
    let crypto = window
        .crypto()
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(crypto.subtle())
}

#[cfg(target_arch = "wasm32")]
fn aes_gcm_params(iv: &[u8]) -> Result<js_sys::Object, RadrootsClientCryptoError> {
    let algo = js_sys::Object::new();
    js_sys::Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let iv_array = js_sys::Uint8Array::from(iv);
    js_sys::Reflect::set(&algo, &"iv".into(), &iv_array.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(algo)
}

#[cfg(target_arch = "wasm32")]
fn algorithm_params(
    name: &str,
    iv: &[u8],
) -> Result<js_sys::Object, RadrootsClientCryptoError> {
    let algo = js_sys::Object::new();
    js_sys::Reflect::set(&algo, &"name".into(), &name.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let iv_array = js_sys::Uint8Array::from(iv);
    js_sys::Reflect::set(&algo, &"iv".into(), &iv_array.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(algo)
}

#[cfg(target_arch = "wasm32")]
async fn encrypt_bytes(
    key: &web_sys::CryptoKey,
    iv: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let algo = aes_gcm_params(iv)?;
    let promise = subtle
        .encrypt_with_object_and_u8_array(&algo, key, plaintext)
        .map_err(|_| RadrootsClientCryptoError::EncryptFailure)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::EncryptFailure)?;
    let array = js_sys::Uint8Array::new(&value);
    let mut out = vec![0u8; array.length() as usize];
    array.copy_to(&mut out);
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
async fn decrypt_bytes(
    key: &web_sys::CryptoKey,
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let algo = aes_gcm_params(iv)?;
    let promise = subtle
        .decrypt_with_object_and_u8_array(&algo, key, ciphertext)
        .map_err(|_| RadrootsClientCryptoError::DecryptFailure)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::DecryptFailure)?;
    let array = js_sys::Uint8Array::new(&value);
    let mut out = vec![0u8; array.length() as usize];
    array.copy_to(&mut out);
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
async fn decrypt_bytes_with_algorithm(
    key: &web_sys::CryptoKey,
    algorithm: &str,
    iv: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let algo = algorithm_params(algorithm, iv)?;
    let promise = subtle
        .decrypt_with_object_and_u8_array(&algo, key, ciphertext)
        .map_err(|_| RadrootsClientCryptoError::DecryptFailure)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::DecryptFailure)?;
    let array = js_sys::Uint8Array::new(&value);
    let mut out = vec![0u8; array.length() as usize];
    array.copy_to(&mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::crypto::{
        RadrootsClientCryptoStoreConfig,
        RadrootsClientLegacyKeyConfig,
        RadrootsClientWebCryptoService,
    };
    use crate::idb::RadrootsClientIdbConfig;

    use super::{RadrootsClientWebCryptoServiceImpl, DEFAULT_IV_LENGTH};

    #[test]
    fn register_store_config_defaults_iv_length() {
        let mut service = RadrootsClientWebCryptoServiceImpl::default();
        service.register_store_config(RadrootsClientCryptoStoreConfig {
            store_id: "store".to_string(),
            legacy_key: None,
            iv_length: None,
        });
        let config = service.resolve_store_config("store");
        assert_eq!(config.iv_length, Some(DEFAULT_IV_LENGTH));
    }

    #[test]
    fn register_store_config_merges_updates() {
        let mut service = RadrootsClientWebCryptoServiceImpl::default();
        service.register_store_config(RadrootsClientCryptoStoreConfig {
            store_id: "store".to_string(),
            legacy_key: None,
            iv_length: Some(16),
        });
        let legacy = RadrootsClientLegacyKeyConfig {
            idb_config: RadrootsClientIdbConfig::new("db", "store"),
            key_name: "key".to_string(),
            iv_length: 12,
            algorithm: "AES-GCM".to_string(),
        };
        service.register_store_config(RadrootsClientCryptoStoreConfig {
            store_id: "store".to_string(),
            legacy_key: Some(legacy.clone()),
            iv_length: None,
        });
        let config = service.resolve_store_config("store");
        assert_eq!(config.iv_length, Some(16));
        assert_eq!(config.legacy_key, Some(legacy));
    }
}
