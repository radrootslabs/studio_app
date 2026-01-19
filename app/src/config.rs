#![forbid(unsafe_code)]

use std::collections::BTreeMap;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub key_maps: AppKeyMapConfig,
}

impl AppConfig {
    pub fn empty() -> Self {
        Self {
            key_maps: AppKeyMapConfig::empty(),
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
    use super::{app_config_default, app_config_from_env, AppConfig, AppKeyMapConfig};

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
}
