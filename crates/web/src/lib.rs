#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{JsCast as _, JsValue};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use nostr::nips::nip19::ToBech32;
#[cfg(target_arch = "wasm32")]
use nostr::signer::NostrSigner;
#[cfg(target_arch = "wasm32")]
use nostr_browser_signer::{BrowserSigner, Error as BrowserSignerError};
#[cfg(target_arch = "wasm32")]
use radroots_studio_app_core::{
    HomeActionKind, HomeActionResult, HomeActionState, IdentityGateState, RadrootsAccountCustody,
    RadrootsAccountSummary, RadrootsApp, RadrootsAppBackend, RadrootsLocationCountry,
    RadrootsLocationCountryCenterLookupResult, RadrootsLocationCountryListResult,
    RadrootsLocationPoint, RadrootsLocationResolverError, RadrootsLocationReverseOptions,
    RadrootsResolvedLocation, RadrootsReverseLocationLookupResult, SetupActionState,
};
#[cfg(any(target_arch = "wasm32", test))]
use radroots_studio_app_core::{
    RadrootsOfflineGeocoderPlatform, RadrootsOfflineGeocoderState,
    RadrootsOfflineGeocoderUnavailableKind,
};
#[cfg(target_arch = "wasm32")]
use radroots_geocoder::{
    Geocoder, GeocoderCountryListResult, GeocoderError, GeocoderPoint, GeocoderReverseOptions,
    GeocoderReverseResult,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
const GEOCODER_DB_ASSET_PATH: &str = "assets/geocoder/geonames.db";
#[cfg(target_arch = "wasm32")]
const GEOCODER_REVISION_ASSET_PATH: &str = "assets/geocoder/geonames.revision";

#[cfg(any(target_arch = "wasm32", test))]
fn offline_geocoder_missing_build_asset_state(
    debug_message: impl Into<String>,
) -> RadrootsOfflineGeocoderState {
    RadrootsOfflineGeocoderState::unavailable(
        RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
        RadrootsOfflineGeocoderPlatform::Web,
        debug_message,
    )
}

#[cfg(any(target_arch = "wasm32", test))]
fn offline_geocoder_initialization_failed_state(
    asset_revision: impl Into<String>,
    debug_message: impl Into<String>,
) -> RadrootsOfflineGeocoderState {
    RadrootsOfflineGeocoderState::unavailable_with_revision(
        RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
        RadrootsOfflineGeocoderPlatform::Web,
        asset_revision,
        debug_message,
    )
}

#[cfg(any(target_arch = "wasm32", test))]
fn is_valid_asset_revision(revision: &str) -> bool {
    revision.len() == 64 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(target_arch = "wasm32")]
fn js_error_message(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "javascript error".to_owned())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_response(path: &str) -> Result<web_sys::Response, String> {
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_owned())?;
    let response_value = JsFuture::from(window.fetch_with_str(path))
        .await
        .map_err(|err| format!("failed to fetch {path}: {}", js_error_message(err)))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|_| format!("fetch for {path} did not return a response"))?;
    if !response.ok() {
        return Err(format!(
            "fetch for {path} failed with http {}",
            response.status()
        ));
    }
    Ok(response)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_text_asset(path: &str) -> Result<String, String> {
    let response = fetch_response(path).await?;
    let text_promise = response.text().map_err(|err| {
        format!(
            "failed to read text body for {path}: {}",
            js_error_message(err)
        )
    })?;
    let text_value = JsFuture::from(text_promise).await.map_err(|err| {
        format!(
            "failed to load text asset {path}: {}",
            js_error_message(err)
        )
    })?;
    text_value
        .as_string()
        .ok_or_else(|| format!("text asset {path} did not decode to a string"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes_asset(path: &str) -> Result<Vec<u8>, String> {
    let response = fetch_response(path).await?;
    let buffer_promise = response.array_buffer().map_err(|err| {
        format!(
            "failed to read binary body for {path}: {}",
            js_error_message(err)
        )
    })?;
    let buffer = JsFuture::from(buffer_promise).await.map_err(|err| {
        format!(
            "failed to load binary asset {path}: {}",
            js_error_message(err)
        )
    })?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

#[cfg(target_arch = "wasm32")]
async fn initialize_offline_geocoder() -> Result<Geocoder, RadrootsOfflineGeocoderState> {
    let revision_text = fetch_text_asset(GEOCODER_REVISION_ASSET_PATH)
        .await
        .map_err(offline_geocoder_missing_build_asset_state)?;
    let revision = revision_text.trim().to_owned();
    if !is_valid_asset_revision(revision.as_str()) {
        return Err(offline_geocoder_missing_build_asset_state(format!(
            "web geocoder revision asset invalid at {GEOCODER_REVISION_ASSET_PATH}"
        )));
    }

    let bytes = fetch_bytes_asset(GEOCODER_DB_ASSET_PATH)
        .await
        .map_err(|debug_message| {
            RadrootsOfflineGeocoderState::unavailable_with_revision(
                RadrootsOfflineGeocoderUnavailableKind::MissingBuildAsset,
                RadrootsOfflineGeocoderPlatform::Web,
                revision.clone(),
                debug_message,
            )
        })?;

    Geocoder::open_bytes(bytes.as_slice()).map_err(|source| {
        offline_geocoder_initialization_failed_state(
            revision,
            format!("failed to open wasm geocoder from {GEOCODER_DB_ASSET_PATH}: {source}"),
        )
    })
}

#[cfg(target_arch = "wasm32")]
fn map_reverse_result(result: GeocoderReverseResult) -> RadrootsResolvedLocation {
    RadrootsResolvedLocation {
        id: result.id,
        name: result.name,
        admin1_id: result.admin1_id,
        admin1_name: result.admin1_name,
        country_id: result.country_id,
        country_name: result.country_name,
        point: RadrootsLocationPoint {
            lat: result.latitude,
            lng: result.longitude,
        },
    }
}

#[cfg(target_arch = "wasm32")]
fn map_country_result(result: GeocoderCountryListResult) -> RadrootsLocationCountry {
    RadrootsLocationCountry {
        country_id: result.country_id,
        country_name: result.country,
        center: RadrootsLocationPoint {
            lat: result.lat,
            lng: result.lng,
        },
    }
}

#[cfg(target_arch = "wasm32")]
fn map_country_center_error(source: GeocoderError) -> RadrootsLocationResolverError {
    match source {
        GeocoderError::CountryCenterNotFound { country_id } => {
            RadrootsLocationResolverError::CountryCenterNotFound { country_id }
        }
        other => RadrootsLocationResolverError::QueryFailed {
            message: other.to_string(),
        },
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct ConnectedSigner {
    account_id: String,
    npub: String,
    signer: BrowserSigner,
}

#[cfg(target_arch = "wasm32")]
enum WebConnectionState {
    Disconnected,
    Connecting,
    Ready(ConnectedSigner),
}

#[cfg(target_arch = "wasm32")]
struct WebBackendState {
    connection: WebConnectionState,
    pending_result: Option<Result<ConnectedSigner, String>>,
    offline_geocoder_state: RadrootsOfflineGeocoderState,
    pending_offline_geocoder_update: Option<RadrootsOfflineGeocoderState>,
    geocoder: Option<Rc<Geocoder>>,
    pending_reverse_lookup_result: Option<RadrootsReverseLocationLookupResult>,
    pending_country_list_result: Option<RadrootsLocationCountryListResult>,
    pending_country_center_result: Option<RadrootsLocationCountryCenterLookupResult>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct WebBackend {
    state: Rc<RefCell<WebBackendState>>,
}

#[cfg(target_arch = "wasm32")]
impl WebBackend {
    fn new() -> Self {
        let backend = Self {
            state: Rc::new(RefCell::new(WebBackendState {
                connection: WebConnectionState::Disconnected,
                pending_result: None,
                offline_geocoder_state: RadrootsOfflineGeocoderState::Initializing,
                pending_offline_geocoder_update: None,
                geocoder: None,
                pending_reverse_lookup_result: None,
                pending_country_list_result: None,
                pending_country_center_result: None,
            })),
        };
        backend.start_offline_geocoder_init();
        backend
    }

    fn identity_state_for_ready(connected: &ConnectedSigner) -> IdentityGateState {
        let _ = &connected.signer;
        IdentityGateState::Ready {
            account_id: connected.account_id.clone(),
        }
    }

    fn account_summary_for_ready(connected: &ConnectedSigner) -> RadrootsAccountSummary {
        RadrootsAccountSummary {
            account_id: connected.account_id.clone(),
            npub: connected.npub.clone(),
            label: Some("browser signer".to_owned()),
            custody: RadrootsAccountCustody::BrowserSigner,
        }
    }

    fn connect_error_message(err: BrowserSignerError) -> String {
        match err {
            BrowserSignerError::NoGlobalWindowObject | BrowserSignerError::NamespaceNotFound(_) => {
                "No NIP-07 browser signer detected.".to_owned()
            }
            other => format!("Browser signer connection failed: {other}"),
        }
    }

    fn disconnect_signer(&self) -> IdentityGateState {
        let mut state = self.state.borrow_mut();
        state.connection = WebConnectionState::Disconnected;
        state.pending_result = None;
        IdentityGateState::Missing
    }

    fn start_offline_geocoder_init(&self) {
        let shared_state = Rc::clone(&self.state);
        wasm_bindgen_futures::spawn_local(async move {
            let result = initialize_offline_geocoder().await;
            let mut state = shared_state.borrow_mut();
            match result {
                Ok(geocoder) => {
                    state.geocoder = Some(Rc::new(geocoder));
                    state.offline_geocoder_state = RadrootsOfflineGeocoderState::Ready;
                    state.pending_offline_geocoder_update =
                        Some(RadrootsOfflineGeocoderState::Ready);
                }
                Err(offline_geocoder_state) => {
                    state.geocoder = None;
                    state.offline_geocoder_state = offline_geocoder_state.clone();
                    state.pending_offline_geocoder_update = Some(offline_geocoder_state);
                }
            }
        });
    }

    fn ready_geocoder(&self) -> Result<Rc<Geocoder>, RadrootsLocationResolverError> {
        let state = self.state.borrow();
        match &state.offline_geocoder_state {
            RadrootsOfflineGeocoderState::Initializing => {
                Err(RadrootsLocationResolverError::Initializing)
            }
            RadrootsOfflineGeocoderState::Unavailable { .. } => {
                Err(RadrootsLocationResolverError::Unavailable)
            }
            RadrootsOfflineGeocoderState::Ready => {
                state
                    .geocoder
                    .clone()
                    .ok_or_else(|| RadrootsLocationResolverError::QueryFailed {
                        message: "web geocoder was ready without an initialized engine".to_owned(),
                    })
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl RadrootsAppBackend for WebBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        let state = self.state.borrow();
        match &state.connection {
            WebConnectionState::Ready(connected) => Ok(Self::identity_state_for_ready(connected)),
            WebConnectionState::Disconnected | WebConnectionState::Connecting => {
                Ok(IdentityGateState::Missing)
            }
        }
    }

    fn load_account_roster(&self) -> Result<Vec<RadrootsAccountSummary>, String> {
        let state = self.state.borrow();
        match &state.connection {
            WebConnectionState::Ready(connected) => {
                Ok(vec![Self::account_summary_for_ready(connected)])
            }
            WebConnectionState::Disconnected | WebConnectionState::Connecting => Ok(Vec::new()),
        }
    }

    fn offline_geocoder_state(&self) -> Option<RadrootsOfflineGeocoderState> {
        Some(self.state.borrow().offline_geocoder_state.clone())
    }

    fn poll_offline_geocoder_state(&self) -> Result<Option<RadrootsOfflineGeocoderState>, String> {
        Ok(self
            .state
            .borrow_mut()
            .pending_offline_geocoder_update
            .take())
    }

    fn reverse_location(
        &self,
        point: RadrootsLocationPoint,
        options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<Vec<RadrootsResolvedLocation>, RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        let options = options.map(|options| GeocoderReverseOptions {
            limit: options.limit,
            degree_offset: options.degree_offset,
        });
        geocoder
            .reverse(
                GeocoderPoint {
                    lat: point.lat,
                    lng: point.lng,
                },
                options,
            )
            .map(|results| results.into_iter().map(map_reverse_result).collect())
            .map_err(|source| RadrootsLocationResolverError::QueryFailed {
                message: source.to_string(),
            })
    }

    fn request_reverse_location_lookup(
        &self,
        point: RadrootsLocationPoint,
        options: Option<RadrootsLocationReverseOptions>,
    ) -> Result<(), RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        {
            let mut state = self.state.borrow_mut();
            state.pending_reverse_lookup_result = None;
        }
        let shared_state = Rc::clone(&self.state);
        wasm_bindgen_futures::spawn_local(async move {
            let options = options.map(|options| GeocoderReverseOptions {
                limit: options.limit,
                degree_offset: options.degree_offset,
            });
            let result = geocoder
                .reverse(
                    GeocoderPoint {
                        lat: point.lat,
                        lng: point.lng,
                    },
                    options,
                )
                .map(|results| results.into_iter().map(map_reverse_result).collect())
                .map_err(|source| RadrootsLocationResolverError::QueryFailed {
                    message: source.to_string(),
                });
            shared_state.borrow_mut().pending_reverse_lookup_result = Some(result);
        });
        Ok(())
    }

    fn poll_reverse_location_lookup_result(
        &self,
    ) -> Result<Option<RadrootsReverseLocationLookupResult>, String> {
        Ok(self.state.borrow_mut().pending_reverse_lookup_result.take())
    }

    fn request_location_country_list(&self) -> Result<(), RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        {
            let mut state = self.state.borrow_mut();
            state.pending_country_list_result = None;
        }
        let shared_state = Rc::clone(&self.state);
        wasm_bindgen_futures::spawn_local(async move {
            let result = geocoder
                .country_list()
                .map(|results| results.into_iter().map(map_country_result).collect())
                .map_err(|source| RadrootsLocationResolverError::QueryFailed {
                    message: source.to_string(),
                });
            shared_state.borrow_mut().pending_country_list_result = Some(result);
        });
        Ok(())
    }

    fn poll_location_country_list_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryListResult>, String> {
        Ok(self.state.borrow_mut().pending_country_list_result.take())
    }

    fn request_location_country_center_lookup(
        &self,
        country_id: &str,
    ) -> Result<(), RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        {
            let mut state = self.state.borrow_mut();
            state.pending_country_center_result = None;
        }
        let shared_state = Rc::clone(&self.state);
        let country_id = country_id.to_owned();
        wasm_bindgen_futures::spawn_local(async move {
            let result = geocoder
                .country_center(country_id.as_str())
                .map(|point| RadrootsLocationPoint {
                    lat: point.lat,
                    lng: point.lng,
                })
                .map_err(map_country_center_error);
            shared_state.borrow_mut().pending_country_center_result = Some(result);
        });
        Ok(())
    }

    fn poll_location_country_center_lookup_result(
        &self,
    ) -> Result<Option<RadrootsLocationCountryCenterLookupResult>, String> {
        Ok(self.state.borrow_mut().pending_country_center_result.take())
    }

    fn list_location_countries(
        &self,
    ) -> Result<Vec<RadrootsLocationCountry>, RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        geocoder
            .country_list()
            .map(|results| results.into_iter().map(map_country_result).collect())
            .map_err(|source| RadrootsLocationResolverError::QueryFailed {
                message: source.to_string(),
            })
    }

    fn location_country_center(
        &self,
        country_id: &str,
    ) -> Result<RadrootsLocationPoint, RadrootsLocationResolverError> {
        let geocoder = self.ready_geocoder()?;
        geocoder
            .country_center(country_id)
            .map(|point| RadrootsLocationPoint {
                lat: point.lat,
                lng: point.lng,
            })
            .map_err(map_country_center_error)
    }

    fn setup_action_state(&self) -> SetupActionState {
        let state = self.state.borrow();
        match &state.connection {
            WebConnectionState::Connecting => SetupActionState {
                label: "Connecting Browser Signer...".to_owned(),
                enabled: false,
                pending: true,
            },
            WebConnectionState::Disconnected => SetupActionState {
                label: "Connect Browser Signer".to_owned(),
                enabled: true,
                pending: false,
            },
            WebConnectionState::Ready(_) => SetupActionState {
                label: "Browser Signer Connected".to_owned(),
                enabled: false,
                pending: false,
            },
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        {
            let state = self.state.borrow();
            match &state.connection {
                WebConnectionState::Connecting => return Ok(None),
                WebConnectionState::Ready(connected) => {
                    return Ok(Some(Self::identity_state_for_ready(connected)));
                }
                WebConnectionState::Disconnected => {}
            }
        }

        let signer = BrowserSigner::new().map_err(Self::connect_error_message)?;
        {
            let mut state = self.state.borrow_mut();
            state.connection = WebConnectionState::Connecting;
            state.pending_result = None;
        }

        let shared_state = Rc::clone(&self.state);
        wasm_bindgen_futures::spawn_local(async move {
            let result = match signer.get_public_key().await {
                Ok(public_key) => match public_key.to_bech32() {
                    Ok(npub) => Ok(ConnectedSigner {
                        account_id: public_key.to_hex(),
                        npub,
                        signer,
                    }),
                    Err(source) => Err(format!("Failed to encode npub: {source}")),
                },
                Err(source) => Err(format!("Browser signer connection failed: {source}")),
            };

            let mut state = shared_state.borrow_mut();
            state.pending_result = Some(result);
        });

        Ok(None)
    }

    fn home_action_states(&self) -> Vec<HomeActionState> {
        let state = self.state.borrow();
        match &state.connection {
            WebConnectionState::Ready(_) => vec![HomeActionState {
                kind: HomeActionKind::DisconnectSigner,
                label: "Disconnect Browser Signer".to_owned(),
                enabled: true,
                pending: false,
            }],
            WebConnectionState::Disconnected | WebConnectionState::Connecting => Vec::new(),
        }
    }

    fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
        match action {
            HomeActionKind::DisconnectSigner => {
                Ok(HomeActionResult::IdentityState(self.disconnect_signer()))
            }
            HomeActionKind::BackupSecretKey
            | HomeActionKind::RemoveLocalKey
            | HomeActionKind::ResetDevice => Ok(HomeActionResult::None),
        }
    }

    fn request_select_account(
        &self,
        account_id: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        let state = self.state.borrow();
        match &state.connection {
            WebConnectionState::Ready(connected) if connected.account_id == account_id => {
                Ok(Some(Self::identity_state_for_ready(connected)))
            }
            WebConnectionState::Ready(_) => Err("unknown browser signer account".to_owned()),
            WebConnectionState::Disconnected | WebConnectionState::Connecting => Ok(None),
        }
    }

    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        let mut state = self.state.borrow_mut();
        let Some(result) = state.pending_result.take() else {
            return Ok(None);
        };

        match result {
            Ok(connected) => {
                let identity = Self::identity_state_for_ready(&connected);
                state.connection = WebConnectionState::Ready(connected);
                Ok(Some(identity))
            }
            Err(err) => {
                state.connection = WebConnectionState::Disconnected;
                Err(err)
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn loading_text_element() -> Option<web_sys::Element> {
    let window = web_sys::window()?;
    let document = window.document()?;
    document.get_element_by_id("loading_text")
}

#[cfg(target_arch = "wasm32")]
fn clear_loading_text() {
    if let Some(loading_text) = loading_text_element() {
        loading_text.remove();
    }
}

#[cfg(target_arch = "wasm32")]
fn show_loading_failure() {
    if let Some(loading_text) = loading_text_element() {
        loading_text.set_inner_html("<p>failed to start radroots app</p>");
    }
}

#[cfg(target_arch = "wasm32")]
async fn launch_app() -> Result<(), String> {
    let web_options = eframe::WebOptions::default();
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_owned())?;
    let document = window
        .document()
        .ok_or_else(|| "document unavailable".to_owned())?;
    let canvas = document
        .get_element_by_id("radroots_studio_app_canvas")
        .ok_or_else(|| "radroots_studio_app_canvas missing".to_owned())?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "radroots_studio_app_canvas is not a canvas element".to_owned())?;

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(WebBackend::new()))))),
        )
        .await
        .map_err(|err| format!("failed to start radroots app: {err:?}"))
}

#[cfg(target_arch = "wasm32")]
pub fn launch() {
    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };
    let _ = eframe::WebLogger::init(log_level);

    wasm_bindgen_futures::spawn_local(async {
        match launch_app().await {
            Ok(()) => clear_loading_text(),
            Err(err) => {
                log::error!("{err}");
                show_loading_failure();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_build_asset_state_is_stable() {
        let state =
            offline_geocoder_missing_build_asset_state("web geocoder asset missing from build");

        assert_eq!(state.summary_label(), "Offline geocoder unavailable");
        assert_eq!(
            state.user_message(),
            Some("Offline geocoder is not available in this build.")
        );
        assert_eq!(
            state.technical_message(),
            Some("The offline geocoder data file is missing from this app build.")
        );
        assert_eq!(
            state.debug_message(),
            Some("web geocoder asset missing from build")
        );
    }

    #[test]
    fn wasm_revision_validation_matches_stamped_sha256_contract() {
        assert!(is_valid_asset_revision(
            "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c"
        ));
        assert!(!is_valid_asset_revision("abcd"));
        assert!(!is_valid_asset_revision(
            "not-a-valid-revision-because-it-is-not-hexadecimal-or-64-bytes-long"
        ));
    }

    #[test]
    fn initialization_failed_state_includes_revision_context() {
        let state = offline_geocoder_initialization_failed_state(
            "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c",
            "failed to open wasm geocoder bytes",
        );
        let diagnostic = state.diagnostic().expect("diagnostic");

        assert_eq!(diagnostic.platform_code, "web");
        assert_eq!(
            diagnostic.asset_revision.as_deref(),
            Some("6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c")
        );
        assert_eq!(diagnostic.code, "initialization_failed");
        assert_eq!(
            state.debug_message(),
            Some("failed to open wasm geocoder bytes")
        );
    }

    #[test]
    fn location_resolver_unavailable_code_is_stable() {
        assert_eq!(
            radroots_studio_app_core::RadrootsLocationResolverError::Unavailable.code(),
            "unavailable"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn launch() {}
