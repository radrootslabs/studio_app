#![allow(unsafe_code)]

#[cfg(target_os = "ios")]
use eframe::egui::ViewportBuilder;
#[cfg(target_os = "ios")]
use radroots_studio_app_core::{
    APP_NAME, IdentityGateState, RadrootsApp, RadrootsAppBackend, SetupActionState,
};

#[cfg(target_os = "ios")]
struct IosBackend;

#[cfg(target_os = "ios")]
impl RadrootsAppBackend for IosBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        Ok(IdentityGateState::Unsupported {
            reason: "Secure onboarding is not yet available on iOS.".to_owned(),
        })
    }

    fn setup_action_state(&self) -> SetupActionState {
        SetupActionState {
            label: "Not Yet Available".to_owned(),
            enabled: false,
            pending: false,
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        Ok(Some(IdentityGateState::Unsupported {
            reason: "Secure onboarding is not yet available on iOS.".to_owned(),
        }))
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
}
