#![forbid(unsafe_code)]

use eframe::egui;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

mod account_roster;
mod home_location_tools;
mod location_resolver;
mod offline_geocoder;
mod remote_signer;
mod secret_keys;

pub const APP_NAME: &str = "Rad Roots";

pub use account_roster::{RadrootsAccountCustody, RadrootsAccountSummary};
pub use location_resolver::{
    RadrootsLocationCountry, RadrootsLocationCountryCenterLookupResult,
    RadrootsLocationCountryListResult, RadrootsLocationPoint, RadrootsLocationResolverError,
    RadrootsLocationReverseOptions, RadrootsResolvedLocation, RadrootsReverseLocationLookupResult,
};
pub use offline_geocoder::{
    RadrootsOfflineGeocoderDiagnostic, RadrootsOfflineGeocoderPlatform,
    RadrootsOfflineGeocoderState, RadrootsOfflineGeocoderUnavailableKind,
};
pub use remote_signer::{
    RadrootsPendingRemoteSignerConnection, RadrootsRemoteSignerPreview,
    RadrootsRemoteSignerSignedNote,
};
pub use secret_keys::{RadrootsSecretImportMode, RadrootsSecretImportRequest};

use home_location_tools::HomeLocationTools;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasteActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeActionState {
    pub kind: HomeActionKind,
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeActionKind {
    BackupSecretKey,
    RevealRawSecretKey,
    RemoveLocalKey,
    ResetDevice,
    DisconnectSigner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeActionResult {
    None,
    IdentityState(IdentityGateState),
    RevealEncryptedSecretKey { ncryptsec: String },
    RevealRawSecretKey { nsec: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityGateState {
    Missing,
    Ready { account_id: String },
    Unsupported { reason: String },
}

pub trait RadrootsAppBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String>;
    fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
        Ok(Vec::new())
    }
    fn offline_geocoder_state(&self) -> Option<RadrootsOfflineGeocoderState> {
        None
    }
    fn poll_offline_geocoder_state(&self) -> Result<Option<RadrootsOfflineGeocoderState>, String> {
        Ok(None)
    }
    fn setup_action_state(&self) -> SetupActionState;
    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String>;
    fn home_setup_action_state(&self) -> Option<SetupActionState> {
        None
    }
    fn request_home_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn import_action_state(&self) -> Option<ImportActionState> {
        None
    }
    fn request_import_action(
        &self,
        _request: &RadrootsSecretImportRequest,
    ) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn import_paste_action_state(&self) -> Option<PasteActionState> {
        None
    }
    fn request_import_paste_action(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn remote_signer_action_state(&self) -> Option<SetupActionState> {
        None
    }
    fn preview_remote_signer_connection(
        &self,
        _input: &str,
    ) -> Result<RadrootsRemoteSignerPreview, String> {
        Err("remote signer onboarding is not available in this build".to_owned())
    }
    fn request_remote_signer_connection(
        &self,
        _input: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn pending_remote_signer_connection(
        &self,
    ) -> Result<Option<RadrootsPendingRemoteSignerConnection>, String> {
        Ok(None)
    }
    fn request_cancel_pending_remote_signer_connection(&self) -> Result<(), String> {
        Ok(())
    }
    fn remote_signer_note_action_state(&self) -> Option<SetupActionState> {
        None
    }
    fn request_remote_signer_note_action(&self, _content: &str) -> Result<(), String> {
        Ok(())
    }
    fn poll_remote_signer_note_action_result(
        &self,
    ) -> Result<Option<RadrootsRemoteSignerSignedNote>, String> {
        Ok(None)
    }
    fn home_action_states(&self) -> Vec<HomeActionState> {
        Vec::new()
    }
    fn request_home_action(&self, _action: HomeActionKind) -> Result<HomeActionResult, String> {
        Ok(HomeActionResult::None)
    }
    fn request_secret_key_backup_action(
        &self,
        _password: &str,
    ) -> Result<HomeActionResult, String> {
        Ok(HomeActionResult::None)
    }
    fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
        Ok(None)
    }
    fn request_select_account(
        &self,
        _account_id: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn reverse_location(
        &self,
        _point: RadrootsLocationPoint,
        _options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
    fn request_reverse_location_lookup(
        &self,
        _point: RadrootsLocationPoint,
        _options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
    fn poll_reverse_location_lookup_result(
        &self,
    ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
        Ok(None)
    }
    fn request_location_country_list(&self) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
    fn poll_location_country_list_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryListResult>, String> {
        Ok(None)
    }
    fn request_location_country_center_lookup(
        &self,
        _country_id: &str,
    ) -> Result<(), RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
    fn poll_location_country_center_lookup_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryCenterLookupResult>, String> {
        Ok(None)
    }
    fn list_location_countries(
        &self,
    ) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
    fn location_country_center(
        &self,
        _country_id: &str,
    ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
        Err(RadrootsLocationResolverError::Unsupported)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppScreen {
    Setup,
    Home { account_id: String },
}

const RAW_SECRET_REVEAL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
enum RevealedSecretMaterial {
    EncryptedSecretKey(Zeroizing<String>),
    RawSecretKey {
        nsec: Zeroizing<String>,
        revealed_at: Instant,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoteSignerEntryState {
    Closed,
    Editing,
    Review(RadrootsRemoteSignerPreview),
    WaitingApproval(RadrootsPendingRemoteSignerConnection),
}

impl RevealedSecretMaterial {
    fn label(&self) -> &'static str {
        match self {
            Self::EncryptedSecretKey(_) => "Encrypted Secret Key",
            Self::RawSecretKey { .. } => "Raw Secret Key",
        }
    }

    fn value(&self) -> &str {
        match self {
            Self::EncryptedSecretKey(ncryptsec) => ncryptsec.as_str(),
            Self::RawSecretKey { nsec, .. } => nsec.as_str(),
        }
    }

    fn dismiss_label(&self) -> &'static str {
        match self {
            Self::EncryptedSecretKey(_) => "Dismiss Encrypted Secret Key",
            Self::RawSecretKey { .. } => "Dismiss Raw Secret Key",
        }
    }

    fn is_raw(&self) -> bool {
        matches!(self, Self::RawSecretKey { .. })
    }

    fn raw_secret_expired(&self) -> bool {
        match self {
            Self::RawSecretKey { revealed_at, .. } => {
                revealed_at.elapsed() >= RAW_SECRET_REVEAL_TIMEOUT
            }
            Self::EncryptedSecretKey(_) => false,
        }
    }
}

pub struct RadrootsApp {
    backend: Box<dyn RadrootsAppBackend>,
    screen: AppScreen,
    account_roster: Vec<RadrootsAccountSummary>,
    offline_geocoder_state: Option<RadrootsOfflineGeocoderState>,
    status_message: Option<String>,
    home_location_tools: HomeLocationTools,
    pending_home_confirmation: Option<HomeActionKind>,
    pending_import_mode: Option<RadrootsSecretImportMode>,
    remote_signer_entry_state: RemoteSignerEntryState,
    remote_signer_input: Zeroizing<String>,
    secret_key_input: Zeroizing<String>,
    import_password_input: Zeroizing<String>,
    pending_secret_key_backup_entry: bool,
    secret_key_backup_password_input: Zeroizing<String>,
    secret_key_backup_password_confirm_input: Zeroizing<String>,
    remote_signer_note_input: Zeroizing<String>,
    revealed_secret_material: Option<RevealedSecretMaterial>,
}

impl RadrootsApp {
    fn clear_secret_import_entry(&mut self) {
        self.pending_import_mode = None;
        self.secret_key_input.clear();
        self.import_password_input.clear();
    }

    fn clear_secret_key_backup_entry(&mut self) {
        self.pending_secret_key_backup_entry = false;
        self.secret_key_backup_password_input.clear();
        self.secret_key_backup_password_confirm_input.clear();
    }

    fn clear_revealed_secret_material(&mut self) {
        self.revealed_secret_material = None;
    }

    fn clear_remote_signer_entry(&mut self) {
        self.remote_signer_entry_state = RemoteSignerEntryState::Closed;
        self.remote_signer_input.clear();
    }

    fn clear_secret_key_ui_state(&mut self) {
        self.clear_remote_signer_entry();
        self.clear_secret_import_entry();
        self.clear_secret_key_backup_entry();
        self.clear_revealed_secret_material();
    }

    fn open_import_entry(&mut self) {
        self.pending_import_mode = Some(RadrootsSecretImportMode::EncryptedSecretKey);
        self.secret_key_input.clear();
        self.import_password_input.clear();
        self.status_message = None;
    }

    fn import_mode(&self) -> RadrootsSecretImportMode {
        self.pending_import_mode.unwrap_or_default()
    }

    fn set_import_mode(&mut self, mode: RadrootsSecretImportMode) {
        self.pending_import_mode = Some(mode);
        self.secret_key_input.clear();
        self.import_password_input.clear();
        self.status_message = None;
    }

    fn secret_import_request(&self) -> Result<RadrootsSecretImportRequest, String> {
        let mode = self.import_mode();
        let secret_text = self.secret_key_input.trim().to_owned();
        if secret_text.is_empty() {
            return Err(match mode {
                RadrootsSecretImportMode::EncryptedSecretKey => {
                    "enter an encrypted secret key to continue".to_owned()
                }
                RadrootsSecretImportMode::RawSecretKey => {
                    "enter a raw secret key to continue".to_owned()
                }
            });
        }

        let password = if mode.requires_password() {
            if self.import_password_input.is_empty() {
                return Err("enter a password to import the encrypted secret key".to_owned());
            }
            Some(self.import_password_input.to_string())
        } else {
            None
        };

        Ok(RadrootsSecretImportRequest {
            mode,
            secret_text,
            password,
        })
    }

    fn request_secret_key_backup_action(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();

        if self.secret_key_backup_password_input.is_empty() {
            self.status_message =
                Some("enter a password to create an encrypted secret key backup".to_owned());
            return;
        }

        if self.secret_key_backup_password_input != self.secret_key_backup_password_confirm_input {
            self.status_message = Some("backup passwords do not match".to_owned());
            return;
        }

        match self
            .backend
            .request_secret_key_backup_action(self.secret_key_backup_password_input.as_str())
        {
            Ok(result) => {
                self.clear_secret_key_backup_entry();
                self.apply_home_action_result(result);
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn sync_revealed_secret_material_lifetime(&mut self) {
        if self
            .revealed_secret_material
            .as_ref()
            .is_some_and(RevealedSecretMaterial::raw_secret_expired)
        {
            self.clear_revealed_secret_material();
        }
    }

    fn clear_raw_secret_when_app_unfocused(&mut self, ctx: &egui::Context) {
        if self
            .revealed_secret_material
            .as_ref()
            .is_some_and(RevealedSecretMaterial::is_raw)
            && ctx.input(|input| input.viewport().focused == Some(false))
        {
            self.clear_revealed_secret_material();
        }
    }

    pub fn new(backend: Box<dyn RadrootsAppBackend>) -> Self {
        let mut app = Self {
            backend,
            screen: AppScreen::Setup,
            account_roster: Vec::new(),
            offline_geocoder_state: None,
            status_message: None,
            home_location_tools: HomeLocationTools::new(),
            pending_home_confirmation: None,
            pending_import_mode: None,
            remote_signer_entry_state: RemoteSignerEntryState::Closed,
            remote_signer_input: Zeroizing::new(String::new()),
            secret_key_input: Zeroizing::new(String::new()),
            import_password_input: Zeroizing::new(String::new()),
            pending_secret_key_backup_entry: false,
            secret_key_backup_password_input: Zeroizing::new(String::new()),
            secret_key_backup_password_confirm_input: Zeroizing::new(String::new()),
            remote_signer_note_input: Zeroizing::new(String::new()),
            revealed_secret_material: None,
        };
        app.offline_geocoder_state = app.backend.offline_geocoder_state();
        match app.backend.load_identity_state() {
            Ok(state) => app.apply_identity_state(state),
            Err(err) => {
                app.screen = AppScreen::Setup;
                app.status_message = Some(err);
            }
        }
        app.sync_remote_signer_entry_from_backend();
        app
    }

    fn refresh_account_roster(&mut self) {
        match self.backend.load_account_roster() {
            Ok(account_roster) => {
                self.account_roster = account_roster;
            }
            Err(err) => {
                self.account_roster.clear();
                self.status_message = Some(err);
            }
        }
    }

    fn apply_identity_state(&mut self, state: IdentityGateState) {
        match state {
            IdentityGateState::Missing => {
                self.screen = AppScreen::Setup;
                self.account_roster.clear();
                self.status_message = None;
                self.home_location_tools.clear();
                self.pending_home_confirmation = None;
                self.clear_secret_key_ui_state();
            }
            IdentityGateState::Ready { account_id } => {
                self.screen = AppScreen::Home { account_id };
                self.status_message = None;
                self.refresh_account_roster();
                self.home_location_tools.clear();
                self.pending_home_confirmation = None;
                self.clear_secret_key_ui_state();
            }
            IdentityGateState::Unsupported { reason } => {
                self.screen = AppScreen::Setup;
                self.account_roster.clear();
                self.status_message = Some(reason);
                self.home_location_tools.clear();
                self.pending_home_confirmation = None;
                self.clear_secret_key_ui_state();
            }
        }
    }

    fn request_setup_action(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        match self.backend.request_setup_action() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_home_setup_action(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        self.pending_home_confirmation = None;
        self.clear_remote_signer_entry();
        self.clear_secret_import_entry();
        match self.backend.request_home_setup_action() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => self.refresh_account_roster(),
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_import_action(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        let request = match self.secret_import_request() {
            Ok(request) => request,
            Err(err) => {
                self.status_message = Some(err);
                return;
            }
        };
        match self.backend.request_import_action(&request) {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_import_paste_action(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        match self.backend.request_import_paste_action() {
            Ok(Some(secret_key)) => {
                self.secret_key_input = Zeroizing::new(secret_key);
            }
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn open_remote_signer_entry(&mut self) {
        self.remote_signer_entry_state = RemoteSignerEntryState::Editing;
        self.remote_signer_input.clear();
        self.status_message = None;
    }

    fn sync_remote_signer_entry_from_backend(&mut self) {
        match self.backend.pending_remote_signer_connection() {
            Ok(Some(pending)) => {
                if !matches!(
                    self.remote_signer_entry_state,
                    RemoteSignerEntryState::Editing | RemoteSignerEntryState::Review(_)
                ) {
                    self.remote_signer_entry_state =
                        RemoteSignerEntryState::WaitingApproval(pending);
                }
            }
            Ok(None) => {
                if matches!(
                    self.remote_signer_entry_state,
                    RemoteSignerEntryState::WaitingApproval(_)
                ) {
                    self.clear_remote_signer_entry();
                }
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_remote_signer_preview(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        match self
            .backend
            .preview_remote_signer_connection(self.remote_signer_input.as_str())
        {
            Ok(preview) => {
                self.remote_signer_entry_state = RemoteSignerEntryState::Review(preview);
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_remote_signer_connect(&mut self) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        let pending_summary = match &self.remote_signer_entry_state {
            RemoteSignerEntryState::Review(preview) => preview.pending_summary(),
            _ => {
                self.status_message =
                    Some("review the remote signer details before connecting".to_owned());
                return;
            }
        };
        match self
            .backend
            .request_remote_signer_connection(self.remote_signer_input.as_str())
        {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {
                self.remote_signer_entry_state =
                    RemoteSignerEntryState::WaitingApproval(pending_summary);
                self.sync_remote_signer_entry_from_backend();
            }
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_cancel_pending_remote_signer(&mut self) {
        self.status_message = None;
        match self
            .backend
            .request_cancel_pending_remote_signer_connection()
        {
            Ok(()) => self.clear_remote_signer_entry(),
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_remote_signer_note_action(&mut self) {
        self.status_message = None;
        match self
            .backend
            .request_remote_signer_note_action(self.remote_signer_note_input.as_str())
        {
            Ok(()) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_select_account(&mut self, account_id: &str) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        self.pending_home_confirmation = None;
        self.clear_secret_key_ui_state();
        match self.backend.request_select_account(account_id) {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => self.refresh_account_roster(),
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_home_action(&mut self, action: HomeActionKind) {
        self.status_message = None;
        self.clear_revealed_secret_material();
        match self.backend.request_home_action(action) {
            Ok(result) => self.apply_home_action_result(result),
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn apply_home_action_result(&mut self, result: HomeActionResult) {
        match result {
            HomeActionResult::IdentityState(state) => self.apply_identity_state(state),
            HomeActionResult::RevealEncryptedSecretKey { ncryptsec } => {
                self.revealed_secret_material = Some(RevealedSecretMaterial::EncryptedSecretKey(
                    Zeroizing::new(ncryptsec),
                ));
                self.pending_home_confirmation = None;
            }
            HomeActionResult::RevealRawSecretKey { nsec } => {
                self.revealed_secret_material = Some(RevealedSecretMaterial::RawSecretKey {
                    nsec: Zeroizing::new(nsec),
                    revealed_at: Instant::now(),
                });
                self.pending_home_confirmation = None;
            }
            HomeActionResult::None => {}
        }
    }

    fn home_action_requires_confirmation(action: HomeActionKind) -> bool {
        !matches!(action, HomeActionKind::BackupSecretKey)
    }

    fn home_action_confirmation_message(action: HomeActionKind) -> &'static str {
        match action {
            HomeActionKind::BackupSecretKey => {
                "This exports the current local secret key in encrypted form for backup."
            }
            HomeActionKind::RevealRawSecretKey => {
                "This reveals the current local secret key in plaintext. Use encrypted backup instead when possible."
            }
            HomeActionKind::RemoveLocalKey => {
                "This removes the current key from this device and returns the app to setup."
            }
            HomeActionKind::ResetDevice => {
                "This removes all app-managed local identity state from this device and returns the app to setup."
            }
            HomeActionKind::DisconnectSigner => {
                "This disconnects the current external signer from the app. It does not delete the signer key."
            }
        }
    }

    fn sync_backend(&mut self) {
        match self.backend.poll_offline_geocoder_state() {
            Ok(Some(state)) => {
                self.offline_geocoder_state = Some(state);
            }
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
        match self.backend.poll_home_action_result() {
            Ok(Some(result)) => self.apply_home_action_result(result),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
        match self.backend.poll_remote_signer_note_action_result() {
            Ok(Some(result)) => {
                self.remote_signer_note_input.clear();
                self.status_message = Some(format!(
                    "Signed remote kind 1 note: {}",
                    result.event_id_hex
                ));
            }
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
        match self.backend.poll_reverse_location_lookup_result() {
            Ok(Some(result)) => self.home_location_tools.apply_reverse_lookup_result(result),
            Ok(None) => {}
            Err(err) => {
                self.home_location_tools
                    .apply_reverse_lookup_poll_error(err);
            }
        }
        match self.backend.poll_location_country_list_result() {
            Ok(Some(result)) => self.home_location_tools.apply_country_list_result(result),
            Ok(None) => {}
            Err(err) => {
                self.home_location_tools.apply_country_list_poll_error(err);
            }
        }
        match self.backend.poll_location_country_center_lookup_result() {
            Ok(Some(result)) => self.home_location_tools.apply_country_center_result(result),
            Ok(None) => {}
            Err(err) => {
                self.home_location_tools
                    .apply_country_center_poll_error(err);
            }
        }
        match self.backend.poll_identity_state() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
        self.sync_remote_signer_entry_from_backend();
    }

    fn render_import_entry(
        &mut self,
        ui: &mut egui::Ui,
        import_action: &ImportActionState,
        import_paste_action: Option<&PasteActionState>,
    ) {
        let import_mode = self.import_mode();
        ui.vertical_centered(|ui| {
            ui.set_max_width(ui.available_width().min(560.0));
            ui.label(import_mode.helper_text());
            ui.add_space(8.0);
            if ui.button(import_mode.switch_label()).clicked() {
                self.set_import_mode(import_mode.toggle());
            }
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut *self.secret_key_input)
                    .hint_text(import_mode.hint_text())
                    .desired_width(ui.available_width()),
            );
            if import_mode.requires_password() {
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut *self.import_password_input)
                        .password(true)
                        .hint_text("Enter Backup Password")
                        .desired_width(ui.available_width()),
                );
            }
            ui.add_space(8.0);
            if let Some(import_paste_action) = import_paste_action {
                let paste_clicked = ui
                    .add_enabled(
                        import_paste_action.enabled,
                        egui::Button::new(import_paste_action.label.clone()),
                    )
                    .clicked();
                if paste_clicked {
                    self.request_import_paste_action();
                }
                ui.add_space(8.0);
            }
            ui.horizontal_centered(|ui| {
                let confirm_clicked = ui
                    .add_enabled(
                        import_action.enabled,
                        egui::Button::new(import_action.label.clone()),
                    )
                    .clicked();
                if confirm_clicked {
                    self.request_import_action();
                }

                if ui.button("Cancel").clicked() {
                    self.clear_secret_import_entry();
                    self.status_message = None;
                }
            });
        });
    }

    fn render_secret_key_backup_entry(&mut self, ui: &mut egui::Ui, action: &HomeActionState) {
        ui.vertical_centered(|ui| {
            ui.set_max_width(ui.available_width().min(560.0));
            ui.label("Create an encrypted backup of the current local secret key.");
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut *self.secret_key_backup_password_input)
                    .password(true)
                    .hint_text("Enter Backup Password")
                    .desired_width(ui.available_width()),
            );
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut *self.secret_key_backup_password_confirm_input)
                    .password(true)
                    .hint_text("Confirm Backup Password")
                    .desired_width(ui.available_width()),
            );
            ui.add_space(8.0);
            ui.horizontal_centered(|ui| {
                let confirm_clicked = ui
                    .add_enabled(action.enabled, egui::Button::new(action.label.clone()))
                    .clicked();
                if confirm_clicked {
                    self.request_secret_key_backup_action();
                }

                if ui.button("Cancel").clicked() {
                    self.clear_secret_key_backup_entry();
                    self.status_message = None;
                }
            });
        });
    }

    fn render_remote_signer_entry(&mut self, ui: &mut egui::Ui, action: &SetupActionState) {
        ui.vertical_centered(|ui| {
            ui.set_max_width(ui.available_width().min(560.0));
            match &self.remote_signer_entry_state {
                RemoteSignerEntryState::Closed => {}
                RemoteSignerEntryState::Editing => {
                    ui.label(
                        "Connect an approved remote signer using its bunker uri or discovery url.",
                    );
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut *self.remote_signer_input)
                            .hint_text("bunker://... or http://localhost/connect?uri=...")
                            .desired_width(ui.available_width()),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_centered(|ui| {
                        if ui
                            .add_enabled(action.enabled, egui::Button::new("Review Remote Signer"))
                            .clicked()
                        {
                            self.request_remote_signer_preview();
                        }
                        if ui.button("Cancel").clicked() {
                            self.clear_remote_signer_entry();
                            self.status_message = None;
                        }
                    });
                }
                RemoteSignerEntryState::Review(preview) => {
                    ui.label("Review the remote signer before connecting.");
                    ui.add_space(8.0);
                    ui.monospace(format!("source: {}", preview.source_label));
                    ui.monospace(format!("signer: {}", preview.signer_npub));
                    if preview.relays.is_empty() {
                        ui.label("No relays were provided by this signer.");
                    } else {
                        ui.label("Relays");
                        for relay in &preview.relays {
                            ui.monospace(relay);
                        }
                    }
                    ui.add_space(8.0);
                    if preview.requested_permissions.is_empty() {
                        ui.label("No additional permissions are requested in this slice.");
                    } else {
                        ui.label("Requested permissions");
                        for permission in &preview.requested_permissions {
                            ui.monospace(permission);
                        }
                    }
                    ui.add_space(8.0);
                    ui.horizontal_centered(|ui| {
                        if ui
                            .add_enabled(action.enabled, egui::Button::new(action.label.clone()))
                            .clicked()
                        {
                            self.request_remote_signer_connect();
                        }
                        if ui.button("Cancel").clicked() {
                            self.clear_remote_signer_entry();
                            self.status_message = None;
                        }
                    });
                }
                RemoteSignerEntryState::WaitingApproval(pending) => {
                    ui.label(action.label.as_str());
                    if pending.auth_url.is_some() {
                        ui.add_space(8.0);
                        ui.label(
                            "Authorize the remote signer in the browser, then keep this screen open while the app waits for the replayed response.",
                        );
                    } else if action.label == "Remote Signer Approval Check Retrying" {
                        ui.add_space(8.0);
                        ui.label(
                            "The app is retrying approval checks after a relay or network failure.",
                        );
                    } else {
                        ui.add_space(8.0);
                        ui.label("Remote signer connection is waiting for signer approval.");
                    }
                    ui.add_space(8.0);
                    ui.monospace(format!("signer: {}", pending.signer_npub));
                    if pending.relays.is_empty() {
                        ui.label("No relays were provided by this signer.");
                    } else {
                        ui.label("Relays");
                        for relay in &pending.relays {
                            ui.monospace(relay);
                        }
                    }
                    if let Some(auth_url) = &pending.auth_url {
                        ui.add_space(8.0);
                        ui.label("Authorization url");
                        ui.monospace(auth_url);
                    }
                    ui.add_space(8.0);
                    if ui.button("Cancel Pending Remote Signer").clicked() {
                        self.request_cancel_pending_remote_signer();
                    }
                }
            }
        });
    }

    fn render_home_account_section(&mut self, ui: &mut egui::Ui) {
        let AppScreen::Home { account_id } = &self.screen else {
            return;
        };
        let selected_account_id = account_id.clone();
        let selected_summary = self
            .account_roster
            .iter()
            .find(|account| account.account_id == selected_account_id)
            .cloned();

        ui.label("home");
        ui.add_space(8.0);
        ui.label("A signing identity is configured.");
        ui.add_space(12.0);

        if let Some(summary) = selected_summary {
            ui.label(summary.display_label());
            ui.monospace(format!("account id: {}", summary.account_id));
            ui.monospace(format!("npub: {}", summary.npub));
            ui.monospace(format!("custody: {}", summary.custody.label()));
            if summary.custody == RadrootsAccountCustody::RemoteSigner {
                if let Some(note_action) = self.backend.remote_signer_note_action_state() {
                    if note_action.pending {
                        ui.ctx().request_repaint();
                    }
                    ui.add_space(16.0);
                    ui.label("Remote signer note");
                    ui.add_space(8.0);
                    ui.add(
                        egui::TextEdit::multiline(&mut *self.remote_signer_note_input)
                            .hint_text("Write a kind 1 note to sign through the remote signer")
                            .desired_rows(3)
                            .desired_width(ui.available_width().min(560.0)),
                    );
                    ui.add_space(8.0);
                    if ui
                        .add_enabled(note_action.enabled, egui::Button::new(note_action.label))
                        .clicked()
                    {
                        self.request_remote_signer_note_action();
                    }
                }
            }
        } else {
            ui.label("Selected account details are unavailable.");
            ui.monospace(format!("account id: {selected_account_id}"));
        }

        if !self.account_roster.is_empty() {
            ui.add_space(16.0);
            ui.label("Accounts");
            let mut next_selected_account_id = None;
            for account in &self.account_roster {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let is_selected = account.account_id == selected_account_id;
                    ui.label(account.display_label());
                    ui.monospace(account.npub.as_str());
                    ui.monospace(account.custody.label());
                    if is_selected {
                        ui.label("selected");
                    } else if ui.button("Select Account").clicked() {
                        next_selected_account_id = Some(account.account_id.clone());
                    }
                });
            }
            if let Some(account_id) = next_selected_account_id {
                self.request_select_account(account_id.as_str());
            }
        }

        let home_setup_action = self.backend.home_setup_action_state();
        let import_action = self.backend.import_action_state();
        let import_paste_action = self.backend.import_paste_action_state();
        let remote_signer_action = self.backend.remote_signer_action_state();
        if home_setup_action.is_some() || import_action.is_some() || remote_signer_action.is_some()
        {
            ui.add_space(16.0);
            ui.label("Add account");
        }

        if let Some(home_setup_action) = home_setup_action {
            if home_setup_action.pending {
                ui.ctx().request_repaint();
            }
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    home_setup_action.enabled,
                    egui::Button::new(home_setup_action.label),
                )
                .clicked()
            {
                self.request_home_setup_action();
            }
        }

        if let Some(import_action) = import_action {
            if import_action.pending {
                ui.ctx().request_repaint();
            }
            if let Some(import_paste_action) = &import_paste_action {
                if import_paste_action.pending {
                    ui.ctx().request_repaint();
                }
            }
            ui.add_space(8.0);
            if self.pending_import_mode.is_some() {
                self.render_import_entry(ui, &import_action, import_paste_action.as_ref());
            } else if ui.button(import_action.label).clicked() {
                self.open_import_entry();
            }
        }

        if let Some(remote_signer_action) = remote_signer_action {
            if remote_signer_action.pending {
                ui.ctx().request_repaint();
            }
            ui.add_space(8.0);
            if matches!(
                self.remote_signer_entry_state,
                RemoteSignerEntryState::Closed
            ) {
                if ui
                    .add_enabled(
                        remote_signer_action.enabled,
                        egui::Button::new(remote_signer_action.label),
                    )
                    .clicked()
                {
                    self.open_remote_signer_entry();
                }
            } else {
                self.render_remote_signer_entry(ui, &remote_signer_action);
            }
        }
    }

    fn render_offline_geocoder_status(&self, ui: &mut egui::Ui) {
        let Some(state) = &self.offline_geocoder_state else {
            return;
        };

        ui.add_space(16.0);
        ui.label(state.summary_label());

        if let Some(user_message) = state.user_message() {
            ui.add_space(6.0);
            ui.label(user_message);
            ui.add_space(6.0);
            ui.collapsing("Offline geocoder details", |ui| {
                if let Some(diagnostic) = state.diagnostic() {
                    ui.label(diagnostic.technical_message);
                    ui.add_space(6.0);
                    ui.monospace(format!("platform: {}", diagnostic.platform_code));
                    ui.monospace(format!(
                        "asset revision: {}",
                        diagnostic.asset_revision.as_deref().unwrap_or("unknown")
                    ));
                    ui.monospace(format!("diagnostic code: {}", diagnostic.code));
                    if ui.button("Copy Offline Geocoder Diagnostic").clicked() {
                        ui.ctx().copy_text(diagnostic.export_text());
                    }
                }
                if cfg!(debug_assertions) {
                    if let Some(debug_message) = state.debug_message() {
                        ui.add_space(6.0);
                        ui.monospace(debug_message);
                    }
                }
            });
        }
    }
}

impl eframe::App for RadrootsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_backend();
        self.sync_revealed_secret_material_lifetime();
        self.clear_raw_secret_when_app_unfocused(ctx);
        if matches!(
            self.offline_geocoder_state,
            Some(RadrootsOfflineGeocoderState::Initializing)
        ) {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.home_location_tools.is_pending() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self
            .revealed_secret_material
            .as_ref()
            .is_some_and(RevealedSecretMaterial::is_raw)
        {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading(APP_NAME);
                ui.add_space(12.0);

                match self.screen.clone() {
                    AppScreen::Setup => {
                        let action = self.backend.setup_action_state();
                        if action.pending {
                            ctx.request_repaint();
                        }
                        let import_action = self.backend.import_action_state();
                        if let Some(import_action) = &import_action {
                            if import_action.pending {
                                ctx.request_repaint();
                            }
                        }
                        let import_paste_action = self.backend.import_paste_action_state();
                        if let Some(import_paste_action) = &import_paste_action {
                            if import_paste_action.pending {
                                ctx.request_repaint();
                            }
                        }
                        let remote_signer_action = self.backend.remote_signer_action_state();
                        if let Some(remote_signer_action) = &remote_signer_action {
                            if remote_signer_action.pending {
                                ctx.request_repaint();
                            }
                        }

                        ui.label("setup");
                        ui.add_space(8.0);
                        ui.label("A signing identity is required before the app can continue.");
                        ui.add_space(16.0);
                        let clicked = ui
                            .add_enabled(action.enabled, egui::Button::new(action.label))
                            .clicked();
                        if clicked {
                            self.request_setup_action();
                        }

                        if let Some(import_action) = import_action {
                            ui.add_space(12.0);
                            if self.pending_import_mode.is_some() {
                                self.render_import_entry(
                                    ui,
                                    &import_action,
                                    import_paste_action.as_ref(),
                                );
                            } else if ui.button(import_action.label).clicked() {
                                self.open_import_entry();
                            }
                        }

                        if let Some(remote_signer_action) = remote_signer_action {
                            ui.add_space(12.0);
                            if matches!(self.remote_signer_entry_state, RemoteSignerEntryState::Closed)
                            {
                                if ui
                                    .add_enabled(
                                        remote_signer_action.enabled,
                                        egui::Button::new(remote_signer_action.label),
                                    )
                                    .clicked()
                                {
                                    self.open_remote_signer_entry();
                                }
                            } else {
                                self.render_remote_signer_entry(ui, &remote_signer_action);
                            }
                        }
                    }
                    AppScreen::Home { .. } => {
                        self.render_home_account_section(ui);
                        self.home_location_tools.render(
                            ui,
                            self.backend.as_ref(),
                            self.offline_geocoder_state.as_ref(),
                        );

                        let actions = self.backend.home_action_states();
                        for (index, action) in actions.into_iter().enumerate() {
                            ui.add_space(if index == 0 { 20.0 } else { 12.0 });
                            if action.pending {
                                ctx.request_repaint();
                            }

                            if action.kind == HomeActionKind::BackupSecretKey
                                && self.pending_secret_key_backup_entry
                            {
                                self.render_secret_key_backup_entry(ui, &action);
                            } else if action.kind == HomeActionKind::BackupSecretKey
                                && ui
                                    .add_enabled(
                                        action.enabled,
                                        egui::Button::new(action.label.clone()),
                                    )
                                    .clicked()
                            {
                                self.pending_secret_key_backup_entry = true;
                                self.secret_key_backup_password_input.clear();
                                self.secret_key_backup_password_confirm_input.clear();
                                self.status_message = None;
                            } else if Self::home_action_requires_confirmation(action.kind)
                                && self.pending_home_confirmation == Some(action.kind)
                            {
                                ui.vertical_centered(|ui| {
                                    ui.set_max_width(ui.available_width().min(560.0));
                                    ui.label(Self::home_action_confirmation_message(action.kind));
                                    ui.add_space(8.0);
                                    ui.horizontal_centered(|ui| {
                                        let confirm_clicked = ui
                                            .add_enabled(
                                                action.enabled,
                                                egui::Button::new(action.label.clone()),
                                            )
                                            .clicked();
                                        if confirm_clicked {
                                            self.request_home_action(action.kind);
                                        }

                                        if ui.button("Cancel").clicked() {
                                            self.pending_home_confirmation = None;
                                            self.status_message = None;
                                        }
                                    });
                                });
                            } else if Self::home_action_requires_confirmation(action.kind)
                                && self.pending_home_confirmation.is_none()
                                && ui.button(action.label.clone()).clicked()
                            {
                                self.pending_home_confirmation = Some(action.kind);
                            } else if !Self::home_action_requires_confirmation(action.kind)
                                && ui
                                    .add_enabled(
                                        action.enabled,
                                        egui::Button::new(action.label.clone()),
                                    )
                                    .clicked()
                            {
                                self.request_home_action(action.kind);
                            }
                        }

                        if let Some((label, value, dismiss_label, is_raw)) =
                            self.revealed_secret_material.as_ref().map(|material| {
                                (
                                    material.label(),
                                    material.value().to_owned(),
                                    material.dismiss_label(),
                                    material.is_raw(),
                                )
                            })
                        {
                            ui.add_space(20.0);
                            ui.label(label);
                            ui.add_space(8.0);
                            ui.monospace(value);
                            if is_raw {
                                ui.add_space(8.0);
                                ui.label(
                                    "Raw secret reveal clears automatically after 30 seconds or when the app loses focus.",
                                );
                            }
                            ui.add_space(8.0);
                            if ui.button(dismiss_label).clicked() {
                                self.clear_revealed_secret_material();
                            }
                        }
                    }
                }

                if let Some(message) = &self.status_message {
                    ui.add_space(16.0);
                    ui.label(message);
                }

                self.render_offline_geocoder_status(ui);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{
        FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD, FIXTURE_BOB, fixture_identity_ncryptsec,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    #[derive(Clone)]
    struct MockBackend {
        load: Result<IdentityGateState, String>,
        account_roster: Rc<RefCell<Vec<RadrootsAccountSummary>>>,
        offline_geocoder_state: Rc<RefCell<Option<RadrootsOfflineGeocoderState>>>,
        offline_geocoder_poll:
            Rc<RefCell<VecDeque<Result<Option<RadrootsOfflineGeocoderState>, String>>>>,
        action_state: Rc<RefCell<SetupActionState>>,
        home_setup_action_state: Rc<RefCell<Option<SetupActionState>>>,
        import_action_state: Rc<RefCell<Option<ImportActionState>>>,
        import_paste_action_state: Rc<RefCell<Option<PasteActionState>>>,
        remote_signer_action_state: Rc<RefCell<Option<SetupActionState>>>,
        remote_signer_preview: Rc<RefCell<VecDeque<Result<RadrootsRemoteSignerPreview, String>>>>,
        remote_signer_request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        pending_remote_signer: Rc<RefCell<Option<RadrootsPendingRemoteSignerConnection>>>,
        cancel_pending_remote_signer: Rc<RefCell<VecDeque<Result<(), String>>>>,
        remote_signer_note_action_state: Rc<RefCell<Option<SetupActionState>>>,
        remote_signer_note_request: Rc<RefCell<VecDeque<Result<(), String>>>>,
        remote_signer_note_poll:
            Rc<RefCell<VecDeque<Result<Option<RadrootsRemoteSignerSignedNote>, String>>>>,
        home_action_states: Rc<RefCell<Vec<HomeActionState>>>,
        request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        home_setup_request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        import_request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        import_paste_request: Rc<RefCell<VecDeque<Result<Option<String>, String>>>>,
        secret_key_backup_request: Rc<RefCell<VecDeque<Result<HomeActionResult, String>>>>,
        home_request: Rc<RefCell<VecDeque<(HomeActionKind, Result<HomeActionResult, String>)>>>,
        home_poll: Rc<RefCell<VecDeque<Result<Option<HomeActionResult>, String>>>>,
        reverse_lookup_request: Rc<RefCell<VecDeque<Result<(), RadrootsLocationResolverError>>>>,
        reverse_lookup_poll:
            Rc<RefCell<VecDeque<Result<Option<RadrootsReverseLocationLookupResult>, String>>>>,
        select_account_request:
            Rc<RefCell<VecDeque<(String, Result<Option<IdentityGateState>, String>)>>>,
        poll: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
    }

    impl MockBackend {
        fn new(
            load: Result<IdentityGateState, String>,
            request: Vec<Result<Option<IdentityGateState>, String>>,
            poll: Vec<Result<Option<IdentityGateState>, String>>,
            action_state: SetupActionState,
        ) -> Self {
            Self {
                load,
                account_roster: Rc::new(RefCell::new(Vec::new())),
                offline_geocoder_state: Rc::new(RefCell::new(None)),
                offline_geocoder_poll: Rc::new(RefCell::new(VecDeque::new())),
                action_state: Rc::new(RefCell::new(action_state)),
                home_setup_action_state: Rc::new(RefCell::new(None)),
                import_action_state: Rc::new(RefCell::new(None)),
                import_paste_action_state: Rc::new(RefCell::new(None)),
                remote_signer_action_state: Rc::new(RefCell::new(None)),
                remote_signer_preview: Rc::new(RefCell::new(VecDeque::new())),
                remote_signer_request: Rc::new(RefCell::new(VecDeque::new())),
                pending_remote_signer: Rc::new(RefCell::new(None)),
                cancel_pending_remote_signer: Rc::new(RefCell::new(VecDeque::new())),
                remote_signer_note_action_state: Rc::new(RefCell::new(None)),
                remote_signer_note_request: Rc::new(RefCell::new(VecDeque::new())),
                remote_signer_note_poll: Rc::new(RefCell::new(VecDeque::new())),
                home_action_states: Rc::new(RefCell::new(Vec::new())),
                request: Rc::new(RefCell::new(request.into())),
                home_setup_request: Rc::new(RefCell::new(VecDeque::new())),
                import_request: Rc::new(RefCell::new(VecDeque::new())),
                import_paste_request: Rc::new(RefCell::new(VecDeque::new())),
                secret_key_backup_request: Rc::new(RefCell::new(VecDeque::new())),
                home_request: Rc::new(RefCell::new(VecDeque::new())),
                home_poll: Rc::new(RefCell::new(VecDeque::new())),
                reverse_lookup_request: Rc::new(RefCell::new(VecDeque::new())),
                reverse_lookup_poll: Rc::new(RefCell::new(VecDeque::new())),
                select_account_request: Rc::new(RefCell::new(VecDeque::new())),
                poll: Rc::new(RefCell::new(poll.into())),
            }
        }

        fn with_account_roster(self, account_roster: Vec<RadrootsAccountSummary>) -> Self {
            *self.account_roster.borrow_mut() = account_roster;
            self
        }

        fn with_offline_geocoder_state(
            self,
            state: RadrootsOfflineGeocoderState,
            poll: Vec<Result<Option<RadrootsOfflineGeocoderState>, String>>,
        ) -> Self {
            *self.offline_geocoder_state.borrow_mut() = Some(state);
            self.offline_geocoder_poll.borrow_mut().extend(poll);
            self
        }

        fn with_import_action(
            self,
            action_state: ImportActionState,
            request: Vec<Result<Option<IdentityGateState>, String>>,
        ) -> Self {
            *self.import_action_state.borrow_mut() = Some(action_state);
            self.import_request.borrow_mut().extend(request);
            self
        }

        fn with_home_setup_action(
            self,
            action_state: SetupActionState,
            request: Vec<Result<Option<IdentityGateState>, String>>,
        ) -> Self {
            *self.home_setup_action_state.borrow_mut() = Some(action_state);
            self.home_setup_request.borrow_mut().extend(request);
            self
        }

        fn with_import_paste_action(
            self,
            action_state: PasteActionState,
            request: Vec<Result<Option<String>, String>>,
        ) -> Self {
            *self.import_paste_action_state.borrow_mut() = Some(action_state);
            self.import_paste_request.borrow_mut().extend(request);
            self
        }

        fn with_remote_signer_action(self, action_state: SetupActionState) -> Self {
            *self.remote_signer_action_state.borrow_mut() = Some(action_state);
            self
        }

        fn with_remote_signer_preview(
            self,
            preview: Vec<Result<RadrootsRemoteSignerPreview, String>>,
        ) -> Self {
            self.remote_signer_preview.borrow_mut().extend(preview);
            self
        }

        fn with_remote_signer_request(
            self,
            request: Vec<Result<Option<IdentityGateState>, String>>,
        ) -> Self {
            self.remote_signer_request.borrow_mut().extend(request);
            self
        }

        fn with_pending_remote_signer(
            self,
            pending: Option<RadrootsPendingRemoteSignerConnection>,
        ) -> Self {
            *self.pending_remote_signer.borrow_mut() = pending;
            self
        }

        fn with_cancel_pending_remote_signer(self, request: Vec<Result<(), String>>) -> Self {
            self.cancel_pending_remote_signer
                .borrow_mut()
                .extend(request);
            self
        }

        fn with_home_action(
            self,
            action_state: HomeActionState,
            request: Vec<Result<HomeActionResult, String>>,
        ) -> Self {
            self.home_action_states
                .borrow_mut()
                .push(action_state.clone());
            self.home_request.borrow_mut().extend(
                request
                    .into_iter()
                    .map(|result| (action_state.kind, result)),
            );
            self
        }

        fn with_secret_key_backup_request(
            self,
            request: Vec<Result<HomeActionResult, String>>,
        ) -> Self {
            self.secret_key_backup_request.borrow_mut().extend(request);
            self
        }

        fn with_home_action_poll(
            self,
            poll: Vec<Result<Option<HomeActionResult>, String>>,
        ) -> Self {
            self.home_poll.borrow_mut().extend(poll);
            self
        }

        fn with_reverse_lookup(
            self,
            request: Vec<Result<(), RadrootsLocationResolverError>>,
            poll: Vec<Result<Option<RadrootsReverseLocationLookupResult>, String>>,
        ) -> Self {
            self.reverse_lookup_request.borrow_mut().extend(request);
            self.reverse_lookup_poll.borrow_mut().extend(poll);
            self
        }

        fn with_select_account(
            self,
            account_id: &str,
            request: Vec<Result<Option<IdentityGateState>, String>>,
        ) -> Self {
            self.select_account_request.borrow_mut().extend(
                request
                    .into_iter()
                    .map(|result| (account_id.to_owned(), result)),
            );
            self
        }
    }

    impl RadrootsAppBackend for MockBackend {
        fn load_identity_state(&self) -> Result<IdentityGateState, String> {
            self.load.clone()
        }

        fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
            Ok(self.account_roster.borrow().clone())
        }

        fn offline_geocoder_state(&self) -> Option<RadrootsOfflineGeocoderState> {
            self.offline_geocoder_state.borrow().clone()
        }

        fn poll_offline_geocoder_state(
            &self,
        ) -> Result<Option<RadrootsOfflineGeocoderState>, String> {
            self.offline_geocoder_poll
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn setup_action_state(&self) -> SetupActionState {
            self.action_state.borrow().clone()
        }

        fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
            self.request
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err("missing request response".into()))
        }

        fn home_setup_action_state(&self) -> Option<SetupActionState> {
            self.home_setup_action_state.borrow().clone()
        }

        fn request_home_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
            self.home_setup_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn import_action_state(&self) -> Option<ImportActionState> {
            self.import_action_state.borrow().clone()
        }

        fn request_import_action(
            &self,
            _request: &RadrootsSecretImportRequest,
        ) -> Result<Option<IdentityGateState>, String> {
            self.import_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn request_secret_key_backup_action(
            &self,
            _password: &str,
        ) -> Result<HomeActionResult, String> {
            self.secret_key_backup_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(HomeActionResult::None))
        }

        fn import_paste_action_state(&self) -> Option<PasteActionState> {
            self.import_paste_action_state.borrow().clone()
        }

        fn request_import_paste_action(&self) -> Result<Option<String>, String> {
            self.import_paste_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn remote_signer_action_state(&self) -> Option<SetupActionState> {
            self.remote_signer_action_state.borrow().clone()
        }

        fn preview_remote_signer_connection(
            &self,
            _input: &str,
        ) -> Result<RadrootsRemoteSignerPreview, String> {
            self.remote_signer_preview
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err("missing remote signer preview".into()))
        }

        fn request_remote_signer_connection(
            &self,
            _input: &str,
        ) -> Result<Option<IdentityGateState>, String> {
            self.remote_signer_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn pending_remote_signer_connection(
            &self,
        ) -> Result<Option<RadrootsPendingRemoteSignerConnection>, String> {
            Ok(self.pending_remote_signer.borrow().clone())
        }

        fn request_cancel_pending_remote_signer_connection(&self) -> Result<(), String> {
            let result = self
                .cancel_pending_remote_signer
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(()));
            if result.is_ok() {
                *self.pending_remote_signer.borrow_mut() = None;
            }
            result
        }

        fn remote_signer_note_action_state(&self) -> Option<SetupActionState> {
            self.remote_signer_note_action_state.borrow().clone()
        }

        fn request_remote_signer_note_action(&self, _content: &str) -> Result<(), String> {
            self.remote_signer_note_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn poll_remote_signer_note_action_result(
            &self,
        ) -> Result<Option<RadrootsRemoteSignerSignedNote>, String> {
            self.remote_signer_note_poll
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn home_action_states(&self) -> Vec<HomeActionState> {
            self.home_action_states.borrow().clone()
        }

        fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
            let Some((expected_action, response)) = self.home_request.borrow_mut().pop_front()
            else {
                return Err("missing home action response".into());
            };
            if expected_action != action {
                return Err(format!(
                    "unexpected home action request: expected {:?}, got {:?}",
                    expected_action, action
                ));
            }
            response
        }

        fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
            self.home_poll.borrow_mut().pop_front().unwrap_or(Ok(None))
        }

        fn request_select_account(
            &self,
            account_id: &str,
        ) -> Result<Option<IdentityGateState>, String> {
            let Some((expected_account_id, response)) =
                self.select_account_request.borrow_mut().pop_front()
            else {
                return Err("missing select-account response".into());
            };
            if expected_account_id != account_id {
                return Err(format!(
                    "unexpected account selection request: expected {expected_account_id}, got {account_id}"
                ));
            }
            response
        }

        fn request_reverse_location_lookup(
            &self,
            _point: RadrootsLocationPoint,
            _options: Option<RadrootsLocationReverseOptions>,
        ) -> Result<(), RadrootsLocationResolverError> {
            self.reverse_lookup_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Err(RadrootsLocationResolverError::Unsupported))
        }

        fn poll_reverse_location_lookup_result(
            &self,
        ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
            self.reverse_lookup_poll
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
            self.poll.borrow_mut().pop_front().unwrap_or(Ok(None))
        }
    }

    fn fixture_account_summary() -> RadrootsAccountSummary {
        RadrootsAccountSummary {
            account_id: FIXTURE_ALICE.account_id.into(),
            npub: FIXTURE_ALICE.npub.into(),
            label: Some("fixture alice".into()),
            custody: RadrootsAccountCustody::LocalManaged,
        }
    }

    fn fixture_bob_account_summary() -> RadrootsAccountSummary {
        RadrootsAccountSummary {
            account_id: FIXTURE_BOB.account_id.into(),
            npub: FIXTURE_BOB.npub.into(),
            label: Some("fixture bob".into()),
            custody: RadrootsAccountCustody::LocalManaged,
        }
    }

    fn fixture_ready_state() -> IdentityGateState {
        IdentityGateState::Ready {
            account_id: FIXTURE_ALICE.account_id.into(),
        }
    }

    fn fixture_home_screen() -> AppScreen {
        AppScreen::Home {
            account_id: FIXTURE_ALICE.account_id.into(),
        }
    }

    fn fixture_remote_signer_preview() -> RadrootsRemoteSignerPreview {
        RadrootsRemoteSignerPreview {
            source_label: "discovery url".into(),
            signer_npub: FIXTURE_BOB.npub.into(),
            relays: vec!["ws://localhost:8080".into()],
            requested_permissions: vec!["sign_event:kind:1".into(), "switch_relays".into()],
        }
    }

    fn fixture_pending_remote_signer() -> RadrootsPendingRemoteSignerConnection {
        fixture_remote_signer_preview().pending_summary()
    }

    #[test]
    fn startup_missing_key_enters_setup() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));
        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn startup_ready_key_enters_home() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(fixture_ready_state()),
            vec![],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));
        assert_eq!(app.screen, fixture_home_screen());
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn startup_ready_key_loads_account_roster() {
        let app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_account_roster(vec![fixture_account_summary()]),
        ));

        assert_eq!(app.account_roster, vec![fixture_account_summary()]);
    }

    #[test]
    fn startup_restores_pending_remote_signer_connection() {
        let app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_remote_signer_action(SetupActionState {
                label: "Connect Remote Signer".into(),
                enabled: true,
                pending: false,
            })
            .with_pending_remote_signer(Some(fixture_pending_remote_signer())),
        ));

        assert_eq!(
            app.remote_signer_entry_state,
            RemoteSignerEntryState::WaitingApproval(fixture_pending_remote_signer())
        );
    }

    #[test]
    fn startup_unsupported_shows_reason() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Unsupported {
                reason: "unsupported".into(),
            }),
            vec![],
            vec![],
            SetupActionState {
                label: "Connect Browser Signer".into(),
                enabled: false,
                pending: false,
            },
        )));
        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message.as_deref(), Some("unsupported"));
    }

    #[test]
    fn deferred_setup_action_transitions_to_home_after_poll() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![Ok(None)],
            vec![Ok(Some(fixture_ready_state()))],
            SetupActionState {
                label: "Connect Browser Signer".into(),
                enabled: true,
                pending: false,
            },
        )));

        app.request_setup_action();
        assert_eq!(app.screen, AppScreen::Setup);

        app.sync_backend();

        assert_eq!(app.screen, fixture_home_screen());
    }

    #[test]
    fn immediate_setup_action_transitions_to_home() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![Ok(Some(fixture_ready_state()))],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));

        app.request_setup_action();

        assert_eq!(app.screen, fixture_home_screen());
    }

    #[test]
    fn home_setup_action_transitions_to_new_selected_account() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_account_roster(vec![
                fixture_account_summary(),
                fixture_bob_account_summary(),
            ])
            .with_home_setup_action(
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(Some(IdentityGateState::Ready {
                    account_id: FIXTURE_BOB.account_id.into(),
                }))],
            ),
        ));

        app.request_home_setup_action();

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: FIXTURE_BOB.account_id.into(),
            }
        );
        assert_eq!(app.account_roster.len(), 2);
    }

    #[test]
    fn select_account_transitions_to_requested_account() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_account_roster(vec![
                fixture_account_summary(),
                fixture_bob_account_summary(),
            ])
            .with_select_account(
                FIXTURE_BOB.account_id,
                vec![Ok(Some(IdentityGateState::Ready {
                    account_id: FIXTURE_BOB.account_id.into(),
                }))],
            ),
        ));

        app.request_select_account(FIXTURE_BOB.account_id);

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: FIXTURE_BOB.account_id.into(),
            }
        );
    }

    #[test]
    fn home_remove_action_transitions_to_setup() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::RemoveLocalKey,
                    label: "Remove Key From This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::IdentityState(
                    IdentityGateState::Missing,
                ))],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::RemoveLocalKey);
        app.request_home_action(HomeActionKind::RemoveLocalKey);

        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message, None);
        assert_eq!(app.pending_home_confirmation, None);
    }

    #[test]
    fn failed_home_remove_action_keeps_home_screen_and_message() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::RemoveLocalKey,
                    label: "Remove Key From This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Err("remove failed".into())],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::RemoveLocalKey);
        app.request_home_action(HomeActionKind::RemoveLocalKey);

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.status_message.as_deref(), Some("remove failed"));
        assert_eq!(
            app.pending_home_confirmation,
            Some(HomeActionKind::RemoveLocalKey)
        );
    }

    #[test]
    fn home_reset_action_transitions_to_setup() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::ResetDevice,
                    label: "Reset This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::IdentityState(
                    IdentityGateState::Missing,
                ))],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::ResetDevice);
        app.request_home_action(HomeActionKind::ResetDevice);

        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message, None);
        assert_eq!(app.pending_home_confirmation, None);
    }

    #[test]
    fn failed_home_reset_action_keeps_home_screen_and_message() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::ResetDevice,
                    label: "Reset This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Err("reset failed".into())],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::ResetDevice);
        app.request_home_action(HomeActionKind::ResetDevice);

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.status_message.as_deref(), Some("reset failed"));
        assert_eq!(
            app.pending_home_confirmation,
            Some(HomeActionKind::ResetDevice)
        );
    }

    #[test]
    fn import_action_transitions_to_home() {
        let encrypted_secret_key =
            fixture_identity_ncryptsec(&FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD)
                .expect("fixture encrypted secret key");
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_import_action(
                ImportActionState {
                    label: "Import Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(Some(fixture_ready_state()))],
            ),
        ));

        app.pending_import_mode = Some(RadrootsSecretImportMode::EncryptedSecretKey);
        app.secret_key_input = Zeroizing::new(encrypted_secret_key);
        app.import_password_input = Zeroizing::new(FIXTURE_BACKUP_PASSWORD.into());
        app.request_import_action();

        assert_eq!(app.screen, fixture_home_screen());
        assert_eq!(app.pending_import_mode, None);
        assert_eq!(app.secret_key_input.as_str(), "");
        assert_eq!(app.import_password_input.as_str(), "");
    }

    #[test]
    fn import_paste_action_populates_secret_key_input() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_import_action(
                ImportActionState {
                    label: "Import Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![],
            )
            .with_import_paste_action(
                PasteActionState {
                    label: "Paste Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(Some(FIXTURE_ALICE.nsec.into()))],
            ),
        ));

        app.pending_import_mode = Some(RadrootsSecretImportMode::EncryptedSecretKey);
        app.request_import_paste_action();

        assert_eq!(app.secret_key_input.as_str(), FIXTURE_ALICE.nsec);
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn remote_signer_preview_moves_entry_into_review_state() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_remote_signer_action(SetupActionState {
                label: "Connect Remote Signer".into(),
                enabled: true,
                pending: false,
            })
            .with_remote_signer_preview(vec![Ok(fixture_remote_signer_preview())]),
        ));

        app.open_remote_signer_entry();
        app.remote_signer_input =
            Zeroizing::new("http://localhost/connect?uri=bunker%3A%2F%2Fexample".into());
        app.request_remote_signer_preview();

        assert_eq!(
            app.remote_signer_entry_state,
            RemoteSignerEntryState::Review(fixture_remote_signer_preview())
        );
    }

    #[test]
    fn remote_signer_connect_enters_waiting_state() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_remote_signer_action(SetupActionState {
                label: "Connect Remote Signer".into(),
                enabled: true,
                pending: false,
            })
            .with_remote_signer_request(vec![Ok(None)])
            .with_pending_remote_signer(Some(fixture_pending_remote_signer())),
        ));

        app.remote_signer_entry_state =
            RemoteSignerEntryState::Review(fixture_remote_signer_preview());
        app.remote_signer_input =
            Zeroizing::new("http://localhost/connect?uri=bunker%3A%2F%2Fexample".into());
        app.request_remote_signer_connect();

        assert_eq!(
            app.remote_signer_entry_state,
            RemoteSignerEntryState::WaitingApproval(fixture_pending_remote_signer())
        );
    }

    #[test]
    fn cancel_pending_remote_signer_clears_waiting_state() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_remote_signer_action(SetupActionState {
                label: "Connect Remote Signer".into(),
                enabled: true,
                pending: false,
            })
            .with_pending_remote_signer(Some(fixture_pending_remote_signer()))
            .with_cancel_pending_remote_signer(vec![Ok(())]),
        ));

        app.request_cancel_pending_remote_signer();

        assert_eq!(
            app.remote_signer_entry_state,
            RemoteSignerEntryState::Closed
        );
    }

    #[test]
    fn encrypted_backup_home_action_reveals_secret_key_without_leaving_home() {
        let encrypted_secret_key =
            fixture_identity_ncryptsec(&FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD)
                .expect("fixture encrypted secret key");
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::BackupSecretKey,
                    label: "Back Up Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![],
            )
            .with_secret_key_backup_request(vec![Ok(
                HomeActionResult::RevealEncryptedSecretKey {
                    ncryptsec: encrypted_secret_key.clone(),
                },
            )]),
        ));

        app.pending_secret_key_backup_entry = true;
        app.secret_key_backup_password_input = Zeroizing::new(FIXTURE_BACKUP_PASSWORD.into());
        app.secret_key_backup_password_confirm_input =
            Zeroizing::new(FIXTURE_BACKUP_PASSWORD.into());
        app.request_secret_key_backup_action();

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.pending_home_confirmation, None);
        assert_eq!(app.pending_secret_key_backup_entry, false);
        let Some(RevealedSecretMaterial::EncryptedSecretKey(value)) =
            app.revealed_secret_material.as_ref()
        else {
            panic!("expected encrypted secret backup");
        };
        assert_eq!(value.as_str(), encrypted_secret_key);
    }

    #[test]
    fn deferred_encrypted_backup_home_action_reveals_secret_key_after_poll() {
        let encrypted_secret_key =
            fixture_identity_ncryptsec(&FIXTURE_ALICE, FIXTURE_BACKUP_PASSWORD)
                .expect("fixture encrypted secret key");
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::BackupSecretKey,
                    label: "Back Up Secret Key".into(),
                    enabled: true,
                    pending: true,
                },
                vec![],
            )
            .with_secret_key_backup_request(vec![Ok(HomeActionResult::None)])
            .with_home_action_poll(vec![Ok(Some(
                HomeActionResult::RevealEncryptedSecretKey {
                    ncryptsec: encrypted_secret_key.clone(),
                },
            ))]),
        ));

        app.pending_secret_key_backup_entry = true;
        app.secret_key_backup_password_input = Zeroizing::new(FIXTURE_BACKUP_PASSWORD.into());
        app.secret_key_backup_password_confirm_input =
            Zeroizing::new(FIXTURE_BACKUP_PASSWORD.into());
        app.request_secret_key_backup_action();
        assert_eq!(app.revealed_secret_material, None);

        app.sync_backend();

        let Some(RevealedSecretMaterial::EncryptedSecretKey(value)) =
            app.revealed_secret_material.as_ref()
        else {
            panic!("expected encrypted secret backup");
        };
        assert_eq!(value.as_str(), encrypted_secret_key);
    }

    #[test]
    fn raw_secret_reveal_home_action_uses_advanced_path() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::RevealRawSecretKey,
                    label: "Reveal Raw Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::RevealRawSecretKey {
                    nsec: FIXTURE_ALICE.nsec.into(),
                })],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::RevealRawSecretKey);
        app.request_home_action(HomeActionKind::RevealRawSecretKey);

        let Some(RevealedSecretMaterial::RawSecretKey { nsec, .. }) =
            app.revealed_secret_material.as_ref()
        else {
            panic!("expected raw secret reveal");
        };
        assert_eq!(nsec.as_str(), FIXTURE_ALICE.nsec);
    }

    #[test]
    fn raw_secret_reveal_expires_after_timeout() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(fixture_ready_state()),
            vec![],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));
        app.revealed_secret_material = Some(RevealedSecretMaterial::RawSecretKey {
            nsec: Zeroizing::new(FIXTURE_ALICE.nsec.into()),
            revealed_at: Instant::now() - RAW_SECRET_REVEAL_TIMEOUT - Duration::from_secs(1),
        });

        app.sync_revealed_secret_material_lifetime();

        assert_eq!(app.revealed_secret_material, None);
    }

    #[test]
    fn raw_secret_reveal_clears_when_app_loses_focus() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(fixture_ready_state()),
            vec![],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));
        app.revealed_secret_material = Some(RevealedSecretMaterial::RawSecretKey {
            nsec: Zeroizing::new(FIXTURE_ALICE.nsec.into()),
            revealed_at: Instant::now(),
        });

        let ctx = egui::Context::default();
        ctx.input_mut(|input| {
            input
                .raw
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .focused = Some(false);
        });
        app.clear_raw_secret_when_app_unfocused(&ctx);

        assert_eq!(app.revealed_secret_material, None);
    }

    #[test]
    fn deferred_home_location_lookup_updates_after_poll() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(fixture_ready_state()),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_offline_geocoder_state(RadrootsOfflineGeocoderState::Ready, vec![])
            .with_reverse_lookup(
                vec![Ok(())],
                vec![Ok(Some(Ok(vec![RadrootsResolvedLocation {
                    id: 7,
                    name: "Paris".into(),
                    admin1_id: Some(11),
                    admin1_name: Some("Ile-de-France".into()),
                    country_id: "FR".into(),
                    country_name: Some("France".into()),
                    point: RadrootsLocationPoint {
                        lat: 48.8566,
                        lng: 2.3522,
                    },
                }])))],
            ),
        ));

        app.home_location_tools
            .set_query_inputs("48.8566", "2.3522");
        app.home_location_tools
            .begin_resolve_with_backend(app.backend.as_ref());
        assert!(app.home_location_tools.is_pending());

        app.sync_backend();

        assert_eq!(app.home_location_tools.status_message(), None);
        assert_eq!(
            app.home_location_tools
                .lookup_result()
                .as_ref()
                .map(|result| result.matches[0].name.as_str()),
            Some("Paris")
        );
    }

    #[test]
    fn startup_uses_initial_offline_geocoder_state() {
        let app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_offline_geocoder_state(RadrootsOfflineGeocoderState::Initializing, vec![]),
        ));

        assert_eq!(
            app.offline_geocoder_state,
            Some(RadrootsOfflineGeocoderState::Initializing)
        );
    }

    #[test]
    fn offline_geocoder_state_updates_after_poll() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_offline_geocoder_state(
                RadrootsOfflineGeocoderState::Initializing,
                vec![Ok(Some(RadrootsOfflineGeocoderState::Ready))],
            ),
        ));

        app.sync_backend();

        assert_eq!(
            app.offline_geocoder_state,
            Some(RadrootsOfflineGeocoderState::Ready)
        );
    }

    #[test]
    fn offline_geocoder_failure_keeps_user_and_debug_messages() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_offline_geocoder_state(
                RadrootsOfflineGeocoderState::Initializing,
                vec![Ok(Some(RadrootsOfflineGeocoderState::unavailable(
                    RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
                    RadrootsOfflineGeocoderPlatform::Desktop,
                    "failed to open staged geocoder db",
                )))],
            ),
        ));

        app.sync_backend();

        assert_eq!(
            app.offline_geocoder_state,
            Some(RadrootsOfflineGeocoderState::unavailable(
                RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
                RadrootsOfflineGeocoderPlatform::Desktop,
                "failed to open staged geocoder db",
            ))
        );
        assert_eq!(
            app.offline_geocoder_state
                .as_ref()
                .and_then(RadrootsOfflineGeocoderState::user_message),
            Some("Offline geocoder could not be initialized on this device.")
        );
        assert_eq!(
            app.offline_geocoder_state
                .as_ref()
                .and_then(RadrootsOfflineGeocoderState::debug_message),
            Some("failed to open staged geocoder db")
        );
        let diagnostic = app
            .offline_geocoder_state
            .as_ref()
            .and_then(RadrootsOfflineGeocoderState::diagnostic)
            .unwrap();
        assert_eq!(diagnostic.platform_code, "desktop");
        assert_eq!(diagnostic.asset_revision, None);
        assert_eq!(diagnostic.code, "initialization_failed");
        assert!(
            !diagnostic
                .export_text()
                .contains("failed to open staged geocoder db")
        );
    }
}
