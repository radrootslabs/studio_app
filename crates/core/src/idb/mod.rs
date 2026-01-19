pub mod config;
pub mod encrypted_store;
pub mod error;
pub mod keyval;
pub mod store;
pub mod types;
pub mod value;

pub use config::{
    IDB_CONFIG_CIPHER_AES_GCM,
    IDB_CONFIG_CIPHER_SQL,
    IDB_CONFIG_CRYPTO_REGISTRY,
    IDB_CONFIG_DATASTORE,
    IDB_CONFIG_KEYSTORE,
    IDB_CONFIG_KEYSTORE_NOSTR,
    IDB_CONFIG_TANGLE,
    IDB_STORE_CIPHER_AES_GCM,
    IDB_STORE_CIPHER_SQL,
    IDB_STORE_CIPHER_SUFFIX,
    IDB_STORE_CRYPTO_REGISTRY,
    IDB_STORE_DATASTORE,
    IDB_STORE_KEYSTORE,
    IDB_STORE_KEYSTORE_NOSTR,
    IDB_STORE_TANGLE,
    RADROOTS_IDB_CONFIGS,
    RADROOTS_IDB_DATABASE,
    RADROOTS_IDB_STORES,
};
pub use types::RadrootsClientIdbConfig;
pub use value::{idb_value_as_bytes, RadrootsClientIdbValue};
pub use error::{RadrootsClientIdbStoreError, RadrootsClientIdbStoreErrorMessage};
pub use keyval::{idb_clear, idb_del, idb_get, idb_keys, idb_set};
pub use encrypted_store::{
    RadrootsClientWebEncryptedStore,
    RadrootsClientWebEncryptedStoreConfig,
};
pub use store::{idb_store_bootstrap, idb_store_ensure, idb_store_exists};
