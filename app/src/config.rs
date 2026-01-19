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

#[cfg(test)]
mod tests {
    use super::AppKeyMapConfig;

    #[test]
    fn key_map_config_defaults_empty() {
        let config = AppKeyMapConfig::empty();
        assert!(config.key_map.is_empty());
        assert!(config.param_map.is_empty());
        assert!(config.obj_map.is_empty());
    }
}
