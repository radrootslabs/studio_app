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
    HomeActionKind, HomeActionResult, HomeActionState, IdentityGateState, ImportActionState,
    RadrootsAccountCustody, RadrootsAccountSummary, RadrootsLocationCountry,
    RadrootsLocationCountryCenterLookupResult, RadrootsLocationCountryListResult,
    RadrootsLocationPoint, RadrootsLocationResolverError, RadrootsLocationReverseOptions,
    RadrootsOfflineGeocoderState, RadrootsResolvedLocation, RadrootsReverseLocationLookupResult,
    RadrootsSecretImportMode, RadrootsSecretImportRequest, SetupActionState,
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
#[cfg(any(target_os = "android", test))]
use std::sync::Mutex;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(any(target_os = "android", test))]
use zeroize::Zeroizing;

#[cfg(any(target_os = "android", test))]
mod country_lookup;
#[cfg(any(target_os = "android", test))]
mod offline_geocoder;
#[cfg(target_os = "android")]
mod remote_signer;
#[cfg(any(target_os = "android", test))]
mod reverse_lookup;
#[cfg(any(target_os = "android", test))]
mod security;
#[cfg(any(target_os = "android", test))]
mod storage;
#[cfg(any(target_os = "android", test))]
mod vault;

#[cfg(any(target_os = "android", test))]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
struct AndroidBackend {
    country_lookup: country_lookup::AndroidCountryLookup,
    offline_geocoder: offline_geocoder::AndroidOfflineGeocoder,
    #[cfg(target_os = "android")]
    remote_signer: remote_signer::AndroidRemoteSigner,
    reverse_lookup: reverse_lookup::AndroidReverseLookup,
}

#[cfg(any(target_os = "android", test))]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
enum PendingSecretKeyExport {
    EncryptedBackup { password: Zeroizing<String> },
    RawReveal,
}

#[cfg(any(target_os = "android", test))]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
static PENDING_SECRET_KEY_EXPORT: Mutex<Option<PendingSecretKeyExport>> = Mutex::new(None);

#[cfg(any(target_os = "android", test))]
impl RadrootsAppBackend for AndroidBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            let status = manager
                .selected_account_status()
                .map_err(|source| source.to_string())?;
            return remote_signer::identity_state_from_status(status);
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(Self::unsupported_identity_state())
        }
    }

    fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            return Self::account_roster_from_manager(&manager);
        }

        #[cfg(not(target_os = "android"))]
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
        #[cfg(target_os = "android")]
        {
            return offline_geocoder::reverse_location(
                &self.offline_geocoder.current_state(),
                point,
                options,
            );
        }

        #[cfg(not(target_os = "android"))]
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
        #[cfg(target_os = "android")]
        {
            return self.reverse_lookup.begin(
                self.offline_geocoder.current_state(),
                point,
                options,
            );
        }

        #[cfg(not(target_os = "android"))]
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
        #[cfg(target_os = "android")]
        {
            return self
                .country_lookup
                .begin_list(self.offline_geocoder.current_state());
        }

        #[cfg(not(target_os = "android"))]
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
        #[cfg(target_os = "android")]
        {
            return self
                .country_lookup
                .begin_center(self.offline_geocoder.current_state(), country_id.to_owned());
        }

        #[cfg(not(target_os = "android"))]
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
        #[cfg(target_os = "android")]
        {
            return offline_geocoder::list_countries(&self.offline_geocoder.current_state());
        }

        #[cfg(not(target_os = "android"))]
        {
            Err(RadrootsLocationResolverError::Unsupported)
        }
    }

    fn location_country_center(
        &self,
        country_id: &str,
    ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
        #[cfg(target_os = "android")]
        {
            return offline_geocoder::country_center(
                &self.offline_geocoder.current_state(),
                country_id,
            );
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = country_id;
            Err(RadrootsLocationResolverError::Unsupported)
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

    fn home_setup_action_state(&self) -> Option<SetupActionState> {
        Some(self.setup_action_state())
    }

    fn request_home_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        self.request_setup_action()
    }

    fn import_action_state(&self) -> Option<ImportActionState> {
        #[cfg(target_os = "android")]
        {
            return Some(ImportActionState {
                label: "Import Secret Key".to_owned(),
                enabled: true,
                pending: false,
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }

    fn request_import_action(
        &self,
        request: &RadrootsSecretImportRequest,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            return Self::import_local_identity(&manager, request).map(Some);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = request;
            Ok(None)
        }
    }

    fn request_select_account(
        &self,
        account_id: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            let manager = Self::accounts_manager()?;
            let account_id = radroots_identity::RadrootsIdentityId::try_from(account_id)
                .map_err(|_| "invalid account id".to_owned())?;
            manager
                .select_account(&account_id)
                .map_err(|source| source.to_string())?;
            return self.load_identity_state().map(Some);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = account_id;
            Ok(None)
        }
    }

    fn remote_signer_action_state(&self) -> Option<SetupActionState> {
        #[cfg(target_os = "android")]
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

        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }

    fn preview_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<radroots_studio_app_core::RadrootsRemoteSignerPreview, String> {
        #[cfg(target_os = "android")]
        {
            return remote_signer::preview_connection(input);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = input;
            Err("remote signer onboarding is not available in this build".to_owned())
        }
    }

    fn request_remote_signer_connection(
        &self,
        input: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            self.remote_signer.begin_connect(input)?;
            return Ok(None);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = input;
            Ok(None)
        }
    }

    fn pending_remote_signer_connection(
        &self,
    ) -> Result<Option<radroots_studio_app_core::RadrootsPendingRemoteSignerConnection>, String> {
        #[cfg(target_os = "android")]
        {
            return self.remote_signer.pending_connection();
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(None)
        }
    }

    fn request_cancel_pending_remote_signer_connection(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            return remote_signer::cancel_pending_connection();
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(())
        }
    }

    fn remote_signer_note_action_state(&self) -> Option<SetupActionState> {
        #[cfg(target_os = "android")]
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

        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }

    fn request_remote_signer_note_action(&self, content: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            return self.remote_signer.begin_sign_kind1_note_selected(content);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = content;
            Ok(())
        }
    }

    fn poll_remote_signer_note_action_result(
        &self,
    ) -> Result<Option<radroots_studio_app_core::RadrootsRemoteSignerSignedNote>, String> {
        #[cfg(target_os = "android")]
        {
            return self
                .remote_signer
                .take_note_update()
                .transpose()
                .map(|result| result.flatten());
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(None)
        }
    }

    fn home_action_states(&self) -> Vec<HomeActionState> {
        #[cfg(target_os = "android")]
        {
            let secret_key_export_pending = Self::secret_key_export_pending();
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
                        enabled: !secret_key_export_pending,
                        pending: secret_key_export_pending,
                    },
                    HomeActionState {
                        kind: HomeActionKind::RevealRawSecretKey,
                        label: "Reveal Raw Secret Key".to_owned(),
                        enabled: !secret_key_export_pending,
                        pending: secret_key_export_pending,
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

        #[cfg(not(target_os = "android"))]
        {
            Vec::new()
        }
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        #[cfg(target_os = "android")]
        {
            return match action {
                HomeActionKind::BackupSecretKey => Ok(HomeActionResult::None),
                HomeActionKind::RevealRawSecretKey => {
                    Self::begin_raw_secret_key_reveal().map(|()| HomeActionResult::None)
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
                HomeActionKind::DisconnectSigner => {
                    let manager = Self::accounts_manager()?;
                    remote_signer::disconnect_selected_remote_signer(&manager)
                        .map(HomeActionResult::IdentityState)
                }
            };
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = action;
            Ok(HomeActionResult::None)
        }
    }

    fn request_secret_key_backup_action(&self, password: &str) -> Result<HomeActionResult, String> {
        #[cfg(target_os = "android")]
        {
            return Self::begin_encrypted_secret_key_backup(password)
                .map(|()| HomeActionResult::None);
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = password;
            Ok(HomeActionResult::None)
        }
    }

    fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
        #[cfg(target_os = "android")]
        {
            return Self::poll_secret_key_export();
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(None)
        }
    }

    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        #[cfg(target_os = "android")]
        {
            return self
                .remote_signer
                .take_update()
                .transpose()
                .map(|state| state.flatten());
        }

        #[cfg(not(target_os = "android"))]
        {
            Ok(None)
        }
    }
}

#[cfg(any(target_os = "android", test))]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
impl AndroidBackend {
    fn new() -> Self {
        #[cfg(target_os = "android")]
        let offline_geocoder = offline_geocoder::AndroidOfflineGeocoder::start();

        #[cfg(not(target_os = "android"))]
        let offline_geocoder = offline_geocoder::AndroidOfflineGeocoder::from_state(
            RadrootsOfflineGeocoderState::unavailable(
                radroots_studio_app_core::RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                radroots_studio_app_core::RadrootsOfflineGeocoderPlatform::Android,
                "android offline geocoder initialization is only wired on android targets",
            ),
        );

        Self {
            country_lookup: country_lookup::AndroidCountryLookup::new(),
            offline_geocoder,
            #[cfg(target_os = "android")]
            remote_signer: remote_signer::AndroidRemoteSigner::new(),
            reverse_lookup: reverse_lookup::AndroidReverseLookup::new(),
        }
    }

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

    fn account_roster_from_manager(
        manager: &RadrootsNostrAccountsManager,
    ) -> Result<Vec<RadrootsAccountSummary>, String> {
        manager
            .list_accounts()
            .map_err(|source| source.to_string())?
            .into_iter()
            .map(|record| {
                #[cfg(target_os = "android")]
                let custody = remote_signer::custody_for_account_id(record.account_id.as_str())?;
                #[cfg(not(target_os = "android"))]
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

    fn export_selected_local_encrypted_secret_key(
        manager: &RadrootsNostrAccountsManager,
        password: &str,
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
        identity
            .encrypt_secret_key_ncryptsec(password)
            .map_err(|source| source.to_string())
    }

    fn export_selected_local_raw_secret_key(
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

    #[cfg(target_os = "android")]
    fn begin_encrypted_secret_key_backup(password: &str) -> Result<(), String> {
        *PENDING_SECRET_KEY_EXPORT
            .lock()
            .map_err(|_| "failed to store pending encrypted secret key backup".to_owned())? =
            Some(PendingSecretKeyExport::EncryptedBackup {
                password: Zeroizing::new(password.to_owned()),
            });
        if let Err(source) =
            security::begin_user_presence_verification("back up the current secret key")
        {
            *PENDING_SECRET_KEY_EXPORT
                .lock()
                .map_err(|_| "failed to clear pending encrypted secret key backup".to_owned())? =
                None;
            return Err(source.to_string());
        }
        Ok(())
    }

    #[cfg(not(target_os = "android"))]
    fn begin_encrypted_secret_key_backup(password: &str) -> Result<(), String> {
        let _ = password;
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn begin_raw_secret_key_reveal() -> Result<(), String> {
        *PENDING_SECRET_KEY_EXPORT
            .lock()
            .map_err(|_| "failed to store pending raw secret key reveal".to_owned())? =
            Some(PendingSecretKeyExport::RawReveal);
        if let Err(source) =
            security::begin_user_presence_verification("reveal the current secret key")
        {
            *PENDING_SECRET_KEY_EXPORT
                .lock()
                .map_err(|_| "failed to clear pending raw secret key reveal".to_owned())? = None;
            return Err(source.to_string());
        }
        Ok(())
    }

    #[cfg(not(target_os = "android"))]
    fn begin_raw_secret_key_reveal() -> Result<(), String> {
        Ok(())
    }

    #[cfg(target_os = "android")]
    fn secret_key_export_pending() -> bool {
        security::is_user_presence_verification_pending().unwrap_or(false)
    }

    #[cfg(not(target_os = "android"))]
    fn secret_key_export_pending() -> bool {
        false
    }

    #[cfg(target_os = "android")]
    fn poll_secret_key_export() -> Result<Option<HomeActionResult>, String> {
        match security::take_user_presence_verification_result()
            .map_err(|source| source.to_string())?
        {
            Some(security::AndroidUserPresenceVerificationResult::Verified) => {
                let manager = Self::accounts_manager()?;
                let pending_export = PENDING_SECRET_KEY_EXPORT
                    .lock()
                    .map_err(|_| "failed to take pending secret key export".to_owned())?
                    .take();
                match pending_export {
                    Some(PendingSecretKeyExport::EncryptedBackup { password }) => {
                        Self::export_selected_local_encrypted_secret_key(
                            &manager,
                            password.as_str(),
                        )
                        .map(|ncryptsec| {
                            Some(HomeActionResult::RevealEncryptedSecretKey { ncryptsec })
                        })
                    }
                    Some(PendingSecretKeyExport::RawReveal) => {
                        Self::export_selected_local_raw_secret_key(&manager)
                            .map(|nsec| Some(HomeActionResult::RevealRawSecretKey { nsec }))
                    }
                    None => Err("missing pending secret key export request".to_owned()),
                }
            }
            Some(security::AndroidUserPresenceVerificationResult::Failed(message)) => {
                *PENDING_SECRET_KEY_EXPORT
                    .lock()
                    .map_err(|_| "failed to clear pending secret key export".to_owned())? = None;
                Err(message)
            }
            None => Ok(None),
        }
    }

    #[cfg(not(target_os = "android"))]
    fn poll_secret_key_export() -> Result<Option<HomeActionResult>, String> {
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
        remote_signer::purge_all_custody_state()?;
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
        Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(AndroidBackend::new()))))),
    )
    .map_err(|err| err.to_string())
}

#[cfg(target_os = "android")]
#[allow(improper_ctypes_definitions)]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn android_main(android_app: AndroidApp) {
    if let Err(err) = run_android_app(android_app) {
        log::error!("android launcher failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{
        FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD, fixture_identity_ncryptsec,
    };

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
        let IdentityGateState::Ready { account_id } = state else {
            panic!("expected ready identity state");
        };

        assert!(!account_id.is_empty());
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
    fn export_selected_local_raw_secret_key_returns_nsec() {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();

        manager
            .upsert_identity(&identity, Some("primary".into()), true)
            .expect("store identity");

        let nsec =
            AndroidBackend::export_selected_local_raw_secret_key(&manager).expect("export secret");

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

        let ncryptsec = AndroidBackend::export_selected_local_encrypted_secret_key(
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

        let state = AndroidBackend::import_local_identity(
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

        let state = AndroidBackend::import_local_identity(
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
            Some(fixture_account_id.clone())
        );
        assert_eq!(manager.list_accounts().expect("list").len(), 1);
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
