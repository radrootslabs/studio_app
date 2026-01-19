use super::RadrootsClientCryptoError;
use crate::crypto::random::fill_random;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect, Uint8Array};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{CryptoKey, SubtleCrypto};

const KEY_ID_BYTES_LENGTH: usize = 16;
#[cfg(target_arch = "wasm32")]
const WRAP_IV_LENGTH: usize = 12;

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", byte);
    }
    out
}

pub fn crypto_key_id_create() -> Result<String, RadrootsClientCryptoError> {
    let mut bytes = [0u8; KEY_ID_BYTES_LENGTH];
    fill_random(&mut bytes)?;
    Ok(bytes_to_hex(&bytes))
}

#[cfg(target_arch = "wasm32")]
fn subtle_crypto() -> Result<SubtleCrypto, RadrootsClientCryptoError> {
    let window = web_sys::window().ok_or(RadrootsClientCryptoError::CryptoUndefined)?;
    let crypto = window
        .crypto()
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(crypto.subtle())
}

#[cfg(target_arch = "wasm32")]
fn encrypt_decrypt_usages() -> Array {
    let usages = Array::new();
    usages.push(&"encrypt".into());
    usages.push(&"decrypt".into());
    usages
}

#[cfg(target_arch = "wasm32")]
fn aes_gcm_algorithm(length: u32) -> Result<Object, RadrootsClientCryptoError> {
    let algo = Object::new();
    Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Reflect::set(
        &algo,
        &"length".into(),
        &wasm_bindgen::JsValue::from_f64(length as f64),
    )
    .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(algo)
}

#[cfg(target_arch = "wasm32")]
fn aes_gcm_params(iv: &[u8]) -> Result<Object, RadrootsClientCryptoError> {
    let algo = Object::new();
    Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let iv_array = Uint8Array::from(iv);
    Reflect::set(&algo, &"iv".into(), &iv_array.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(algo)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_key_generate() -> Result<CryptoKey, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let algo = aes_gcm_algorithm(256)?;
    let usages = encrypt_decrypt_usages();
    let promise = subtle
        .generate_key_with_object(&algo, true, &usages.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    value
        .dyn_into::<CryptoKey>()
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_key_export_raw(
    key: &CryptoKey,
) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let promise = subtle
        .export_key("raw", key)
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let array = Uint8Array::new(&value);
    let mut out = vec![0u8; array.length() as usize];
    array.copy_to(&mut out);
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_key_import_raw(
    raw: &[u8],
) -> Result<CryptoKey, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let algo = aes_gcm_algorithm(256)?;
    let usages = encrypt_decrypt_usages();
    let data = Uint8Array::from(raw);
    let data_obj = data.unchecked_ref::<Object>();
    let promise = subtle
        .import_key_with_object("raw", data_obj, &algo, false, &usages.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    value
        .dyn_into::<CryptoKey>()
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_key_wrap(
    kek: &CryptoKey,
    raw_key: &mut [u8],
) -> Result<(Vec<u8>, Vec<u8>), RadrootsClientCryptoError> {
    let subtle = subtle_crypto().map_err(|_| RadrootsClientCryptoError::WrapFailure)?;
    let mut wrap_iv = vec![0u8; WRAP_IV_LENGTH];
    fill_random(&mut wrap_iv).map_err(|_| RadrootsClientCryptoError::WrapFailure)?;
    let algo = aes_gcm_params(&wrap_iv).map_err(|_| RadrootsClientCryptoError::WrapFailure)?;
    let promise = subtle
        .encrypt_with_object_and_u8_array(&algo, kek, raw_key)
        .map_err(|_| RadrootsClientCryptoError::WrapFailure)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::WrapFailure)?;
    let array = Uint8Array::new(&value);
    let mut wrapped = vec![0u8; array.length() as usize];
    array.copy_to(&mut wrapped);
    raw_key.fill(0);
    Ok((wrapped, wrap_iv))
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_key_unwrap(
    kek: &CryptoKey,
    wrapped_key: &[u8],
    wrap_iv: &[u8],
) -> Result<CryptoKey, RadrootsClientCryptoError> {
    let subtle = subtle_crypto().map_err(|_| RadrootsClientCryptoError::UnwrapFailure)?;
    let algo = aes_gcm_params(wrap_iv).map_err(|_| RadrootsClientCryptoError::UnwrapFailure)?;
    let promise = subtle
        .decrypt_with_object_and_u8_array(&algo, kek, wrapped_key)
        .map_err(|_| RadrootsClientCryptoError::UnwrapFailure)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::UnwrapFailure)?;
    let array = Uint8Array::new(&value);
    let mut raw = vec![0u8; array.length() as usize];
    array.copy_to(&mut raw);
    crypto_key_import_raw(&raw)
        .await
        .map_err(|_| RadrootsClientCryptoError::UnwrapFailure)
}

#[cfg(test)]
mod tests {
    use super::crypto_key_id_create;

    #[test]
    fn key_id_is_hex() {
        let key_id = crypto_key_id_create().expect("key id");
        assert_eq!(key_id.len(), 32);
        assert!(key_id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
