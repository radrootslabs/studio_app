#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use radroots_studio_app_core::idb::{
    RadrootsClientIdbConfig,
    IDB_CONFIG_DATASTORE,
    IDB_CONFIG_KEYSTORE_NOSTR,
};

pub type RadrootsAppDatastoreKeyParam = fn(&str) -> String;
pub type RadrootsAppDatastoreKeyMap = BTreeMap<&'static str, &'static str>;
pub type RadrootsAppDatastoreKeyParamMap = BTreeMap<&'static str, RadrootsAppDatastoreKeyParam>;
pub type RadrootsAppDatastoreKeyObjMap = BTreeMap<&'static str, &'static str>;
pub type RadrootsAppKeystoreKeyMap = BTreeMap<&'static str, &'static str>;

pub const APP_DATASTORE_KEY_NOSTR_KEY: &str = "nostr:key";
pub const APP_DATASTORE_KEY_EULA_DATE: &str = "app:eula:date";
pub const APP_DATASTORE_KEY_OBJ_SETTINGS: &str = "cfg:data";
pub const APP_DATASTORE_KEY_OBJ_STATE: &str = "app:data";
pub const APP_DATASTORE_KEY_LOG_ENTRY: &str = "log:entry";
pub const APP_KEYSTORE_KEY_NOSTR_DEFAULT: &str = "nostr:default";

pub fn app_datastore_param_nostr_profile(public_key: &str) -> String {
    format!("nostr:{public_key}:profile")
}

pub fn app_datastore_param_radroots_profile(public_key: &str) -> String {
    format!("radroots:{public_key}:profile")
}

pub fn app_datastore_param_log_entry(entry_id: &str) -> String {
    format!("{APP_DATASTORE_KEY_LOG_ENTRY}:{entry_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppKeyMapConfig {
    pub key_map: RadrootsAppDatastoreKeyMap,
    pub param_map: RadrootsAppDatastoreKeyParamMap,
    pub obj_map: RadrootsAppDatastoreKeyObjMap,
}

impl RadrootsAppKeyMapConfig {
    pub fn empty() -> Self {
        Self {
            key_map: BTreeMap::new(),
            param_map: BTreeMap::new(),
            obj_map: BTreeMap::new(),
        }
    }
}

pub fn app_key_maps_default() -> RadrootsAppKeyMapConfig {
    let mut key_map = BTreeMap::new();
    key_map.insert("nostr_key", APP_DATASTORE_KEY_NOSTR_KEY);
    key_map.insert("eula_date", APP_DATASTORE_KEY_EULA_DATE);
    let mut param_map = BTreeMap::new();
    param_map.insert("nostr_profile", app_datastore_param_nostr_profile as RadrootsAppDatastoreKeyParam);
    param_map.insert(
        "radroots_profile",
        app_datastore_param_radroots_profile as RadrootsAppDatastoreKeyParam,
    );
    param_map.insert("log_entry", app_datastore_param_log_entry as RadrootsAppDatastoreKeyParam);
    let mut obj_map = BTreeMap::new();
    obj_map.insert("cfg_data", APP_DATASTORE_KEY_OBJ_SETTINGS);
    obj_map.insert("app_data", APP_DATASTORE_KEY_OBJ_STATE);
    RadrootsAppKeyMapConfig {
        key_map,
        param_map,
        obj_map,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppConfigError {
    MissingKeyMap(&'static str),
    MissingParamMap(&'static str),
    MissingObjMap(&'static str),
    MissingKeystoreKeyMap(&'static str),
}

pub type RadrootsAppConfigResult<T> = Result<T, RadrootsAppConfigError>;

impl RadrootsAppConfigError {
    pub const fn message(&self) -> &'static str {
        match self {
            RadrootsAppConfigError::MissingKeyMap(_) => "error.app.config.key_map_missing",
            RadrootsAppConfigError::MissingParamMap(_) => "error.app.config.param_map_missing",
            RadrootsAppConfigError::MissingObjMap(_) => "error.app.config.obj_map_missing",
            RadrootsAppConfigError::MissingKeystoreKeyMap(_) => "error.app.config.keystore_map_missing",
        }
    }
}

impl std::fmt::Display for RadrootsAppConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppConfigError {}

pub fn app_key_maps_validate(config: &RadrootsAppKeyMapConfig) -> RadrootsAppConfigResult<()> {
    if !config.key_map.contains_key("nostr_key") {
        return Err(RadrootsAppConfigError::MissingKeyMap("nostr_key"));
    }
    if !config.key_map.contains_key("eula_date") {
        return Err(RadrootsAppConfigError::MissingKeyMap("eula_date"));
    }
    if !config.param_map.contains_key("nostr_profile") {
        return Err(RadrootsAppConfigError::MissingParamMap("nostr_profile"));
    }
    if !config.param_map.contains_key("radroots_profile") {
        return Err(RadrootsAppConfigError::MissingParamMap("radroots_profile"));
    }
    if !config.param_map.contains_key("log_entry") {
        return Err(RadrootsAppConfigError::MissingParamMap("log_entry"));
    }
    if !config.obj_map.contains_key("cfg_data") {
        return Err(RadrootsAppConfigError::MissingObjMap("cfg_data"));
    }
    if !config.obj_map.contains_key("app_data") {
        return Err(RadrootsAppConfigError::MissingObjMap("app_data"));
    }
    Ok(())
}

pub fn app_keystore_key_maps_validate(config: &RadrootsAppKeystoreKeyMap) -> RadrootsAppConfigResult<()> {
    if !config.contains_key("nostr_default") {
        return Err(RadrootsAppConfigError::MissingKeystoreKeyMap("nostr_default"));
    }
    Ok(())
}

pub fn app_datastore_key(config: &RadrootsAppKeyMapConfig, key: &'static str) -> RadrootsAppConfigResult<&'static str> {
    config
        .key_map
        .get(key)
        .copied()
        .ok_or(RadrootsAppConfigError::MissingKeyMap(key))
}

pub fn app_datastore_obj_key(
    config: &RadrootsAppKeyMapConfig,
    key: &'static str,
) -> RadrootsAppConfigResult<&'static str> {
    config
        .obj_map
        .get(key)
        .copied()
        .ok_or(RadrootsAppConfigError::MissingObjMap(key))
}

pub fn app_datastore_param_key(
    config: &RadrootsAppKeyMapConfig,
    key: &'static str,
) -> RadrootsAppConfigResult<RadrootsAppDatastoreKeyParam> {
    config
        .param_map
        .get(key)
        .copied()
        .ok_or(RadrootsAppConfigError::MissingParamMap(key))
}

pub fn app_datastore_key_nostr_key(config: &RadrootsAppKeyMapConfig) -> RadrootsAppConfigResult<&'static str> {
    app_datastore_key(config, "nostr_key")
}

pub fn app_datastore_key_eula_date(config: &RadrootsAppKeyMapConfig) -> RadrootsAppConfigResult<&'static str> {
    app_datastore_key(config, "eula_date")
}

pub fn app_datastore_obj_key_settings(config: &RadrootsAppKeyMapConfig) -> RadrootsAppConfigResult<&'static str> {
    app_datastore_obj_key(config, "cfg_data")
}

pub fn app_datastore_obj_key_state(config: &RadrootsAppKeyMapConfig) -> RadrootsAppConfigResult<&'static str> {
    app_datastore_obj_key(config, "app_data")
}

pub fn app_keystore_key(
    config: &RadrootsAppKeystoreKeyMap,
    key: &'static str,
) -> RadrootsAppConfigResult<&'static str> {
    config
        .get(key)
        .copied()
        .ok_or(RadrootsAppConfigError::MissingKeystoreKeyMap(key))
}

pub fn app_keystore_key_nostr_default(config: &RadrootsAppKeystoreKeyMap) -> RadrootsAppConfigResult<&'static str> {
    app_keystore_key(config, "nostr_default")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppKeystoreConfig {
    pub nostr_store: RadrootsClientIdbConfig,
    pub key_map: RadrootsAppKeystoreKeyMap,
}

impl RadrootsAppKeystoreConfig {
    pub fn default_config() -> Self {
        Self {
            nostr_store: IDB_CONFIG_KEYSTORE_NOSTR,
            key_map: app_keystore_key_maps_default(),
        }
    }
}

pub fn app_keystore_key_maps_default() -> RadrootsAppKeystoreKeyMap {
    let mut map = BTreeMap::new();
    map.insert("nostr_default", APP_KEYSTORE_KEY_NOSTR_DEFAULT);
    map
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppDatastoreConfig {
    pub idb_config: RadrootsClientIdbConfig,
    pub key_maps: RadrootsAppKeyMapConfig,
}

impl RadrootsAppDatastoreConfig {
    pub fn default_config(key_maps: RadrootsAppKeyMapConfig) -> Self {
        Self {
            idb_config: IDB_CONFIG_DATASTORE,
            key_maps,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RadrootsAppAssetConfig {
    pub sql_wasm_url: Option<String>,
    pub geocoder_db_url: Option<String>,
}

pub fn app_assets_sql_wasm_url(config: &RadrootsAppConfig) -> Option<&str> {
    config.assets.sql_wasm_url.as_deref()
}

pub fn app_assets_geocoder_db_url(config: &RadrootsAppConfig) -> Option<&str> {
    config.assets.geocoder_db_url.as_deref()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppConfig {
    pub datastore: RadrootsAppDatastoreConfig,
    pub keystore: RadrootsAppKeystoreConfig,
    pub assets: RadrootsAppAssetConfig,
}

impl RadrootsAppConfig {
    pub fn empty() -> Self {
        let key_maps = RadrootsAppKeyMapConfig::empty();
        Self {
            datastore: RadrootsAppDatastoreConfig::default_config(key_maps),
            keystore: RadrootsAppKeystoreConfig::default_config(),
            assets: RadrootsAppAssetConfig::default(),
        }
    }

    pub fn from_key_maps(key_maps: RadrootsAppKeyMapConfig) -> Self {
        Self {
            datastore: RadrootsAppDatastoreConfig::default_config(key_maps),
            keystore: RadrootsAppKeystoreConfig::default_config(),
            assets: RadrootsAppAssetConfig::default(),
        }
    }

    pub fn validate(&self) -> RadrootsAppConfigResult<()> {
        app_key_maps_validate(&self.datastore.key_maps)?;
        app_keystore_key_maps_validate(&self.keystore.key_map)?;
        Ok(())
    }
}

pub fn app_config_default() -> RadrootsAppConfig {
    RadrootsAppConfig::from_key_maps(app_key_maps_default())
}

pub fn app_config_from_env() -> RadrootsAppConfig {
    app_config_default()
}

#[cfg(test)]
mod tests {
    use super::{
        app_config_default,
        app_config_from_env,
        app_datastore_param_nostr_profile,
        app_datastore_param_log_entry,
        app_datastore_key_eula_date,
        app_datastore_key_nostr_key,
        app_datastore_obj_key_state,
        app_datastore_obj_key_settings,
        app_key_maps_validate,
        app_keystore_key_maps_default,
        app_keystore_key_maps_validate,
        app_keystore_key_nostr_default,
        app_keystore_key,
        app_datastore_param_key,
        app_assets_geocoder_db_url,
        app_assets_sql_wasm_url,
        RadrootsAppAssetConfig,
        RadrootsAppConfig,
        RadrootsAppConfigError,
        RadrootsAppDatastoreConfig,
        RadrootsAppKeyMapConfig,
        RadrootsAppKeystoreConfig,
        RadrootsAppKeystoreKeyMap,
        APP_DATASTORE_KEY_EULA_DATE,
        APP_DATASTORE_KEY_NOSTR_KEY,
        APP_DATASTORE_KEY_OBJ_STATE,
        APP_DATASTORE_KEY_OBJ_SETTINGS,
        APP_DATASTORE_KEY_LOG_ENTRY,
        APP_KEYSTORE_KEY_NOSTR_DEFAULT,
    };
    use radroots_studio_app_core::idb::{IDB_CONFIG_DATASTORE, IDB_CONFIG_KEYSTORE_NOSTR};
    use std::collections::BTreeMap;

    #[test]
    fn key_map_config_defaults_empty() {
        let config = RadrootsAppKeyMapConfig::empty();
        assert!(config.key_map.is_empty());
        assert!(config.param_map.is_empty());
        assert!(config.obj_map.is_empty());
    }

    #[test]
    fn app_config_defaults_empty() {
        let config = RadrootsAppConfig::empty();
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
        let empty = RadrootsAppConfig::empty();
        assert!(empty.validate().is_err());
    }

    #[test]
    fn keystore_config_defaults_to_nostr_store() {
        let config = RadrootsAppKeystoreConfig::default_config();
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
    fn asset_config_defaults_empty() {
        let config = RadrootsAppAssetConfig::default();
        assert!(config.sql_wasm_url.is_none());
        assert!(config.geocoder_db_url.is_none());
    }

    #[test]
    fn datastore_config_defaults_to_idb_store() {
        let key_maps = RadrootsAppKeyMapConfig::empty();
        let config = RadrootsAppDatastoreConfig::default_config(key_maps);
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
            Some(&APP_DATASTORE_KEY_OBJ_SETTINGS)
        );
        assert_eq!(
            config.obj_map.get("app_data"),
            Some(&APP_DATASTORE_KEY_OBJ_STATE)
        );
        assert_eq!(app_datastore_param_nostr_profile("abc"), "nostr:abc:profile");
        assert_eq!(
            app_datastore_param_log_entry("entry"),
            format!("{APP_DATASTORE_KEY_LOG_ENTRY}:entry")
        );
    }

    #[test]
    fn key_map_validation_requires_expected_keys() {
        let config = super::app_key_maps_default();
        assert!(app_key_maps_validate(&config).is_ok());
        let mut missing = RadrootsAppKeyMapConfig::empty();
        missing.key_map.insert("nostr_key", APP_DATASTORE_KEY_NOSTR_KEY);
        let err = app_key_maps_validate(&missing).expect_err("missing keys");
        assert_eq!(err, RadrootsAppConfigError::MissingKeyMap("eula_date"));
    }

    #[test]
    fn keystore_map_validation_requires_expected_keys() {
        let map = app_keystore_key_maps_default();
        assert!(app_keystore_key_maps_validate(&map).is_ok());
        let empty: RadrootsAppKeystoreKeyMap = BTreeMap::new();
        let err = app_keystore_key_maps_validate(&empty)
            .expect_err("missing keys");
        assert_eq!(err, RadrootsAppConfigError::MissingKeystoreKeyMap("nostr_default"));
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
            app_datastore_obj_key_settings(&config).expect("cfg key"),
            APP_DATASTORE_KEY_OBJ_SETTINGS
        );
        assert_eq!(
            app_datastore_obj_key_state(&config).expect("app key"),
            APP_DATASTORE_KEY_OBJ_STATE
        );
        let nostr_param = app_datastore_param_key(&config, "nostr_profile").expect("param");
        assert_eq!(nostr_param("abc"), "nostr:abc:profile");
        let log_param = app_datastore_param_key(&config, "log_entry").expect("param");
        assert_eq!(log_param("entry"), format!("{APP_DATASTORE_KEY_LOG_ENTRY}:entry"));
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

    #[test]
    fn asset_accessors_read_defaults() {
        let config = app_config_default();
        assert_eq!(app_assets_sql_wasm_url(&config), None);
        assert_eq!(app_assets_geocoder_db_url(&config), None);
    }
}
