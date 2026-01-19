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

const DEFAULT_KDF_ITERATIONS: u32 = 210_000;
#[cfg(target_arch = "wasm32")]
const KDF_HASH: &str = "SHA-256";

pub fn crypto_kdf_iterations_default() -> u32 {
    DEFAULT_KDF_ITERATIONS
}

pub fn crypto_kdf_salt_create(length: usize) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let mut salt = vec![0u8; length];
    fill_random(&mut salt)?;
    Ok(salt)
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
fn derive_key_usages() -> Array {
    let usages = Array::new();
    usages.push(&"deriveKey".into());
    usages
}

#[cfg(target_arch = "wasm32")]
fn encrypt_decrypt_usages() -> Array {
    let usages = Array::new();
    usages.push(&"encrypt".into());
    usages.push(&"decrypt".into());
    usages
}

#[cfg(target_arch = "wasm32")]
fn pbkdf2_params(
    salt: &[u8],
    iterations: u32,
) -> Result<Object, RadrootsClientCryptoError> {
    let params = Object::new();
    Reflect::set(&params, &"name".into(), &"PBKDF2".into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    let salt_array = Uint8Array::from(salt);
    Reflect::set(&params, &"salt".into(), &salt_array.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Reflect::set(
        &params,
        &"iterations".into(),
        &wasm_bindgen::JsValue::from_f64(iterations as f64),
    )
    .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Reflect::set(&params, &"hash".into(), &KDF_HASH.into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(params)
}

#[cfg(target_arch = "wasm32")]
fn aes_gcm_algorithm() -> Result<Object, RadrootsClientCryptoError> {
    let algo = Object::new();
    Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Reflect::set(
        &algo,
        &"length".into(),
        &wasm_bindgen::JsValue::from_f64(256.0),
    )
    .map_err(|_| RadrootsClientCryptoError::CryptoUndefined)?;
    Ok(algo)
}

#[cfg(target_arch = "wasm32")]
pub async fn crypto_kdf_derive_kek(
    material: &[u8],
    salt: &[u8],
    iterations: u32,
) -> Result<CryptoKey, RadrootsClientCryptoError> {
    let subtle = subtle_crypto()?;
    let key_data = Uint8Array::from(material);
    let key_data_obj = key_data.unchecked_ref::<Object>();
    let base_promise = subtle
        .import_key_with_str(
            "raw",
            key_data_obj,
            "PBKDF2",
            false,
            &derive_key_usages().into(),
        )
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    let base_value = JsFuture::from(base_promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    let base_key = base_value
        .dyn_into::<CryptoKey>()
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)?;

    let pbkdf2 = pbkdf2_params(salt, iterations).map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    let aes_gcm = aes_gcm_algorithm().map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    let promise = subtle
        .derive_key_with_object_and_object(
            &pbkdf2,
            &base_key,
            &aes_gcm,
            false,
            &encrypt_decrypt_usages().into(),
        )
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)?;
    value
        .dyn_into::<CryptoKey>()
        .map_err(|_| RadrootsClientCryptoError::KdfFailure)
}

#[cfg(test)]
mod tests {
    use super::{crypto_kdf_iterations_default, crypto_kdf_salt_create};

    #[test]
    fn kdf_defaults_match_spec() {
        assert_eq!(crypto_kdf_iterations_default(), 210_000);
    }

    #[test]
    fn kdf_salt_length_matches_request() {
        let salt = crypto_kdf_salt_create(16).expect("salt");
        assert_eq!(salt.len(), 16);
    }
}
