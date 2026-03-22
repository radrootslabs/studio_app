#[cfg(target_os = "ios")]
use eframe::egui::ViewportBuilder;
#[cfg(target_os = "ios")]
use radroots_studio_app_apple_security::verify_user_presence;
#[cfg(any(target_os = "ios", test))]
use radroots_studio_app_core::IdentityGateState;
#[cfg(target_os = "ios")]
use radroots_studio_app_core::{
    APP_NAME, HomeActionKind, HomeActionResult, HomeActionState, ImportActionState,
    PasteActionState, RadrootsApp, RadrootsAppBackend, RadrootsLocationCountry,
    RadrootsLocationCountryCenterLookupResult, RadrootsLocationCountryListResult,
    RadrootsLocationPoint, RadrootsLocationResolverError, RadrootsLocationReverseOptions,
    RadrootsOfflineGeocoderPlatform, RadrootsOfflineGeocoderState,
    RadrootsOfflineGeocoderUnavailableKind, RadrootsResolvedLocation,
    RadrootsReverseLocationLookupResult, SetupActionState,
};
#[cfg(any(target_os = "ios", test))]
use radroots_studio_app_core::{RadrootsAccountCustody, RadrootsAccountSummary};
#[cfg(any(target_os = "ios", test))]
use radroots_studio_app_core::{RadrootsSecretImportMode, RadrootsSecretImportRequest};
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
mod country_lookup;
#[cfg(any(target_os = "ios", test))]
mod offline_geocoder;
#[cfg(target_os = "ios")]
mod remote_signer;
#[cfg(any(target_os = "ios", test))]
mod reverse_lookup;
#[cfg(any(target_os = "ios", test))]
mod storage;

#[cfg(any(target_os = "ios", test))]
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
struct IosBackend {
    country_lookup: country_lookup::IosCountryLookup,
    offline_geocoder: offline_geocoder::IosOfflineGeocoder,
    #[cfg(target_os = "ios")]
    remote_signer: remote_signer::IosRemoteSigner,
    reverse_lookup: reverse_lookup::IosReverseLookup,
}

#[cfg(target_os = "ios")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn radroots_ios_clipboard_text_copy() -> *mut std::ffi::c_char;
    fn radroots_ios_string_free(value: *mut std::ffi::c_char);
}

#[cfg(any(target_os = "ios", test))]
impl IosBackend {
    #[cfg(target_os = "ios")]
    fn new() -> Self {
        let offline_geocoder = match storage::app_data_root() {
            Ok(app_data_root) => offline_geocoder::IosOfflineGeocoder::start(app_data_root),
            Err(debug_message) => offline_geocoder::IosOfflineGeocoder::from_state(
                RadrootsOfflineGeocoderState::unavailable(
                    RadrootsOfflineGeocoderUnavailableKind::InternalError,
                    RadrootsOfflineGeocoderPlatform::Ios,
                    debug_message,
                ),
            ),
        };

        Self {
            country_lookup: country_lookup::IosCountryLookup::new(),
            offline_geocoder,
            #[cfg(target_os = "ios")]
            remote_signer: remote_signer::IosRemoteSigner::new(),
            reverse_lookup: reverse_lookup::IosReverseLookup::new(),
        }
    }

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

    fn account_roster_from_manager(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<Vec<RadrootsAccountSummary>, String> {
        manager
            .list_accounts()
            .map_err(|source| source.to_string())?
            .into_iter()
            .map(|record| {
                #[cfg(target_os = "ios")]
                let custody = remote_signer::custody_for_account_id(record.account_id.as_str())?;
                #[cfg(not(target_os = "ios"))]
                let custody = RadrootsAccountCustody::LocalManaged;
                Ok(RadrootsAccountSummary {
                    account_id: record.account_id.to_string(),
                    npub: record.public_identity.public_key_npub,
                    label: record.label,
                    custody,
                })
            })
            .collect()
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

    fn export_selected_local_encrypted_secret_key(
        manager: &RadrootsNostrAccountsManager,
        password: &str,
    ) -> Result<String, String> {
        Self::authorize_secret_key_backup()?;

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
        identity
            .encrypt_secret_key_ncryptsec(password)
            .map_err(|source| source.to_string())
    }

    fn export_selected_local_raw_secret_key(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<String, String> {
        Self::authorize_secret_key_reveal()?;

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

    fn import_local_identity(
        manager: &RadrootsNostrAccountsManager,
        request: &RadrootsSecretImportRequest,
    ) -> Result<IdentityGateState, String> {
        let identity = match request.mode {
            RadrootsSecretImportMode::EncryptedSecretKey => {
                let Some(password) = request.password.as_deref() else {
                    return Err("password is required to import an encrypted secret key".to_owned());
                };
                RadrootsIdentity::from_encrypted_secret_key_str(
                    request.secret_text.as_str(),
                    password,
                )
                .map_err(|_| "invalid encrypted secret key or password".to_owned())?
            }
            RadrootsSecretImportMode::RawSecretKey => {
                RadrootsIdentity::from_secret_key_str(request.secret_text.as_str())
                    .map_err(|_| "invalid raw secret key".to_owned())?
            }
        };

        manager
            .upsert_identity(&identity, None, true)
            .map_err(|source| source.to_string())?;

        Self::identity_state_from_manager(manager)
    }

    fn normalize_clipboard_secret_key_text(clipboard_text: &str) -> Result<String, String> {
        let trimmed = clipboard_text.trim();
        if trimmed.is_empty() {
            return Err("clipboard does not contain text".to_owned());
        }

        Ok(match trimmed.len() == clipboard_text.len() {
            true => clipboard_text.to_owned(),
            false => trimmed.to_owned(),
        })
    }

    #[cfg(target_os = "ios")]
    #[allow(unsafe_code)]
    fn paste_secret_key_from_clipboard() -> Result<String, String> {
        let clipboard_text_ptr = unsafe { radroots_ios_clipboard_text_copy() };
        if clipboard_text_ptr.is_null() {
            return Err("clipboard does not contain text".to_owned());
        }

        let clipboard_text = unsafe {
            let value = std::ffi::CStr::from_ptr(clipboard_text_ptr)
                .to_string_lossy()
                .into_owned();
            radroots_ios_string_free(clipboard_text_ptr);
            value
        };

        Self::normalize_clipboard_secret_key_text(&clipboard_text)
    }

    #[cfg(target_os = "ios")]
    fn authorize_secret_key_reveal() -> Result<(), String> {
        verify_user_presence("reveal the current secret key").map_err(|source| source.to_string())
    }

    #[cfg(target_os = "ios")]
    fn authorize_secret_key_backup() -> Result<(), String> {
        verify_user_presence("back up the current secret key").map_err(|source| source.to_string())
    }

    #[cfg(not(target_os = "ios"))]
    fn authorize_secret_key_reveal() -> Result<(), String> {
        Ok(())
    }

    #[cfg(not(target_os = "ios"))]
    fn authorize_secret_key_backup() -> Result<(), String> {
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
        let status = manager
            .selected_account_status()
            .map_err(|source| source.to_string())?;
        remote_signer::identity_state_from_status(status)
    }

    fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
        let manager = Self::accounts_manager()?;
        Self::account_roster_from_manager(&manager)
    }

    fn offline_geocoder_state(&self) -> Option<RadrootsOfflineGeocoderState> {
        Some(self.offline_geocoder.current_state())
    }

    fn poll_offline_geocoder_state(&self) -> Result<Option<RadrootsOfflineGeocoderState>, String> {
        Ok(self.offline_geocoder.take_update())
    }

    fn reverse_location(
        &self,
        point: RadrootsLocationPoint,
        options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError> {
        #[cfg(target_os = "ios")]
        {
            let app_data_root = storage::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::reverse_location(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
                point,
                options,
            );
        }

        #[cfg(not(target_os = "ios"))]
        {
            let _ = (point, options);
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn request_reverse_location_lookup(
        &self,
        point: RadrootsLocationPoint,
        options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<(), RadrootsLocationResolverError> {
        let app_data_root = storage::app_data_root()
            .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
        self.reverse_lookup.begin(
            app_data_root,
            self.offline_geocoder.current_state(),
            point,
            options,
        )
    }

    fn poll_reverse_location_lookup_result(
        &self,
    ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
        Ok(self.reverse_lookup.take_update())
    }

    fn request_location_country_list(&self) -> Result<(), RadrootsLocationResolverError> {
        let app_data_root = storage::app_data_root()
            .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
        self.country_lookup
            .begin_list(app_data_root, self.offline_geocoder.current_state())
    }

    fn poll_location_country_list_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryListResult>, String> {
        Ok(self.country_lookup.take_list_update())
    }

    fn request_location_country_center_lookup(
        &self,
        country_id: &str,
    ) -> Result<(), RadrootsLocationResolverError> {
        let app_data_root = storage::app_data_root()
            .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
        self.country_lookup.begin_center(
            app_data_root,
            self.offline_geocoder.current_state(),
            country_id.to_owned(),
        )
    }

    fn poll_location_country_center_lookup_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryCenterLookupResult>, String> {
        Ok(self.country_lookup.take_center_update())
    }

    fn list_location_countries(
        &self,
    ) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
        #[cfg(target_os = "ios")]
        {
            let app_data_root = storage::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::list_countries(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
            );
        }

        #[cfg(not(target_os = "ios"))]
        {
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn location_country_center(
        &self,
        country_id: &str,
    ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
        #[cfg(target_os = "ios")]
        {
            let app_data_root = storage::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::country_center(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
                country_id,
            );
        }

        #[cfg(not(target_os = "ios"))]
        {
            let _ = country_id;
            Err(RadrootsLocationResolverError::Unsupported)
        }
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

    fn home_setup_action_state(&self) -> Option<SetupActionState> {
        Some(self.setup_action_state())
    }

    fn request_home_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        self.request_setup_action()
    }

    fn import_action_state(&self) -> Option<ImportActionState> {
        Some(ImportActionState {
            label: "Import Secret Key".to_owned(),
            enabled: true,
            pending: false,
        })
    }

    fn request_import_action(
        &self,
        request: &RadrootsSecretImportRequest,
    ) -> Result<Option<IdentityGateState>, String> {
        let manager = Self::accounts_manager()?;
        Self::import_local_identity(&manager, request).map(Some)
    }

    fn request_select_account(
        &self,
        account_id: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        let manager = Self::accounts_manager()?;
        let account_id = radroots_identity::RadrootsIdentityId::try_from(account_id)
            .map_err(|_| "invalid account id".to_owned())?;
        manager
            .select_account(&account_id)
            .map_err(|source| source.to_string())?;
        self.load_identity_state().map(Some)
    }

    fn remote_signer_action_state(&self) -> Option<SetupActionState> {
        Some(
            self.remote_signer
                .action_state()
                .unwrap_or_else(|_| SetupActionState {
                    label: "Connect Remote Signer".to_owned(),
                    enabled: !self.remote_signer.is_connecting(),
                    pending: self.remote_signer.is_connecting(),
                }),
        )
    }

    fn preview_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<radroots_studio_app_core::RadrootsRemoteSignerPreview, String> {
        remote_signer::preview_connection(input)
    }

    fn request_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        self.remote_signer.begin_connect(input)?;
        Ok(None)
    }

    fn pending_remote_signer_connection(
        &self,
    ) -> Result<Option<radroots_studio_app_core::RadrootsPendingRemoteSignerConnection>, String> {
        remote_signer::pending_connection()
    }

    fn request_cancel_pending_remote_signer_connection(&self) -> Result<(), String> {
        remote_signer::cancel_pending_connection()
    }

    fn import_paste_action_state(&self) -> Option<PasteActionState> {
        Some(PasteActionState {
            label: "Paste Secret Key".to_owned(),
            enabled: true,
            pending: false,
        })
    }

    fn request_import_paste_action(&self) -> Result<Option<String>, String> {
        Self::paste_secret_key_from_clipboard().map(Some)
    }

    fn home_action_states(&self) -> Vec<HomeActionState> {
        let Ok(manager) = Self::accounts_manager() else {
            return Vec::new();
        };
        let Ok(status) = manager
            .selected_account_status()
            .map_err(|source| source.to_string())
        else {
            return Vec::new();
        };

        match status {
            RadrootsNostrSelectedAccountStatus::NotConfigured => Vec::new(),
            RadrootsNostrSelectedAccountStatus::PublicOnly { account } => {
                if matches!(
                    remote_signer::custody_for_account_id(account.account_id.as_str()),
                    Ok(RadrootsAccountCustody::RemoteSigner)
                ) {
                    vec![HomeActionState {
                        kind: HomeActionKind::DisconnectSigner,
                        label: "Disconnect Remote Signer".to_owned(),
                        enabled: true,
                        pending: false,
                    }]
                } else {
                    Vec::new()
                }
            }
            RadrootsNostrSelectedAccountStatus::Ready { .. } => vec![
                HomeActionState {
                    kind: HomeActionKind::BackupSecretKey,
                    label: "Back Up Secret Key".to_owned(),
                    enabled: true,
                    pending: false,
                },
                HomeActionState {
                    kind: HomeActionKind::RevealRawSecretKey,
                    label: "Reveal Raw Secret Key".to_owned(),
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
            ],
        }
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        let manager = Self::accounts_manager()?;
        match action {
            HomeActionKind::BackupSecretKey => Ok(HomeActionResult::None),
            HomeActionKind::RevealRawSecretKey => {
                Self::export_selected_local_raw_secret_key(&manager)
                    .map(|nsec| HomeActionResult::RevealRawSecretKey { nsec })
            }
            HomeActionKind::RemoveLocalKey => {
                Self::remove_selected_local_identity(&manager).map(HomeActionResult::IdentityState)
            }
            HomeActionKind::ResetDevice => {
                let accounts_path = storage::accounts_path()?;
                Self::reset_local_device_state(&manager, accounts_path.as_path())
                    .map(HomeActionResult::IdentityState)
            }
            HomeActionKind::DisconnectSigner => {
                remote_signer::disconnect_selected_remote_signer(&manager)
                    .map(HomeActionResult::IdentityState)
            }
        }
    }

    fn request_secret_key_backup_action(&self, password: &str) -> Result<HomeActionResult, String> {
        let manager = Self::accounts_manager()?;
        Self::export_selected_local_encrypted_secret_key(&manager, password)
            .map(|ncryptsec| HomeActionResult::RevealEncryptedSecretKey { ncryptsec })
    }

    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        self.remote_signer
            .take_update()
            .transpose()
            .map(|state| state.flatten())
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
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(IosBackend::new()))))),
    )
    .map_err(|err| err.to_string())
}

#[cfg(not(target_os = "ios"))]
pub fn run() -> Result<(), String> {
    Err("radroots-app-ios can only launch on an ios target".to_owned())
}

pub const ENTRYPOINT_SYMBOL: &str = "radroots_ios_run";

#[allow(unsafe_code)]
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
    use radroots_studio_app_test_support::{
        FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD, fixture_identity_ncryptsec,
    };
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
        let IdentityGateState::Ready { account_id } = state else {
            panic!("expected ready identity state");
        };

        assert!(!account_id.is_empty());
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
    fn export_selected_local_raw_secret_key_returns_nsec() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let nsec =
            IosBackend::export_selected_local_raw_secret_key(&manager).expect("export secret");

        assert_eq!(nsec, identity.nsec());
        assert!(nsec.starts_with("nsec1"));
    }

    #[test]
    fn export_selected_local_encrypted_secret_key_returns_ncryptsec() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let fixture_identity =
            RadrootsIdentity::from_secret_key_str(FIXTURE_ALICE.secret_key_hex).expect("fixture");

        manager
            .upsert_identity(&fixture_identity, Some("primary".into()), true)
            .expect("store identity");

        let ncryptsec = IosBackend::export_selected_local_encrypted_secret_key(
            &manager,
            FIXTURE_BACKUP_PASSWORD,
        )
        .expect("export encrypted secret");

        let restored = RadrootsIdentity::from_encrypted_secret_key_str(
            ncryptsec.as_str(),
            FIXTURE_BACKUP_PASSWORD,
        )
        .expect("restore encrypted secret");

        assert_eq!(restored.secret_key_hex(), FIXTURE_ALICE.secret_key_hex);
    }

    #[test]
    fn import_local_identity_imports_raw_secret_key_and_selects_account() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        let state = IosBackend::import_local_identity(
            &manager,
            &RadrootsSecretImportRequest {
                mode: RadrootsSecretImportMode::RawSecretKey,
                secret_text: identity.nsec(),
                password: None,
            },
        )
        .expect("import");

        assert_eq!(
            state,
            IdentityGateState::Ready {
                account_id: identity.id().to_string(),
            }
        );
        assert_eq!(
            manager.selected_account_id().expect("selected"),
            Some(identity.id())
        );
        assert_eq!(manager.list_accounts().expect("list").len(), 1);
        assert_eq!(
            manager
                .export_secret_hex(&identity.id())
                .expect("export secret"),
            Some(identity.secret_key_hex())
        );
    }

    #[test]
    fn import_local_identity_imports_encrypted_secret_key_and_selects_account() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let encrypted_secret_key =
            fixture_identity_ncryptsec(&FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD)
                .expect("fixture encrypted secret key");
        let fixture_identity =
            RadrootsIdentity::from_secret_key_str(FIXTURE_ALICE.secret_key_hex).expect("fixture");
        let fixture_account_id = fixture_identity.id();

        let state = IosBackend::import_local_identity(
            &manager,
            &RadrootsSecretImportRequest {
                mode: RadrootsSecretImportMode::EncryptedSecretKey,
                secret_text: encrypted_secret_key,
                password: Some(FIXTURE_BACKUP_PASSWORD.to_owned()),
            },
        )
        .expect("import");

        assert_eq!(
            state,
            IdentityGateState::Ready {
                account_id: fixture_account_id.to_string(),
            }
        );
        assert_eq!(
            manager.selected_account_id().expect("selected"),
            Some(fixture_account_id)
        );
        assert_eq!(manager.list_accounts().expect("list").len(), 1);
        assert_eq!(
            manager
                .export_secret_hex(&fixture_identity.id())
                .expect("export secret"),
            Some(FIXTURE_ALICE.secret_key_hex.to_owned())
        );
    }

    #[test]
    fn account_roster_from_manager_lists_local_managed_account() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let roster = IosBackend::account_roster_from_manager(&manager).expect("account roster");

        assert_eq!(roster.len(), 1);
        assert_eq!(roster[0].account_id, identity.id().to_string());
        assert_eq!(roster[0].npub, identity.npub());
        assert_eq!(roster[0].label.as_deref(), Some("primary"));
        assert_eq!(roster[0].custody, RadrootsAccountCustody::LocalManaged);
    }

    #[test]
    fn normalize_clipboard_secret_key_text_trims_wrapping_whitespace() {
        let clipboard_text = format!("  {} \n", FIXTURE_ALICE.nsec);
        let normalized = IosBackend::normalize_clipboard_secret_key_text(clipboard_text.as_str())
            .expect("normalize secret key");

        assert_eq!(normalized, FIXTURE_ALICE.nsec);
    }

    #[test]
    fn normalize_clipboard_secret_key_text_rejects_blank_text() {
        assert_eq!(
            IosBackend::normalize_clipboard_secret_key_text(" \n\t"),
            Err("clipboard does not contain text".to_owned())
        );
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
