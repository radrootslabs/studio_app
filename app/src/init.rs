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
        let err = futures::executor::block_on(app_init_backends())
            .expect_err("idb bootstrap should error on non-wasm");
        assert_eq!(
            err,
            AppInitError::Idb(RadrootsClientIdbStoreError::IdbUndefined)
        );
    }
}
