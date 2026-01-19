#[cfg(target_arch = "wasm32")]
use crate::idb::{RADROOTS_IDB_DATABASE, RADROOTS_IDB_STORES};

use super::RadrootsClientIdbStoreError;

#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Promise, Reflect};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{IdbDatabase, IdbFactory};

#[cfg(target_arch = "wasm32")]
fn idb_factory() -> Result<IdbFactory, RadrootsClientIdbStoreError> {
    let window = web_sys::window().ok_or(RadrootsClientIdbStoreError::IdbUndefined)?;
    let factory = window
        .indexed_db()
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?;
    factory.ok_or(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(target_arch = "wasm32")]
async fn idb_database_exists(
    factory: &IdbFactory,
    database: &str,
) -> Result<bool, RadrootsClientIdbStoreError> {
    let promise = match factory.databases() {
        Ok(promise) => promise,
        Err(_) => return Ok(true),
    };
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?;
    let list = Array::from(&value);
    for entry in list.iter() {
        let name = Reflect::get(&entry, &JsValue::from_str("name"))
            .ok()
            .and_then(|value| value.as_string());
        if name.as_deref() == Some(database) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(target_arch = "wasm32")]
fn idb_missing_stores(db: &IdbDatabase, stores: &[String]) -> Vec<String> {
    let names = db.object_store_names();
    stores
        .iter()
        .filter(|store| !names.contains(store))
        .cloned()
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn map_open_error(err: JsValue) -> RadrootsClientIdbStoreError {
    let Some(exception) = err.dyn_ref::<web_sys::DomException>() else {
        return RadrootsClientIdbStoreError::OperationFailure;
    };
    if exception.name() == "VersionError" {
        RadrootsClientIdbStoreError::VersionError
    } else {
        RadrootsClientIdbStoreError::OperationFailure
    }
}

#[cfg(target_arch = "wasm32")]
async fn idb_open(
    database: &str,
    version: Option<u32>,
    stores: &[String],
) -> Result<IdbDatabase, RadrootsClientIdbStoreError> {
    let factory = idb_factory()?;
    let request = match version {
        Some(version) => factory
            .open_with_u32(database, version)
            .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?,
        None => factory
            .open(database)
            .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)?,
    };
    let stores = stores.to_vec();
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
                .unwrap_or_else(|| JsValue::from_str("idb_open_failed"));
            let _ = reject_error.call1(&JsValue::UNDEFINED, &err);
        }) as Box<dyn FnMut(_)>);
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();

        let request_upgrade = request.clone();
        let stores_upgrade = stores.clone();
        let reject_upgrade = reject.clone();
        let on_upgrade = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            if stores_upgrade.is_empty() {
                return;
            }
            let Ok(value) = request_upgrade.result() else {
                let _ = reject_upgrade.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("idb_open_failed"),
                );
                return;
            };
            let Ok(db) = value.dyn_into::<IdbDatabase>() else {
                let _ = reject_upgrade.call1(
                    &JsValue::UNDEFINED,
                    &JsValue::from_str("idb_open_failed"),
                );
                return;
            };
            let names = db.object_store_names();
            for store in &stores_upgrade {
                if names.contains(store) {
                    continue;
                }
                if db.create_object_store(store).is_err() {
                    let _ = reject_upgrade.call1(
                        &JsValue::UNDEFINED,
                        &JsValue::from_str("idb_store_create_failed"),
                    );
                    return;
                }
            }
        }) as Box<dyn FnMut(_)>);
        request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
        on_upgrade.forget();
    });
    let value = JsFuture::from(promise).await.map_err(map_open_error)?;
    value
        .dyn_into::<IdbDatabase>()
        .map_err(|_| RadrootsClientIdbStoreError::OperationFailure)
}

#[cfg(target_arch = "wasm32")]
async fn idb_store_ensure_all(
    database: &str,
    stores: &[String],
) -> Result<(), RadrootsClientIdbStoreError> {
    if stores.is_empty() {
        return Ok(());
    }
    let mut target_stores = stores.to_vec();
    target_stores.sort();
    target_stores.dedup();
    let mut attempt = 0;
    while attempt < 5 {
        attempt += 1;
        let db = idb_open(database, None, &[]).await?;
        let missing = idb_missing_stores(&db, &target_stores);
        let version = db.version() as u32;
        db.close();
        if missing.is_empty() {
            return Ok(());
        }
        let next_version = version.saturating_add(1);
        match idb_open(database, Some(next_version), &missing).await {
            Ok(upgraded) => {
                let still_missing = idb_missing_stores(&upgraded, &target_stores);
                upgraded.close();
                if still_missing.is_empty() {
                    return Ok(());
                }
            }
            Err(RadrootsClientIdbStoreError::VersionError) => continue,
            Err(err) => return Err(err),
        }
    }
    Err(RadrootsClientIdbStoreError::OperationFailure)
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_store_ensure(
    database: &str,
    store: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    if database == RADROOTS_IDB_DATABASE {
        idb_store_bootstrap(database, None).await?;
        if RADROOTS_IDB_STORES.contains(&store) {
            return Ok(());
        }
    }
    idb_store_ensure_all(database, &[store.to_string()]).await
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_store_bootstrap(
    database: &str,
    stores: Option<&[&str]>,
) -> Result<(), RadrootsClientIdbStoreError> {
    let target_stores: Vec<String> = match stores {
        Some(stores) => stores.iter().map(|store| (*store).to_string()).collect(),
        None if database == RADROOTS_IDB_DATABASE => RADROOTS_IDB_STORES
            .iter()
            .map(|store| (*store).to_string())
            .collect(),
        None => Vec::new(),
    };
    if target_stores.is_empty() {
        return Ok(());
    }
    idb_store_ensure_all(database, &target_stores).await
}

#[cfg(target_arch = "wasm32")]
pub async fn idb_store_exists(
    database: &str,
    store: &str,
) -> Result<bool, RadrootsClientIdbStoreError> {
    let factory = idb_factory()?;
    let known = idb_database_exists(&factory, database).await?;
    if !known {
        return Ok(false);
    }
    let db = idb_open(database, None, &[]).await?;
    let exists = db.object_store_names().contains(store);
    db.close();
    Ok(exists)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_store_ensure(
    _database: &str,
    _store: &str,
) -> Result<(), RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_store_bootstrap(
    _database: &str,
    _stores: Option<&[&str]>,
) -> Result<(), RadrootsClientIdbStoreError> {
    Err(RadrootsClientIdbStoreError::IdbUndefined)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn idb_store_exists(
    _database: &str,
    _store: &str,
) -> Result<bool, RadrootsClientIdbStoreError> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{idb_store_bootstrap, idb_store_ensure, idb_store_exists};
    use crate::idb::RadrootsClientIdbStoreError;

    #[test]
    fn non_wasm_returns_idb_undefined() {
        let err = futures::executor::block_on(idb_store_ensure("db", "store"))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
    }

    #[test]
    fn non_wasm_bootstrap_returns_idb_undefined() {
        let err = futures::executor::block_on(idb_store_bootstrap("db", None))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsClientIdbStoreError::IdbUndefined);
    }

    #[test]
    fn non_wasm_exists_returns_false() {
        let exists = futures::executor::block_on(idb_store_exists("db", "store"))
            .expect("exists");
        assert!(!exists);
    }
}
