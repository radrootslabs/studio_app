#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppGeolocationPoint {
    pub lat: f64,
    pub lng: f64,
}

pub fn geop_is_valid(point: Option<AppGeolocationPoint>) -> bool {
    if let Some(point) = point {
        !(point.lat == 0.0 && point.lng == 0.0)
    } else {
        false
    }
}

pub fn geop_init() -> AppGeolocationPoint {
    AppGeolocationPoint { lat: 0.0, lng: 0.0 }
}

#[cfg(test)]
mod tests {
    use super::{geop_init, geop_is_valid, AppGeolocationPoint};

    #[test]
    fn geop_is_valid_checks_coords() {
        assert!(!geop_is_valid(None));
        assert!(!geop_is_valid(Some(geop_init())));
        assert!(geop_is_valid(Some(AppGeolocationPoint { lat: 1.0, lng: 1.0 })));
    }
}
