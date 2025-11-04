use serde::{Deserialize, Serialize};
use tangle_core::types::IResultPass;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

pub trait ModelExecutor {
    fn query_one(&self, sql: &str, binds: &[BindValue]) -> Result<serde_json::Value, String>;
    fn query_all(&self, sql: &str, binds: &[BindValue]) -> Result<Vec<serde_json::Value>, String>;
    fn execute(&self, sql: &str, binds: &[BindValue]) -> Result<u64, String>;
}

pub struct WasmDb(pub(crate) crate::wasm_executor::WasmExecutor);

pub fn new_wasm_db() -> WasmDb {
    WasmDb(crate::wasm_executor::WasmExecutor::new())
}

impl ModelExecutor for WasmDb {
    fn query_one(&self, sql: &str, binds: &[BindValue]) -> Result<serde_json::Value, String> {
        self.0.query_one(sql, binds)
    }
    fn query_all(&self, sql: &str, binds: &[BindValue]) -> Result<Vec<serde_json::Value>, String> {
        self.0.query_all(sql, binds)
    }
    fn execute(&self, sql: &str, binds: &[BindValue]) -> Result<u64, String> {
        self.0.execute(sql, binds)
    }
}

pub fn reset_all<E: ModelExecutor>(exec: &E) -> Result<IResultPass, String> {
    let sql = "DELETE FROM location_gcs;
DELETE FROM trade_product;
DELETE FROM nostr_profile;
DELETE FROM nostr_relay;
DELETE FROM media_image;
DELETE FROM log_error;
DELETE FROM farm;
DELETE FROM nostr_profile_relay;
DELETE FROM farm_location;
DELETE FROM trade_product_location;
DELETE FROM trade_product_media;";
    exec.execute(sql, &[]).map(|_| IResultPass { pass: true })
}
