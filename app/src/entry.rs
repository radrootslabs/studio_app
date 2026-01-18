use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::App;

#[wasm_bindgen(start)]
pub fn start() {
    mount_to_body(App);
}
