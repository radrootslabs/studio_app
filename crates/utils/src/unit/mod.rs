#![forbid(unsafe_code)]

use crate::error::RadrootsAppUtilsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaUnit {
    Ac,
    Ha,
    Ft2,
    M2,
}

impl AreaUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            AreaUnit::Ac => "ac",
            AreaUnit::Ha => "ha",
            AreaUnit::Ft2 => "ft2",
            AreaUnit::M2 => "m2",
        }
    }
}

pub const AREA_UNITS: [AreaUnit; 4] = [AreaUnit::Ac, AreaUnit::Ha, AreaUnit::Ft2, AreaUnit::M2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassUnit {
    Kg,
    Lb,
    G,
}

impl MassUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            MassUnit::Kg => "kg",
            MassUnit::Lb => "lb",
            MassUnit::G => "g",
        }
    }
}

pub const MASS_UNITS: [MassUnit; 3] = [MassUnit::Kg, MassUnit::Lb, MassUnit::G];

pub fn parse_mass_unit(val: Option<&str>) -> Option<MassUnit> {
    match val {
        Some("kg") => Some(MassUnit::Kg),
        Some("lb") => Some(MassUnit::Lb),
        Some("g") => Some(MassUnit::G),
        _ => None,
    }
}

pub fn parse_mass_unit_default(val: Option<&str>) -> MassUnit {
    parse_mass_unit(val).unwrap_or(MassUnit::Kg)
}

pub fn mass_to_g(value: f64, unit: &str) -> Result<f64, RadrootsAppUtilsError> {
    let mass_unit = parse_mass_unit(Some(unit)).ok_or(RadrootsAppUtilsError::InvalidInput)?;
    let grams = match mass_unit {
        MassUnit::Kg => value * 1000.0,
        MassUnit::Lb => value * 453.592,
        MassUnit::G => value,
    };
    Ok(grams)
}

pub fn parse_area_unit(val: Option<&str>) -> Option<AreaUnit> {
    match val {
        Some("ac") => Some(AreaUnit::Ac),
        Some("ha") => Some(AreaUnit::Ha),
        Some("ft2") => Some(AreaUnit::Ft2),
        Some("m2") => Some(AreaUnit::M2),
        _ => None,
    }
}

pub fn parse_area_unit_default(val: Option<&str>) -> AreaUnit {
    parse_area_unit(val).unwrap_or(AreaUnit::Ac)
}

#[cfg(test)]
mod tests {
    use super::{
        mass_to_g, parse_area_unit, parse_area_unit_default, parse_mass_unit,
        parse_mass_unit_default, AreaUnit, MassUnit, AREA_UNITS, MASS_UNITS,
    };

    #[test]
    fn area_units_are_sorted() {
        assert_eq!(AREA_UNITS.len(), 4);
        assert_eq!(AREA_UNITS[0], AreaUnit::Ac);
        assert_eq!(AREA_UNITS[1], AreaUnit::Ha);
        assert_eq!(AREA_UNITS[2], AreaUnit::Ft2);
        assert_eq!(AREA_UNITS[3], AreaUnit::M2);
    }

    #[test]
    fn mass_units_are_sorted() {
        assert_eq!(MASS_UNITS.len(), 3);
        assert_eq!(MASS_UNITS[0], MassUnit::Kg);
        assert_eq!(MASS_UNITS[1], MassUnit::Lb);
        assert_eq!(MASS_UNITS[2], MassUnit::G);
    }

    #[test]
    fn parse_mass_units() {
        assert_eq!(parse_mass_unit(Some("kg")), Some(MassUnit::Kg));
        assert_eq!(parse_mass_unit(Some("lb")), Some(MassUnit::Lb));
        assert_eq!(parse_mass_unit(Some("g")), Some(MassUnit::G));
        assert_eq!(parse_mass_unit(Some("other")), None);
    }

    #[test]
    fn parse_mass_unit_defaults_to_kg() {
        assert_eq!(parse_mass_unit_default(None), MassUnit::Kg);
    }

    #[test]
    fn parse_area_units() {
        assert_eq!(parse_area_unit(Some("ac")), Some(AreaUnit::Ac));
        assert_eq!(parse_area_unit(Some("ha")), Some(AreaUnit::Ha));
        assert_eq!(parse_area_unit(Some("ft2")), Some(AreaUnit::Ft2));
        assert_eq!(parse_area_unit(Some("m2")), Some(AreaUnit::M2));
        assert_eq!(parse_area_unit(Some("other")), None);
    }

    #[test]
    fn parse_area_unit_defaults_to_ac() {
        assert_eq!(parse_area_unit_default(None), AreaUnit::Ac);
    }

    #[test]
    fn mass_to_g_handles_units() {
        let grams = mass_to_g(2.0, "kg").expect("kg");
        assert_eq!(grams, 2000.0);
        let grams = mass_to_g(2.0, "g").expect("g");
        assert_eq!(grams, 2.0);
    }
}
