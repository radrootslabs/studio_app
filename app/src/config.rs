#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use radroots_studio_app_core::idb::{
    RadrootsClientIdbConfig,
    IDB_CONFIG_DATASTORE,
    IDB_CONFIG_KEYSTORE_NOSTR,
};

pub type AppDatastoreKeyParam = fn(&str) -> String;
pub type AppDatastoreKeyMap = BTreeMap<&'static str, &'static str>;
pub type AppDatastoreKeyParamMap = BTreeMap<&'static str, AppDatastoreKeyParam>;
pub type AppDatastoreKeyObjMap = BTreeMap<&'static str, &'static str>;

pub const APP_DATASTORE_KEY_NOSTR_KEY: &str = "nostr:key";
pub const APP_DATASTORE_KEY_EULA_DATE: &str = "app:eula:date";
pub const APP_DATASTORE_KEY_OBJ_CFG_DATA: &str = "cfg:data";
pub const APP_DATASTORE_KEY_OBJ_APP_DATA: &str = "app:data";

pub fn app_datastore_param_nostr_profile(public_key: &str) -> String {
    format!("nostr:{public_key}:profile")
}

pub fn app_datastore_param_radroots_profile(public_key: &str) -> String {
    format!("radroots:{public_key}:profile")
}

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

pub fn app_key_maps_default() -> AppKeyMapConfig {
    let mut key_map = BTreeMap::new();
    key_map.insert("nostr_key", APP_DATASTORE_KEY_NOSTR_KEY);
    key_map.insert("eula_date", APP_DATASTORE_KEY_EULA_DATE);
    let mut param_map = BTreeMap::new();
    param_map.insert("nostr_profile", app_datastore_param_nostr_profile as AppDatastoreKeyParam);
    param_map.insert(
        "radroots_profile",
        app_datastore_param_radroots_profile as AppDatastoreKeyParam,
    );
    let mut obj_map = BTreeMap::new();
    obj_map.insert("cfg_data", APP_DATASTORE_KEY_OBJ_CFG_DATA);
    obj_map.insert("app_data", APP_DATASTORE_KEY_OBJ_APP_DATA);
    AppKeyMapConfig {
        key_map,
        param_map,
        obj_map,
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
pub struct AppDatastoreConfig {
    pub idb_config: RadrootsClientIdbConfig,
    pub key_maps: AppKeyMapConfig,
}

impl AppDatastoreConfig {
    pub fn default_config(key_maps: AppKeyMapConfig) -> Self {
        Self {
            idb_config: IDB_CONFIG_DATASTORE,
            key_maps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub datastore: AppDatastoreConfig,
    pub keystore: AppKeystoreConfig,
}

impl AppConfig {
    pub fn empty() -> Self {
        let key_maps = AppKeyMapConfig::empty();
        Self {
            datastore: AppDatastoreConfig::default_config(key_maps),
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
        app_datastore_param_nostr_profile,
        AppConfig,
        AppDatastoreConfig,
        AppKeyMapConfig,
        AppKeystoreConfig,
        APP_DATASTORE_KEY_EULA_DATE,
        APP_DATASTORE_KEY_NOSTR_KEY,
        APP_DATASTORE_KEY_OBJ_APP_DATA,
        APP_DATASTORE_KEY_OBJ_CFG_DATA,
    };
    use radroots_studio_app_core::idb::{IDB_CONFIG_DATASTORE, IDB_CONFIG_KEYSTORE_NOSTR};

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
        assert!(config.datastore.key_maps.key_map.is_empty());
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

    #[test]
    fn datastore_config_defaults_to_idb_store() {
        let key_maps = AppKeyMapConfig::empty();
        let config = AppDatastoreConfig::default_config(key_maps);
        assert_eq!(config.idb_config, IDB_CONFIG_DATASTORE);
        assert!(config.key_maps.key_map.is_empty());
    }

    #[test]
    fn key_map_defaults_include_fixture_keys() {
        let config = super::app_key_maps_default();
        assert_eq!(
            config.key_map.get("nostr_key"),
            Some(&APP_DATASTORE_KEY_NOSTR_KEY)
        );
        assert_eq!(
            config.key_map.get("eula_date"),
            Some(&APP_DATASTORE_KEY_EULA_DATE)
        );
        assert_eq!(
            config.obj_map.get("cfg_data"),
            Some(&APP_DATASTORE_KEY_OBJ_CFG_DATA)
        );
        assert_eq!(
            config.obj_map.get("app_data"),
            Some(&APP_DATASTORE_KEY_OBJ_APP_DATA)
        );
        assert_eq!(app_datastore_param_nostr_profile("abc"), "nostr:abc:profile");
    }
}
