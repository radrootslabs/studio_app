#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::JsCast as _;
#[cfg(target_arch = "wasm32")]
use radroots_studio_app_core::{IdentityGateState, RadrootsApp, RadrootsAppBackend};

#[cfg(target_arch = "wasm32")]
struct WebBackend;

#[cfg(target_arch = "wasm32")]
impl RadrootsAppBackend for WebBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        Ok(IdentityGateState::Unsupported {
            reason: "Local secure onboarding is not enabled for the web target.".to_owned(),
        })
    }

    fn generate_new_key(&self) -> Result<IdentityGateState, String> {
        Ok(IdentityGateState::Unsupported {
            reason: "Local secure onboarding is not enabled for the web target.".to_owned(),
        })
    }
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
        let web_options = eframe::WebOptions::default();
        let window = web_sys::window().expect("window");
        let document = window.document().expect("document");
        let canvas = document
            .get_element_by_id("radroots_studio_app_canvas")
            .expect("radroots_studio_app_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("canvas");

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|_cc| Ok(Box::new(RadrootsApp::new(Box::new(WebBackend))))),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            if result.is_ok() {
                loading_text.remove();
            } else {
                loading_text.set_inner_html("<p>failed to start radroots app</p>");
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn launch() {}
