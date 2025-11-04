use base64::{Engine as _, engine::general_purpose};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("Base64 error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Key error: {0}")]
    KeyError(#[from] nostr_sdk::key::Error),
    #[error("error.keystore.key_not_found")]
    KeyNotFound,
    #[error("Storage error")]
    Storage,
}

pub type KeystoreResult<T> = std::result::Result<T, KeystoreError>;

pub trait Storage {
    fn read_text(&self, path: &str) -> KeystoreResult<Option<String>>;
    fn write_text(&self, path: &str, data: &str) -> KeystoreResult<()>;
    fn exists(&self, path: &str) -> KeystoreResult<bool>;
}

#[cfg(target_arch = "wasm32")]
pub struct BrowserStorage;

#[cfg(target_arch = "wasm32")]
impl Storage for BrowserStorage {
    fn read_text(&self, path: &str) -> KeystoreResult<Option<String>> {
        let storage = window()
            .ok_or(KeystoreError::Storage)?
            .local_storage()
            .map_err(|_| KeystoreError::Storage)?
            .ok_or(KeystoreError::Storage)?;
        storage.get_item(path).map_err(|_| KeystoreError::Storage)
    }

    fn write_text(&self, path: &str, data: &str) -> KeystoreResult<()> {
        let storage = window()
            .ok_or(KeystoreError::Storage)?
            .local_storage()
            .map_err(|_| KeystoreError::Storage)?
            .ok_or(KeystoreError::Storage)?;
        storage
            .set_item(path, data)
            .map_err(|_| KeystoreError::Storage)?;
        Ok(())
    }

    fn exists(&self, path: &str) -> KeystoreResult<bool> {
        let storage = window()
            .ok_or(KeystoreError::Storage)?
            .local_storage()
            .map_err(|_| KeystoreError::Storage)?
            .ok_or(KeystoreError::Storage)?;
        storage
            .get_item(path)
            .map_err(|_| KeystoreError::Storage)
            .map(|v| v.is_some())
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct NullStorage;

#[cfg(not(target_arch = "wasm32"))]
impl Storage for NullStorage {
    fn read_text(&self, _path: &str) -> KeystoreResult<Option<String>> {
        Ok(None)
    }
    fn write_text(&self, _path: &str, _data: &str) -> KeystoreResult<()> {
        Err(KeystoreError::Storage)
    }
    fn exists(&self, _path: &str) -> KeystoreResult<bool> {
        Ok(false)
    }
}

fn get_keystore_key<S: Storage>(st: &S) -> KeystoreResult<Vec<u8>> {
    let key_path = "keystore.key";
    if st.exists(key_path)? {
        let s = st.read_text(key_path)?.ok_or(KeystoreError::Storage)?;
        let id = s.parse::<Uuid>()?;
        Ok(id.as_bytes().to_vec())
    } else {
        let id = Uuid::new_v4();
        st.write_text(key_path, &id.to_string())?;
        Ok(id.as_bytes().to_vec())
    }
}

fn obfuscate<S: Storage>(st: &S, data: &str) -> KeystoreResult<String> {
    let keystore_key = get_keystore_key(st)?;
    let xored: Vec<u8> = data
        .as_bytes()
        .iter()
        .zip(keystore_key.iter().cycle())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    Ok(general_purpose::STANDARD_NO_PAD.encode(xored))
}

fn deobfuscate<S: Storage>(st: &S, data: &str) -> KeystoreResult<String> {
    let keystore_key = get_keystore_key(st)?;
    let decoded = general_purpose::STANDARD_NO_PAD.decode(data)?;
    let xored: Vec<u8> = decoded
        .iter()
        .zip(keystore_key.iter().cycle())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    Ok(String::from_utf8(xored)?)
}

fn read_keystore_file<S: Storage>(st: &S) -> KeystoreResult<serde_json::Value> {
    let content = st
        .read_text("keystore.json")?
        .unwrap_or_else(|| "{}".to_string());
    Ok(serde_json::from_str(&content)?)
}

fn write_keystore_file<S: Storage>(st: &S, secrets: &serde_json::Value) -> KeystoreResult<()> {
    let content = serde_json::to_string_pretty(secrets)?;
    st.write_text("keystore.json", &content)?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn storage() -> BrowserStorage {
    BrowserStorage
}

#[cfg(not(target_arch = "wasm32"))]
fn storage() -> NullStorage {
    NullStorage
}

pub fn key_add(keys: &nostr_sdk::Keys) -> KeystoreResult<()> {
    let st = storage();
    let mut secrets = read_keystore_file(&st).unwrap_or(json!({}));
    let obfuscated_key = obfuscate(&st, keys.secret_key().to_secret_hex().as_str())?;
    secrets[keys.public_key().to_hex()] = json!(obfuscated_key);
    write_keystore_file(&st, &secrets)?;
    Ok(())
}

pub fn key_read(public_key: &str) -> KeystoreResult<nostr_sdk::Keys> {
    let st = storage();
    let secrets = read_keystore_file(&st)?;
    let obfuscated_key = secrets[public_key]
        .as_str()
        .ok_or(KeystoreError::KeyNotFound)?;
    let secret_key = deobfuscate(&st, obfuscated_key)?;
    Ok(nostr_sdk::Keys::parse(&secret_key)?)
}

pub fn key_delete(public_key: &str) -> KeystoreResult<()> {
    let st = storage();
    let mut secrets = read_keystore_file(&st)?;
    secrets.as_object_mut().map(|obj| obj.remove(public_key));
    write_keystore_file(&st, &secrets)?;
    Ok(())
}

pub fn keys_read_all() -> KeystoreResult<Vec<String>> {
    let st = storage();
    let secrets = read_keystore_file(&st)?;
    Ok(crate::util::json_keys(&secrets))
}

pub fn reset() -> KeystoreResult<()> {
    let st = storage();
    write_keystore_file(&st, &json!({}))?;
    Ok(())
}
