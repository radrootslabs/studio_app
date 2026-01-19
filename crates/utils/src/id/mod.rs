#![forbid(unsafe_code)]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use uuid::Uuid;

pub fn uuidv4() -> String {
    Uuid::new_v4().to_string()
}

pub fn uuidv7() -> String {
    Uuid::now_v7().to_string()
}

pub fn uuidv4_b64url() -> String {
    URL_SAFE_NO_PAD.encode(Uuid::new_v4().as_bytes())
}

pub fn uuidv7_b64url() -> String {
    URL_SAFE_NO_PAD.encode(Uuid::now_v7().as_bytes())
}

pub fn d_tag_create() -> String {
    uuidv7_b64url()
}

#[cfg(test)]
mod tests {
    use super::{d_tag_create, uuidv4, uuidv4_b64url, uuidv7, uuidv7_b64url};

    #[test]
    fn uuidv4_has_expected_length() {
        assert_eq!(uuidv4().len(), 36);
    }

    #[test]
    fn uuidv7_has_expected_length() {
        assert_eq!(uuidv7().len(), 36);
    }

    #[test]
    fn uuidv4_b64url_has_expected_length() {
        assert_eq!(uuidv4_b64url().len(), 22);
    }

    #[test]
    fn uuidv7_b64url_has_expected_length() {
        assert_eq!(uuidv7_b64url().len(), 22);
    }

    #[test]
    fn d_tag_create_uses_uuidv7_b64url() {
        assert_eq!(d_tag_create().len(), 22);
    }
}
