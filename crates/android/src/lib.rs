#![allow(unsafe_code)]

#[cfg(target_os = "android")]
use android_logger::Config;
#[cfg(target_os = "android")]
use eframe::egui::ViewportBuilder;
#[cfg(any(target_os = "android", test))]
use radroots_studio_app_core::RadrootsAppBackend;
#[cfg(target_os = "android")]
use radroots_studio_app_core::{APP_NAME, RadrootsApp};
#[cfg(any(target_os = "android", test))]
use radroots_studio_app_core::{
    HomeActionKind, HomeActionResult, HomeActionState, IdentityGateState, SetupActionState,
};
#[cfg(any(target_os = "android", test))]
use radroots_identity::RadrootsIdentity;
#[cfg(test)]
use radroots_nostr_accounts::prelude::RadrootsNostrAccountRecord;
#[cfg(any(target_os = "android", test))]
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
#[cfg(any(target_os = "android", test))]
use radroots_nostr_accounts::prelude::RadrootsNostrSelectedAccountStatus;
#[cfg(any(target_os = "android", test))]
use std::path::Path;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(any(target_os = "android", test))]
use zeroize::Zeroizing;

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

    fn home_action_states(&self) -> Vec<HomeActionState> {
        #[cfg(target_os = "android")]
        {
            let recovery_key_export_pending = Self::recovery_key_export_pending();
            return vec![
                HomeActionState {
                    kind: HomeActionKind::BackupRecoveryKey,
                    label: "Back Up Recovery Key".to_owned(),
                    enabled: !recovery_key_export_pending,
                    pending: recovery_key_export_pending,
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
            ];
        }

        #[cfg(not(target_os = "android"))]
        {
            Vec::new()
        }
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        #[cfg(target_os = "android")]
        {
            return match action {
                HomeActionKind::BackupRecoveryKey => {
                    Self::begin_recovery_key_export().map(|()| HomeActionResult::None)
                }
                HomeActionKind::RemoveLocalKey => {
                    let manager = Self::accounts_manager()?;
                    Self::remove_selected_local_identity(&manager)
                        .map(HomeActionResult::IdentityState)
                }
                HomeActionKind::ResetDevice => {
                    let manager = Self::accounts_manager()?;
                    let accounts_path = storage::accounts_path()?;
                    Self::reset_local_device_state(&manager, accounts_path.as_path())
                        .map(HomeActionResult::IdentityState)
                }
                HomeActionKind::DisconnectSigner => Ok(HomeActionResult::None),
            };
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = action;
            Ok(HomeActionResult::None)
        }
    }

    fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
        #[cfg(target_os = "android")]
        {
            return Self::poll_recovery_key_export();
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

    fn export_selected_local_recovery_key(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<String, String> {
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

    #[cfg(target_os = "android")]
    fn begin_recovery_key_export() -> Result<(), String> {
        security::begin_user_presence_verification("reveal the current recovery key")
            .map_err(|source| source.to_string())
    }

    #[cfg(not(target_os = "android"))]
    fn begin_recovery_key_export() -> Result<(), String> {
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn recovery_key_export_pending() -> bool {
        security::is_user_presence_verification_pending().unwrap_or(false)
    }

    #[cfg(not(target_os = "android"))]
    fn recovery_key_export_pending() -> bool {
        false
    }

    #[cfg(target_os = "android")]
    fn poll_recovery_key_export() -> Result<Option<HomeActionResult>, String> {
        match security::take_user_presence_verification_result()
            .map_err(|source| source.to_string())?
        {
            Some(security::AndroidUserPresenceVerificationResult::Verified) => {
                let manager = Self::accounts_manager()?;
                Self::export_selected_local_recovery_key(&manager)
                    .map(|nsec| Some(HomeActionResult::RevealRecoveryKey { nsec }))
            }
            Some(security::AndroidUserPresenceVerificationResult::Failed(message)) => Err(message),
            None => Ok(None),
        }
    }

    #[cfg(not(target_os = "android"))]
    fn poll_recovery_key_export() -> Result<Option<HomeActionResult>, String> {
        Ok(None)
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
            Err(source) => Err(format!("failed to remove android accounts file: {source}")),
        }
    }

    #[cfg(target_os = "android")]
    fn reset_local_device_state(
        manager: &RadrootsNostrAccountsManager,
        accounts_path: &Path,
    ) -> Result<IdentityGateState, String> {
        let state = Self::remove_all_local_identities(manager)?;
        Self::remove_accounts_file_if_present(accounts_path)?;
        Ok(state)
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

    #[test]
    fn remove_all_local_identities_clears_every_account() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();

        manager
            .generate_identity(Some("first".into()), true)
            .expect("generate first");
        manager
            .generate_identity(Some("second".into()), false)
            .expect("generate second");

        let state = AndroidBackend::remove_all_local_identities(&manager).expect("reset state");

        assert_eq!(state, IdentityGateState::Missing);
        assert_eq!(manager.list_accounts().expect("list accounts").len(), 0);
        assert_eq!(manager.selected_account_id().expect("selected"), None);
    }

    #[test]
    fn export_selected_local_recovery_key_returns_nsec() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let nsec =
            AndroidBackend::export_selected_local_recovery_key(&manager).expect("export recovery");

        assert_eq!(nsec, identity.nsec());
        assert!(nsec.starts_with("nsec1"));
    }

    #[test]
    fn remove_accounts_file_if_present_deletes_existing_file() {
        let unique = format!(
            "radroots-android-reset-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::write(&path, b"{}").expect("write accounts file");

        AndroidBackend::remove_accounts_file_if_present(path.as_path()).expect("remove file");

        assert!(!path.exists());
    }
}
