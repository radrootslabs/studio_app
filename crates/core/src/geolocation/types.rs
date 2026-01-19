use async_trait::async_trait;

use super::RadrootsClientGeolocationError;

pub type RadrootsClientGeolocationResult<T> =
    Result<T, RadrootsClientGeolocationError>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadrootsClientGeolocationPosition {
    pub lat: f64,
    pub lng: f64,
    pub accuracy: Option<f64>,
    pub altitude: Option<f64>,
    pub altitude_accuracy: Option<f64>,
}

#[async_trait(?Send)]
pub trait RadrootsClientGeolocation {
    async fn current(
        &self,
    ) -> RadrootsClientGeolocationResult<RadrootsClientGeolocationPosition>;
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientGeolocationPosition;

    #[test]
    fn position_tracks_optional_fields() {
        let position = RadrootsClientGeolocationPosition {
            lat: 1.0,
            lng: 2.0,
            accuracy: Some(3.0),
            altitude: None,
            altitude_accuracy: Some(4.0),
        };
        assert_eq!(position.lat, 1.0);
        assert_eq!(position.lng, 2.0);
        assert_eq!(position.accuracy, Some(3.0));
        assert_eq!(position.altitude, None);
        assert_eq!(position.altitude_accuracy, Some(4.0));
    }
}
