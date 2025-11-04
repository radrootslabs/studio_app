use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use tangle_model::types::{new_wasm_db, BindValue, ModelExecutor, WasmDb};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Db {
    exec: WasmDb,
}

fn parse_binds(binds: &JsValue) -> Vec<BindValue> {
    if binds.is_undefined() || binds.is_null() {
        Vec::new()
    } else {
        from_value(binds.clone()).unwrap_or_default()
    }
}

#[wasm_bindgen]
impl Db {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Db {
        Db {
            exec: new_wasm_db(),
        }
    }

    pub fn execute(&self, sql: &str, binds: &JsValue) -> Result<u64, JsValue> {
        let binds = parse_binds(binds);
        self.exec
            .execute(sql, &binds)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn query_one(&self, sql: &str, binds: &JsValue) -> Result<JsValue, JsValue> {
        let binds = parse_binds(binds);
        self.exec
            .query_one(sql, &binds)
            .and_then(|v| to_value(&v).map_err(|e| e.to_string()))
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn query_all(&self, sql: &str, binds: &JsValue) -> Result<JsValue, JsValue> {
        let binds = parse_binds(binds);
        self.exec
            .query_all(sql, &binds)
            .and_then(|v| to_value(&v).map_err(|e| e.to_string()))
            .map_err(|e| JsValue::from_str(&e))
    }
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
