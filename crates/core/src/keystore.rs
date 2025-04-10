use base64::{engine::general_purpose, Engine as _};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::util::json_keys;

#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("UUID error: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
    #[error("Base64 error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Key error: {0}")]
    KeyError(#[from] nostr_sdk::key::Error),
    #[error("error.keystore.key_not_found")]
    KeyNotFound,
}

pub type KeystoreResult<T> = std::result::Result<T, KeystoreError>;

fn get_keystore_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("keystore.json")
}

fn get_keystore_key(data_dir: &std::path::Path) -> Vec<u8> {
    let keystore_file = data_dir.join("keystore.key");
    let keystore_key = if keystore_file.exists() {
        std::fs::read_to_string(&keystore_file)
            .map_err(KeystoreError::FileError)
            .and_then(|s| s.parse::<Uuid>().map_err(KeystoreError::UuidError))
    } else {
        let new_keystore_key = Uuid::new_v4();
        let _ = std::fs::create_dir_all(data_dir).map_err(KeystoreError::FileError);
        let _ = std::fs::write(keystore_file, new_keystore_key.to_string())
            .map_err(KeystoreError::FileError);
        Ok(new_keystore_key)
    };
    keystore_key
        .expect("Couldn't unwrap keystore key")
        .as_bytes()
        .to_vec()
}

fn obfuscate(data: &str, data_dir: &std::path::Path) -> String {
    let keystore_key = get_keystore_key(data_dir);
    let xored: Vec<u8> = data
        .as_bytes()
        .iter()
        .zip(keystore_key.iter().cycle())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    general_purpose::STANDARD_NO_PAD.encode(xored)
}

fn deobfuscate(data: &str, data_dir: &std::path::Path) -> KeystoreResult<String> {
    let keystore_key = get_keystore_key(data_dir);
    let decoded = general_purpose::STANDARD_NO_PAD
        .decode(data)
        .map_err(KeystoreError::Base64Error)?;
    let xored: Vec<u8> = decoded
        .iter()
        .zip(keystore_key.iter().cycle())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();
    String::from_utf8(xored).map_err(KeystoreError::Utf8Error)
}

fn read_keystore_file(data_dir: &std::path::Path) -> KeystoreResult<serde_json::Value> {
    let content = match std::fs::read_to_string(get_keystore_path(data_dir)) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::from("{}"),
        Err(e) => return Err(e.into()),
    };
    Ok(serde_json::from_str(&content)?)
}

fn write_keystore_file(
    data_dir: &std::path::Path,
    secrets: &serde_json::Value,
) -> KeystoreResult<()> {
    let content = serde_json::to_string_pretty(secrets)?;
    std::fs::write(get_keystore_path(data_dir), content)?;
    Ok(())
}

pub fn key_add(keys: &nostr_sdk::Keys, data_dir: &std::path::Path) -> KeystoreResult<()> {
    let mut secrets = read_keystore_file(data_dir).unwrap_or(json!({}));
    let obfuscated_key = obfuscate(keys.secret_key().to_secret_hex().as_str(), data_dir);
    secrets[keys.public_key().to_hex()] = json!(obfuscated_key);
    write_keystore_file(data_dir, &secrets)?;
    Ok(())
}

pub fn key_read(public_key: &str, data_dir: &std::path::Path) -> KeystoreResult<nostr_sdk::Keys> {
    let secrets = read_keystore_file(data_dir)?;
    let obfuscated_key = secrets[public_key]
        .as_str()
        .ok_or(KeystoreError::KeyNotFound)?;
    let secret_key = deobfuscate(obfuscated_key, data_dir)?;
    nostr_sdk::Keys::parse(&secret_key).map_err(KeystoreError::KeyError)
}

pub fn key_delete(public_key: &str, data_dir: &std::path::Path) -> KeystoreResult<()> {
    let mut secrets = read_keystore_file(data_dir)?;
    secrets.as_object_mut().map(|obj| obj.remove(public_key));
    write_keystore_file(data_dir, &secrets)?;
    Ok(())
}

pub fn keys_read_all(data_dir: &std::path::Path) -> KeystoreResult<Vec<String>> {
    let secrets = read_keystore_file(data_dir)?;
    let results = json_keys(&secrets);
    Ok(results)
}

pub fn reset(data_dir: &std::path::Path) -> KeystoreResult<()> {
    write_keystore_file(data_dir, &json!({}))?;
    Ok(())
}
