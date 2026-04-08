#[cfg(target_os = "ios")]
use radroots_studio_app_apple_security::{APPLE_NOSTR_SERVICE, RadrootsAppleKeychainVault};
#[cfg(target_os = "ios")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
};
use radroots_runtime_paths::{
    RadrootsHostEnvironment, RadrootsPathOverrides, RadrootsPathProfile, RadrootsPathResolver,
    RadrootsPaths, RadrootsPlatform, RadrootsRuntimeNamespace,
};
use std::path::{Path, PathBuf};
#[cfg(target_os = "ios")]
use std::sync::Arc;

fn app_namespace() -> Result<RadrootsRuntimeNamespace, String> {
    RadrootsRuntimeNamespace::app("app")
        .map_err(|source| format!("failed to resolve ios app namespace: {source}"))
}

fn mobile_base_root_from_home(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("RadRoots")
}

fn app_paths_from_home(home: &Path) -> Result<RadrootsPaths, String> {
    let resolver =
        RadrootsPathResolver::new(RadrootsPlatform::Ios, RadrootsHostEnvironment::default());
    let namespace = app_namespace()?;
    resolver
        .resolve(
            RadrootsPathProfile::MobileNative,
            &RadrootsPathOverrides::mobile(RadrootsPaths::from_base_root(
                mobile_base_root_from_home(home),
            )),
        )
        .map(|roots| roots.namespaced(&namespace))
        .map_err(|source| format!("failed to resolve ios mobile-native roots: {source}"))
}

#[cfg(target_os = "ios")]
pub(crate) fn accounts_path() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "failed to resolve ios app container home directory".to_owned())?;
    let accounts_path = accounts_path_from_home(home.as_path())?;
    if let Some(parent) = accounts_path.parent() {
        ensure_private_directory_tree(parent)?;
    }
    Ok(accounts_path)
}

#[cfg(target_os = "ios")]
pub(crate) fn app_data_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "failed to resolve ios app container home directory".to_owned())?;
    let root = app_data_root_from_home(home.as_path())?;
    ensure_private_directory_tree(root.as_path())?;
    Ok(root)
}

#[cfg(target_os = "ios")]
pub(crate) fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
    let store = Arc::new(RadrootsNostrFileAccountStore::new(accounts_path()?));
    let vault = Arc::new(RadrootsAppleKeychainVault::new_device_local(
        APPLE_NOSTR_SERVICE,
    ));
    RadrootsNostrAccountsManager::new(store, vault).map_err(|source| source.to_string())
}

fn accounts_path_from_home(home: &Path) -> Result<PathBuf, String> {
    Ok(app_data_root_from_home(home)?
        .join("nostr")
        .join("accounts.json"))
}

fn app_data_root_from_home(home: &Path) -> Result<PathBuf, String> {
    Ok(app_paths_from_home(home)?.data)
}

#[cfg(target_os = "ios")]
fn ensure_private_directory_tree(leaf: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(leaf)
        .map_err(|source| format!("failed to create ios app data directory: {source}"))?;
    std::fs::set_permissions(leaf, std::fs::Permissions::from_mode(0o700))
        .map_err(|source| format!("failed to set ios app data permissions: {source}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_path_uses_ios_mobile_native_layout() {
        let home = PathBuf::from("/var/mobile/Containers/Data/Application/example");

        assert_eq!(
            accounts_path_from_home(home.as_path()).expect("accounts path"),
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/data/apps/app/nostr/accounts.json"
            )
        );
    }

    #[test]
    fn app_data_root_uses_ios_mobile_native_layout() {
        let home = PathBuf::from("/var/mobile/Containers/Data/Application/example");

        assert_eq!(
            app_data_root_from_home(home.as_path()).expect("app data root"),
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/data/apps/app"
            )
        );
    }

    #[test]
    fn mobile_paths_follow_shared_logical_root_model() {
        let home = PathBuf::from("/var/mobile/Containers/Data/Application/example");
        let paths = app_paths_from_home(home.as_path()).expect("mobile paths");

        assert_eq!(
            paths.config,
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/config/apps/app"
            )
        );
        assert_eq!(
            paths.data,
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/data/apps/app"
            )
        );
        assert_eq!(
            paths.secrets,
            PathBuf::from(
                "/var/mobile/Containers/Data/Application/example/Library/Application Support/RadRoots/secrets/apps/app"
            )
        );
    }
}
