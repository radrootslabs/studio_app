#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;

pub fn parse_int(value: &str, fallback: i64) -> i64 {
    value.trim().parse::<i64>().unwrap_or(fallback)
}

pub fn parse_float(value: &str, fallback: f64) -> f64 {
    value.trim().parse::<f64>().unwrap_or(fallback)
}

pub fn num_str<T: ToString>(value: T) -> String {
    value.to_string()
}

pub fn num_interval_range(min: i64, max: i64) -> Result<i64, RadrootsAppUtilsError> {
    if min > max {
        return Err(RadrootsAppUtilsError::InvalidInput);
    }
    if min == max {
        return Ok(min);
    }
    let min_i128 = i128::from(min);
    let max_i128 = i128::from(max);
    let range = max_i128 - min_i128 + 1;
    if range <= 0 || range > i128::from(u64::MAX) {
        return Err(RadrootsAppUtilsError::InvalidInput);
    }
    let range_u64 = range as u64;
    let max_acceptable = u64::MAX - (u64::MAX % range_u64);
    loop {
        let value = random_u64()?;
        if value < max_acceptable {
            let offset = (value % range_u64) as i128;
            return Ok((min_i128 + offset) as i64);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn random_u64() -> Result<u64, RadrootsAppUtilsError> {
    let window = web_sys::window().ok_or(RadrootsAppUtilsError::Unavailable)?;
    let crypto = window
        .crypto()
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let array = js_sys::Uint8Array::new_with_length(8);
    crypto
        .get_random_values_with_u8_array(&array)
        .map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    let mut bytes = [0u8; 8];
    array.copy_to(&mut bytes);
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(not(target_arch = "wasm32"))]
fn random_u64() -> Result<u64, RadrootsAppUtilsError> {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).map_err(|_| RadrootsAppUtilsError::Unavailable)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{num_interval_range, num_str, parse_float, parse_int};
    use crate::error::RadrootsAppUtilsError;

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

    #[test]
    fn num_interval_range_rejects_invalid() {
        let err = num_interval_range(2, 1).unwrap_err();
        assert_eq!(err, RadrootsAppUtilsError::InvalidInput);
    }

    #[test]
    fn num_interval_range_single_value() {
        let value = num_interval_range(4, 4).expect("single value");
        assert_eq!(value, 4);
    }

    #[test]
    fn num_interval_range_within_bounds() {
        let value = num_interval_range(1, 3).expect("range");
        assert!((1..=3).contains(&value));
    }
}
