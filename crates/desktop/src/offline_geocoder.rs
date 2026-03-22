use radroots_studio_app_core::{
    RadrootsOfflineGeocoderState, RadrootsOfflineGeocoderUnavailableKind,
};
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
            .unwrap_or_else(|_| {
                RadrootsOfflineGeocoderState::unavailable(
                    RadrootsOfflineGeocoderUnavailableKind::InternalError,
                    "desktop offline geocoder state lock poisoned",
                )
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
        Err((kind, debug_message)) => {
            RadrootsOfflineGeocoderState::unavailable(kind, debug_message)
        }
    }
}

fn initialize_offline_geocoder_inner(
    app_data_root: &Path,
) -> Result<(), (RadrootsOfflineGeocoderUnavailableKind, String)> {
    let source_path = runtime_asset_path().map_err(|debug_message| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InternalError,
            debug_message,
        )
    })?;
    if !source_path.is_file() {
        return Err((
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "desktop bundled geocoder asset missing at {}",
                source_path.display()
            ),
        ));
    }

    let staged_path = staged_db_path(app_data_root);
    stage_runtime_asset(source_path.as_path(), staged_path.as_path()).map_err(|debug_message| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            debug_message,
        )
    })?;
    Geocoder::open_path(staged_path.as_path())
        .map(|_| ())
        .map_err(|source| {
            (
                RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
                format!("failed to open staged geocoder db: {source}"),
            )
        })
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
        let state = RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            "desktop bundled geocoder asset missing at /tmp/geonames.db",
        );

        assert_eq!(
            state,
            RadrootsOfflineGeocoderState::Unavailable {
                kind: RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                debug_message: "desktop bundled geocoder asset missing at /tmp/geonames.db"
                    .to_owned(),
            }
        );
    }
}
