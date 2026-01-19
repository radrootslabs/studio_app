use super::{RadrootsClientIdbStoreError, RadrootsClientIdbValue};

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Promise};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{IdbRequest, IdbTransactionMode};

#[cfg(target_arch = "wasm32")]
use super::store::idb_open;

#[cfg(target_arch = "wasm32")]
async fn idb_request(request: IdbRequest) -> Result<JsValue, RadrootsClientIdbStoreError> {
    let promise = Promise::new(&mut |resolve, reject| {
        let request_success = request.clone();
        let resolve = resolve.clone();
        let reject_success = reject.clone();
        let on_success = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            match request_success.result() {
                Ok(value) => {
                    let _ = resolve.call1(&JsValue::UNDEFINED, &value);
                }
                Err(err) => {
                    let _ = reject_success.call1(&JsValue::UNDEFINED, &err);
                }
            }
        }) as Box<dyn FnMut(_)>);
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        on_success.forget();

        let request_error = request.clone();
        let reject_error = reject.clone();
        let on_error = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            let err = request_error
                .error()
                .map(JsValue::from)
                .unwrap_or_else(|| JsValue::from_str("idb_request_failed"));
            let _ = reject_error.call1(&JsValue::UNDEFINED, &err);
        }) as Box<dyn FnMut(_)>);
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    });
    JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)
}

#[cfg(target_arch = "wasm32")]
async fn idb_store_request(
    database: &str,
    store: &str,
    mode: IdbTransactionMode,
    build_request: impl FnOnce(web_sys::IdbObjectStore) -> Result<IdbRequest, JsValue>,
) -> Result<JsValue, RadrootsClientIdbStoreError> {
    let db = idb_open(database, None, &[]).await?;
    let transaction = db
        .transaction_with_str_and_mode(store, mode)
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?;
    let object_store = transaction
        .object_store(store)
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?;
    let request = build_request(object_store)
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?;
    let value = idb_request(request).await?;
    db.close();
    Ok(value)
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_get(
    database: &str,
    store: &str,
    key: &str,
) -> Result<Option<RadrootsClientIdbValue>, RadrootsClientIdbStoreError> {
    let value = idb_store_request(database, store, IdbTransactionMode::Readonly, |object_store| {
        object_store.get(&JsValue::from_str(key))
    })
    .await?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    Ok(Some(value))
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_set(
    database: &str,
    store: &str,
    key: &str,
    value: &RadrootsClientIdbValue,
) -> Result<(), RadrootsClientIdbStoreError> {
    let _ = idb_store_request(database, store, IdbTransactionMode::Readwrite, |object_store| {
        object_store.put_with_key(value, &JsValue::from_str(key))
    })
    .await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_del(
    database: &str,
    store: &str,
    key: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    let _ = idb_store_request(database, store, IdbTransactionMode::Readwrite, |object_store| {
        object_store.delete(&JsValue::from_str(key))
    })
    .await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_clear(
    database: &str,
    store: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    let _ = idb_store_request(database, store, IdbTransactionMode::Readwrite, |object_store| {
        object_store.clear()
    })
    .await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_keys(
    database: &str,
    store: &str,
) -> Result<Vec<String>, RadrootsClientIdbStoreError> {
    let value = idb_store_request(database, store, IdbTransactionMode::Readonly, |object_store| {
        object_store.get_all_keys()
    })
    .await?;
    let array = Array::from(&value);
    let mut out = Vec::new();
    for entry in array.iter() {
        if let Some(key) = entry.as_string() {
            out.push(key);
        }
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_get(
    _database: &str,
    _store: &str,
    _key: &str,
) -> Result<Option<RadrootsClientIdbValue>, RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_set(
    _database: &str,
    _store: &str,
    _key: &str,
    _value: &RadrootsClientIdbValue,
) -> Result<(), RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_del(
    _database: &str,
    _store: &str,
    _key: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_clear(
    _database: &str,
    _store: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_keys(
    _database: &str,
    _store: &str,
) -> Result<Vec<String>, RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(test)]
mod tests {
    use super::{idb_clear, idb_del, idb_get, idb_keys, idb_set};
    use crate::idb::RadrootsClientIdbStoreError;

    #[test]
    fn non_wasm_keyval_returns_idb_undefined() {
        let err = futures::executor::block_on(idb_get("db", "store", "key"))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
    }

    #[test]
    fn non_wasm_keyval_mutations_return_idb_undefined() {
        let value = ();
        let err = futures::executor::block_on(idb_set("db", "store", "key", &value))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
        let err = futures::executor::block_on(idb_del("db", "store", "key"))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
        let err = futures::executor::block_on(idb_clear("db", "store"))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
        let err = futures::executor::block_on(idb_keys("db", "store"))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
    }
}
