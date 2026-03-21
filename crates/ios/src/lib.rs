#![allow(unsafe_code)]

#[cfg(target_os = "ios")]
use eframe::egui::ViewportBuilder;
#[cfg(target_os = "ios")]
use radroots_studio_app_apple_security::verify_user_presence;
#[cfg(any(target_os = "ios", test))]
use radroots_studio_app_core::IdentityGateState;
#[cfg(target_os = "ios")]
use radroots_studio_app_core::{
    APP_NAME, HomeActionKind, HomeActionResult, HomeActionState, RadrootsApp, RadrootsAppBackend,
    SetupActionState,
};
#[cfg(any(target_os = "ios", test))]
use radroots_identity::RadrootsIdentity;
#[cfg(any(target_os = "ios", test))]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrSelectedAccountStatus,
};
#[cfg(any(target_os = "ios", test))]
use std::path::Path;
#[cfg(any(target_os = "ios", test))]
use zeroize::Zeroizing;

#[cfg(any(target_os = "ios", test))]
mod storage;

#[cfg(any(target_os = "ios", test))]
struct IosBackend;

#[cfg(any(target_os = "ios", test))]
impl IosBackend {
    #[cfg(target_os = "ios")]
    fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
        storage::accounts_manager()
    }

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

    fn identity_state_from_manager(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<IdentityGateState, String> {
        let status = manager
            .selected_account_status()
            .map_err(|source| source.to_string())?;
        Ok(Self::map_status(status))
    }

    fn generate_local_identity(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<IdentityGateState, String> {
        manager
            .generate_identity(Some("local".to_owned()), true)
            .map_err(|source| source.to_string())?;
        Self::identity_state_from_manager(manager)
    }

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
        Self::identity_state_from_manager(manager)
    }

    fn export_selected_local_secret_key(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<String, String> {
        Self::authorize_secret_key_export()?;

        let Some(account_id) = manager
            .selected_account_id()
            .map_err(|source| source.to_string())?
        else {
            return Err("no selected local identity is available to back up".to_owned());
        };

        let Some(secret_key_hex) = manager
            .export_secret_hex(&account_id)
            .map_err(|source| source.to_string())?
        else {
            return Err("selected local identity does not have an exportable secret".to_owned());
        };

        let secret_key_hex = Zeroizing::new(secret_key_hex);
        let identity = RadrootsIdentity::from_secret_key_str(secret_key_hex.as_str())
            .map_err(|source| source.to_string())?;
        Ok(identity.nsec())
    }

    #[cfg(target_os = "ios")]
    fn authorize_secret_key_export() -> Result<(), String> {
        verify_user_presence("reveal the current secret key").map_err(|source| source.to_string())
    }

    #[cfg(not(target_os = "ios"))]
    fn authorize_secret_key_export() -> Result<(), String> {
        Ok(())
    }

    fn remove_all_local_identities(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<IdentityGateState, String> {
        let account_ids = manager
            .list_accounts()
            .map_err(|source| source.to_string())?
            .into_iter()
            .map(|record| record.account_id)
            .collect::<Vec<_>>();

        for account_id in account_ids {
            manager
                .remove_account(&account_id)
                .map_err(|source| source.to_string())?;
        }

        Self::identity_state_from_manager(manager)
    }

    fn remove_accounts_file_if_present(accounts_path: &Path) -> Result<(), String> {
        match std::fs::remove_file(accounts_path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(format!("failed to remove ios accounts file: {source}")),
        }
    }

    #[cfg(target_os = "ios")]
    fn reset_local_device_state(
        manager: &RadrootsNostrAccountsManager,
        accounts_path: &Path,
    ) -> Result<IdentityGateState, String> {
        let state = Self::remove_all_local_identities(manager)?;
        Self::remove_accounts_file_if_present(accounts_path)?;
        Ok(state)
    }
}

#[cfg(target_os = "ios")]
impl RadrootsAppBackend for IosBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        let manager = Self::accounts_manager()?;
        Self::identity_state_from_manager(&manager)
    }

    fn setup_action_state(&self) -> SetupActionState {
        SetupActionState {
            label: "Generate New Key".to_owned(),
            enabled: true,
            pending: false,
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        let manager = Self::accounts_manager()?;
        Self::generate_local_identity(&manager).map(Some)
    }

    fn home_action_states(&self) -> Vec<HomeActionState> {
        vec![
            HomeActionState {
                kind: HomeActionKind::BackupSecretKey,
                label: "Back Up Secret Key".to_owned(),
                enabled: true,
                pending: false,
            },
            HomeActionState {
                kind: HomeActionKind::RemoveLocalKey,
                label: "Remove Key From This Device".to_owned(),
                enabled: true,
                pending: false,
            },
            HomeActionState {
                kind: HomeActionKind::ResetDevice,
                label: "Reset This Device".to_owned(),
                enabled: true,
                pending: false,
            },
        ]
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        let manager = Self::accounts_manager()?;
        match action {
            HomeActionKind::BackupSecretKey => Self::export_selected_local_secret_key(&manager)
                .map(|nsec| HomeActionResult::RevealSecretKey { nsec }),
            HomeActionKind::RemoveLocalKey => {
                Self::remove_selected_local_identity(&manager).map(HomeActionResult::IdentityState)
            }
            HomeActionKind::ResetDevice => {
                let accounts_path = storage::accounts_path()?;
                Self::reset_local_device_state(&manager, accounts_path.as_path())
                    .map(HomeActionResult::IdentityState)
            }
            HomeActionKind::DisconnectSigner => Ok(HomeActionResult::None),
        }
    }
}

#[cfg(target_os = "ios")]
fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_fullscreen(true),
        ..Default::default()
    }
}

#[cfg(target_os = "ios")]
pub fn run() -> Result<(), String> {
    eframe::run_native(
        APP_NAME,
        native_options(),
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(IosBackend))))),
    )
    .map_err(|err| err.to_string())
}

#[cfg(not(target_os = "ios"))]
pub fn run() -> Result<(), String> {
    Err("radroots-app-ios can only launch on an ios target".to_owned())
}

pub const ENTRYPOINT_SYMBOL: &str = "radroots_ios_run";

#[unsafe(no_mangle)]
pub extern "C" fn radroots_ios_run() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;

    #[test]
    fn non_ios_run_is_rejected() {
        #[cfg(not(target_os = "ios"))]
        assert_eq!(
            run(),
            Err("radroots-app-ios can only launch on an ios target".to_owned())
        );
    }

    #[test]
    fn exported_entrypoint_symbol_is_stable() {
        assert_eq!(ENTRYPOINT_SYMBOL, "radroots_ios_run");
    }

    #[test]
    fn new_ios_manager_starts_in_setup_state() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        assert_eq!(
            IosBackend::identity_state_from_manager(&manager),
            Ok(IdentityGateState::Missing)
        );
    }

    #[test]
    fn local_identity_generation_transitions_to_ready() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        let state = IosBackend::generate_local_identity(&manager).expect("generate identity");
        let IdentityGateState::Ready { account_id, npub } = state else {
            panic!("expected ready identity state");
        };

        assert!(!account_id.is_empty());
        assert!(npub.starts_with("npub1"));
    }

    #[test]
    fn local_identity_removal_transitions_back_to_missing() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        IosBackend::generate_local_identity(&manager).expect("generate identity");
        let state =
            IosBackend::remove_selected_local_identity(&manager).expect("remove selected account");

        assert_eq!(state, IdentityGateState::Missing);
        assert_eq!(
            manager.selected_account_id().expect("selected account"),
            None
        );
    }

    #[test]
    fn remove_all_local_identities_clears_every_account() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        manager
            .generate_identity(Some("first".into()), true)
            .expect("generate first");
        manager
            .generate_identity(Some("second".into()), false)
            .expect("generate second");

        let state = IosBackend::remove_all_local_identities(&manager).expect("reset state");

        assert_eq!(state, IdentityGateState::Missing);
        assert_eq!(manager.list_accounts().expect("list accounts").len(), 0);
        assert_eq!(manager.selected_account_id().expect("selected"), None);
    }

    #[test]
    fn export_selected_local_secret_key_returns_nsec() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let nsec = IosBackend::export_selected_local_secret_key(&manager).expect("export secret");

        assert_eq!(nsec, identity.nsec());
        assert!(nsec.starts_with("nsec1"));
    }

    #[test]
    fn remove_accounts_file_if_present_deletes_existing_file() {
        let unique = format!(
            "radroots-ios-reset-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::write(&path, b"{}").expect("write accounts file");

        IosBackend::remove_accounts_file_if_present(path.as_path()).expect("remove file");

        assert!(!path.exists());
    }
}
