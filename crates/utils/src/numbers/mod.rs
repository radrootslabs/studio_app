#![forbid(unsafe_code)]

pub fn parse_int(value: &str, fallback: i64) -> i64 {
    value.trim().parse::<i64>().unwrap_or(fallback)
}

pub fn parse_float(value: &str, fallback: f64) -> f64 {
    value.trim().parse::<f64>().unwrap_or(fallback)
}

pub fn num_str<T: ToString>(value: T) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::{num_str, parse_float, parse_int};

    #[test]
    fn parse_int_returns_fallback_on_invalid() {
        assert_eq!(parse_int("boom", 42), 42);
    }

    #[test]
    fn parse_int_parses_numbers() {
        assert_eq!(parse_int("123", 0), 123);
    }

    #[test]
    fn parse_float_returns_fallback_on_invalid() {
        assert_eq!(parse_float("boom", 1.5), 1.5);
    }

    #[test]
    fn parse_float_parses_numbers() {
        assert_eq!(parse_float("3.5", 0.0), 3.5);
    }

    #[test]
    fn num_str_formats_numbers() {
        assert_eq!(num_str(42), "42");
    }
}
