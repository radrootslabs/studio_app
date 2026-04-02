#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use directories::BaseDirs;
use eframe::egui;
use image::ImageFormat;
#[cfg(all(target_os = "macos", not(test)))]
use radroots_studio_app_apple_security::verify_user_presence;
#[cfg(target_os = "macos")]
use radroots_studio_app_apple_security::{APPLE_NOSTR_SERVICE, RadrootsAppleKeychainVault};
use radroots_studio_app_core::{
    APP_NAME, HomeActionKind, HomeActionResult, HomeActionState, IdentityGateState,
    ImportActionState, RadrootsAccountCustody, RadrootsAccountSummary, RadrootsApp,
    RadrootsAppBackend, RadrootsLocationCountry, RadrootsLocationCountryCenterLookupResult,
    RadrootsLocationCountryListResult, RadrootsLocationPoint, RadrootsLocationResolverError,
    RadrootsLocationReverseOptions, RadrootsOfflineGeocoderPlatform, RadrootsOfflineGeocoderState,
    RadrootsOfflineGeocoderUnavailableKind, RadrootsResolvedLocation,
    RadrootsReverseLocationLookupResult, RadrootsSecretImportMode, RadrootsSecretImportRequest,
    SetupActionState,
};
#[cfg(target_os = "macos")]
use radroots_identity::RadrootsIdentity;
#[cfg(target_os = "macos")]
use radroots_nostr_accounts::prelude::{
    RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore, RadrootsNostrSelectedAccountStatus,
};
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(target_os = "macos")]
use zeroize::Zeroizing;

mod country_lookup;
mod offline_geocoder;
#[cfg(target_os = "macos")]
mod remote_signer;
mod reverse_lookup;

use country_lookup::DesktopCountryLookup;
use offline_geocoder::DesktopOfflineGeocoder;
#[cfg(target_os = "macos")]
use remote_signer::DesktopRemoteSigner;
use reverse_lookup::DesktopReverseLookup;

const RADROOTS_DESKTOP_ICON_BYTES: &[u8] = include_bytes!("../assets/icons/radroots-logo.ico");

#[cfg(target_os = "macos")]
fn set_macos_app_name() {
    use objc2_foundation::{NSProcessInfo, NSString};

    let process_info = NSProcessInfo::processInfo();
    let process_name = NSString::from_str(APP_NAME);
    process_info.setProcessName(&process_name);
}

#[cfg(not(target_os = "macos"))]
fn set_macos_app_name() {}

fn desktop_icon() -> Option<egui::IconData> {
    let image =
        image::load_from_memory_with_format(RADROOTS_DESKTOP_ICON_BYTES, ImageFormat::Ico).ok()?;
    let image = image.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

struct DesktopBackend {
    country_lookup: DesktopCountryLookup,
    offline_geocoder: DesktopOfflineGeocoder,
    #[cfg(target_os = "macos")]
    remote_signer: DesktopRemoteSigner,
    reverse_lookup: DesktopReverseLookup,
}

impl DesktopBackend {
    fn new() -> Self {
        #[cfg(target_os = "macos")]
        let offline_geocoder = match Self::app_data_root() {
            Ok(app_data_root) => DesktopOfflineGeocoder::start(app_data_root),
            Err(debug_message) => {
                DesktopOfflineGeocoder::from_state(RadrootsOfflineGeocoderState::unavailable(
                    RadrootsOfflineGeocoderUnavailableKind::InternalError,
                    RadrootsOfflineGeocoderPlatform::Desktop,
                    debug_message,
                ))
            }
        };

        #[cfg(not(target_os = "macos"))]
        let offline_geocoder =
            DesktopOfflineGeocoder::from_state(RadrootsOfflineGeocoderState::unavailable(
                RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                RadrootsOfflineGeocoderPlatform::Desktop,
                "desktop offline geocoder initialization is only wired for macos",
            ));

        Self {
            country_lookup: DesktopCountryLookup::new(),
            offline_geocoder,
            #[cfg(target_os = "macos")]
            remote_signer: DesktopRemoteSigner::new(),
            reverse_lookup: DesktopReverseLookup::new(),
        }
    }

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
    fn accounts_path() -> Result<PathBuf, String> {
        Ok(Self::app_data_root()?.join("nostr").join("accounts.json"))
    }

    #[cfg(target_os = "macos")]
    fn accounts_manager() -> Result<RadrootsNostrAccountsManager, String> {
        let accounts_path = Self::accounts_path()?;
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
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn account_roster_from_manager(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<Vec<RadrootsAccountSummary>, String> {
        manager
            .list_accounts()
            .map_err(|source| source.to_string())?
            .into_iter()
            .map(|record| {
                let custody = remote_signer::custody_for_account_id(record.account_id.as_str())?;
                Ok(RadrootsAccountSummary {
                    account_id: record.account_id.to_string(),
                    npub: record.public_identity.public_key_npub,
                    label: record.label,
                    custody,
                })
            })
            .collect()
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

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
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

    #[cfg(all(target_os = "macos", not(test)))]
    fn authorize_secret_key_reveal() -> Result<(), String> {
        verify_user_presence("reveal the current secret key").map_err(|source| source.to_string())
    }

    #[cfg(any(not(target_os = "macos"), test))]
    fn authorize_secret_key_reveal() -> Result<(), String> {
        Ok(())
    }

    #[cfg(all(target_os = "macos", not(test)))]
    fn authorize_secret_key_backup() -> Result<(), String> {
        verify_user_presence("back up the current secret key").map_err(|source| source.to_string())
    }

    #[cfg(any(not(target_os = "macos"), test))]
    fn authorize_secret_key_backup() -> Result<(), String> {
        Ok(())
    }

    #[cfg(target_os = "macos")]
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

        let status = manager
            .selected_account_status()
            .map_err(|source| source.to_string())?;
        Ok(Self::map_status(status))
    }

    #[cfg(target_os = "macos")]
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

        let status = manager
            .selected_account_status()
            .map_err(|source| source.to_string())?;
        Ok(Self::map_status(status))
    }

    #[cfg(target_os = "macos")]
    fn remove_accounts_file_if_present(accounts_path: &Path) -> Result<(), String> {
        match std::fs::remove_file(accounts_path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(format!("failed to remove accounts file: {source}")),
        }
    }

    #[cfg(target_os = "macos")]
    fn reset_local_device_state(
        manager: &RadrootsNostrAccountsManager,
        accounts_path: &Path,
    ) -> Result<IdentityGateState, String> {
        remote_signer::purge_all_custody_state()?;
        let state = Self::remove_all_local_identities(manager)?;
        Self::remove_accounts_file_if_present(accounts_path)?;
        Ok(state)
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
            return remote_signer::identity_state_from_status(status);
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(IdentityGateState::Unsupported {
                reason: "Local secure onboarding is only implemented for macOS in this slice."
                    .to_owned(),
            })
        }
    }

    fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            return Self::account_roster_from_manager(&manager);
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(Vec::new())
        }
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
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::reverse_location(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
                point,
                options,
            );
        }

        #[cfg(not(target_os = "macos"))]
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
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return self.reverse_lookup.begin(
                app_data_root,
                self.offline_geocoder.current_state(),
                point,
                options,
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (point, options);
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn poll_reverse_location_lookup_result(
        &self,
    ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
        Ok(self.reverse_lookup.take_update())
    }

    fn request_location_country_list(&self) -> Result<(), RadrootsLocationResolverError> {
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return self
                .country_lookup
                .begin_list(app_data_root, self.offline_geocoder.current_state());
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(RadrootsLocationResolverError::Unsupported)
        }
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
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return self.country_lookup.begin_center(
                app_data_root,
                self.offline_geocoder.current_state(),
                country_id.to_owned(),
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = country_id;
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn poll_location_country_center_lookup_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryCenterLookupResult>, String> {
        Ok(self.country_lookup.take_center_update())
    }

    fn list_location_countries(
        &self,
    ) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::list_countries(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn location_country_center(
        &self,
        country_id: &str,
    ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
        #[cfg(target_os = "macos")]
        {
            let app_data_root = Self::app_data_root()
                .map_err(|message| RadrootsLocationResolverError::QueryFailed { message })?;
            return offline_geocoder::country_center(
                app_data_root.as_path(),
                &self.offline_geocoder.current_state(),
                country_id,
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = country_id;
            Err(RadrootsLocationResolverError::Unsupported)
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

    fn home_setup_action_state(&self) -> Option<SetupActionState> {
        Some(self.setup_action_state())
    }

    fn request_home_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        self.request_setup_action()
    }

    fn import_action_state(&self) -> Option<ImportActionState> {
        #[cfg(target_os = "macos")]
        {
            return Some(ImportActionState {
                label: "Import Secret Key".to_owned(),
                enabled: true,
                pending: false,
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn request_import_action(
        &self,
        request: &RadrootsSecretImportRequest,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            return Self::import_local_identity(&manager, request).map(Some);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = request;
            Ok(None)
        }
    }

    fn remote_signer_action_state(&self) -> Option<SetupActionState> {
        #[cfg(target_os = "macos")]
        {
            return Some(
                self.remote_signer
                    .action_state()
                    .unwrap_or_else(|_| SetupActionState {
                        label: "Connect Remote Signer".to_owned(),
                        enabled: !self.remote_signer.is_connecting(),
                        pending: self.remote_signer.is_connecting(),
                    }),
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn preview_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<radroots_studio_app_core::RadrootsRemoteSignerPreview, String> {
        #[cfg(target_os = "macos")]
        {
            return remote_signer::preview_connection(input);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = input;
            Err("remote signer onboarding is not available in this build".to_owned())
        }
    }

    fn request_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            self.remote_signer.begin_connect(input)?;
            return Ok(None);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = input;
            Ok(None)
        }
    }

    fn pending_remote_signer_connection(
        &self,
    ) -> Result<Option<radroots_studio_app_core::RadrootsPendingRemoteSignerConnection>, String> {
        #[cfg(target_os = "macos")]
        {
            return self.remote_signer.pending_connection();
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }

    fn request_cancel_pending_remote_signer_connection(&self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            return remote_signer::cancel_pending_connection();
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(())
        }
    }

    fn remote_signer_note_action_state(&self) -> Option<SetupActionState> {
        #[cfg(target_os = "macos")]
        {
            return Some(
                self.remote_signer
                    .note_action_state()
                    .unwrap_or(SetupActionState {
                        label: "Sign Remote Kind 1 Note".to_owned(),
                        enabled: false,
                        pending: false,
                    }),
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    fn request_remote_signer_note_action(&self, content: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            return self.remote_signer.begin_sign_kind1_note_selected(content);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = content;
            Ok(())
        }
    }

    fn poll_remote_signer_note_action_result(
        &self,
    ) -> Result<Option<radroots_studio_app_core::RadrootsRemoteSignerSignedNote>, String> {
        #[cfg(target_os = "macos")]
        {
            return self
                .remote_signer
                .take_note_update()
                .transpose()
                .map(|result| result.flatten());
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }

    fn request_select_account(
        &self,
        account_id: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            let account_id = radroots_identity::RadrootsIdentityId::try_from(account_id)
                .map_err(|_| "invalid account id".to_owned())?;
            manager
                .select_account(&account_id)
                .map_err(|source| source.to_string())?;
            return self.load_identity_state().map(Some);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = account_id;
            Ok(None)
        }
    }

    fn home_action_states(&self) -> Vec<HomeActionState> {
        #[cfg(target_os = "macos")]
        {
            let Ok(manager) = Self::accounts_manager() else {
                return Vec::new();
            };
            let Ok(status) = manager
                .selected_account_status()
                .map_err(|source| source.to_string())
            else {
                return Vec::new();
            };

            return match status {
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
            };
        }

        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            return match action {
                HomeActionKind::BackupSecretKey => Ok(HomeActionResult::None),
                HomeActionKind::RevealRawSecretKey => {
                    Self::export_selected_local_raw_secret_key(&manager)
                        .map(|nsec| HomeActionResult::RevealRawSecretKey { nsec })
                }
                HomeActionKind::RemoveLocalKey => Self::remove_selected_local_identity(&manager)
                    .map(HomeActionResult::IdentityState),
                HomeActionKind::ResetDevice => {
                    let accounts_path = Self::accounts_path()?;
                    Self::reset_local_device_state(&manager, accounts_path.as_path())
                        .map(HomeActionResult::IdentityState)
                }
                HomeActionKind::DisconnectSigner => {
                    remote_signer::disconnect_selected_remote_signer(&manager)
                        .map(HomeActionResult::IdentityState)
                }
            };
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = action;
            Ok(HomeActionResult::None)
        }
    }

    fn request_secret_key_backup_action(&self, password: &str) -> Result<HomeActionResult, String> {
        #[cfg(target_os = "macos")]
        {
            let manager = Self::accounts_manager()?;
            return Self::export_selected_local_encrypted_secret_key(&manager, password)
                .map(|ncryptsec| HomeActionResult::RevealEncryptedSecretKey { ncryptsec });
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = password;
            Ok(HomeActionResult::None)
        }
    }

    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "macos")]
        {
            return self
                .remote_signer
                .take_update()
                .transpose()
                .map(|state| state.flatten());
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }
}

fn main() -> eframe::Result<()> {
    set_macos_app_name();

    let viewport = {
        let viewport = egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([480.0, 320.0]);
        if let Some(icon) = desktop_icon() {
            viewport.with_icon(icon)
        } else {
            viewport
        }
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(DesktopBackend::new()))))),
    )
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::DesktopBackend;
    use radroots_studio_app_apple_security::RadrootsAppleKeychainVault;
    use radroots_studio_app_core::{
        IdentityGateState, RadrootsSecretImportMode, RadrootsSecretImportRequest,
    };
    use radroots_studio_app_test_support::{
        FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD, fixture_identity_ncryptsec,
    };
    use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
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

    #[test]
    fn remove_all_local_identities_clears_every_account() {
        let manager =
            radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager::new_in_memory();

        manager
            .generate_identity(Some("first".into()), true)
            .expect("generate first");
        manager
            .generate_identity(Some("second".into()), false)
            .expect("generate second");

        let state = DesktopBackend::remove_all_local_identities(&manager).expect("reset state");

        assert_eq!(state, IdentityGateState::Missing);
        assert_eq!(manager.list_accounts().expect("list accounts").len(), 0);
        assert_eq!(manager.selected_account_id().expect("selected"), None);
    }

    #[test]
    fn export_selected_local_raw_secret_key_returns_nsec() {
        let manager =
            radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let nsec =
            DesktopBackend::export_selected_local_raw_secret_key(&manager).expect("export secret");

        assert_eq!(nsec, identity.nsec());
        assert!(nsec.starts_with("nsec1"));
    }

    #[test]
    fn export_selected_local_encrypted_secret_key_returns_ncryptsec() {
        let manager =
            radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager::new_in_memory();
        let fixture_identity =
            RadrootsIdentity::from_secret_key_str(FIXTURE_ALICE.secret_key_hex).expect("fixture");

        manager
            .upsert_identity(&fixture_identity, Some("primary".into()), true)
            .expect("store identity");

        let ncryptsec = DesktopBackend::export_selected_local_encrypted_secret_key(
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
        let manager =
            radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        let state = DesktopBackend::import_local_identity(
            &manager,
            &RadrootsSecretImportRequest {
                mode: RadrootsSecretImportMode::RawSecretKey,
                secret_text: identity.nsec(),
                password: None,
            },
        )
        .expect("import identity");

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
        let manager =
            radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager::new_in_memory();
        let encrypted_secret_key =
            fixture_identity_ncryptsec(&FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD)
                .expect("fixture encrypted secret key");
        let fixture_identity =
            RadrootsIdentity::from_secret_key_str(FIXTURE_ALICE.secret_key_hex).expect("fixture");
        let fixture_account_id = fixture_identity.id();

        let state = DesktopBackend::import_local_identity(
            &manager,
            &RadrootsSecretImportRequest {
                mode: RadrootsSecretImportMode::EncryptedSecretKey,
                secret_text: encrypted_secret_key,
                password: Some(FIXTURE_BACKUP_PASSWORD.to_owned()),
            },
        )
        .expect("import identity");

        assert_eq!(
            state,
            IdentityGateState::Ready {
                account_id: fixture_account_id.to_string(),
            }
        );
        assert_eq!(
            manager.selected_account_id().expect("selected"),
            Some(fixture_account_id.clone())
        );
        assert_eq!(
            manager
                .export_secret_hex(&fixture_account_id)
                .expect("export secret"),
            Some(FIXTURE_ALICE.secret_key_hex.to_owned())
        );
    }

    #[test]
    fn remove_accounts_file_if_present_deletes_existing_file() {
        let unique = format!(
            "radroots-desktop-reset-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::write(&path, b"{}").expect("write accounts file");

        DesktopBackend::remove_accounts_file_if_present(path.as_path()).expect("remove file");

        assert!(!path.exists());
    }
}
