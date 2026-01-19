use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

#[cfg(target_arch = "wasm32")]
use crate::crypto::RadrootsClientCryptoError;
#[cfg(target_arch = "wasm32")]
use crate::crypto::{
    crypto_kdf_iterations_default,
    crypto_kdf_salt_create,
};
use crate::crypto::RadrootsClientKeyMaterialProvider;

use super::{
    RadrootsClientBackupBundle,
    RadrootsClientBackupError,
};
#[cfg(target_arch = "wasm32")]
use super::RadrootsClientBackupBundleEnvelope;

pub fn backup_bytes_to_b64(bytes: &[u8]) -> Result<String, RadrootsClientBackupError> {
    Ok(STANDARD.encode(bytes))
}

pub fn backup_b64_to_bytes(value: &str) -> Result<Vec<u8>, RadrootsClientBackupError> {
    STANDARD
        .decode(value)
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)
}

#[cfg(target_arch = "wasm32")]
fn map_crypto_error(
    err: RadrootsClientCryptoError,
    fallback: RadrootsClientBackupError,
) -> RadrootsClientBackupError {
    match err {
        RadrootsClientCryptoError::CryptoUndefined => {
            RadrootsClientBackupError::CryptoUndefined
        }
        _ => fallback,
    }
}

#[cfg(target_arch = "wasm32")]
async fn encrypt_bytes(
    key: &web_sys::CryptoKey,
    iv: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, RadrootsClientBackupError> {
    let window = web_sys::window().ok_or(RadrootsClientBackupError::CryptoUndefined)?;
    let crypto = window
        .crypto()
        .map_err(|_| RadrootsClientBackupError::CryptoUndefined)?;
    let subtle = crypto.subtle();
    let algo = js_sys::Object::new();
    js_sys::Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
    let iv_array = js_sys::Uint8Array::from(iv);
    js_sys::Reflect::set(&algo, &"iv".into(), &iv_array.into())
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
    let promise = subtle
        .encrypt_with_object_and_u8_array(&algo, key, plaintext)
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
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
) -> Result<Vec<u8>, RadrootsClientBackupError> {
    let window = web_sys::window().ok_or(RadrootsClientBackupError::CryptoUndefined)?;
    let crypto = window
        .crypto()
        .map_err(|_| RadrootsClientBackupError::CryptoUndefined)?;
    let subtle = crypto.subtle();
    let algo = js_sys::Object::new();
    js_sys::Reflect::set(&algo, &"name".into(), &"AES-GCM".into())
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    let iv_array = js_sys::Uint8Array::from(iv);
    js_sys::Reflect::set(&algo, &"iv".into(), &iv_array.into())
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    let promise = subtle
        .decrypt_with_object_and_u8_array(&algo, key, ciphertext)
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    let array = js_sys::Uint8Array::new(&value);
    let mut out = vec![0u8; array.length() as usize];
    array.copy_to(&mut out);
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub async fn backup_bundle_encode(
    bundle: &RadrootsClientBackupBundle,
    provider: &dyn RadrootsClientKeyMaterialProvider,
) -> Result<Vec<u8>, RadrootsClientBackupError> {
    let json = serde_json::to_string(bundle)
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
    let plaintext = json.into_bytes();
    let salt = crypto_kdf_salt_create(16)
        .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::EncodeFailure))?;
    let iterations = crypto_kdf_iterations_default();
    let mut material = provider
        .get_key_material()
        .await
        .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::EncodeFailure))?;
    let kek = crate::crypto::crypto_kdf_derive_kek(&material, &salt, iterations)
        .await
        .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::EncodeFailure))?;
    material.fill(0);
    let mut iv = vec![0u8; 12];
    crate::crypto::random::fill_random(&mut iv)
        .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::EncodeFailure))?;
    let ciphertext = encrypt_bytes(&kek, &iv, &plaintext).await?;
    let envelope = RadrootsClientBackupBundleEnvelope {
        version: 1,
        created_at: js_sys::Date::now() as u64,
        kdf_salt_b64: backup_bytes_to_b64(&salt)?,
        kdf_iterations: iterations,
        iv_b64: backup_bytes_to_b64(&iv)?,
        ciphertext_b64: backup_bytes_to_b64(&ciphertext)?,
    };
    let encoded = serde_json::to_string(&envelope)
        .map_err(|_| RadrootsClientBackupError::EncodeFailure)?;
    Ok(encoded.into_bytes())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn backup_bundle_encode(
    _bundle: &RadrootsClientBackupBundle,
    _provider: &dyn RadrootsClientKeyMaterialProvider,
) -> Result<Vec<u8>, RadrootsClientBackupError> {
    Err(RadrootsClientBackupError::CryptoUndefined)
}

#[cfg(target_arch = "wasm32")]
pub async fn backup_bundle_decode(
    blob: &[u8],
    provider: &dyn RadrootsClientKeyMaterialProvider,
) -> Result<RadrootsClientBackupBundle, RadrootsClientBackupError> {
    let json = std::str::from_utf8(blob)
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    let envelope: RadrootsClientBackupBundleEnvelope = serde_json::from_str(json)
        .map_err(|_| RadrootsClientBackupError::InvalidBundle)?;
    let salt = backup_b64_to_bytes(&envelope.kdf_salt_b64)?;
    let iv = backup_b64_to_bytes(&envelope.iv_b64)?;
    let ciphertext = backup_b64_to_bytes(&envelope.ciphertext_b64)?;
    let mut material = provider
        .get_key_material()
        .await
        .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::DecodeFailure))?;
    let kek = crate::crypto::crypto_kdf_derive_kek(
        &material,
        &salt,
        envelope.kdf_iterations,
    )
    .await
    .map_err(|e| map_crypto_error(e, RadrootsClientBackupError::DecodeFailure))?;
    material.fill(0);
    let plaintext = decrypt_bytes(&kek, &iv, &ciphertext).await?;
    let payload = std::str::from_utf8(&plaintext)
        .map_err(|_| RadrootsClientBackupError::DecodeFailure)?;
    serde_json::from_str(payload)
        .map_err(|_| RadrootsClientBackupError::InvalidBundle)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn backup_bundle_decode(
    _blob: &[u8],
    _provider: &dyn RadrootsClientKeyMaterialProvider,
) -> Result<RadrootsClientBackupBundle, RadrootsClientBackupError> {
    Err(RadrootsClientBackupError::CryptoUndefined)
}

#[cfg(test)]
mod tests {
    use super::{backup_b64_to_bytes, backup_bytes_to_b64};

    #[test]
    fn base64_roundtrip() {
        let data = b"radroots";
        let encoded = backup_bytes_to_b64(data).expect("encode");
        let decoded = backup_b64_to_bytes(&encoded).expect("decode");
        assert_eq!(decoded, data);
    }

    #[test]
    fn base64_decode_rejects_invalid() {
        let err = backup_b64_to_bytes("not-base64").expect_err("invalid");
        assert_eq!(err, super::RadrootsClientBackupError::DecodeFailure);
    }
}
