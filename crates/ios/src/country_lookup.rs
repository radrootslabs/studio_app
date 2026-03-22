#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

#[cfg(target_os = "ios")]
use crate::offline_geocoder;
use radroots_studio_app_core::{
    RadrootsLocationCountryCenterLookupResult, RadrootsLocationCountryListResult,
    RadrootsLocationResolverError, RadrootsOfflineGeocoderState,
};
#[cfg(target_os = "ios")]
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct IosCountryLookup {
    country_list_result: Arc<Mutex<Option<RadrootsLocationCountryListResult>>>,
    country_list_changed: Arc<AtomicBool>,
    country_list_pending: Arc<AtomicBool>,
    country_center_result: Arc<Mutex<Option<RadrootsLocationCountryCenterLookupResult>>>,
    country_center_changed: Arc<AtomicBool>,
    country_center_pending: Arc<AtomicBool>,
}

impl IosCountryLookup {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "ios")]
    pub(crate) fn begin_list(
        &self,
        app_data_root: PathBuf,
        geocoder_state: RadrootsOfflineGeocoderState,
    ) -> Result<(), RadrootsLocationResolverError> {
        if self.country_list_pending.swap(true, Ordering::AcqRel) {
            return Err(RadrootsLocationResolverError::QueryFailed {
                message: "offline country list query is already running".to_owned(),
            });
        }

        if let Ok(mut slot) = self.country_list_result.lock() {
            *slot = None;
        }

        let result = Arc::clone(&self.country_list_result);
        let changed = Arc::clone(&self.country_list_changed);
        let pending = Arc::clone(&self.country_list_pending);
        std::thread::spawn(move || {
            let lookup_result =
                offline_geocoder::list_countries(app_data_root.as_path(), &geocoder_state);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(lookup_result);
                changed.store(true, Ordering::Release);
            }
            pending.store(false, Ordering::Release);
        });

        Ok(())
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn begin_list(
        &self,
        _app_data_root: std::path::PathBuf,
        _geocoder_state: RadrootsOfflineGeocoderState,
    ) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }

    #[cfg(target_os = "ios")]
    pub(crate) fn begin_center(
        &self,
        app_data_root: PathBuf,
        geocoder_state: RadrootsOfflineGeocoderState,
        country_id: String,
    ) -> Result<(), RadrootsLocationResolverError> {
        if self.country_center_pending.swap(true, Ordering::AcqRel) {
            return Err(RadrootsLocationResolverError::QueryFailed {
                message: "offline country center query is already running".to_owned(),
            });
        }

        if let Ok(mut slot) = self.country_center_result.lock() {
            *slot = None;
        }

        let result = Arc::clone(&self.country_center_result);
        let changed = Arc::clone(&self.country_center_changed);
        let pending = Arc::clone(&self.country_center_pending);
        std::thread::spawn(move || {
            let lookup_result = offline_geocoder::country_center(
                app_data_root.as_path(),
                &geocoder_state,
                &country_id,
            );
            if let Ok(mut slot) = result.lock() {
                *slot = Some(lookup_result);
                changed.store(true, Ordering::Release);
            }
            pending.store(false, Ordering::Release);
        });

        Ok(())
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn begin_center(
        &self,
        _app_data_root: std::path::PathBuf,
        _geocoder_state: RadrootsOfflineGeocoderState,
        _country_id: String,
    ) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }

    pub(crate) fn take_list_update(&self) -> Option<RadrootsLocationCountryListResult> {
        if !self.country_list_changed.swap(false, Ordering::AcqRel) {
            return None;
        }

        match self.country_list_result.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => Some(Err(RadrootsLocationResolverError::QueryFailed {
                message: "ios country list result lock poisoned".to_owned(),
            })),
        }
    }

    pub(crate) fn take_center_update(&self) -> Option<RadrootsLocationCountryCenterLookupResult> {
        if !self.country_center_changed.swap(false, Ordering::AcqRel) {
            return None;
        }

        match self.country_center_result.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => Some(Err(RadrootsLocationResolverError::QueryFailed {
                message: "ios country center result lock poisoned".to_owned(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_core::{RadrootsLocationCountry, RadrootsLocationPoint};

    fn sample_countries() -> RadrootsLocationCountryListResult {
        Ok(vec![RadrootsLocationCountry {
            country_id: "BR".to_owned(),
            country_name: Some("Brazil".to_owned()),
            center: RadrootsLocationPoint {
                lat: -14.235,
                lng: -51.9253,
            },
        }])
    }

    #[test]
    fn take_list_update_is_none_until_tracker_changes() {
        let tracker = IosCountryLookup::new();

        assert_eq!(tracker.take_list_update(), None);
    }

    #[test]
    fn take_list_update_returns_queued_result_once() {
        let tracker = IosCountryLookup::new();
        *tracker.country_list_result.lock().unwrap() = Some(sample_countries());
        tracker.country_list_changed.store(true, Ordering::Release);

        assert!(matches!(tracker.take_list_update(), Some(Ok(results)) if results.len() == 1));
        assert_eq!(tracker.take_list_update(), None);
    }

    #[test]
    fn take_center_update_returns_queued_result_once() {
        let tracker = IosCountryLookup::new();
        *tracker.country_center_result.lock().unwrap() = Some(Ok(RadrootsLocationPoint {
            lat: -14.235,
            lng: -51.9253,
        }));
        tracker
            .country_center_changed
            .store(true, Ordering::Release);

        assert!(matches!(tracker.take_center_update(), Some(Ok(point)) if point.lat == -14.235));
        assert_eq!(tracker.take_center_update(), None);
    }
}
