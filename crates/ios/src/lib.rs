#![allow(unsafe_code)]

#[cfg(target_os = "ios")]
use eframe::egui::ViewportBuilder;
#[cfg(any(target_os = "ios", test))]
use radroots_studio_app_core::IdentityGateState;
#[cfg(target_os = "ios")]
use radroots_studio_app_core::{APP_NAME, RadrootsApp, RadrootsAppBackend, SetupActionState};
#[cfg(any(target_os = "ios", test))]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrSelectedAccountStatus,
};

#[cfg(any(target_os = "ios", test))]
mod security;
#[cfg(any(target_os = "ios", test))]
mod storage;
#[cfg(any(target_os = "ios", test))]
mod vault;

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
}
