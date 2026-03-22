#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadrootsLocationPoint {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadrootsLocationReverseOptions {
    pub limit: usize,
    pub degree_offset: f64,
}

impl Default for RadrootsLocationReverseOptions {
    fn default() -> Self {
        Self {
            limit: 1,
            degree_offset: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadrootsResolvedLocation {
    pub id: i64,
    pub name: String,
    pub admin1_id: Option<i64>,
    pub admin1_name: Option<String>,
    pub country_id: String,
    pub country_name: Option<String>,
    pub point: RadrootsLocationPoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadrootsLocationCountry {
    pub country_id: String,
    pub country_name: Option<String>,
    pub center: RadrootsLocationPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsLocationResolverError {
    Unsupported,
    Initializing,
    Unavailable,
    CountryCenterNotFound { country_id: String },
    QueryFailed { message: String },
}

impl RadrootsLocationResolverError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Initializing => "initializing",
            Self::Unavailable => "unavailable",
            Self::CountryCenterNotFound { .. } => "country_center_not_found",
            Self::QueryFailed { .. } => "query_failed",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Unsupported => "Offline location resolution is not available on this platform.",
            Self::Initializing => {
                "Offline location resolution is still initializing on this device."
            }
            Self::Unavailable => "Offline location resolution is not available on this device.",
            Self::CountryCenterNotFound { .. } => "The requested country center is not available.",
            Self::QueryFailed { .. } => "The offline location query could not be completed.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_options_default_matches_geocoder_defaults() {
        let options = RadrootsLocationReverseOptions::default();

        assert_eq!(options.limit, 1);
        assert_eq!(options.degree_offset, 0.5);
    }

    #[test]
    fn location_resolver_error_codes_are_stable() {
        assert_eq!(
            RadrootsLocationResolverError::Unsupported.code(),
            "unsupported"
        );
        assert_eq!(
            RadrootsLocationResolverError::Initializing.code(),
            "initializing"
        );
        assert_eq!(
            RadrootsLocationResolverError::Unavailable.code(),
            "unavailable"
        );
        assert_eq!(
            RadrootsLocationResolverError::CountryCenterNotFound {
                country_id: "US".to_owned(),
            }
            .code(),
            "country_center_not_found"
        );
        assert_eq!(
            RadrootsLocationResolverError::QueryFailed {
                message: "sqlite failed".to_owned(),
            }
            .code(),
            "query_failed"
        );
    }
}
