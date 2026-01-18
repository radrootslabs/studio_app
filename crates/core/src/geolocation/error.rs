use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientGeolocationError {
    PermissionDenied,
    LocationUnavailable,
    PositionUnavailable,
    Timeout,
    BlockedByPermissionsPolicy,
    UnknownError,
}

pub type RadrootsClientGeolocationErrorMessage = &'static str;

impl RadrootsClientGeolocationError {
    pub const fn message(self) -> RadrootsClientGeolocationErrorMessage {
        match self {
            RadrootsClientGeolocationError::PermissionDenied => {
                "error.client.geolocation.permission_denied"
            }
            RadrootsClientGeolocationError::LocationUnavailable => {
                "error.client.geolocation.location_unavailable"
            }
            RadrootsClientGeolocationError::PositionUnavailable => {
                "error.client.geolocation.position_unavailable"
            }
            RadrootsClientGeolocationError::Timeout => {
                "error.client.geolocation.timeout"
            }
            RadrootsClientGeolocationError::BlockedByPermissionsPolicy => {
                "error.client.geolocation.blocked_by_permissions_policy"
            }
            RadrootsClientGeolocationError::UnknownError => {
                "error.client.geolocation.unknown_error"
            }
        }
    }
}

impl fmt::Display for RadrootsClientGeolocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientGeolocationError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientGeolocationError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientGeolocationError::PermissionDenied,
                "error.client.geolocation.permission_denied",
            ),
            (
                RadrootsClientGeolocationError::LocationUnavailable,
                "error.client.geolocation.location_unavailable",
            ),
            (
                RadrootsClientGeolocationError::PositionUnavailable,
                "error.client.geolocation.position_unavailable",
            ),
            (
                RadrootsClientGeolocationError::Timeout,
                "error.client.geolocation.timeout",
            ),
            (
                RadrootsClientGeolocationError::BlockedByPermissionsPolicy,
                "error.client.geolocation.blocked_by_permissions_policy",
            ),
            (
                RadrootsClientGeolocationError::UnknownError,
                "error.client.geolocation.unknown_error",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
