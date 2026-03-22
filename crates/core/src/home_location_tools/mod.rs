use crate::{
    RadrootsAppBackend, RadrootsOfflineGeocoderState, RadrootsReverseLocationLookupResult,
};
use eframe::egui;

mod reverse_lookup;

#[cfg(test)]
use reverse_lookup::HomeLocationLookupResult;
use reverse_lookup::ReverseLookupTools;

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct HomeLocationTools {
    reverse_lookup: ReverseLookupTools,
}

impl HomeLocationTools {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn clear(&mut self) {
        self.reverse_lookup.clear();
    }

    #[cfg(test)]
    pub(crate) fn set_query_inputs(
        &mut self,
        latitude: impl Into<String>,
        longitude: impl Into<String>,
    ) {
        self.reverse_lookup.set_query_inputs(latitude, longitude);
    }

    pub(crate) fn render(
        &mut self,
        ui: &mut egui::Ui,
        backend: &dyn RadrootsAppBackend,
        offline_geocoder_state: Option<&RadrootsOfflineGeocoderState>,
    ) {
        self.reverse_lookup
            .render(ui, backend, offline_geocoder_state);
    }

    pub(crate) fn apply_reverse_lookup_result(
        &mut self,
        result: RadrootsReverseLocationLookupResult,
    ) {
        self.reverse_lookup.apply_result(result);
    }

    pub(crate) fn apply_reverse_lookup_poll_error(&mut self, message: String) {
        self.reverse_lookup.apply_poll_error(message);
    }

    #[cfg(test)]
    pub(crate) fn begin_resolve_with_backend(&mut self, backend: &dyn RadrootsAppBackend) {
        self.reverse_lookup.begin_resolve_with_backend(backend);
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.reverse_lookup.is_pending()
    }

    #[cfg(test)]
    pub(crate) fn status_message(&self) -> Option<&str> {
        self.reverse_lookup.status_message()
    }

    #[cfg(test)]
    pub(crate) fn lookup_result(&self) -> Option<&HomeLocationLookupResult> {
        self.reverse_lookup.lookup_result()
    }
}
