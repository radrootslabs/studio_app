#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

#[cfg(target_os = "ios")]
use crate::offline_geocoder;
use radroots_studio_app_core::{
    RadrootsLocationPoint, RadrootsLocationResolverError, RadrootsLocationReverseOptions,
    RadrootsOfflineGeocoderState, RadrootsReverseLocationLookupResult,
};
#[cfg(target_os = "ios")]
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct IosReverseLookup {
    result: Arc<Mutex<Option<RadrootsReverseLocationLookupResult>>>,
    changed: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
}

impl IosReverseLookup {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "ios")]
    pub(crate) fn begin(
        &self,
        app_data_root: PathBuf,
        geocoder_state: RadrootsOfflineGeocoderState,
        point: RadrootsLocationPoint,
        options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<(), RadrootsLocationResolverError> {
        if self.pending.swap(true, Ordering::AcqRel) {
            return Err(RadrootsLocationResolverError::QueryFailed {
                message: "offline location query is already running".to_owned(),
            });
        }

        if let Ok(mut slot) = self.result.lock() {
            *slot = None;
        }

        let result = Arc::clone(&self.result);
        let changed = Arc::clone(&self.changed);
        let pending = Arc::clone(&self.pending);
        std::thread::spawn(move || {
            let lookup_result = offline_geocoder::reverse_location(
                app_data_root.as_path(),
                &geocoder_state,
                point,
                options,
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
    pub(crate) fn begin(
        &self,
        _app_data_root: std::path::PathBuf,
        _geocoder_state: RadrootsOfflineGeocoderState,
        _point: RadrootsLocationPoint,
        _options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }

    pub(crate) fn take_update(&self) -> Option<RadrootsReverseLocationLookupResult> {
        if !self.changed.swap(false, Ordering::AcqRel) {
            return None;
        }

        match self.result.lock() {
            Ok(mut slot) => slot.take(),
            Err(_) => Some(Err(RadrootsLocationResolverError::QueryFailed {
                message: "ios reverse lookup result lock poisoned".to_owned(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_core::RadrootsResolvedLocation;

    fn sample_result() -> RadrootsReverseLocationLookupResult {
        Ok(vec![RadrootsResolvedLocation {
            id: 7,
            name: "example".to_owned(),
            admin1_id: None,
            admin1_name: None,
            country_id: "US".to_owned(),
            country_name: Some("United States".to_owned()),
            point: RadrootsLocationPoint { lat: 1.0, lng: 2.0 },
        }])
    }

    #[test]
    fn take_update_is_none_until_tracker_changes() {
        let tracker = IosReverseLookup::new();

        assert_eq!(tracker.take_update(), None);
    }

    #[test]
    fn take_update_returns_queued_result_once() {
        let tracker = IosReverseLookup::new();
        *tracker.result.lock().unwrap() = Some(sample_result());
        tracker.changed.store(true, Ordering::Release);

        assert!(matches!(tracker.take_update(), Some(Ok(results)) if results.len() == 1));
        assert_eq!(tracker.take_update(), None);
    }
}
