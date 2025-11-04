use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use tangle_core::{
    keystore,
    nostr::keys::{
        lib_nostr_keys_gen, lib_nostr_keys_parse, lib_nostr_public_key_hex,
        lib_nostr_secret_key_hex,
    },
};
use tangle_model::{types::new_wasm_db, wasm_executor::WasmExecutor};
use wasm_bindgen::prelude::*;

pub mod ffi;
pub mod migrate;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn init(db_name: Option<String>) -> Result<JsValue, JsValue> {
    WasmExecutor::init(db_name).await;
    let db = ffi::Db::new();
    migrate::apply_migrations(db)
        .await
        .map(|_| ffi::IResultPass { pass: true })
        .and_then(|p| to_value(&p).map_err(|e| JsValue::from_str(&e.to_string())))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IError {
    pub err: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IResult<T> {
    pub result: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IResultList<T> {
    pub results: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IResultPass {
    pub pass: bool,
}

#[wasm_bindgen]
pub fn keys_nostr_gen() -> Result<JsValue, JsValue> {
    let keys = lib_nostr_keys_gen();
    keystore::key_add(&keys).map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResult {
        result: keys.public_key().to_hex(),
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn keys_nostr_add(secret_key: String) -> Result<JsValue, JsValue> {
    let keys = lib_nostr_keys_parse(secret_key).map_err(|e| JsValue::from_str(&e.to_string()))?;
    keystore::key_add(&keys).map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResult {
        result: lib_nostr_public_key_hex(keys),
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn keys_nostr_read(public_key: String) -> Result<JsValue, JsValue> {
    let keys = keystore::key_read(&public_key).map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResult {
        result: lib_nostr_secret_key_hex(keys),
    })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn keys_nostr_read_all() -> Result<JsValue, JsValue> {
    let results = keystore::keys_read_all().map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResultList { results }).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn keys_nostr_delete(public_key: String) -> Result<JsValue, JsValue> {
    keystore::key_delete(&public_key).map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResultPass { pass: true }).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn keys_nostr_keystore_reset() -> Result<JsValue, JsValue> {
    keystore::reset().map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_value(&IResultPass { pass: true }).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn model_tables_reset() -> Result<JsValue, JsValue> {
    let exec = new_wasm_db();
    tangle_model::types::reset_all(&exec)
        .map_err(|e| JsValue::from_str(&e))
        .and_then(|p| {
            to_value(&ffi::IResultPass { pass: p.pass })
                .map_err(|e| JsValue::from_str(&e.to_string()))
        })
}
