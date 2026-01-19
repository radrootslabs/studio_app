#![forbid(unsafe_code)]

use regex::Regex;

pub const SYMBOL_BULLET: &str = "\u{2022}";
pub const SYMBOL_DASH: &str = "\u{2014}";
pub const SYMBOL_UP: &str = "\u{2191}";
pub const SYMBOL_DOWN: &str = "\u{2193}";
pub const SYMBOL_PERCENT: &str = "%";

pub fn fmt_cl(classes: Option<&str>) -> String {
    classes.unwrap_or("").to_string()
}

pub fn value_constrain(regex_charset: &Regex, value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut buf = [0u8; 4];
    for ch in value.chars() {
        let encoded = ch.encode_utf8(&mut buf);
        if regex_charset.is_match(encoded) {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        fmt_cl, value_constrain, SYMBOL_BULLET, SYMBOL_DASH, SYMBOL_DOWN, SYMBOL_PERCENT,
        SYMBOL_UP,
    };

    #[test]
    fn symbols_match_expected_values() {
        assert_eq!(SYMBOL_BULLET, "\u{2022}");
        assert_eq!(SYMBOL_DASH, "\u{2014}");
        assert_eq!(SYMBOL_UP, "\u{2191}");
        assert_eq!(SYMBOL_DOWN, "\u{2193}");
        assert_eq!(SYMBOL_PERCENT, "%");
    }

    #[test]
    fn fmt_cl_handles_none() {
        assert_eq!(fmt_cl(None), "");
        assert_eq!(fmt_cl(Some("a b")), "a b");
    }

    #[test]
    fn value_constrain_filters_chars() {
        let regex = regex::Regex::new("[0-9]").expect("regex");
        assert_eq!(value_constrain(&regex, "a1b2c"), "12");
    }
}
