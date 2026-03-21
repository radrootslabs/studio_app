#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use directories::BaseDirs;
use eframe::egui;
#[cfg(target_os = "macos")]
use radroots_studio_app_apple_security::{APPLE_NOSTR_SERVICE, RadrootsAppleKeychainVault};
use radroots_studio_app_core::{
    APP_NAME, HomeActionState, IdentityGateState, RadrootsApp, RadrootsAppBackend, SetupActionState,
};
#[cfg(target_os = "macos")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore, RadrootsNostrSelectedAccountStatus,
};
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(target_os = "macos")]
fn set_macos_app_name() {
    use objc2_foundation::{NSProcessInfo, NSString};

    let process_info = NSProcessInfo::processInfo();
    let process_name = NSString::from_str(APP_NAME);
    process_info.setProcessName(&process_name);
}

#[cfg(not(target_os = "macos"))]
fn set_macos_app_name() {}

struct DesktopBackend;

impl DesktopBackend {
    #[cfg(target_os = "macos")]
    fn radroots_root() -> Result<PathBuf, String> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| "failed to resolve home directory".to_owned())?;
        Ok(base_dirs.home_dir().join(".radroots"))
    }

    #[cfg(target_os = "macos")]
    fn app_data_root() -> Result<PathBuf, String> {
        Ok(Self::radroots_root()?.join("app").join("desktop"))
    }

    #[cfg(target_os = "macos")]
    fn private_directory_chain(root: &Path, leaf: &Path) -> Result<Vec<PathBuf>, String> {
        let relative = leaf
            .strip_prefix(root)
            .map_err(|_| "private directory escaped radroots root".to_owned())?;
        let mut current = root.to_path_buf();
        let mut chain = vec![current.clone()];
        for component in relative.components() {
            current.push(component);
            chain.push(current.clone());
        }
        Ok(chain)
    }

    #[cfg(target_os = "macos")]
    fn ensure_private_directory_tree(leaf: &Path) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(leaf)
            .map_err(|source| format!("failed to create accounts directory: {source}"))?;

        for path in Self::private_directory_chain(&Self::radroots_root()?, leaf)? {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).map_err(
                |source| format!("failed to set private directory permissions: {source}"),
            )?;
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
        let accounts_path = Self::app_data_root()?.join("nostr").join("accounts.json");
        if let Some(parent) = accounts_path.parent() {
            Self::ensure_private_directory_tree(parent)?;
        }

        let store = Arc::new(RadrootsNostrFileAccountStore::new(accounts_path));
        let vault = Arc::new(RadrootsAppleKeychainVault::new(APPLE_NOSTR_SERVICE));
        RadrootsNostrAccountsManager::new(store, vault).map_err(|source| source.to_string())
    }

    #[cfg(target_os = "macos")]
    fn map_status(status: RadrootsNostrSelectedAccountStatus) -> IdentityGateState {
        match status {
            RadrootsNostrSelectedAccountStatus::NotConfigured => IdentityGateState::Missing,
            RadrootsNostrSelectedAccountStatus::PublicOnly { .. } => IdentityGateState::Missing,
            RadrootsNostrSelectedAccountStatus::Ready { account } => IdentityGateState::Ready {
                account_id: account.account_id.to_string(),
                npub: account.public_identity.public_key_npub,
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn remove_selected_local_identity(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<IdentityGateState, String> {
        let Some(account_id) = manager
            .selected_account_id()
            .map_err(|source| source.to_string())?
        else {
            return Ok(IdentityGateState::Missing);
        };

        manager
            .remove_account(&account_id)
            .map_err(|source| source.to_string())?;
        let status = manager
            .selected_account_status()
            .map_err(|source| source.to_string())?;
        Ok(Self::map_status(status))
    }
}

impl RadrootsAppBackend for DesktopBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            let status = manager
                .selected_account_status()
                .map_err(|source| source.to_string())?;
            return Ok(Self::map_status(status));
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(IdentityGateState::Unsupported {
                reason: "Local secure onboarding is only implemented for macOS in this slice."
                    .to_owned(),
            })
        }
    }

    fn setup_action_state(&self) -> SetupActionState {
        #[cfg(target_os = "macos")]
        {
            return SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: true,
                pending: false,
            };
        }

        #[cfg(not(target_os = "macos"))]
        {
            SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: false,
                pending: false,
            }
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            manager
                .generate_identity(Some("local".to_owned()), true)
                .map_err(|source| source.to_string())?;
            return self.load_identity_state().map(Some);
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(Some(IdentityGateState::Unsupported {
                reason: "Local secure onboarding is only implemented for macOS in this slice."
                    .to_owned(),
            }))
        }
    }

    fn home_remove_action_state(&self) -> Option<HomeActionState> {
        #[cfg(target_os = "macos")]
        {
            return Some(HomeActionState {
                label: "Remove Key From This Device".to_owned(),
                enabled: true,
                pending: false,
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn request_home_remove_action(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            return Self::remove_selected_local_identity(&manager).map(Some);
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }
}

fn main() -> eframe::Result<()> {
    set_macos_app_name();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(DesktopBackend))))),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::DesktopBackend;
    use radroots_studio_app_apple_security::RadrootsAppleKeychainVault;
    use radroots_identity::RadrootsIdentityId;
    use radroots_nostr_accounts::prelude::RadrootsNostrSecretVault;
    use std::path::PathBuf;

    #[test]
    fn private_directory_chain_covers_only_radroots_subtree() {
        let root = PathBuf::from("/tmp/example/.radroots");
        let leaf = root.join("app").join("desktop").join("nostr");

        let chain = DesktopBackend::private_directory_chain(&root, &leaf).unwrap();

        assert_eq!(
            chain,
            vec![
                PathBuf::from("/tmp/example/.radroots"),
                PathBuf::from("/tmp/example/.radroots/app"),
                PathBuf::from("/tmp/example/.radroots/app/desktop"),
                PathBuf::from("/tmp/example/.radroots/app/desktop/nostr"),
            ]
        );
    }

    #[test]
    fn apple_keychain_vault_round_trips_secret_hex() {
        let vault = RadrootsAppleKeychainVault::new("org.radroots.app.tests.desktop.roundtrip");
        let account_id = RadrootsIdentityId::parse(
            "3bf0c63f0f4478a288f6b67f0429dbf7f5119d4fa7218a4c40ef1378f80f7606",
        )
        .expect("account id");

        let _ = vault.remove_secret(&account_id);

        vault
            .store_secret_hex(
                &account_id,
                "a0468b0f2f5de9db868fb563b13632eb92ec4697dd4fddbdca0488f1a1b2c3d4",
            )
            .expect("store secret");

        assert_eq!(
            vault.load_secret_hex(&account_id).expect("load secret"),
            Some("a0468b0f2f5de9db868fb563b13632eb92ec4697dd4fddbdca0488f1a1b2c3d4".to_owned())
        );

        vault.remove_secret(&account_id).expect("remove secret");
        assert_eq!(
            vault.load_secret_hex(&account_id).expect("load missing"),
            None
        );
    }
}
