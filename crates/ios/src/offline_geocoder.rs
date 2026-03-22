#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

use radroots_studio_app_core::{RadrootsOfflineGeocoderState, RadrootsOfflineGeocoderUnavailableKind};
use radroots_geocoder::Geocoder;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const GEOCODER_ASSET_FILENAME: &str = "geonames.db";
const GEOCODER_REVISION_FILENAME: &str = "geonames.revision";

#[derive(Clone)]
pub(crate) struct IosOfflineGeocoder {
    current: Arc<Mutex<RadrootsOfflineGeocoderState>>,
    changed: Arc<AtomicBool>,
}

impl IosOfflineGeocoder {
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
                    "ios offline geocoder state lock poisoned",
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
    let source_path = bundled_asset_path().map_err(|debug_message| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InternalError,
            debug_message,
        )
    })?;
    if !source_path.is_file() {
        return Err((
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "ios bundled geocoder asset missing at {}",
                source_path.display()
            ),
        ));
    }

    let revision = bundled_asset_revision(source_path.parent().unwrap_or_else(|| Path::new(".")))?;
    let staged_path = staged_db_path(app_data_root, revision.as_str());
    stage_bundled_asset(source_path.as_path(), staged_path.as_path()).map_err(|debug_message| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            debug_message,
        )
    })?;
    Geocoder::open_path(staged_path.as_path()).map_err(|source| {
        (
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            format!("failed to open staged ios geocoder db: {source}"),
        )
    })?;
    let _ = prune_stale_revisions(staged_geocoder_root(app_data_root), revision.as_str());
    Ok(())
}

fn bundled_asset_path() -> Result<PathBuf, String> {
    let executable_path = std::env::current_exe()
        .map_err(|source| format!("failed to resolve ios executable path: {source}"))?;
    let Some(parent) = executable_path.parent() else {
        return Err("ios executable path did not have a parent directory".to_owned());
    };
    Ok(parent.join(GEOCODER_ASSET_FILENAME))
}

fn bundled_asset_revision(
    asset_dir: &Path,
) -> Result<String, (RadrootsOfflineGeocoderUnavailableKind, String)> {
    let revision_path = asset_dir.join(GEOCODER_REVISION_FILENAME);
    let revision = std::fs::read_to_string(revision_path.as_path()).map_err(|source| {
        (
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "ios bundled geocoder revision asset missing at {}: {source}",
                revision_path.display()
            ),
        )
    })?;
    let revision = revision.trim();
    if !is_valid_revision(revision) {
        return Err((
            RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
            format!(
                "ios bundled geocoder revision asset invalid at {}",
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

fn stage_bundled_asset(source_path: &Path, staged_path: &Path) -> Result<bool, String> {
    let Some(parent) = staged_path.parent() else {
        return Err("staged ios geocoder path did not have a parent directory".to_owned());
    };
    std::fs::create_dir_all(parent)
        .map_err(|source| format!("failed to create ios geocoder directory: {source}"))?;
    if staged_path.is_file() {
        return Ok(false);
    }
    std::fs::copy(source_path, staged_path)
        .map_err(|source| format!("failed to stage ios geocoder asset: {source}"))?;
    Ok(true)
}

fn prune_stale_revisions(staged_root: PathBuf, active_revision: &str) -> Result<(), String> {
    if !staged_root.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(staged_root.as_path())
        .map_err(|source| format!("failed to list ios geocoder revisions: {source}"))?
    {
        let entry = entry
            .map_err(|source| format!("failed to read ios geocoder revision entry: {source}"))?;
        if entry.file_name() == std::ffi::OsStr::new(active_revision) {
            continue;
        }

        let path = entry.path();
        if entry
            .file_type()
            .map_err(|source| format!("failed to inspect ios geocoder revision entry: {source}"))?
            .is_dir()
        {
            std::fs::remove_dir_all(path.as_path()).map_err(|source| {
                format!(
                    "failed to remove stale ios geocoder revision {}: {source}",
                    path.display()
                )
            })?;
        } else {
            std::fs::remove_file(path.as_path()).map_err(|source| {
                format!(
                    "failed to remove stale ios geocoder revision file {}: {source}",
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
    fn staged_db_path_uses_ios_geocoder_directory() {
        let app_data_root = PathBuf::from(
            "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/app/ios",
        );

        assert_eq!(
            staged_db_path(app_data_root.as_path(), "abcd"),
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/app/ios/geocoder/abcd/geonames.db"
            )
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
            "ios bundled geocoder asset missing at /tmp/geonames.db",
        );

        assert_eq!(
            state,
            RadrootsOfflineGeocoderState::Unavailable {
                kind: RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                debug_message: "ios bundled geocoder asset missing at /tmp/geonames.db".to_owned(),
            }
        );
    }

    #[test]
    fn stage_bundled_asset_reuses_existing_staged_copy() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-ios-geocoder-test-{}",
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

        let copied = stage_bundled_asset(source_path.as_path(), staged_path.as_path()).unwrap();

        assert!(!copied);
        assert_eq!(std::fs::read(staged_path.as_path()).unwrap(), b"existing");

        std::fs::remove_dir_all(temp_root.as_path()).unwrap();
    }

    #[test]
    fn prune_stale_revisions_keeps_active_revision_only() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-ios-geocoder-prune-test-{}",
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
    fn bundled_asset_revision_reads_stamped_sidecar() {
        let temp_root = std::env::temp_dir().join(format!(
            "radroots-ios-geocoder-revision-test-{}",
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
            bundled_asset_revision(temp_root.as_path()).unwrap(),
            revision.to_owned()
        );

        std::fs::remove_dir_all(temp_root.as_path()).unwrap();
    }
}
