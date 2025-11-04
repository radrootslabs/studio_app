use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

use crate::types::BindValue;

#[wasm_bindgen(module = "/web/sdk/sqlite/opfs.js")]
extern "C" {
    #[wasm_bindgen(js_name = dbInit)]
    fn db_init(name: Option<String>) -> js_sys::Promise;
    #[wasm_bindgen(js_name = dbQueryOne)]
    fn db_query_one(sql: &str, binds: JsValue) -> JsValue;
    #[wasm_bindgen(js_name = dbQueryAll)]
    fn db_query_all(sql: &str, binds: JsValue) -> JsValue;
    #[wasm_bindgen(js_name = dbExecute)]
    fn db_execute(sql: &str, binds: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct JsBind {
    t: &'static str,
    s: Option<String>,
    n: Option<f64>,
    b: Option<bool>,
}

fn to_js_binds(binds: &[BindValue]) -> JsValue {
    let v: Vec<JsBind> = binds
        .iter()
        .map(|b| match b {
            BindValue::String(s) => JsBind {
                t: "s",
                s: Some(s.clone()),
                n: None,
                b: None,
            },
            BindValue::Number(n) => JsBind {
                t: "n",
                s: None,
                n: Some(*n),
                b: None,
            },
            BindValue::Boolean(x) => JsBind {
                t: "b",
                s: None,
                n: None,
                b: Some(*x),
            },
            BindValue::Null => JsBind {
                t: "null",
                s: None,
                n: None,
                b: None,
            },
        })
        .collect();
    to_value(&v).unwrap_or(JsValue::NULL)
}

pub struct WasmExecutor {
    _priv: (),
}

impl WasmExecutor {
    pub fn new() -> Self {
        Self { _priv: () }
    }

    pub async fn init(name: Option<String>) {
        let _ = wasm_bindgen_futures::JsFuture::from(db_init(name)).await;
    }

    pub fn query_one(&self, sql: &str, binds: &[BindValue]) -> Result<serde_json::Value, String> {
        let js = db_query_one(sql, to_js_binds(binds));
        from_value::<serde_json::Value>(js).map_err(|e| e.to_string()) // ✨
    }

    pub fn query_all(
        &self,
        sql: &str,
        binds: &[BindValue],
    ) -> Result<Vec<serde_json::Value>, String> {
        let js = db_query_all(sql, to_js_binds(binds));
        from_value::<Vec<serde_json::Value>>(js).map_err(|e| e.to_string()) // ✨
    }

    pub fn execute(&self, sql: &str, binds: &[BindValue]) -> Result<u64, String> {
        let js = db_execute(sql, to_js_binds(binds));
        js.as_f64()
            .map(|n| n as u64)
            .ok_or_else(|| "execute failed".to_string())
    }
}
