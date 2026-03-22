use crate::{
    RadrootsAppBackend, RadrootsLocationPoint, RadrootsLocationReverseOptions,
    RadrootsOfflineGeocoderState, RadrootsResolvedLocation,
};
use eframe::egui;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HomeLocationLookupResult {
    pub queried_point: RadrootsLocationPoint,
    pub matches: Vec<RadrootsResolvedLocation>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct HomeLocationTools {
    latitude_input: String,
    longitude_input: String,
    status_message: Option<String>,
    lookup_result: Option<HomeLocationLookupResult>,
}

impl HomeLocationTools {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
        self.latitude_input.clear();
        self.longitude_input.clear();
        self.status_message = None;
        self.lookup_result = None;
    }

    pub(crate) fn render(
        &mut self,
        ui: &mut egui::Ui,
        backend: &dyn RadrootsAppBackend,
        offline_geocoder_state: Option<&RadrootsOfflineGeocoderState>,
    ) {
        ui.add_space(20.0);
        ui.label("Offline location lookup");
        ui.add_space(8.0);
        ui.label("Resolve a latitude and longitude pair using the on-device geocoder.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Latitude");
            ui.add(
                egui::TextEdit::singleline(&mut self.latitude_input)
                    .hint_text("12.34")
                    .desired_width(140.0),
            );
            ui.add_space(8.0);
            ui.label("Longitude");
            ui.add(
                egui::TextEdit::singleline(&mut self.longitude_input)
                    .hint_text("-56.78")
                    .desired_width(140.0),
            );
        });
        ui.add_space(8.0);

        let resolve_enabled = is_resolve_enabled(offline_geocoder_state);
        if ui
            .add_enabled(
                resolve_enabled,
                egui::Button::new("Resolve Offline Location"),
            )
            .clicked()
        {
            self.resolve_with_backend(backend);
        }

        if let Some(helper_message) = availability_message(offline_geocoder_state) {
            ui.add_space(8.0);
            ui.label(helper_message);
        }

        if let Some(message) = &self.status_message {
            ui.add_space(8.0);
            ui.label(message);
        }

        if let Some(result) = &self.lookup_result {
            ui.add_space(12.0);
            ui.label(format!(
                "Query: {}, {}",
                format_coordinate(result.queried_point.lat),
                format_coordinate(result.queried_point.lng),
            ));
            for resolved in result.matches.iter().take(3) {
                ui.add_space(8.0);
                ui.label(resolved.name.as_str());
                if let Some(admin1_name) = &resolved.admin1_name {
                    ui.label(admin1_name.as_str());
                }
                if let Some(country_name) = &resolved.country_name {
                    ui.label(country_name.as_str());
                } else {
                    ui.label(resolved.country_id.as_str());
                }
                ui.monospace(format!(
                    "{}, {}",
                    format_coordinate(resolved.point.lat),
                    format_coordinate(resolved.point.lng),
                ));
            }
        }
    }

    fn resolve_with_backend(&mut self, backend: &dyn RadrootsAppBackend) {
        self.status_message = None;
        self.lookup_result = None;

        let query_point = match self.parse_query_point() {
            Ok(point) => point,
            Err(message) => {
                self.status_message = Some(message);
                return;
            }
        };

        match backend.reverse_location(query_point, Some(RadrootsLocationReverseOptions::default()))
        {
            Ok(matches) if matches.is_empty() => {
                self.status_message =
                    Some("No offline location matched that coordinate.".to_owned());
            }
            Ok(matches) => {
                self.lookup_result = Some(HomeLocationLookupResult {
                    queried_point: query_point,
                    matches,
                });
            }
            Err(error) => {
                self.status_message = Some(error.user_message().to_owned());
            }
        }
    }

    fn parse_query_point(&self) -> Result<RadrootsLocationPoint, String> {
        let lat = parse_coordinate(self.latitude_input.as_str(), "latitude", -90.0, 90.0)?;
        let lng = parse_coordinate(self.longitude_input.as_str(), "longitude", -180.0, 180.0)?;
        Ok(RadrootsLocationPoint { lat, lng })
    }
}

fn is_resolve_enabled(state: Option<&RadrootsOfflineGeocoderState>) -> bool {
    matches!(state, None | Some(RadrootsOfflineGeocoderState::Ready))
}

fn availability_message(state: Option<&RadrootsOfflineGeocoderState>) -> Option<&str> {
    match state {
        Some(RadrootsOfflineGeocoderState::Initializing) => {
            Some("Offline location resolution is still initializing on this device.")
        }
        Some(RadrootsOfflineGeocoderState::Unavailable { .. }) => {
            state.and_then(RadrootsOfflineGeocoderState::user_message)
        }
        Some(RadrootsOfflineGeocoderState::Ready) | None => None,
    }
}

fn parse_coordinate(raw: &str, label: &str, min: f64, max: f64) -> Result<f64, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} is required"));
    }

    let value = trimmed
        .parse::<f64>()
        .map_err(|_| format!("{label} must be a valid number"))?;
    if !value.is_finite() {
        return Err(format!("{label} must be a finite number"));
    }
    if value < min || value > max {
        return Err(format!("{label} must be between {min} and {max}"));
    }

    Ok(value)
}

fn format_coordinate(value: f64) -> String {
    format!("{value:.4}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        IdentityGateState, RadrootsLocationCountry, RadrootsLocationResolverError, SetupActionState,
    };

    #[derive(Clone)]
    struct ResolveBackend {
        response: Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError>,
    }

    impl RadrootsAppBackend for ResolveBackend {
        fn load_identity_state(&self) -> Result<IdentityGateState, String> {
            Ok(IdentityGateState::Missing)
        }

        fn setup_action_state(&self) -> SetupActionState {
            SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: true,
                pending: false,
            }
        }

        fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
            Ok(None)
        }

        fn reverse_location(
            &self,
            _point: RadrootsLocationPoint,
            _options: Option<RadrootsLocationReverseOptions>,
        ) -> Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError> {
            self.response.clone()
        }

        fn list_location_countries(
            &self,
        ) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
            Err(RadrootsLocationResolverError::Unsupported)
        }

        fn location_country_center(
            &self,
            _country_id: &str,
        ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    #[test]
    fn clear_resets_inputs_and_feedback() {
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "10.5".to_owned();
        tools.longitude_input = "20.5".to_owned();
        tools.status_message = Some("lookup failed".to_owned());
        tools.lookup_result = Some(HomeLocationLookupResult {
            queried_point: RadrootsLocationPoint {
                lat: 10.5,
                lng: 20.5,
            },
            matches: Vec::new(),
        });

        tools.clear();

        assert_eq!(tools.latitude_input, "");
        assert_eq!(tools.longitude_input, "");
        assert_eq!(tools.status_message, None);
        assert_eq!(tools.lookup_result, None);
    }

    #[test]
    fn parse_query_point_accepts_trimmed_valid_coordinates() {
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = " 12.34 ".to_owned();
        tools.longitude_input = "\n-56.78\t".to_owned();

        assert_eq!(
            tools.parse_query_point(),
            Ok(RadrootsLocationPoint {
                lat: 12.34,
                lng: -56.78,
            })
        );
    }

    #[test]
    fn parse_query_point_rejects_missing_and_out_of_range_values() {
        let mut tools = HomeLocationTools::new();

        assert_eq!(
            tools.parse_query_point(),
            Err("latitude is required".to_owned())
        );

        tools.latitude_input = "91".to_owned();
        tools.longitude_input = "20".to_owned();
        assert_eq!(
            tools.parse_query_point(),
            Err("latitude must be between -90 and 90".to_owned())
        );

        tools.latitude_input = "10".to_owned();
        tools.longitude_input = "-181".to_owned();
        assert_eq!(
            tools.parse_query_point(),
            Err("longitude must be between -180 and 180".to_owned())
        );
    }

    #[test]
    fn availability_message_matches_geocoder_state() {
        assert_eq!(
            availability_message(Some(&RadrootsOfflineGeocoderState::Initializing)),
            Some("Offline location resolution is still initializing on this device.")
        );
        assert_eq!(
            availability_message(Some(&RadrootsOfflineGeocoderState::Ready)),
            None
        );
    }

    #[test]
    fn resolve_with_backend_stores_matches() {
        let backend = ResolveBackend {
            response: Ok(vec![RadrootsResolvedLocation {
                id: 1,
                name: "Oslo".to_owned(),
                admin1_id: Some(2),
                admin1_name: Some("Oslo".to_owned()),
                country_id: "NO".to_owned(),
                country_name: Some("Norway".to_owned()),
                point: RadrootsLocationPoint {
                    lat: 59.9139,
                    lng: 10.7522,
                },
            }]),
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();

        tools.resolve_with_backend(&backend);

        assert_eq!(tools.status_message, None);
        assert_eq!(
            tools
                .lookup_result
                .as_ref()
                .map(|result| result.matches.len()),
            Some(1)
        );
    }

    #[test]
    fn resolve_with_backend_uses_user_safe_query_error_message() {
        let backend = ResolveBackend {
            response: Err(RadrootsLocationResolverError::Unavailable),
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();

        tools.resolve_with_backend(&backend);

        assert_eq!(
            tools.status_message.as_deref(),
            Some("Offline location resolution is not available on this device.")
        );
        assert_eq!(tools.lookup_result, None);
    }
}
