#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppConfigRole {
    Public,
}

impl Default for AppConfigRole {
    fn default() -> Self {
        AppConfigRole::Public
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfigData {
    pub nostr_public_key: Option<String>,
    pub nostr_profile: Option<String>,
    pub role: Option<AppConfigRole>,
    pub nip05_request: Option<bool>,
    pub nip05_key: Option<String>,
}

impl Default for AppConfigData {
    fn default() -> Self {
        Self {
            nostr_public_key: None,
            nostr_profile: None,
            role: None,
            nip05_request: None,
            nip05_key: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppAppData {
    pub active_key: String,
    pub role: AppConfigRole,
    pub eula_date: String,
    pub nip05_key: Option<String>,
}

impl Default for AppAppData {
    fn default() -> Self {
        Self {
            active_key: String::new(),
            role: AppConfigRole::default(),
            eula_date: String::new(),
            nip05_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppAppData, AppConfigData, AppConfigRole};

    #[test]
    fn config_role_defaults_to_public() {
        assert_eq!(AppConfigRole::default(), AppConfigRole::Public);
    }

    #[test]
    fn config_data_defaults_empty() {
        let data = AppConfigData::default();
        assert!(data.nostr_public_key.is_none());
        assert!(data.nostr_profile.is_none());
        assert!(data.role.is_none());
        assert!(data.nip05_request.is_none());
        assert!(data.nip05_key.is_none());
    }

    #[test]
    fn app_data_defaults_empty() {
        let data = AppAppData::default();
        assert_eq!(data.active_key, "");
        assert_eq!(data.role, AppConfigRole::Public);
        assert_eq!(data.eula_date, "");
        assert!(data.nip05_key.is_none());
    }
}
