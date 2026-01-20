use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{app_logging_init, RadrootsApp};

#[wasm_bindgen(start)]
pub fn start() {
    let _ = app_logging_init(None);
    mount_to_body(RadrootsApp);
}
