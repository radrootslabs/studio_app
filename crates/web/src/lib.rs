#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsCast as _;
#[cfg(target_arch = "wasm32")]
use nostr::nips::nip19::ToBech32;
#[cfg(target_arch = "wasm32")]
use nostr::signer::NostrSigner;
#[cfg(target_arch = "wasm32")]
use nostr_browser_signer::{BrowserSigner, Error as BrowserSignerError};
#[cfg(target_arch = "wasm32")]
use radroots_studio_app_core::{IdentityGateState, RadrootsApp, RadrootsAppBackend, SetupActionState};

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
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct WebBackend {
    state: Rc<RefCell<WebBackendState>>,
}

#[cfg(target_arch = "wasm32")]
impl WebBackend {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(WebBackendState {
                connection: WebConnectionState::Disconnected,
                pending_result: None,
            })),
        }
    }

    fn identity_state_for_ready(connected: &ConnectedSigner) -> IdentityGateState {
        let _ = &connected.signer;
        IdentityGateState::Ready {
            account_id: connected.account_id.clone(),
            npub: connected.npub.clone(),
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

#[cfg(not(target_arch = "wasm32"))]
pub fn launch() {}
