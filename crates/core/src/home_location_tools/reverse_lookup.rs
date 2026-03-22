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
pub(super) struct ReverseLookupTools {
    latitude_input: String,
    longitude_input: String,
    lookup_state: HomeLocationLookupState,
}

impl ReverseLookupTools {
    pub(super) fn clear(&mut self) {
        self.latitude_input.clear();
        self.longitude_input.clear();
        self.lookup_state = HomeLocationLookupState::Idle;
    }

    #[cfg(test)]
    pub(super) fn set_query_inputs(
        &mut self,
        latitude: impl Into<String>,
        longitude: impl Into<String>,
    ) {
        self.latitude_input = latitude.into();
        self.longitude_input = longitude.into();
    }

    pub(super) fn render(
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

    pub(super) fn begin_resolve_with_backend(&mut self, backend: &dyn RadrootsAppBackend) {
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

    pub(super) fn apply_result(&mut self, result: RadrootsReverseLocationLookupResult) {
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

    pub(super) fn apply_poll_error(&mut self, message: String) {
        self.lookup_state = HomeLocationLookupState::Failed { message };
    }

    pub(super) fn is_pending(&self) -> bool {
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

    pub(super) fn status_message(&self) -> Option<&str> {
        match &self.lookup_state {
            HomeLocationLookupState::Idle | HomeLocationLookupState::Ready(_) => None,
            HomeLocationLookupState::Pending { .. } => Some("Resolving offline location..."),
            HomeLocationLookupState::Failed { message } => Some(message.as_str()),
        }
    }

    pub(super) fn lookup_result(&self) -> Option<&HomeLocationLookupResult> {
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

    fn resolve_backend(
        start_response: Result<(), RadrootsLocationResolverError>,
    ) -> (
        ResolveBackend,
        Rc<
            RefCell<
                Vec<(
                    RadrootsLocationPoint,
                    Option<RadrootsLocationReverseOptions>,
                )>,
            >,
        >,
    ) {
        let requested = Rc::new(RefCell::new(Vec::new()));
        (
            ResolveBackend {
                start_response,
                requested: requested.clone(),
            },
            requested,
        )
    }

    #[test]
    fn begin_resolve_requests_three_results() {
        let (backend, requested) = resolve_backend(Ok(()));
        let mut tools = ReverseLookupTools::default();
        tools.set_query_inputs("12.5", "-42.25");

        tools.begin_resolve_with_backend(&backend);

        let requested = requested.borrow();
        assert_eq!(requested.len(), 1);
        assert_eq!(
            requested[0].0,
            RadrootsLocationPoint {
                lat: 12.5,
                lng: -42.25,
            }
        );
        assert_eq!(
            requested[0].1,
            Some(RadrootsLocationReverseOptions {
                limit: 3,
                ..RadrootsLocationReverseOptions::default()
            })
        );
        assert!(tools.is_pending());
    }

    #[test]
    fn begin_resolve_rejects_out_of_range_coordinates() {
        let (backend, requested) = resolve_backend(Ok(()));
        let mut tools = ReverseLookupTools::default();
        tools.set_query_inputs("200", "10");

        tools.begin_resolve_with_backend(&backend);

        assert!(requested.borrow().is_empty());
        assert_eq!(
            tools.status_message(),
            Some("latitude must be between -90 and 90")
        );
        assert!(!tools.is_pending());
    }

    #[test]
    fn apply_result_keeps_up_to_three_matches_available() {
        let mut tools = ReverseLookupTools::default();
        tools.lookup_state = HomeLocationLookupState::Pending {
            queried_point: RadrootsLocationPoint {
                lat: 1.25,
                lng: -2.5,
            },
        };

        tools.apply_result(Ok(vec![
            sample_result(1, "one"),
            sample_result(2, "two"),
            sample_result(3, "three"),
        ]));

        let result = tools.lookup_result().expect("lookup result");
        assert_eq!(result.matches.len(), 3);
        assert_eq!(result.matches[0].name, "one");
        assert_eq!(result.matches[2].name, "three");
    }

    #[test]
    fn apply_poll_error_sets_failed_status() {
        let mut tools = ReverseLookupTools::default();

        tools.apply_poll_error("background worker failed".to_owned());

        assert_eq!(tools.status_message(), Some("background worker failed"));
    }

    fn sample_result(id: i64, name: &str) -> RadrootsResolvedLocation {
        RadrootsResolvedLocation {
            id,
            name: name.to_owned(),
            admin1_id: None,
            admin1_name: Some("state".to_owned()),
            country_id: "US".to_owned(),
            country_name: Some("United States".to_owned()),
            point: RadrootsLocationPoint { lat: 1.0, lng: 2.0 },
        }
    }
}
