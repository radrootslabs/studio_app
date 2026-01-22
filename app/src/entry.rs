use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{app_logging_init, app_theme_init, RadrootsApp};

#[wasm_bindgen(start)]
pub fn start() {
    let _ = app_logging_init(None);
    let _ = app_theme_init();
    mount_to_body(RadrootsApp);
}
