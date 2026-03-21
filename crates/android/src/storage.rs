#[cfg(target_os = "android")]
use crate::security::{ANDROID_NOSTR_SERVICE, resolve_nostr_storage_root};
#[cfg(target_os = "android")]
use crate::vault::RadrootsAndroidKeystoreVault;
#[cfg(target_os = "android")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
};
use std::path::Path;
use std::path::PathBuf;
#[cfg(target_os = "android")]
use std::sync::Arc;

#[cfg(target_os = "android")]
pub(crate) fn accounts_path() -> Result<PathBuf, String> {
    let root = resolve_nostr_storage_root().map_err(|source| source.to_string())?;
    let accounts_path = accounts_path_from_root(root.as_path());
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

pub(crate) fn accounts_path_from_root(root: &Path) -> PathBuf {
    root.join("accounts.json")
}

#[cfg(target_os = "android")]
fn ensure_directory_tree(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|source| format!("failed to create android accounts directory: {source}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_path_uses_android_no_backup_layout() {
        let root = PathBuf::from(
            "/data/user/0/org.radroots.app.android/no_backup/RadRoots/app/android/nostr",
        );

        assert_eq!(
            accounts_path_from_root(root.as_path()),
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/app/android/nostr/accounts.json"
            )
        );
    }
}
