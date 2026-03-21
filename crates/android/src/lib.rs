#![allow(unsafe_code)]

#[cfg(target_os = "android")]
use android_logger::Config;
#[cfg(target_os = "android")]
use eframe::egui::ViewportBuilder;
#[cfg(any(target_os = "android", test))]
use radroots_studio_app_core::RadrootsAppBackend;
#[cfg(any(target_os = "android", test))]
use radroots_studio_app_core::{HomeActionState, IdentityGateState, SetupActionState};
#[cfg(target_os = "android")]
use radroots_studio_app_core::{RadrootsApp, APP_NAME};
#[cfg(test)]
use radroots_identity::RadrootsIdentity;
#[cfg(test)]
use radroots_nostr_accounts::prelude::RadrootsNostrAccountRecord;
#[cfg(any(target_os = "android", test))]
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
#[cfg(any(target_os = "android", test))]
use radroots_nostr_accounts::prelude::RadrootsNostrSelectedAccountStatus;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(any(target_os = "android", test))]
mod security;
#[cfg(any(target_os = "android", test))]
mod storage;
#[cfg(any(target_os = "android", test))]
mod vault;

#[cfg(any(target_os = "android", test))]
struct AndroidBackend;

#[cfg(any(target_os = "android", test))]
impl RadrootsAppBackend for AndroidBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            return Self::identity_state_from_manager(&manager);
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(Self::unsupported_identity_state())
        }
    }

    fn setup_action_state(&self) -> SetupActionState {
        #[cfg(target_os = "android")]
        {
            return Self::enabled_setup_action_state();
        }

        #[cfg(not(target_os = "android"))]
        {
            Self::unsupported_setup_action_state()
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            return Self::generate_local_identity(&manager).map(Some);
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(Some(Self::unsupported_identity_state()))
        }
    }

    fn home_remove_action_state(&self) -> Option<HomeActionState> {
        #[cfg(target_os = "android")]
        {
            return Some(HomeActionState {
                label: "Remove Key From This Device".to_owned(),
                enabled: true,
                pending: false,
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }

    fn request_home_remove_action(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            return Self::remove_selected_local_identity(&manager).map(Some);
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(None)
        }
    }
}

#[cfg(any(target_os = "android", test))]
impl AndroidBackend {
    #[cfg(target_os = "android")]
    fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
        #[cfg(target_os = "android")]
        {
            return storage::accounts_manager();
        }
    }

    #[cfg(test)]
    fn unsupported_identity_state() -> IdentityGateState {
        IdentityGateState::Unsupported {
            reason: ANDROID_SETUP_UNAVAILABLE_REASON.to_owned(),
        }
    }

    #[cfg(test)]
    fn unsupported_setup_action_state() -> SetupActionState {
        SetupActionState {
            label: "Generate New Key".to_owned(),
            enabled: false,
            pending: false,
        }
    }

    fn enabled_setup_action_state() -> SetupActionState {
        SetupActionState {
            label: "Generate New Key".to_owned(),
            enabled: true,
            pending: false,
        }
    }

    fn map_status(status: RadrootsNostrSelectedAccountStatus) -> IdentityGateState {
        match status {
            RadrootsNostrSelectedAccountStatus::Ready { account } => IdentityGateState::Ready {
                account_id: account.account_id.to_string(),
                npub: account.public_identity.public_key_npub,
            },
            RadrootsNostrSelectedAccountStatus::NotConfigured
            | RadrootsNostrSelectedAccountStatus::PublicOnly { .. } => IdentityGateState::Missing,
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
}

#[cfg(any(target_os = "android", test))]
#[cfg(test)]
const ANDROID_SETUP_UNAVAILABLE_REASON: &str = "Secure onboarding is not yet available on Android.";

#[cfg(target_os = "android")]
fn native_options(android_app: AndroidApp) -> eframe::NativeOptions {
    eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        android_app: Some(android_app),
        viewport: ViewportBuilder::default().with_title(APP_NAME),
        ..Default::default()
    }
}

#[cfg(target_os = "android")]
fn run_android_app(android_app: AndroidApp) -> Result<(), String> {
    android_logger::init_once(Config::default().with_max_level(log::LevelFilter::Info));
    eframe::run_native(
        APP_NAME,
        native_options(android_app),
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(AndroidBackend))))),
    )
    .map_err(|err| err.to_string())
}

#[cfg(target_os = "android")]
#[allow(improper_ctypes_definitions)]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(android_app: AndroidApp) {
    if let Err(err) = run_android_app(android_app) {
        log::error!("android launcher failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_backend_reports_android_disabled_state_off_target() {
        assert_eq!(
            AndroidBackend::unsupported_identity_state(),
            IdentityGateState::Unsupported {
                reason: ANDROID_SETUP_UNAVAILABLE_REASON.to_owned(),
            }
        );
        assert_eq!(
            AndroidBackend::unsupported_setup_action_state(),
            SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: false,
                pending: false,
            }
        );
    }

    #[test]
    fn android_backend_enables_setup_action_when_android_keygen_is_wired() {
        assert_eq!(
            AndroidBackend::enabled_setup_action_state(),
            SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: true,
                pending: false,
            }
        );
    }

    #[test]
    fn android_backend_maps_ready_account_to_ready_state() {
        let identity = RadrootsIdentity::generate();
        let account =
            RadrootsNostrAccountRecord::new(identity.to_public(), Some("local".into()), 0);

        let state = AndroidBackend::map_status(RadrootsNostrSelectedAccountStatus::Ready {
            account: account.clone(),
        });

        assert_eq!(
            state,
            IdentityGateState::Ready {
                account_id: account.account_id.to_string(),
                npub: account.public_identity.public_key_npub,
            }
        );
    }

    #[test]
    fn android_backend_maps_fresh_and_public_only_accounts_to_missing() {
        let public_only_identity = RadrootsIdentity::generate();
        let public_only_account =
            RadrootsNostrAccountRecord::new(public_only_identity.to_public(), None, 0);

        assert_eq!(
            AndroidBackend::map_status(RadrootsNostrSelectedAccountStatus::NotConfigured),
            IdentityGateState::Missing
        );
        assert_eq!(
            AndroidBackend::map_status(RadrootsNostrSelectedAccountStatus::PublicOnly {
                account: public_only_account,
            }),
            IdentityGateState::Missing
        );
    }

    #[test]
    fn fresh_android_manager_starts_in_setup_state() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        assert_eq!(
            AndroidBackend::identity_state_from_manager(&manager),
            Ok(IdentityGateState::Missing)
        );
    }

    #[test]
    fn local_identity_generation_transitions_android_to_ready() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        let state = AndroidBackend::generate_local_identity(&manager).expect("generate identity");
        let IdentityGateState::Ready { account_id, npub } = state else {
            panic!("expected ready identity state");
        };

        assert!(!account_id.is_empty());
        assert!(npub.starts_with("npub1"));
    }

    #[test]
    fn local_identity_removal_transitions_android_back_to_missing() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        AndroidBackend::generate_local_identity(&manager).expect("generate identity");
        let state = AndroidBackend::remove_selected_local_identity(&manager)
            .expect("remove selected account");

        assert_eq!(state, IdentityGateState::Missing);
        assert_eq!(
            manager.selected_account_id().expect("selected account"),
            None
        );
    }
}
