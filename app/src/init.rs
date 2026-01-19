#![forbid(unsafe_code)]

use std::fmt;

use radroots_studio_app_core::datastore::{RadrootsClientDatastoreError, RadrootsClientWebDatastore};
use radroots_studio_app_core::idb::RadrootsClientIdbStoreError;
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

#[cfg(test)]
mod tests {
    use super::{AppInitError, AppInitErrorMessage};
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
}
