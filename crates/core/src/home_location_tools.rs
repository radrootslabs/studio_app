use crate::{
    RadrootsAppBackend, RadrootsLocationPoint, RadrootsLocationReverseOptions,
    RadrootsOfflineGeocoderState, RadrootsResolvedLocation, RadrootsReverseLocationLookupResult,
};
use eframe::egui;

const HOME_LOOKUP_RESULT_LIMIT: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HomeLocationLookupResult {
    pub queried_point: RadrootsLocationPoint,
    pub matches: Vec<RadrootsResolvedLocation>,
}

#[derive(Debug, Clone, PartialEq)]
enum HomeLocationLookupState {
    Idle,
    Pending {
        queried_point: RadrootsLocationPoint,
    },
    Ready(HomeLocationLookupResult),
    Failed {
        message: String,
    },
}

impl Default for HomeLocationLookupState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct HomeLocationTools {
    latitude_input: String,
    longitude_input: String,
    lookup_state: HomeLocationLookupState,
}

impl HomeLocationTools {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
        self.latitude_input.clear();
        self.longitude_input.clear();
        self.lookup_state = HomeLocationLookupState::Idle;
    }

    #[cfg(test)]
    pub(crate) fn set_query_inputs(
        &mut self,
        latitude: impl Into<String>,
        longitude: impl Into<String>,
    ) {
        self.latitude_input = latitude.into();
        self.longitude_input = longitude.into();
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

        let resolve_enabled = is_resolve_enabled(offline_geocoder_state) && !self.is_pending();
        if ui
            .add_enabled(
                resolve_enabled,
                egui::Button::new(self.resolve_button_label()),
            )
            .clicked()
        {
            self.begin_resolve_with_backend(backend);
        }

        if let Some(helper_message) = availability_message(offline_geocoder_state) {
            ui.add_space(8.0);
            ui.label(helper_message);
        }

        if let Some(message) = self.status_message() {
            ui.add_space(8.0);
            ui.label(message);
        }

        if let Some(result) = self.lookup_result() {
            ui.add_space(12.0);
            ui.label(format!(
                "Query: {}, {}",
                format_coordinate(result.queried_point.lat),
                format_coordinate(result.queried_point.lng),
            ));
            for resolved in result.matches.iter().take(HOME_LOOKUP_RESULT_LIMIT) {
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

    pub(crate) fn begin_resolve_with_backend(&mut self, backend: &dyn RadrootsAppBackend) {
        self.lookup_state = HomeLocationLookupState::Idle;

        let query_point = match self.parse_query_point() {
            Ok(point) => point,
            Err(message) => {
                self.lookup_state = HomeLocationLookupState::Failed { message };
                return;
            }
        };

        let options = RadrootsLocationReverseOptions {
            limit: HOME_LOOKUP_RESULT_LIMIT,
            ..RadrootsLocationReverseOptions::default()
        };
        match backend.request_reverse_location_lookup(query_point, Some(options)) {
            Ok(()) => {
                self.lookup_state = HomeLocationLookupState::Pending {
                    queried_point: query_point,
                };
            }
            Err(error) => {
                self.lookup_state = HomeLocationLookupState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    pub(crate) fn apply_reverse_lookup_result(
        &mut self,
        result: RadrootsReverseLocationLookupResult,
    ) {
        let queried_point = match self.lookup_state {
            HomeLocationLookupState::Pending { queried_point } => queried_point,
            HomeLocationLookupState::Idle
            | HomeLocationLookupState::Ready(_)
            | HomeLocationLookupState::Failed { .. } => return,
        };

        match result {
            Ok(matches) if matches.is_empty() => {
                self.lookup_state = HomeLocationLookupState::Failed {
                    message: "No offline location matched that coordinate.".to_owned(),
                };
            }
            Ok(matches) => {
                self.lookup_state = HomeLocationLookupState::Ready(HomeLocationLookupResult {
                    queried_point,
                    matches,
                });
            }
            Err(error) => {
                self.lookup_state = HomeLocationLookupState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    pub(crate) fn apply_reverse_lookup_poll_error(&mut self, message: String) {
        self.lookup_state = HomeLocationLookupState::Failed { message };
    }

    pub(crate) fn is_pending(&self) -> bool {
        matches!(self.lookup_state, HomeLocationLookupState::Pending { .. })
    }

    fn parse_query_point(&self) -> Result<RadrootsLocationPoint, String> {
        let lat = parse_coordinate(self.latitude_input.as_str(), "latitude", -90.0, 90.0)?;
        let lng = parse_coordinate(self.longitude_input.as_str(), "longitude", -180.0, 180.0)?;
        Ok(RadrootsLocationPoint { lat, lng })
    }

    fn resolve_button_label(&self) -> &'static str {
        if self.is_pending() {
            "Resolving Offline Location..."
        } else {
            "Resolve Offline Location"
        }
    }

    pub(crate) fn status_message(&self) -> Option<&str> {
        match &self.lookup_state {
            HomeLocationLookupState::Idle | HomeLocationLookupState::Ready(_) => None,
            HomeLocationLookupState::Pending { .. } => Some("Resolving offline location..."),
            HomeLocationLookupState::Failed { message } => Some(message.as_str()),
        }
    }

    pub(crate) fn lookup_result(&self) -> Option<&HomeLocationLookupResult> {
        match &self.lookup_state {
            HomeLocationLookupState::Ready(result) => Some(result),
            HomeLocationLookupState::Idle
            | HomeLocationLookupState::Pending { .. }
            | HomeLocationLookupState::Failed { .. } => None,
        }
    }
}

fn is_resolve_enabled(state: Option<&RadrootsOfflineGeocoderState>) -> bool {
    matches!(state, Some(RadrootsOfflineGeocoderState::Ready))
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
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct ResolveBackend {
        start_response: Result<(), RadrootsLocationResolverError>,
        requested: Rc<
            RefCell<
                Vec<(
                    RadrootsLocationPoint,
                    Option<RadrootsLocationReverseOptions>,
                )>,
            >,
        >,
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

        fn request_reverse_location_lookup(
            &self,
            point: RadrootsLocationPoint,
            options: Option<RadrootsLocationReverseOptions>,
        ) -> Result<(), RadrootsLocationResolverError> {
            self.requested.borrow_mut().push((point, options));
            self.start_response.clone()
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
        tools.lookup_state = HomeLocationLookupState::Failed {
            message: "lookup failed".to_owned(),
        };

        tools.clear();

        assert_eq!(tools.latitude_input, "");
        assert_eq!(tools.longitude_input, "");
        assert_eq!(tools.lookup_state, HomeLocationLookupState::Idle);
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
    fn begin_resolve_with_backend_starts_pending_lookup_with_three_result_limit() {
        let requested = Rc::new(RefCell::new(Vec::new()));
        let backend = ResolveBackend {
            start_response: Ok(()),
            requested: requested.clone(),
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();

        tools.begin_resolve_with_backend(&backend);

        assert_eq!(
            tools.lookup_state,
            HomeLocationLookupState::Pending {
                queried_point: RadrootsLocationPoint {
                    lat: 59.9139,
                    lng: 10.7522,
                },
            }
        );
        assert_eq!(requested.borrow().len(), 1);
        assert_eq!(
            requested.borrow()[0],
            (
                RadrootsLocationPoint {
                    lat: 59.9139,
                    lng: 10.7522,
                },
                Some(RadrootsLocationReverseOptions {
                    limit: HOME_LOOKUP_RESULT_LIMIT,
                    degree_offset: 0.5,
                }),
            )
        );
    }

    #[test]
    fn apply_reverse_lookup_result_stores_matches() {
        let requested = Rc::new(RefCell::new(Vec::new()));
        let backend = ResolveBackend {
            start_response: Ok(()),
            requested,
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();
        tools.begin_resolve_with_backend(&backend);

        tools.apply_reverse_lookup_result(Ok(vec![RadrootsResolvedLocation {
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
        }]));

        assert_eq!(tools.status_message(), None);
        assert_eq!(
            tools
                .lookup_result()
                .as_ref()
                .map(|result| result.matches.len()),
            Some(1)
        );
    }

    #[test]
    fn begin_resolve_with_backend_uses_user_safe_query_error_message() {
        let requested = Rc::new(RefCell::new(Vec::new()));
        let backend = ResolveBackend {
            start_response: Err(RadrootsLocationResolverError::Unavailable),
            requested,
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();

        tools.begin_resolve_with_backend(&backend);

        assert_eq!(
            tools.status_message(),
            Some("Offline location resolution is not available on this device.")
        );
        assert_eq!(tools.lookup_result(), None);
    }

    #[test]
    fn apply_reverse_lookup_result_uses_user_safe_query_error_message() {
        let requested = Rc::new(RefCell::new(Vec::new()));
        let backend = ResolveBackend {
            start_response: Ok(()),
            requested,
        };
        let mut tools = HomeLocationTools::new();
        tools.latitude_input = "59.9139".to_owned();
        tools.longitude_input = "10.7522".to_owned();
        tools.begin_resolve_with_backend(&backend);

        tools.apply_reverse_lookup_result(Err(RadrootsLocationResolverError::Unavailable));

        assert_eq!(
            tools.status_message(),
            Some("Offline location resolution is not available on this device.")
        );
        assert_eq!(tools.lookup_result(), None);
    }
}
