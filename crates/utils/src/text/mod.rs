#![forbid(unsafe_code)]

pub const ROOT_SYMBOL: &str = "»`,-";

pub fn text_enc(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

pub fn text_dec(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

#[cfg(test)]
mod tests {
    use super::{text_dec, text_enc, ROOT_SYMBOL};

    #[test]
    fn root_symbol_matches_spec() {
        assert_eq!(ROOT_SYMBOL, "»`,-");
    }

    #[test]
    fn text_enc_dec_roundtrip() {
        let encoded = text_enc("radroots");
        assert_eq!(encoded, b"radroots");
        let decoded = text_dec(&encoded);
        assert_eq!(decoded, "radroots");
    }
}
