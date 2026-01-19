#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_KEYSTORE_NOSTR};

pub type AppDatastoreKeyParam = fn(&str) -> String;
pub type AppDatastoreKeyMap = BTreeMap<&'static str, &'static str>;
pub type AppDatastoreKeyParamMap = BTreeMap<&'static str, AppDatastoreKeyParam>;
pub type AppDatastoreKeyObjMap = BTreeMap<&'static str, &'static str>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppKeyMapConfig {
    pub key_map: AppDatastoreKeyMap,
    pub param_map: AppDatastoreKeyParamMap,
    pub obj_map: AppDatastoreKeyObjMap,
}

impl AppKeyMapConfig {
    pub fn empty() -> Self {
        Self {
            key_map: BTreeMap::new(),
            param_map: BTreeMap::new(),
            obj_map: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppKeystoreConfig {
    pub nostr_store: RadrootsClientIdbConfig,
}

impl AppKeystoreConfig {
    pub const fn default_config() -> Self {
        Self {
            nostr_store: IDB_CONFIG_KEYSTORE_NOSTR,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub key_maps: AppKeyMapConfig,
    pub keystore: AppKeystoreConfig,
}

impl AppConfig {
    pub fn empty() -> Self {
        Self {
            key_maps: AppKeyMapConfig::empty(),
            keystore: AppKeystoreConfig::default_config(),
        }
    }
}

pub fn app_config_default() -> AppConfig {
    AppConfig::empty()
}

pub fn app_config_from_env() -> AppConfig {
    app_config_default()
}

#[cfg(test)]
mod tests {
    use super::{
        app_config_default,
        app_config_from_env,
        AppConfig,
        AppKeyMapConfig,
        AppKeystoreConfig,
    };
    use radroots_studio_app_core::idb::IDB_CONFIG_KEYSTORE_NOSTR;

    #[test]
    fn key_map_config_defaults_empty() {
        let config = AppKeyMapConfig::empty();
        assert!(config.key_map.is_empty());
        assert!(config.param_map.is_empty());
        assert!(config.obj_map.is_empty());
    }

    #[test]
    fn app_config_defaults_empty() {
        let config = AppConfig::empty();
        assert!(config.key_maps.key_map.is_empty());
    }

    #[test]
    fn app_config_helpers_return_defaults() {
        let config = app_config_default();
        let from_env = app_config_from_env();
        assert_eq!(config, from_env);
    }

    #[test]
    fn keystore_config_defaults_to_nostr_store() {
        let config = AppKeystoreConfig::default_config();
        assert_eq!(config.nostr_store, IDB_CONFIG_KEYSTORE_NOSTR);
    }
}
