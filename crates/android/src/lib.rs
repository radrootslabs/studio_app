#![allow(unsafe_code)]

#[cfg(target_os = "android")]
use android_logger::Config;
#[cfg(target_os = "android")]
use eframe::egui::ViewportBuilder;
#[cfg(target_os = "android")]
use radroots_studio_app_core::{APP_NAME, RadrootsApp};
#[cfg(any(target_os = "android", test))]
use radroots_studio_app_core::{IdentityGateState, RadrootsAppBackend, SetupActionState};
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(any(target_os = "android", test))]
struct AndroidBackend;

#[cfg(any(target_os = "android", test))]
impl RadrootsAppBackend for AndroidBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String> {
        Ok(IdentityGateState::Unsupported {
            reason: "Secure onboarding is not yet available on Android.".to_owned(),
        })
    }

    fn setup_action_state(&self) -> SetupActionState {
        SetupActionState {
            label: "Generate New Key".to_owned(),
            enabled: false,
            pending: false,
        }
    }

    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
        Ok(Some(IdentityGateState::Unsupported {
            reason: "Secure onboarding is not yet available on Android.".to_owned(),
        }))
    }
}

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
    fn android_backend_reports_unsupported_onboarding() {
        assert_eq!(
            AndroidBackend.load_identity_state(),
            Ok(IdentityGateState::Unsupported {
                reason: "Secure onboarding is not yet available on Android.".to_owned(),
            })
        );
        assert_eq!(
            AndroidBackend.setup_action_state(),
            SetupActionState {
                label: "Generate New Key".to_owned(),
                enabled: false,
                pending: false,
            }
        );
    }
}
