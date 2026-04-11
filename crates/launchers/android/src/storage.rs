use radroots_studio_app_core::mobile_native_app_storage_layout;
#[cfg(target_os = "android")]
use radroots_studio_app_android_security::{
    ANDROID_NOSTR_SERVICE, RadrootsAndroidKeystoreVault, resolve_radroots_base_root,
};
#[cfg(target_os = "android")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
};
use radroots_runtime_paths::{RadrootsPaths, RadrootsPlatform};
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use std::sync::Arc;

fn app_paths_from_base_root(base_root: &Path) -> Result<RadrootsPaths, String> {
    Ok(mobile_native_app_storage_layout(RadrootsPlatform::Android, base_root)?.app_paths)
}

#[cfg(target_os = "android")]
pub(crate) fn app_data_root() -> Result<PathBuf, String> {
    let base_root = resolve_radroots_base_root().map_err(|source| source.to_string())?;
    let root = app_data_root_from_base_root(base_root.as_path())?;
    ensure_directory_tree(root.as_path())?;
    Ok(root)
}

#[cfg(target_os = "android")]
pub(crate) fn accounts_path() -> Result<PathBuf, String> {
    let base_root = resolve_radroots_base_root().map_err(|source| source.to_string())?;
    let accounts_path = accounts_path_from_base_root(base_root.as_path())?;
    if let Some(parent) = accounts_path.parent() {
        ensure_directory_tree(parent)?;
    }
    Ok(accounts_path)
}

#[cfg(target_os = "android")]
pub(crate) fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
    let store = Arc::new(RadrootsNostrFileAccountStore::new(accounts_path()?));
    let vault = Arc::new(RadrootsAndroidKeystoreVault::new(ANDROID_NOSTR_SERVICE));
    RadrootsNostrAccountsManager::new(store, vault).map_err(|source| source.to_string())
}

pub(crate) fn app_data_root_from_base_root(base_root: &Path) -> Result<PathBuf, String> {
    Ok(app_paths_from_base_root(base_root)?.data)
}

pub(crate) fn accounts_path_from_base_root(base_root: &Path) -> Result<PathBuf, String> {
    Ok(app_data_root_from_base_root(base_root)?
        .join("nostr")
        .join("accounts.json"))
}

#[cfg(target_os = "android")]
fn ensure_directory_tree(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|source| format!("failed to create android app data directory: {source}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_path_uses_android_mobile_native_layout() {
        let base_root = PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots");

        assert_eq!(
            accounts_path_from_base_root(base_root.as_path()).expect("accounts path"),
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/data/apps/app/nostr/accounts.json"
            )
        );
    }

    #[test]
    fn app_data_root_uses_android_mobile_native_layout() {
        let base_root = PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots");

        assert_eq!(
            app_data_root_from_base_root(base_root.as_path()).expect("app data root"),
            PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots/data/apps/app")
        );
    }

    #[test]
    fn mobile_paths_follow_shared_logical_root_model() {
        let base_root = PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots");
        let paths = app_paths_from_base_root(base_root.as_path()).expect("mobile paths");

        assert_eq!(
            paths.config,
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/config/apps/app"
            )
        );
        assert_eq!(
            paths.data,
            PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots/data/apps/app")
        );
        assert_eq!(
            paths.secrets,
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/secrets/apps/app"
            )
        );
    }
}
