use radroots_studio_app_core::{
    RadrootsLocationCountry, RadrootsLocationPoint, RadrootsLocationResolverError,
    RadrootsLocationReverseOptions, RadrootsOfflineGeocoderPlatform, RadrootsOfflineGeocoderState,
    RadrootsOfflineGeocoderUnavailableKind, RadrootsResolvedLocation,
};
use radroots_geocoder::{
    Geocoder, GeocoderCountryListResult, GeocoderError, GeocoderPoint, GeocoderReverseOptions,
    GeocoderReverseResult,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const GEOCODER_ASSET_FILENAME: &str = "geonames.db";
const GEOCODER_REVISION_FILENAME: &str = "geonames.revision";

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
            if let RadrootsOfflineGeocoderState::Unavailable { debug_message, .. } = &state {
                log::warn!("desktop offline geocoder unavailable: {debug_message}");
            }
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
                    RadrootsOfflineGeocoderPlatform::Desktop,
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

pub(crate) fn reverse_location(
    app_data_root: &Path,
    state: &RadrootsOfflineGeocoderState,
    point: RadrootsLocationPoint,
    options: Option<RadrootsLocationReverseOptions>,
) -> Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError> {
    let geocoder = geocoder_for_queries(app_data_root, state)?;
    let options = options.map(|options| GeocoderReverseOptions {
        limit: options.limit,
        degree_offset: options.degree_offset,
    });
    geocoder
        .reverse(
            GeocoderPoint {
                lat: point.lat,
                lng: point.lng,
            },
            options,
        )
        .map(|results| results.into_iter().map(map_reverse_result).collect())
        .map_err(|source| RadrootsLocationResolverError::QueryFailed {
            message: source.to_string(),
        })
}

pub(crate) fn list_countries(
    app_data_root: &Path,
    state: &RadrootsOfflineGeocoderState,
) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
    let geocoder = geocoder_for_queries(app_data_root, state)?;
    geocoder
        .country_list()
        .map(|results| results.into_iter().map(map_country_result).collect())
        .map_err(|source| RadrootsLocationResolverError::QueryFailed {
            message: source.to_string(),
        })
}

pub(crate) fn country_center(
    app_data_root: &Path,
    state: &RadrootsOfflineGeocoderState,
    country_id: &str,
) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
    let geocoder = geocoder_for_queries(app_data_root, state)?;
    geocoder
        .country_center(country_id)
        .map(|point| RadrootsLocationPoint {
            lat: point.lat,
            lng: point.lng,
        })
        .map_err(map_country_center_error)
}

fn initialize_offline_geocoder(app_data_root: &Path) -> RadrootsOfflineGeocoderState {
    let source_path = runtime_asset_path().map_err(|debug_message| {
        RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::InternalError,
            RadrootsOfflineGeocoderPlatform::Desktop,
            debug_message,
        )
    });
    let source_path = match source_path {
        Ok(source_path) => source_path,
        Err(state) => return state,
    };
    if !source_path.is_file() {
        return RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            RadrootsOfflineGeocoderPlatform::Desktop,
            format!(
                "desktop bundled geocoder asset missing at {}",
                source_path.display()
            ),
        );
    }

    let revision =
        match runtime_asset_revision(source_path.parent().unwrap_or_else(|| Path::new("."))) {
            Ok(revision) => revision,
            Err((kind, debug_message)) => {
                return RadrootsOfflineGeocoderState::unavailable(
                    kind,
                    RadrootsOfflineGeocoderPlatform::Desktop,
                    debug_message,
                );
            }
        };
    let staged_path = staged_db_path(app_data_root, revision.as_str());
    if let Err(debug_message) = stage_runtime_asset(source_path.as_path(), staged_path.as_path()) {
        return RadrootsOfflineGeocoderState::unavailable_with_revision(
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            RadrootsOfflineGeocoderPlatform::Desktop,
            revision,
            debug_message,
        );
    }
    if let Err(source) = Geocoder::open_path(staged_path.as_path()) {
        return RadrootsOfflineGeocoderState::unavailable_with_revision(
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            RadrootsOfflineGeocoderPlatform::Desktop,
            revision,
            format!("failed to open staged geocoder db: {source}"),
        );
    }
    let _ = prune_stale_revisions(staged_geocoder_root(app_data_root), revision.as_str());
    RadrootsOfflineGeocoderState::Ready
}

fn runtime_asset_path() -> Result<PathBuf, String> {
    let executable_path = std::env::current_exe()
        .map_err(|source| format!("failed to resolve desktop executable path: {source}"))?;
    let Some(parent) = executable_path.parent() else {
        return Err("desktop executable path did not have a parent directory".to_owned());
    };
    Ok(parent.join(GEOCODER_ASSET_FILENAME))
}

fn geocoder_for_queries(
    app_data_root: &Path,
    state: &RadrootsOfflineGeocoderState,
) -> Result<Geocoder, RadrootsLocationResolverError> {
    match state {
        RadrootsOfflineGeocoderState::Initializing => {
            return Err(RadrootsLocationResolverError::Initializing);
        }
        RadrootsOfflineGeocoderState::Unavailable { .. } => {
            return Err(RadrootsLocationResolverError::Unavailable);
        }
        RadrootsOfflineGeocoderState::Ready => {}
    }

    let source_path = runtime_asset_path()
        .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
    let revision =
        runtime_asset_revision(source_path.parent().unwrap_or_else(|| Path::new(".")))
            .map_err(|(_, message)| RadrootsLocationResolverError::QueryFailed { message })?;
    let staged_path = staged_db_path(app_data_root, revision.as_str());
    stage_runtime_asset(source_path.as_path(), staged_path.as_path())
        .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
    Geocoder::open_path(staged_path.as_path()).map_err(|source| {
        RadrootsLocationResolverError::QueryFailed {
            message: source.to_string(),
        }
    })
}

fn map_reverse_result(result: GeocoderReverseResult) -> RadrootsResolvedLocation {
    RadrootsResolvedLocation {
        id: result.id,
        name: result.name,
        admin1_id: result.admin1_id,
        admin1_name: result.admin1_name,
        country_id: result.country_id,
        country_name: result.country_name,
        point: RadrootsLocationPoint {
            lat: result.latitude,
            lng: result.longitude,
        },
    }
}

fn map_country_result(result: GeocoderCountryListResult) -> RadrootsLocationCountry {
    RadrootsLocationCountry {
        country_id: result.country_id,
        country_name: result.country,
        center: RadrootsLocationPoint {
            lat: result.lat,
            lng: result.lng,
        },
    }
}

fn map_country_center_error(source: GeocoderError) -> RadrootsLocationResolverError {
    match source {
        GeocoderError::CountryCenterNotFound { country_id } => {
            RadrootsLocationResolverError::CountryCenterNotFound { country_id }
        }
        other => RadrootsLocationResolverError::QueryFailed {
            message: other.to_string(),
        },
    }
}

fn runtime_asset_revision(
    asset_dir: &Path,
) -> Result<String, (RadrootsOfflineGeocoderUnavailableKind, String)> {
    let revision_path = asset_dir.join(GEOCODER_REVISION_FILENAME);
    let revision = std::fs::read_to_string(revision_path.as_path()).map_err(|source| {
        (
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "desktop bundled geocoder revision asset missing at {}: {source}",
                revision_path.display()
            ),
        )
    })?;
    let revision = revision.trim();
    if !is_valid_revision(revision) {
        return Err((
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "desktop bundled geocoder revision asset invalid at {}",
                revision_path.display()
            ),
        ));
    }
    Ok(revision.to_owned())
}

fn is_valid_revision(revision: &str) -> bool {
    revision.len() == 64 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn staged_geocoder_root(app_data_root: &Path) -> PathBuf {
    app_data_root.join("geocoder")
}

fn staged_db_path(app_data_root: &Path, revision: &str) -> PathBuf {
    staged_geocoder_root(app_data_root)
        .join(revision)
        .join(GEOCODER_ASSET_FILENAME)
}

fn stage_runtime_asset(source_path: &Path, staged_path: &Path) -> Result<bool, String> {
    let Some(parent) = staged_path.parent() else {
        return Err("staged desktop geocoder path did not have a parent directory".to_owned());
    };
    std::fs::create_dir_all(parent)
        .map_err(|source| format!("failed to create desktop geocoder directory: {source}"))?;
    if staged_path.is_file() {
        return Ok(false);
    }
    std::fs::copy(source_path, staged_path)
        .map_err(|source| format!("failed to stage desktop geocoder asset: {source}"))?;
    Ok(true)
}

fn prune_stale_revisions(staged_root: PathBuf, active_revision: &str) -> Result<(), String> {
    if !staged_root.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(staged_root.as_path())
        .map_err(|source| format!("failed to list desktop geocoder revisions: {source}"))?
    {
        let entry = entry.map_err(|source| {
            format!("failed to read desktop geocoder revision entry: {source}")
        })?;
        if entry.file_name() == std::ffi::OsStr::new(active_revision) {
            continue;
        }

        let path = entry.path();
        if entry
            .file_type()
            .map_err(|source| {
                format!("failed to inspect desktop geocoder revision entry: {source}")
            })?
            .is_dir()
        {
            std::fs::remove_dir_all(path.as_path()).map_err(|source| {
                format!(
                    "failed to remove stale desktop geocoder revision {}: {source}",
                    path.display()
                )
            })?;
        } else {
            std::fs::remove_file(path.as_path()).map_err(|source| {
                format!(
                    "failed to remove stale desktop geocoder revision file {}: {source}",
                    path.display()
                )
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn staged_db_path_uses_app_geocoder_directory() {
        let app_data_root = PathBuf::from("/Users/example/.radroots/data/apps/app");

        assert_eq!(
            staged_db_path(app_data_root.as_path(), "abcd"),
            PathBuf::from("/Users/example/.radroots/data/apps/app/geocoder/abcd/geonames.db")
        );
    }

    #[test]
    fn valid_revision_requires_sha256_hex() {
        assert!(is_valid_revision(
            "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c"
        ));
        assert!(!is_valid_revision("abcd"));
        assert!(!is_valid_revision(
            "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079z"
        ));
    }

    #[test]
    fn missing_asset_maps_to_build_unavailable_message() {
        let state = RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            RadrootsOfflineGeocoderPlatform::Desktop,
            "desktop bundled geocoder asset missing at /tmp/geonames.db",
        );

        assert_eq!(
            state,
            RadrootsOfflineGeocoderState::Unavailable {
                kind: RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                platform: RadrootsOfflineGeocoderPlatform::Desktop,
                asset_revision: None,
                debug_message: "desktop bundled geocoder asset missing at /tmp/geonames.db"
                    .to_owned(),
            }
        );
    }

    #[test]
    fn stage_runtime_asset_reuses_existing_staged_copy() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-desktop-geocoder-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source_path = temp_root.join("source.db");
        let staged_path = temp_root.join("staged").join("geonames.db");

        std::fs::create_dir_all(temp_root.as_path()).unwrap();
        std::fs::write(source_path.as_path(), b"source").unwrap();
        std::fs::create_dir_all(staged_path.parent().unwrap()).unwrap();
        std::fs::write(staged_path.as_path(), b"existing").unwrap();

        let copied = stage_runtime_asset(source_path.as_path(), staged_path.as_path()).unwrap();

        assert!(!copied);
        assert_eq!(std::fs::read(staged_path.as_path()).unwrap(), b"existing");

        std::fs::remove_dir_all(temp_root.as_path()).unwrap();
    }

    #[test]
    fn prune_stale_revisions_keeps_active_revision_only() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-desktop-geocoder-prune-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let staged_root = temp_root.join("geocoder");
        let active_dir = staged_root.join("active");
        let stale_dir = staged_root.join("stale");
        let stale_file = staged_root.join("orphan.txt");

        std::fs::create_dir_all(active_dir.as_path()).unwrap();
        std::fs::create_dir_all(stale_dir.as_path()).unwrap();
        std::fs::write(active_dir.join("geonames.db"), b"active").unwrap();
        std::fs::write(stale_dir.join("geonames.db"), b"stale").unwrap();
        std::fs::write(stale_file.as_path(), b"orphan").unwrap();

        prune_stale_revisions(staged_root.clone(), "active").unwrap();

        assert!(active_dir.exists());
        assert!(!stale_dir.exists());
        assert!(!stale_file.exists());

        std::fs::remove_dir_all(temp_root.as_path()).unwrap();
    }

    #[test]
    fn runtime_asset_revision_reads_stamped_sidecar() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-desktop-geocoder-revision-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let revision_path = temp_root.join(GEOCODER_REVISION_FILENAME);
        let revision = "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c";

        std::fs::create_dir_all(temp_root.as_path()).unwrap();
        std::fs::write(revision_path.as_path(), format!("{revision}\n")).unwrap();

        assert_eq!(
            runtime_asset_revision(temp_root.as_path()).unwrap(),
            revision.to_owned()
        );

        std::fs::remove_dir_all(temp_root.as_path()).unwrap();
    }

    #[test]
    fn reverse_result_mapping_preserves_location_fields() {
        let mapped = map_reverse_result(GeocoderReverseResult {
            id: 42,
            name: "Oslo".to_owned(),
            admin1_id: Some(12),
            admin1_name: Some("Oslo".to_owned()),
            country_id: "NO".to_owned(),
            country_name: Some("Norway".to_owned()),
            latitude: 59.9139,
            longitude: 10.7522,
        });

        assert_eq!(mapped.id, 42);
        assert_eq!(mapped.name, "Oslo");
        assert_eq!(mapped.admin1_id, Some(12));
        assert_eq!(mapped.admin1_name.as_deref(), Some("Oslo"));
        assert_eq!(mapped.country_id, "NO");
        assert_eq!(mapped.country_name.as_deref(), Some("Norway"));
        assert_eq!(mapped.point.lat, 59.9139);
        assert_eq!(mapped.point.lng, 10.7522);
    }

    #[test]
    fn unavailable_state_blocks_queries_until_ready() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-desktop-geocoder-query-state-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let result = list_countries(
            temp_root.as_path(),
            &RadrootsOfflineGeocoderState::Initializing,
        );

        assert_eq!(result, Err(RadrootsLocationResolverError::Initializing));
    }
}
