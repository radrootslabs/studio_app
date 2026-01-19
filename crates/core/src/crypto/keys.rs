use super::{RadrootsClientCryptoError};
use crate::crypto::random::fill_random;

const KEY_ID_BYTES_LENGTH: usize = 16;

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
