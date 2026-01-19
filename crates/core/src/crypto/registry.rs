use crate::crypto::{RadrootsClientCryptoError, RadrootsClientCryptoRegistryExport};
use crate::crypto::{RadrootsClientCryptoKeyEntry, RadrootsClientCryptoStoreIndex};

#[cfg(any(test, target_arch = "wasm32"))]
const STORE_INDEX_PREFIX: &str = "store:";
#[cfg(any(test, target_arch = "wasm32"))]
const KEY_ENTRY_PREFIX: &str = "key:";
#[cfg(target_arch = "wasm32")]
const DEVICE_MATERIAL_KEY: &str = "device:material";

#[cfg(any(test, target_arch = "wasm32"))]
fn store_index_key(store_id: &str) -> String {
    format!("{STORE_INDEX_PREFIX}{store_id}")
}

#[cfg(any(test, target_arch = "wasm32"))]
fn key_entry_key(key_id: &str) -> String {
    format!("{KEY_ENTRY_PREFIX}{key_id}")
}

#[cfg(target_arch = "wasm32")]
use crate::idb::{
    idb_del,
    idb_get,
    idb_keys,
    idb_set,
    idb_store_ensure,
    idb_value_as_bytes,
    IDB_CONFIG_CRYPTO_REGISTRY,
    RadrootsClientIdbStoreError,
};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
fn map_idb_error(err: RadrootsClientIdbStoreError) -> RadrootsClientCryptoError {
    match err {
        RadrootsClientIdbStoreError::IdbUndefined => RadrootsClientCryptoError::IdbUndefined,
        _ => RadrootsClientCryptoError::RegistryFailure,
    }
}

#[cfg(target_arch = "wasm32")]
async fn ensure_idb() -> Result<(), RadrootsClientCryptoError> {
    idb_store_ensure(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(target_arch = "wasm32")]
fn decode_store_index(value: &JsValue) -> Result<RadrootsClientCryptoStoreIndex, RadrootsClientCryptoError> {
    if let Some(text) = value.as_string() {
        return serde_json::from_str(&text)
            .map_err(|_| RadrootsClientCryptoError::RegistryFailure);
    }
    serde_wasm_bindgen::from_value(value.clone())
        .map_err(|_| RadrootsClientCryptoError::RegistryFailure)
}

#[cfg(target_arch = "wasm32")]
fn decode_key_entry(value: &JsValue) -> Result<RadrootsClientCryptoKeyEntry, RadrootsClientCryptoError> {
    if let Some(text) = value.as_string() {
        return serde_json::from_str(&text)
            .map_err(|_| RadrootsClientCryptoError::RegistryFailure);
    }
    serde_wasm_bindgen::from_value(value.clone())
        .map_err(|_| RadrootsClientCryptoError::RegistryFailure)
}

#[cfg(target_arch = "wasm32")]
fn encode_store_index(
    index: &RadrootsClientCryptoStoreIndex,
) -> Result<JsValue, RadrootsClientCryptoError> {
    serde_wasm_bindgen::to_value(index)
        .map_err(|_| RadrootsClientCryptoError::RegistryFailure)
}

#[cfg(target_arch = "wasm32")]
fn encode_key_entry(
    entry: &RadrootsClientCryptoKeyEntry,
) -> Result<JsValue, RadrootsClientCryptoError> {
    serde_wasm_bindgen::to_value(entry)
        .map_err(|_| RadrootsClientCryptoError::RegistryFailure)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_get_store_index(
    store_id: &str,
) -> Result<Option<RadrootsClientCryptoStoreIndex>, RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = store_index_key(store_id);
    let value = idb_get(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
    )
    .await
    .map_err(map_idb_error)?;
    let Some(value) = value else {
        return Ok(None);
    };
    decode_store_index(&value).map(Some)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_set_store_index(
    index: RadrootsClientCryptoStoreIndex,
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = store_index_key(&index.store_id);
    let value = encode_store_index(&index)?;
    idb_set(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
        &value,
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_get_key_entry(
    key_id: &str,
) -> Result<Option<RadrootsClientCryptoKeyEntry>, RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = key_entry_key(key_id);
    let value = idb_get(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
    )
    .await
    .map_err(map_idb_error)?;
    let Some(value) = value else {
        return Ok(None);
    };
    decode_key_entry(&value).map(Some)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_set_key_entry(
    entry: RadrootsClientCryptoKeyEntry,
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = key_entry_key(&entry.key_id);
    let value = encode_key_entry(&entry)?;
    idb_set(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
        &value,
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_list_store_indices(
) -> Result<Vec<RadrootsClientCryptoStoreIndex>, RadrootsClientCryptoError> {
    ensure_idb().await?;
    let keys = idb_keys(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
    )
    .await
    .map_err(map_idb_error)?;
    let mut out = Vec::new();
    for key in keys {
        if !key.starts_with(STORE_INDEX_PREFIX) {
            continue;
        }
        let value = idb_get(
            IDB_CONFIG_CRYPTO_REGISTRY.database,
            IDB_CONFIG_CRYPTO_REGISTRY.store,
            &key,
        )
        .await
        .map_err(map_idb_error)?;
        let Some(value) = value else {
            continue;
        };
        let index = decode_store_index(&value)?;
        out.push(index);
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_list_key_entries(
) -> Result<Vec<RadrootsClientCryptoKeyEntry>, RadrootsClientCryptoError> {
    ensure_idb().await?;
    let keys = idb_keys(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
    )
    .await
    .map_err(map_idb_error)?;
    let mut out = Vec::new();
    for key in keys {
        if !key.starts_with(KEY_ENTRY_PREFIX) {
            continue;
        }
        let value = idb_get(
            IDB_CONFIG_CRYPTO_REGISTRY.database,
            IDB_CONFIG_CRYPTO_REGISTRY.store,
            &key,
        )
        .await
        .map_err(map_idb_error)?;
        let Some(value) = value else {
            continue;
        };
        let entry = decode_key_entry(&value)?;
        out.push(entry);
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_export(
) -> Result<RadrootsClientCryptoRegistryExport, RadrootsClientCryptoError> {
    let stores = crypto_registry_list_store_indices().await?;
    let keys = crypto_registry_list_key_entries().await?;
    Ok(RadrootsClientCryptoRegistryExport { stores, keys })
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_import(
    registry: RadrootsClientCryptoRegistryExport,
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    for store_index in registry.stores {
        crypto_registry_set_store_index(store_index).await?;
    }
    for entry in registry.keys {
        crypto_registry_set_key_entry(entry).await?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_get_device_material(
) -> Result<Option<Vec<u8>>, RadrootsClientCryptoError> {
    ensure_idb().await?;
    let value = idb_get(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        DEVICE_MATERIAL_KEY,
    )
    .await
    .map_err(map_idb_error)?;
    let Some(value) = value else {
        return Ok(None);
    };
    idb_value_as_bytes(&value).ok_or(RadrootsClientCryptoError::RegistryFailure).map(Some)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_set_device_material(
    material: &[u8],
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    let value = Uint8Array::from(material);
    idb_set(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        DEVICE_MATERIAL_KEY,
        &value.into(),
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_clear_store_index(
    store_id: &str,
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = store_index_key(store_id);
    idb_del(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_registry_clear_key_entry(
    key_id: &str,
) -> Result<(), RadrootsClientCryptoError> {
    ensure_idb().await?;
    let key = key_entry_key(key_id);
    idb_del(
        IDB_CONFIG_CRYPTO_REGISTRY.database,
        IDB_CONFIG_CRYPTO_REGISTRY.store,
        &key,
    )
    .await
    .map_err(map_idb_error)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_get_store_index(
    _store_id: &str,
) -> Result<Option<RadrootsClientCryptoStoreIndex>, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_set_store_index(
    _index: RadrootsClientCryptoStoreIndex,
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_get_key_entry(
    _key_id: &str,
) -> Result<Option<RadrootsClientCryptoKeyEntry>, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_set_key_entry(
    _entry: RadrootsClientCryptoKeyEntry,
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_list_store_indices(
) -> Result<Vec<RadrootsClientCryptoStoreIndex>, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_list_key_entries(
) -> Result<Vec<RadrootsClientCryptoKeyEntry>, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_export(
) -> Result<RadrootsClientCryptoRegistryExport, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_import(
    _registry: RadrootsClientCryptoRegistryExport,
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_get_device_material(
) -> Result<Option<Vec<u8>>, RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_set_device_material(
    _material: &[u8],
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_clear_store_index(
    _store_id: &str,
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn crypto_registry_clear_key_entry(
    _key_id: &str,
) -> Result<(), RadrootsClientCryptoError> {
    Err(RadrootsClientCryptoError::IdbUndefined)
}

#[cfg(test)]
mod tests {
    use super::{key_entry_key, store_index_key};

    #[test]
    fn store_index_key_prefixes() {
        assert_eq!(store_index_key("alpha"), "store:alpha");
    }

    #[test]
    fn key_entry_key_prefixes() {
        assert_eq!(key_entry_key("beta"), "key:beta");
    }
}
