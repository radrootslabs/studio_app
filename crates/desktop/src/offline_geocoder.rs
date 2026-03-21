use radroots_studio_app_core::RadrootsOfflineGeocoderState;
use radroots_geocoder::Geocoder;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const GEOCODER_ASSET_FILENAME: &str = "geonames.db";

#[derive(Clone)]
pub(crate) struct DesktopOfflineGeocoder {
    current: Arc<Mutex<RadrootsOfflineGeocoderState>>,
    changed: Arc<AtomicBool>,
}

impl DesktopOfflineGeocoder {
    pub(crate) fn from_state(state: RadrootsOfflineGeocoderState) -> Self {
        Self {
            current: Arc::new(Mutex::new(state)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn start(app_data_root: PathBuf) -> Self {
        let tracker = Self::from_state(RadrootsOfflineGeocoderState::Initializing);

        let current = Arc::clone(&tracker.current);
        let changed = Arc::clone(&tracker.changed);
        std::thread::spawn(move || {
            let state = initialize_offline_geocoder(app_data_root.as_path());
            if let Ok(mut slot) = current.lock() {
                *slot = state;
                changed.store(true, Ordering::Release);
            }
        });

        tracker
    }

    pub(crate) fn current_state(&self) -> RadrootsOfflineGeocoderState {
        self.current
            .lock()
            .map(|state| state.clone())
            .unwrap_or_else(|_| RadrootsOfflineGeocoderState::Unavailable {
                user_message: "Offline geocoder is unavailable on this device.".to_owned(),
                debug_message: "desktop offline geocoder state lock poisoned".to_owned(),
            })
    }

    pub(crate) fn take_update(&self) -> Option<RadrootsOfflineGeocoderState> {
        if self.changed.swap(false, Ordering::AcqRel) {
            Some(self.current_state())
        } else {
            None
        }
    }
}

fn initialize_offline_geocoder(app_data_root: &Path) -> RadrootsOfflineGeocoderState {
    match initialize_offline_geocoder_inner(app_data_root) {
        Ok(()) => RadrootsOfflineGeocoderState::Ready,
        Err(debug_message) => classify_initialize_error(debug_message),
    }
}

fn initialize_offline_geocoder_inner(app_data_root: &Path) -> Result<(), String> {
    let source_path = runtime_asset_path()?;
    if !source_path.is_file() {
        return Err(format!(
            "desktop bundled geocoder asset missing at {}",
            source_path.display()
        ));
    }

    let staged_path = staged_db_path(app_data_root);
    stage_runtime_asset(source_path.as_path(), staged_path.as_path())?;
    Geocoder::open_path(staged_path.as_path())
        .map(|_| ())
        .map_err(|source| format!("failed to open staged geocoder db: {source}"))
}

fn runtime_asset_path() -> Result<PathBuf, String> {
    let executable_path = std::env::current_exe()
        .map_err(|source| format!("failed to resolve desktop executable path: {source}"))?;
    let Some(parent) = executable_path.parent() else {
        return Err("desktop executable path did not have a parent directory".to_owned());
    };
    Ok(parent.join(GEOCODER_ASSET_FILENAME))
}

fn staged_db_path(app_data_root: &Path) -> PathBuf {
    app_data_root.join("geocoder").join(GEOCODER_ASSET_FILENAME)
}

fn stage_runtime_asset(source_path: &Path, staged_path: &Path) -> Result<(), String> {
    let Some(parent) = staged_path.parent() else {
        return Err("staged desktop geocoder path did not have a parent directory".to_owned());
    };
    std::fs::create_dir_all(parent)
        .map_err(|source| format!("failed to create desktop geocoder directory: {source}"))?;
    std::fs::copy(source_path, staged_path)
        .map_err(|source| format!("failed to stage desktop geocoder asset: {source}"))?;
    Ok(())
}

fn classify_initialize_error(debug_message: String) -> RadrootsOfflineGeocoderState {
    let user_message = if debug_message.contains("asset missing") {
        "Offline geocoder is not available in this build.".to_owned()
    } else {
        "Offline geocoder could not be initialized on this device.".to_owned()
    };

    RadrootsOfflineGeocoderState::Unavailable {
        user_message,
        debug_message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_db_path_uses_app_geocoder_directory() {
        let app_data_root = PathBuf::from("/Users/example/.radroots/app/desktop");

        assert_eq!(
            staged_db_path(app_data_root.as_path()),
            PathBuf::from("/Users/example/.radroots/app/desktop/geocoder/geonames.db")
        );
    }

    #[test]
    fn missing_asset_maps_to_build_unavailable_message() {
        let state = classify_initialize_error(
            "desktop bundled geocoder asset missing at /tmp/geonames.db".to_owned(),
        );

        assert_eq!(
            state,
            RadrootsOfflineGeocoderState::Unavailable {
                user_message: "Offline geocoder is not available in this build.".to_owned(),
                debug_message: "desktop bundled geocoder asset missing at /tmp/geonames.db"
                    .to_owned(),
            }
        );
    }
}
