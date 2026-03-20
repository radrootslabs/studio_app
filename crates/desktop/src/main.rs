#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use directories::BaseDirs;
use eframe::egui;
use radroots_studio_app_core::{APP_NAME, IdentityGateState, RadrootsApp, RadrootsAppBackend};
#[cfg(target_os = "macos")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore, RadrootsNostrSecretVaultOsKeyring,
    RadrootsNostrSelectedAccountStatus,
};
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
    fn app_data_root() -> Result<std::path::PathBuf, String> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| "failed to resolve home directory".to_owned())?;
        Ok(base_dirs
            .home_dir()
            .join(".radroots")
            .join("app")
            .join("desktop"))
    }

    #[cfg(target_os = "macos")]
    fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
        let accounts_path = Self::app_data_root()?.join("nostr").join("accounts.json");
        if let Some(parent) = accounts_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|source| format!("failed to create accounts directory: {source}"))?;
        }

        let store = Arc::new(RadrootsNostrFileAccountStore::new(accounts_path));
        let vault = Arc::new(RadrootsNostrSecretVaultOsKeyring::new(
            "org.radroots.app.nostr",
        ));
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

    fn generate_new_key(&self) -> Result<IdentityGateState, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            manager
                .generate_identity(Some("local".to_owned()), true)
                .map_err(|source| source.to_string())?;
            return self.load_identity_state();
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(IdentityGateState::Unsupported {
                reason: "Local secure onboarding is only implemented for macOS in this slice."
                    .to_owned(),
            })
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
