#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use radroots_studio_app::{APP_NAME, RadrootsApp};

#[cfg(target_os = "macos")]
fn set_macos_app_name() {
    use objc2_foundation::{NSProcessInfo, NSString};

    let process_info = NSProcessInfo::processInfo();
    let process_name = NSString::from_str(APP_NAME);
    process_info.setProcessName(&process_name);
}

#[cfg(not(target_os = "macos"))]
fn set_macos_app_name() {}

fn main() -> eframe::Result<()> {
    set_macos_app_name();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(RadrootsApp))),
    )
}
