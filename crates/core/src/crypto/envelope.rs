use super::{RadrootsClientCryptoEnvelope, RadrootsClientCryptoError};

const ENVELOPE_MAGIC: [u8; 4] = [0x52, 0x52, 0x43, 0x45];
const ENVELOPE_VERSION: u8 = 1;
const ENVELOPE_HEADER_LENGTH: usize = 4 + 1 + 1 + 1 + 8;

pub fn crypto_envelope_encode(
    envelope: &RadrootsClientCryptoEnvelope,
) -> Result<Vec<u8>, RadrootsClientCryptoError> {
    let key_bytes = envelope.key_id.as_bytes();
    if key_bytes.len() > u8::MAX as usize {
        return Err(RadrootsClientCryptoError::InvalidKeyId);
    }
    let total_len = ENVELOPE_HEADER_LENGTH
        + key_bytes.len()
        + envelope.iv.len()
        + envelope.ciphertext.len();
    let mut out = vec![0u8; total_len];
    let mut offset = 0;
    out[offset..offset + ENVELOPE_MAGIC.len()].copy_from_slice(&ENVELOPE_MAGIC);
    offset += ENVELOPE_MAGIC.len();
    out[offset] = ENVELOPE_VERSION;
    offset += 1;
    out[offset] = key_bytes.len() as u8;
    offset += 1;
    out[offset] = envelope.iv.len() as u8;
    offset += 1;
    out[offset..offset + 8].copy_from_slice(&envelope.created_at.to_be_bytes());
    offset += 8;
    out[offset..offset + key_bytes.len()].copy_from_slice(key_bytes);
    offset += key_bytes.len();
    out[offset..offset + envelope.iv.len()].copy_from_slice(&envelope.iv);
    offset += envelope.iv.len();
    out[offset..offset + envelope.ciphertext.len()].copy_from_slice(&envelope.ciphertext);
    Ok(out)
}

pub fn crypto_envelope_decode(
    blob: &[u8],
) -> Result<Option<RadrootsClientCryptoEnvelope>, RadrootsClientCryptoError> {
    if blob.len() < ENVELOPE_HEADER_LENGTH {
        return Ok(None);
    }
    if blob[..ENVELOPE_MAGIC.len()] != ENVELOPE_MAGIC {
        return Ok(None);
    }
    let mut offset = ENVELOPE_MAGIC.len();
    let version = blob[offset];
    offset += 1;
    if version != ENVELOPE_VERSION {
        return Err(RadrootsClientCryptoError::InvalidEnvelope);
    }
    let key_len = blob[offset] as usize;
    offset += 1;
    let iv_len = blob[offset] as usize;
    offset += 1;
    if blob.len() < offset + 8 {
        return Err(RadrootsClientCryptoError::InvalidEnvelope);
    }
    let created_at = u64::from_be_bytes(
        blob[offset..offset + 8]
            .try_into()
            .map_err(|_| RadrootsClientCryptoError::InvalidEnvelope)?,
    );
    offset += 8;
    let remaining = blob.len() - offset;
    if remaining < key_len + iv_len + 1 {
        return Err(RadrootsClientCryptoError::InvalidEnvelope);
    }
    let key_end = offset + key_len;
    let iv_end = key_end + iv_len;
    let key_bytes = &blob[offset..key_end];
    let iv = blob[key_end..iv_end].to_vec();
    let ciphertext = blob[iv_end..].to_vec();
    let key_id = std::str::from_utf8(key_bytes)
        .map_err(|_| RadrootsClientCryptoError::InvalidKeyId)?
        .to_string();
    if key_id.is_empty() {
        return Err(RadrootsClientCryptoError::InvalidKeyId);
    }
    Ok(Some(RadrootsClientCryptoEnvelope {
        version,
        key_id,
        iv,
        created_at,
        ciphertext,
    }))
}

#[cfg(test)]
mod tests {
    use super::{crypto_envelope_decode, crypto_envelope_encode};
    use crate::crypto::{RadrootsClientCryptoEnvelope, RadrootsClientCryptoError};

    #[test]
    fn encode_decode_roundtrip() -> Result<(), RadrootsClientCryptoError> {
        let envelope = RadrootsClientCryptoEnvelope {
            version: 1,
            key_id: String::from("key"),
            iv: vec![1, 2, 3],
            created_at: 42,
            ciphertext: vec![4, 5, 6],
        };
        let encoded = crypto_envelope_encode(&envelope)?;
        let decoded = crypto_envelope_decode(&encoded)?.ok_or(RadrootsClientCryptoError::InvalidEnvelope)?;
        assert_eq!(decoded.version, envelope.version);
        assert_eq!(decoded.key_id, envelope.key_id);
        assert_eq!(decoded.iv, envelope.iv);
        assert_eq!(decoded.created_at, envelope.created_at);
        assert_eq!(decoded.ciphertext, envelope.ciphertext);
        Ok(())
    }

    #[test]
    fn decode_rejects_wrong_magic() -> Result<(), RadrootsClientCryptoError> {
        let mut blob = vec![0u8; 16];
        blob[0] = 0x00;
        blob[1] = 0x00;
        blob[2] = 0x00;
        blob[3] = 0x00;
        assert!(crypto_envelope_decode(&blob)?.is_none());
        Ok(())
    }

    #[test]
    fn decode_rejects_short_blob() -> Result<(), RadrootsClientCryptoError> {
        let blob = vec![0u8; 4];
        assert!(crypto_envelope_decode(&blob)?.is_none());
        Ok(())
    }

    #[test]
    fn decode_rejects_missing_ciphertext() {
        let envelope = RadrootsClientCryptoEnvelope {
            version: 1,
            key_id: String::from("key"),
            iv: vec![1, 2, 3],
            created_at: 42,
            ciphertext: Vec::new(),
        };
        let encoded = crypto_envelope_encode(&envelope).expect("encode");
        let err = crypto_envelope_decode(&encoded)
            .expect_err("should fail");
        assert_eq!(err, RadrootsClientCryptoError::InvalidEnvelope);
    }

    #[test]
    fn encode_rejects_large_key_id() {
        let envelope = RadrootsClientCryptoEnvelope {
            version: 1,
            key_id: "k".repeat(256),
            iv: vec![1, 2, 3],
            created_at: 42,
            ciphertext: vec![4],
        };
        let err = crypto_envelope_encode(&envelope).expect_err("should fail");
        assert_eq!(err, RadrootsClientCryptoError::InvalidKeyId);
    }

    #[test]
    fn decode_rejects_empty_key_id() {
        let envelope = RadrootsClientCryptoEnvelope {
            version: 1,
            key_id: String::new(),
            iv: vec![1, 2, 3],
            created_at: 42,
            ciphertext: vec![4, 5, 6],
        };
        let encoded = crypto_envelope_encode(&envelope).expect("encode");
        let err = crypto_envelope_decode(&encoded).expect_err("should fail");
        assert_eq!(err, RadrootsClientCryptoError::InvalidKeyId);
    }

    #[test]
    fn decode_rejects_wrong_version() {
        let mut blob = vec![0x52, 0x52, 0x43, 0x45, 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, b'a', 0, 0];
        let err = crypto_envelope_decode(&blob).expect_err("should fail");
        assert_eq!(err, RadrootsClientCryptoError::InvalidEnvelope);
        blob[4] = 1;
    }
}
