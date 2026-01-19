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
pub type AppKeystoreKeyMap = BTreeMap<&'static str, &'static str>;

pub const APP_DATASTORE_KEY_NOSTR_KEY: &str = "nostr:key";
pub const APP_DATASTORE_KEY_EULA_DATE: &str = "app:eula:date";
pub const APP_DATASTORE_KEY_OBJ_CFG_DATA: &str = "cfg:data";
pub const APP_DATASTORE_KEY_OBJ_APP_DATA: &str = "app:data";
pub const APP_KEYSTORE_KEY_NOSTR_DEFAULT: &str = "nostr:default";

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
pub enum AppConfigError {
    MissingKeyMap(&'static str),
    MissingParamMap(&'static str),
    MissingObjMap(&'static str),
    MissingKeystoreKeyMap(&'static str),
}

pub type AppConfigResult<T> = Result<T, AppConfigError>;

impl AppConfigError {
    pub const fn message(&self) -> &'static str {
        match self {
            AppConfigError::MissingKeyMap(_) => "error.app.config.key_map_missing",
            AppConfigError::MissingParamMap(_) => "error.app.config.param_map_missing",
            AppConfigError::MissingObjMap(_) => "error.app.config.obj_map_missing",
            AppConfigError::MissingKeystoreKeyMap(_) => "error.app.config.keystore_map_missing",
        }
    }
}

impl std::fmt::Display for AppConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for AppConfigError {}

pub fn app_key_maps_validate(config: &AppKeyMapConfig) -> AppConfigResult<()> {
    if !config.key_map.contains_key("nostr_key") {
        return Err(AppConfigError::MissingKeyMap("nostr_key"));
    }
    if !config.key_map.contains_key("eula_date") {
        return Err(AppConfigError::MissingKeyMap("eula_date"));
    }
    if !config.param_map.contains_key("nostr_profile") {
        return Err(AppConfigError::MissingParamMap("nostr_profile"));
    }
    if !config.param_map.contains_key("radroots_profile") {
        return Err(AppConfigError::MissingParamMap("radroots_profile"));
    }
    if !config.obj_map.contains_key("cfg_data") {
        return Err(AppConfigError::MissingObjMap("cfg_data"));
    }
    if !config.obj_map.contains_key("app_data") {
        return Err(AppConfigError::MissingObjMap("app_data"));
    }
    Ok(())
}

pub fn app_keystore_key_maps_validate(config: &AppKeystoreKeyMap) -> AppConfigResult<()> {
    if !config.contains_key("nostr_default") {
        return Err(AppConfigError::MissingKeystoreKeyMap("nostr_default"));
    }
    Ok(())
}

pub fn app_datastore_key(config: &AppKeyMapConfig, key: &'static str) -> AppConfigResult<&'static str> {
    config
        .key_map
        .get(key)
        .copied()
        .ok_or(AppConfigError::MissingKeyMap(key))
}

pub fn app_datastore_obj_key(
    config: &AppKeyMapConfig,
    key: &'static str,
) -> AppConfigResult<&'static str> {
    config
        .obj_map
        .get(key)
        .copied()
        .ok_or(AppConfigError::MissingObjMap(key))
}

pub fn app_datastore_param_key(
    config: &AppKeyMapConfig,
    key: &'static str,
) -> AppConfigResult<AppDatastoreKeyParam> {
    config
        .param_map
        .get(key)
        .copied()
        .ok_or(AppConfigError::MissingParamMap(key))
}

pub fn app_datastore_key_nostr_key(config: &AppKeyMapConfig) -> AppConfigResult<&'static str> {
    app_datastore_key(config, "nostr_key")
}

pub fn app_datastore_key_eula_date(config: &AppKeyMapConfig) -> AppConfigResult<&'static str> {
    app_datastore_key(config, "eula_date")
}

pub fn app_datastore_obj_key_cfg_data(config: &AppKeyMapConfig) -> AppConfigResult<&'static str> {
    app_datastore_obj_key(config, "cfg_data")
}

pub fn app_datastore_obj_key_app_data(config: &AppKeyMapConfig) -> AppConfigResult<&'static str> {
    app_datastore_obj_key(config, "app_data")
}

pub fn app_keystore_key(
    config: &AppKeystoreKeyMap,
    key: &'static str,
) -> AppConfigResult<&'static str> {
    config
        .get(key)
        .copied()
        .ok_or(AppConfigError::MissingKeystoreKeyMap(key))
}

pub fn app_keystore_key_nostr_default(config: &AppKeystoreKeyMap) -> AppConfigResult<&'static str> {
    app_keystore_key(config, "nostr_default")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppKeystoreConfig {
    pub nostr_store: RadrootsClientIdbConfig,
    pub key_map: AppKeystoreKeyMap,
}

impl AppKeystoreConfig {
    pub fn default_config() -> Self {
        Self {
            nostr_store: IDB_CONFIG_KEYSTORE_NOSTR,
            key_map: app_keystore_key_maps_default(),
        }
    }
}

pub fn app_keystore_key_maps_default() -> AppKeystoreKeyMap {
    let mut map = BTreeMap::new();
    map.insert("nostr_default", APP_KEYSTORE_KEY_NOSTR_DEFAULT);
    map
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

    pub fn from_key_maps(key_maps: AppKeyMapConfig) -> Self {
        Self {
            datastore: AppDatastoreConfig::default_config(key_maps),
            keystore: AppKeystoreConfig::default_config(),
        }
    }

    pub fn validate(&self) -> AppConfigResult<()> {
        app_key_maps_validate(&self.datastore.key_maps)?;
        app_keystore_key_maps_validate(&self.keystore.key_map)?;
        Ok(())
    }
}

pub fn app_config_default() -> AppConfig {
    AppConfig::from_key_maps(app_key_maps_default())
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
        app_datastore_key_eula_date,
        app_datastore_key_nostr_key,
        app_datastore_obj_key_app_data,
        app_datastore_obj_key_cfg_data,
        app_key_maps_validate,
        app_keystore_key_maps_default,
        app_keystore_key_maps_validate,
        app_keystore_key_nostr_default,
        app_keystore_key,
        app_datastore_param_key,
        AppConfig,
        AppConfigError,
        AppDatastoreConfig,
        AppKeyMapConfig,
        AppKeystoreConfig,
        AppKeystoreKeyMap,
        APP_DATASTORE_KEY_EULA_DATE,
        APP_DATASTORE_KEY_NOSTR_KEY,
        APP_DATASTORE_KEY_OBJ_APP_DATA,
        APP_DATASTORE_KEY_OBJ_CFG_DATA,
        APP_KEYSTORE_KEY_NOSTR_DEFAULT,
    };
    use radroots_studio_app_core::idb::{IDB_CONFIG_DATASTORE, IDB_CONFIG_KEYSTORE_NOSTR};
    use std::collections::BTreeMap;

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
        assert_eq!(
            config.datastore.key_maps.key_map.get("nostr_key"),
            Some(&APP_DATASTORE_KEY_NOSTR_KEY)
        );
    }

    #[test]
    fn app_config_validate_uses_key_map_rules() {
        let config = app_config_default();
        assert!(config.validate().is_ok());
        let empty = AppConfig::empty();
        assert!(empty.validate().is_err());
    }

    #[test]
    fn keystore_config_defaults_to_nostr_store() {
        let config = AppKeystoreConfig::default_config();
        assert_eq!(config.nostr_store, IDB_CONFIG_KEYSTORE_NOSTR);
    }

    #[test]
    fn keystore_key_maps_defaults_empty() {
        let map = app_keystore_key_maps_default();
        assert_eq!(
            map.get("nostr_default"),
            Some(&APP_KEYSTORE_KEY_NOSTR_DEFAULT)
        );
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

    #[test]
    fn key_map_validation_requires_expected_keys() {
        let config = super::app_key_maps_default();
        assert!(app_key_maps_validate(&config).is_ok());
        let mut missing = AppKeyMapConfig::empty();
        missing.key_map.insert("nostr_key", APP_DATASTORE_KEY_NOSTR_KEY);
        let err = app_key_maps_validate(&missing).expect_err("missing keys");
        assert_eq!(err, AppConfigError::MissingKeyMap("eula_date"));
    }

    #[test]
    fn keystore_map_validation_requires_expected_keys() {
        let map = app_keystore_key_maps_default();
        assert!(app_keystore_key_maps_validate(&map).is_ok());
        let empty: AppKeystoreKeyMap = BTreeMap::new();
        let err = app_keystore_key_maps_validate(&empty)
            .expect_err("missing keys");
        assert_eq!(err, AppConfigError::MissingKeystoreKeyMap("nostr_default"));
    }

    #[test]
    fn datastore_key_accessors_read_defaults() {
        let config = super::app_key_maps_default();
        assert_eq!(
            app_datastore_key_nostr_key(&config).expect("nostr key"),
            APP_DATASTORE_KEY_NOSTR_KEY
        );
        assert_eq!(
            app_datastore_key_eula_date(&config).expect("eula key"),
            APP_DATASTORE_KEY_EULA_DATE
        );
        assert_eq!(
            app_datastore_obj_key_cfg_data(&config).expect("cfg key"),
            APP_DATASTORE_KEY_OBJ_CFG_DATA
        );
        assert_eq!(
            app_datastore_obj_key_app_data(&config).expect("app key"),
            APP_DATASTORE_KEY_OBJ_APP_DATA
        );
        let nostr_param = app_datastore_param_key(&config, "nostr_profile").expect("param");
        assert_eq!(nostr_param("abc"), "nostr:abc:profile");
    }

    #[test]
    fn keystore_key_accessors_read_defaults() {
        let map = app_keystore_key_maps_default();
        assert_eq!(
            app_keystore_key_nostr_default(&map).expect("nostr default"),
            APP_KEYSTORE_KEY_NOSTR_DEFAULT
        );
        assert_eq!(
            app_keystore_key(&map, "nostr_default").expect("nostr default"),
            APP_KEYSTORE_KEY_NOSTR_DEFAULT
        );
    }
}
