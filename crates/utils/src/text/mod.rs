#![forbid(unsafe_code)]

pub const ROOT_SYMBOL: &str = "»`,-";

pub fn text_enc(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

pub fn text_dec(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

pub fn str_cap(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut output = first.to_uppercase().collect::<String>();
    output.push_str(chars.as_str());
    output
}

pub fn str_cap_words(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let mut words = Vec::new();
    for word in value.split(' ') {
        if word.is_empty() {
            continue;
        }
        let capped = str_cap(Some(word));
        if !capped.is_empty() {
            words.push(capped);
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::{str_cap, str_cap_words, text_dec, text_enc, ROOT_SYMBOL};

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

    #[test]
    fn str_cap_handles_none() {
        assert_eq!(str_cap(None), "");
    }

    #[test]
    fn str_cap_uppercases_first_letter() {
        assert_eq!(str_cap(Some("radroots")), "Radroots");
    }

    #[test]
    fn str_cap_words_handles_none() {
        assert_eq!(str_cap_words(None), "");
    }

    #[test]
    fn str_cap_words_caps_each_word() {
        assert_eq!(str_cap_words(Some("rad roots")), "Rad Roots");
    }

    #[test]
    fn str_cap_words_skips_empty_words() {
        assert_eq!(str_cap_words(Some("rad   roots")), "Rad Roots");
    }
}
