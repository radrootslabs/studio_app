#![forbid(unsafe_code)]

use std::fmt;

use radroots_studio_app_core::datastore::{
    RadrootsClientDatastore,
    RadrootsClientDatastoreError,
    RadrootsClientWebDatastore,
};
use radroots_studio_app_core::idb::{
    idb_store_bootstrap,
    RadrootsClientIdbStoreError,
    RADROOTS_IDB_DATABASE,
};
use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientWebKeystoreNostr};

#[cfg(target_arch = "wasm32")]
use leptos::prelude::window;

pub const APP_INIT_STORAGE_KEY: &str = "radroots.app.init.ready";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInitError {
    Idb(RadrootsClientIdbStoreError),
    Datastore(RadrootsClientDatastoreError),
    Keystore(RadrootsClientKeystoreError),
}

pub type AppInitErrorMessage = &'static str;

impl AppInitError {
    pub const fn message(&self) -> AppInitErrorMessage {
        match self {
            AppInitError::Idb(_) => "error.app.init.idb",
            AppInitError::Datastore(_) => "error.app.init.datastore",
            AppInitError::Keystore(_) => "error.app.init.keystore",
        }
    }
}

impl fmt::Display for AppInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppInitError {}

pub struct AppBackends {
    pub datastore: RadrootsClientWebDatastore,
    pub nostr_keystore: RadrootsClientWebKeystoreNostr,
}

pub type AppInitResult<T> = Result<T, AppInitError>;

pub fn app_init_has_completed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        match window.local_storage() {
            Ok(Some(storage)) => match storage.get_item(APP_INIT_STORAGE_KEY) {
                Ok(Some(value)) => value == "1",
                _ => false,
            },
            _ => false,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn app_init_mark_completed() {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(APP_INIT_STORAGE_KEY, "1");
        }
    }
}

pub fn app_init_reset() {
    #[cfg(target_arch = "wasm32")]
    {
        let window = window();
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item(APP_INIT_STORAGE_KEY);
        }
    }
}

pub async fn app_init_backends() -> AppInitResult<AppBackends> {
    idb_store_bootstrap(RADROOTS_IDB_DATABASE, None)
        .await
        .map_err(AppInitError::Idb)?;
    let datastore = RadrootsClientWebDatastore::new(None);
    datastore
        .init()
        .await
        .map_err(AppInitError::Datastore)?;
    let nostr_keystore = RadrootsClientWebKeystoreNostr::new(None);
    Ok(AppBackends {
        datastore,
        nostr_keystore,
    })
}

#[cfg(test)]
mod tests {
    use super::{app_init_backends, AppInitError, AppInitErrorMessage};
    use radroots_studio_app_core::datastore::RadrootsClientDatastoreError;
    use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
    use radroots_studio_app_core::keystore::RadrootsClientKeystoreError;

    #[test]
    fn app_init_error_messages_match_spec() {
        let cases: &[(AppInitError, AppInitErrorMessage)] = &[
            (
                AppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined),
                "error.app.init.idb",
            ),
            (
                AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined),
                "error.app.init.datastore",
            ),
            (
                AppInitError::Keystore(RadrootsClientKeystoreError::IdbUndefined),
                "error.app.init.keystore",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), *expected);
            assert_eq!(err.to_string(), *expected);
        }
    }

    #[test]
    fn app_init_backends_maps_idb_errors() {
        let err = match futures::executor::block_on(app_init_backends()) {
            Ok(_) => panic!("idb bootstrap should error on non-wasm"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            AppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined)
        );
    }

    #[test]
    fn app_init_has_completed_is_false_on_native() {
        assert!(!super::app_init_has_completed());
    }

    #[test]
    fn app_init_reset_is_noop_on_native() {
        super::app_init_mark_completed();
        super::app_init_reset();
    }
}
