use crate::{
    RadrootsAppBackend, RadrootsLocationCountry, RadrootsLocationCountryCenterLookupResult,
    RadrootsLocationCountryListResult, RadrootsLocationPoint, RadrootsOfflineGeocoderState,
};
use eframe::egui;

#[derive(Debug, Clone, PartialEq)]
enum CountryListState {
    Idle,
    Pending,
    Ready(Vec<RadrootsLocationCountry>),
    Failed { message: String },
}

impl Default for CountryListState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CountryCenterLookupResult {
    country_id: String,
    country_name: Option<String>,
    center: RadrootsLocationPoint,
}

#[derive(Debug, Clone, PartialEq)]
enum CountryCenterState {
    Idle,
    Pending { country_id: String },
    Ready(CountryCenterLookupResult),
    Failed { message: String },
}

impl Default for CountryCenterState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct CountryLookupTools {
    countries: CountryListState,
    selected_country_id: Option<String>,
    center: CountryCenterState,
}

impl CountryLookupTools {
    pub(super) fn clear(&mut self) {
        self.countries = CountryListState::Idle;
        self.selected_country_id = None;
        self.center = CountryCenterState::Idle;
    }

    pub(super) fn render(
        &mut self,
        ui: &mut egui::Ui,
        backend: &dyn RadrootsAppBackend,
        offline_geocoder_state: Option<&RadrootsOfflineGeocoderState>,
    ) {
        ui.add_space(20.0);
        ui.label("Offline country lookup");
        ui.add_space(8.0);
        ui.label("Load country data and resolve a country center using the on-device geocoder.");
        ui.add_space(8.0);

        let load_enabled =
            is_country_action_enabled(offline_geocoder_state) && !self.is_list_pending();
        if ui
            .add_enabled(load_enabled, egui::Button::new(self.load_button_label()))
            .clicked()
        {
            self.begin_load_countries(backend);
        }

        if let Some(helper_message) = availability_message(offline_geocoder_state) {
            ui.add_space(8.0);
            ui.label(helper_message);
        }

        if let Some(message) = self.list_status_message() {
            ui.add_space(8.0);
            ui.label(message);
        }

        if let Some(countries) = self.ready_countries().cloned() {
            ui.add_space(8.0);
            let selected_country_id = &mut self.selected_country_id;
            let selected_text =
                country_label_for_id(countries.as_slice(), selected_country_id.as_deref());
            egui::ComboBox::from_label("Country")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for country in countries.as_slice() {
                        let response = ui.selectable_value(
                            selected_country_id,
                            Some(country.country_id.clone()),
                            country_label(country),
                        );
                        if response.clicked() {
                            self.center = CountryCenterState::Idle;
                        }
                    }
                });

            ui.add_space(8.0);
            let center_enabled =
                is_country_action_enabled(offline_geocoder_state) && !self.is_center_pending();
            if ui
                .add_enabled(
                    center_enabled,
                    egui::Button::new(self.center_button_label()),
                )
                .clicked()
            {
                self.begin_resolve_country_center(backend);
            }
        }

        if let Some(message) = self.center_status_message() {
            ui.add_space(8.0);
            ui.label(message);
        }

        if let Some(result) = self.center_result() {
            ui.add_space(12.0);
            ui.label(
                result
                    .country_name
                    .as_deref()
                    .unwrap_or(result.country_id.as_str()),
            );
            ui.monospace(format!(
                "{}, {}",
                format_coordinate(result.center.lat),
                format_coordinate(result.center.lng),
            ));
        }
    }

    pub(super) fn apply_list_result(&mut self, result: RadrootsLocationCountryListResult) {
        match result {
            Ok(countries) if countries.is_empty() => {
                self.countries = CountryListState::Failed {
                    message: "No offline countries are available.".to_owned(),
                };
                self.selected_country_id = None;
                self.center = CountryCenterState::Idle;
            }
            Ok(countries) => {
                self.selected_country_id = selected_country_id_after_refresh(
                    self.selected_country_id.as_deref(),
                    countries.as_slice(),
                );
                self.countries = CountryListState::Ready(countries);
                self.center = CountryCenterState::Idle;
            }
            Err(error) => {
                self.countries = CountryListState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    pub(super) fn apply_list_poll_error(&mut self, message: String) {
        self.countries = CountryListState::Failed { message };
    }

    pub(super) fn apply_center_result(
        &mut self,
        result: RadrootsLocationCountryCenterLookupResult,
    ) {
        let country_id = match &self.center {
            CountryCenterState::Pending { country_id } => country_id.clone(),
            CountryCenterState::Idle
            | CountryCenterState::Ready(_)
            | CountryCenterState::Failed { .. } => return,
        };

        match result {
            Ok(center) => {
                self.center = CountryCenterState::Ready(CountryCenterLookupResult {
                    country_name: self.country_name_for_id(country_id.as_str()),
                    country_id,
                    center,
                });
            }
            Err(error) => {
                self.center = CountryCenterState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    pub(super) fn apply_center_poll_error(&mut self, message: String) {
        self.center = CountryCenterState::Failed { message };
    }

    pub(super) fn is_pending(&self) -> bool {
        self.is_list_pending() || self.is_center_pending()
    }

    fn begin_load_countries(&mut self, backend: &dyn RadrootsAppBackend) {
        self.countries = CountryListState::Idle;
        self.center = CountryCenterState::Idle;

        match backend.request_location_country_list() {
            Ok(()) => {
                self.countries = CountryListState::Pending;
            }
            Err(error) => {
                self.countries = CountryListState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    fn begin_resolve_country_center(&mut self, backend: &dyn RadrootsAppBackend) {
        let Some(country_id) = self.selected_country_id.clone() else {
            self.center = CountryCenterState::Failed {
                message: "Select a country first.".to_owned(),
            };
            return;
        };

        match backend.request_location_country_center_lookup(country_id.as_str()) {
            Ok(()) => {
                self.center = CountryCenterState::Pending { country_id };
            }
            Err(error) => {
                self.center = CountryCenterState::Failed {
                    message: error.user_message().to_owned(),
                };
            }
        }
    }

    fn is_list_pending(&self) -> bool {
        matches!(self.countries, CountryListState::Pending)
    }

    fn is_center_pending(&self) -> bool {
        matches!(self.center, CountryCenterState::Pending { .. })
    }

    fn load_button_label(&self) -> &'static str {
        if self.is_list_pending() {
            "Loading Offline Countries..."
        } else {
            "Load Offline Countries"
        }
    }

    fn center_button_label(&self) -> &'static str {
        if self.is_center_pending() {
            "Resolving Country Center..."
        } else {
            "Resolve Country Center"
        }
    }

    fn list_status_message(&self) -> Option<&str> {
        match &self.countries {
            CountryListState::Idle | CountryListState::Ready(_) => None,
            CountryListState::Pending => Some("Loading offline countries..."),
            CountryListState::Failed { message } => Some(message.as_str()),
        }
    }

    fn center_status_message(&self) -> Option<&str> {
        match &self.center {
            CountryCenterState::Idle | CountryCenterState::Ready(_) => None,
            CountryCenterState::Pending { .. } => Some("Resolving country center..."),
            CountryCenterState::Failed { message } => Some(message.as_str()),
        }
    }

    fn ready_countries(&self) -> Option<&Vec<RadrootsLocationCountry>> {
        match &self.countries {
            CountryListState::Ready(countries) => Some(countries),
            CountryListState::Idle
            | CountryListState::Pending
            | CountryListState::Failed { .. } => None,
        }
    }

    fn center_result(&self) -> Option<&CountryCenterLookupResult> {
        match &self.center {
            CountryCenterState::Ready(result) => Some(result),
            CountryCenterState::Idle
            | CountryCenterState::Pending { .. }
            | CountryCenterState::Failed { .. } => None,
        }
    }

    fn country_name_for_id(&self, country_id: &str) -> Option<String> {
        self.ready_countries()
            .and_then(|countries| {
                countries
                    .iter()
                    .find(|country| country.country_id == country_id)
                    .map(|country| country.country_name.clone())
            })
            .flatten()
    }
}

fn is_country_action_enabled(state: Option<&RadrootsOfflineGeocoderState>) -> bool {
    matches!(state, Some(RadrootsOfflineGeocoderState::Ready))
}

fn availability_message(state: Option<&RadrootsOfflineGeocoderState>) -> Option<&str> {
    match state {
        Some(RadrootsOfflineGeocoderState::Initializing) => {
            Some("Offline country lookup is still initializing on this device.")
        }
        Some(RadrootsOfflineGeocoderState::Unavailable { .. }) => {
            state.and_then(RadrootsOfflineGeocoderState::user_message)
        }
        Some(RadrootsOfflineGeocoderState::Ready) | None => None,
    }
}

fn selected_country_id_after_refresh(
    selected_country_id: Option<&str>,
    countries: &[RadrootsLocationCountry],
) -> Option<String> {
    if let Some(selected_country_id) = selected_country_id {
        if countries
            .iter()
            .any(|country| country.country_id == selected_country_id)
        {
            return Some(selected_country_id.to_owned());
        }
    }

    countries.first().map(|country| country.country_id.clone())
}

fn country_label(country: &RadrootsLocationCountry) -> String {
    country
        .country_name
        .clone()
        .unwrap_or_else(|| country.country_id.clone())
}

fn country_label_for_id(countries: &[RadrootsLocationCountry], country_id: Option<&str>) -> String {
    country_id
        .and_then(|country_id| {
            countries
                .iter()
                .find(|country| country.country_id == country_id)
                .map(country_label)
        })
        .unwrap_or_else(|| "Select a country".to_owned())
}

fn format_coordinate(value: f64) -> String {
    format!("{value:.4}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        IdentityGateState, RadrootsLocationResolverError, RadrootsReverseLocationLookupResult,
        SetupActionState,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    #[derive(Clone)]
    struct CountryBackend {
        list_request: Rc<RefCell<VecDeque<Result<(), RadrootsLocationResolverError>>>>,
        center_request: Rc<RefCell<VecDeque<Result<(), RadrootsLocationResolverError>>>>,
        requested_country_ids: Rc<RefCell<Vec<String>>>,
    }

    impl RadrootsAppBackend for CountryBackend {
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
            _point: RadrootsLocationPoint,
            _options: Option<crate::RadrootsLocationReverseOptions>,
        ) -> Result<(), RadrootsLocationResolverError> {
            Err(RadrootsLocationResolverError::Unsupported)
        }

        fn poll_reverse_location_lookup_result(
            &self,
        ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
            Ok(None)
        }

        fn request_location_country_list(&self) -> Result<(), RadrootsLocationResolverError> {
            self.list_request.borrow_mut().pop_front().unwrap_or(Ok(()))
        }

        fn request_location_country_center_lookup(
            &self,
            country_id: &str,
        ) -> Result<(), RadrootsLocationResolverError> {
            self.requested_country_ids
                .borrow_mut()
                .push(country_id.to_owned());
            self.center_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(()))
        }
    }

    fn country_backend(
        list_request: Vec<Result<(), RadrootsLocationResolverError>>,
        center_request: Vec<Result<(), RadrootsLocationResolverError>>,
    ) -> (CountryBackend, Rc<RefCell<Vec<String>>>) {
        let requested_country_ids = Rc::new(RefCell::new(Vec::new()));
        (
            CountryBackend {
                list_request: Rc::new(RefCell::new(list_request.into())),
                center_request: Rc::new(RefCell::new(center_request.into())),
                requested_country_ids: requested_country_ids.clone(),
            },
            requested_country_ids,
        )
    }

    #[test]
    fn begin_load_countries_enters_pending_state() {
        let (backend, _) = country_backend(vec![Ok(())], Vec::new());
        let mut tools = CountryLookupTools::default();

        tools.begin_load_countries(&backend);

        assert_eq!(
            tools.list_status_message(),
            Some("Loading offline countries...")
        );
        assert!(tools.is_pending());
    }

    #[test]
    fn apply_list_result_selects_first_country() {
        let mut tools = CountryLookupTools::default();

        tools.apply_list_result(Ok(vec![
            sample_country("BR", Some("Brazil"), -14.235, -51.9253),
            sample_country("KE", Some("Kenya"), 0.0236, 37.9062),
        ]));

        assert_eq!(tools.selected_country_id.as_deref(), Some("BR"));
        assert!(matches!(tools.ready_countries(), Some(countries) if countries.len() == 2));
    }

    #[test]
    fn begin_resolve_country_center_uses_selected_country_id() {
        let (backend, requested_country_ids) = country_backend(Vec::new(), vec![Ok(())]);
        let mut tools = CountryLookupTools::default();
        tools.apply_list_result(Ok(vec![
            sample_country("BR", Some("Brazil"), -14.235, -51.9253),
            sample_country("KE", Some("Kenya"), 0.0236, 37.9062),
        ]));
        tools.selected_country_id = Some("KE".to_owned());

        tools.begin_resolve_country_center(&backend);

        assert_eq!(requested_country_ids.borrow().as_slice(), ["KE"]);
        assert_eq!(
            tools.center_status_message(),
            Some("Resolving country center...")
        );
    }

    #[test]
    fn apply_center_result_records_country_center() {
        let mut tools = CountryLookupTools::default();
        tools.apply_list_result(Ok(vec![sample_country(
            "BR",
            Some("Brazil"),
            -14.235,
            -51.9253,
        )]));
        tools.center = CountryCenterState::Pending {
            country_id: "BR".to_owned(),
        };

        tools.apply_center_result(Ok(RadrootsLocationPoint {
            lat: -14.235,
            lng: -51.9253,
        }));

        let result = tools.center_result().expect("country center result");
        assert_eq!(result.country_id, "BR");
        assert_eq!(result.country_name.as_deref(), Some("Brazil"));
        assert_eq!(
            result.center,
            RadrootsLocationPoint {
                lat: -14.235,
                lng: -51.9253,
            }
        );
    }

    fn sample_country(
        country_id: &str,
        country_name: Option<&str>,
        lat: f64,
        lng: f64,
    ) -> RadrootsLocationCountry {
        RadrootsLocationCountry {
            country_id: country_id.to_owned(),
            country_name: country_name.map(str::to_owned),
            center: RadrootsLocationPoint { lat, lng },
        }
    }
}
