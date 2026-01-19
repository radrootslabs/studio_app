use super::RadrootsClientIdbConfig;

pub const RADROOTS_IDB_DATABASE: &str = "radroots-pwa-v1";

pub const IDB_STORE_DATASTORE: &str = "radroots.app.datastore";
pub const IDB_STORE_KEYSTORE: &str = "radroots.security.keystore";
pub const IDB_STORE_KEYSTORE_NOSTR: &str = "radroots.security.keystore.nostr";
pub const IDB_STORE_CRYPTO_REGISTRY: &str = "radroots.security.crypto.registry";
pub const IDB_STORE_CIPHER_AES_GCM: &str = "radroots.security.cipher.aes-gcm";
pub const IDB_STORE_CIPHER_SQL: &str = "radroots.security.cipher.sql";
pub const IDB_STORE_TANGLE: &str = "radroots.storage.tangle.sql";
pub const IDB_STORE_CIPHER_SUFFIX: &str = ".cipher";

pub const IDB_STORE_KEYSTORE_CIPHER: &str = "radroots.security.keystore.cipher";
pub const IDB_STORE_KEYSTORE_NOSTR_CIPHER: &str = "radroots.security.keystore.nostr.cipher";

pub const IDB_CONFIG_DATASTORE: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_DATASTORE);
pub const IDB_CONFIG_KEYSTORE: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_KEYSTORE);
pub const IDB_CONFIG_KEYSTORE_NOSTR: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_KEYSTORE_NOSTR);
pub const IDB_CONFIG_CRYPTO_REGISTRY: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_CRYPTO_REGISTRY);
pub const IDB_CONFIG_CIPHER_AES_GCM: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_CIPHER_AES_GCM);
pub const IDB_CONFIG_CIPHER_SQL: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_CIPHER_SQL);
pub const IDB_CONFIG_TANGLE: RadrootsClientIdbConfig =
    RadrootsClientIdbConfig::new(RADROOTS_IDB_DATABASE, IDB_STORE_TANGLE);

pub const RADROOTS_IDB_CONFIGS: &[RadrootsClientIdbConfig] = &[
    IDB_CONFIG_DATASTORE,
    IDB_CONFIG_KEYSTORE,
    IDB_CONFIG_KEYSTORE_NOSTR,
    IDB_CONFIG_CRYPTO_REGISTRY,
    IDB_CONFIG_CIPHER_AES_GCM,
    IDB_CONFIG_CIPHER_SQL,
    IDB_CONFIG_TANGLE,
];

pub const RADROOTS_IDB_STORES: &[&str] = &[
    IDB_STORE_DATASTORE,
    IDB_STORE_KEYSTORE,
    IDB_STORE_KEYSTORE_NOSTR,
    IDB_STORE_CRYPTO_REGISTRY,
    IDB_STORE_CIPHER_AES_GCM,
    IDB_STORE_CIPHER_SQL,
    IDB_STORE_TANGLE,
    IDB_STORE_KEYSTORE_CIPHER,
    IDB_STORE_KEYSTORE_NOSTR_CIPHER,
];

#[cfg(test)]
mod tests {
    use super::{
        IDB_STORE_KEYSTORE_CIPHER,
        IDB_STORE_KEYSTORE_NOSTR_CIPHER,
        RADROOTS_IDB_CONFIGS,
        RADROOTS_IDB_DATABASE,
        RADROOTS_IDB_STORES,
    };

    #[test]
    fn configs_share_database_name() {
        for config in RADROOTS_IDB_CONFIGS {
            assert_eq!(config.database, RADROOTS_IDB_DATABASE);
        }
    }

    #[test]
    fn stores_include_cipher_variants() {
        assert!(RADROOTS_IDB_STORES.contains(&IDB_STORE_KEYSTORE_CIPHER));
        assert!(RADROOTS_IDB_STORES.contains(&IDB_STORE_KEYSTORE_NOSTR_CIPHER));
    }
}
