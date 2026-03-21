#[cfg(target_os = "ios")]
use radroots_studio_app_apple_security::{APPLE_NOSTR_SERVICE, RadrootsAppleKeychainVault};
#[cfg(target_os = "ios")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
};
use std::path::Path;
use std::path::PathBuf;
#[cfg(target_os = "ios")]
use std::sync::Arc;

#[cfg(target_os = "ios")]
pub(crate) fn accounts_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "failed to resolve ios app container home directory".to_owned())?;
    let accounts_path = accounts_path_from_home(home.as_path());
    if let Some(parent) = accounts_path.parent() {
        ensure_private_directory_tree(parent)?;
    }
    Ok(accounts_path)
}

#[cfg(target_os = "ios")]
pub(crate) fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
    let store = Arc::new(RadrootsNostrFileAccountStore::new(accounts_path()?));
    let vault = Arc::new(RadrootsAppleKeychainVault::new(APPLE_NOSTR_SERVICE));
    RadrootsNostrAccountsManager::new(store, vault).map_err(|source| source.to_string())
}

fn accounts_path_from_home(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("RadRoots")
        .join("app")
        .join("ios")
        .join("nostr")
        .join("accounts.json")
}

#[cfg(target_os = "ios")]
fn ensure_private_directory_tree(leaf: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(leaf)
        .map_err(|source| format!("failed to create ios accounts directory: {source}"))?;
    std::fs::set_permissions(leaf, std::fs::Permissions::from_mode(0o700))
        .map_err(|source| format!("failed to set ios accounts directory permissions: {source}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_path_uses_ios_application_support_layout() {
        let home = PathBuf::from("/var/mobile/Containers/Data/Application/example");

        assert_eq!(
            accounts_path_from_home(home.as_path()),
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/app/ios/nostr/accounts.json"
            )
        );
    }
}
