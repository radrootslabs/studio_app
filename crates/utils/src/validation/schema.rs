#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;
use crate::numbers::{parse_float, parse_int};
use crate::unit::{parse_area_unit, parse_mass_unit, AreaUnit, MassUnit};
use crate::validation::regex::UtilRegex;

pub fn zf_area_unit(value: &str) -> Result<AreaUnit, RadrootsAppUtilsError> {
    parse_area_unit(Some(value)).ok_or(RadrootsAppUtilsError::InvalidInput)
}

pub fn zf_mass_unit(value: &str) -> Result<MassUnit, RadrootsAppUtilsError> {
    parse_mass_unit(Some(value)).ok_or(RadrootsAppUtilsError::InvalidInput)
}

pub fn zf_price_amount(input: &str) -> Result<f64, RadrootsAppUtilsError> {
    let value = parse_float(input, 1.0);
    validate_positive_multiple(value, 0.01)
}

pub fn zf_quantity_amount(input: &str) -> Result<i64, RadrootsAppUtilsError> {
    let value = parse_int(input, 1);
    if value > 0 {
        Ok(value)
    } else {
        Err(RadrootsAppUtilsError::InvalidInput)
    }
}

pub fn zf_price(value: f64) -> Result<f64, RadrootsAppUtilsError> {
    validate_positive_multiple(value, 0.01)
}

pub fn zf_numi_pos(value: i64) -> Result<i64, RadrootsAppUtilsError> {
    if value > 0 {
        Ok(value)
    } else {
        Err(RadrootsAppUtilsError::InvalidInput)
    }
}

pub fn zf_numf_pos(value: f64) -> Result<f64, RadrootsAppUtilsError> {
    if value > 0.0 {
        Ok(value)
    } else {
        Err(RadrootsAppUtilsError::InvalidInput)
    }
}

pub fn zf_email(value: &str) -> Result<&str, RadrootsAppUtilsError> {
    if UtilRegex::email().is_match(value) {
        Ok(value)
    } else {
        Err(RadrootsAppUtilsError::InvalidInput)
    }
}

pub fn zf_username(value: &str) -> Result<&str, RadrootsAppUtilsError> {
    if UtilRegex::profile_name().is_match(value) {
        Ok(value)
    } else {
        Err(RadrootsAppUtilsError::InvalidInput)
    }
}

fn validate_positive_multiple(value: f64, multiple: f64) -> Result<f64, RadrootsAppUtilsError> {
    if value <= 0.0 {
        return Err(RadrootsAppUtilsError::InvalidInput);
    }
    let scaled = value / multiple;
    if (scaled - scaled.round()).abs() > f64::EPSILON {
        return Err(RadrootsAppUtilsError::InvalidInput);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        zf_area_unit, zf_email, zf_mass_unit, zf_numf_pos, zf_numi_pos, zf_price,
        zf_price_amount, zf_quantity_amount, zf_username,
    };

    #[test]
    fn zf_area_unit_accepts_valid() {
        assert!(zf_area_unit("ac").is_ok());
        assert!(zf_area_unit("invalid").is_err());
    }

    #[test]
    fn zf_mass_unit_accepts_valid() {
        assert!(zf_mass_unit("kg").is_ok());
        assert!(zf_mass_unit("invalid").is_err());
    }

    #[test]
    fn zf_price_amount_validates_positive_multiple() {
        assert!(zf_price_amount("1.25").is_ok());
        assert!(zf_price_amount("-1").is_err());
    }

    #[test]
    fn zf_quantity_amount_requires_positive_int() {
        assert!(zf_quantity_amount("2").is_ok());
        assert!(zf_quantity_amount("0").is_err());
    }

    #[test]
    fn zf_price_validates_multiple() {
        assert!(zf_price(1.25).is_ok());
        assert!(zf_price(-1.0).is_err());
    }

    #[test]
    fn zf_num_pos_validates_positive() {
        assert!(zf_numi_pos(1).is_ok());
        assert!(zf_numi_pos(0).is_err());
        assert!(zf_numf_pos(1.0).is_ok());
        assert!(zf_numf_pos(0.0).is_err());
    }

    #[test]
    fn zf_email_validates_format() {
        assert!(zf_email("user@example.com").is_ok());
        assert!(zf_email("bad").is_err());
    }

    #[test]
    fn zf_username_validates_profile_name() {
        assert!(zf_username("user_name").is_ok());
        assert!(zf_username("x").is_err());
    }
}
