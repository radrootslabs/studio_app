#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadrootsAppRole {
    Public,
}

impl Default for RadrootsAppRole {
    fn default() -> Self {
        RadrootsAppRole::Public
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppSettings {
    pub nostr_public_key: Option<String>,
    pub nostr_profile: Option<String>,
    pub role: Option<RadrootsAppRole>,
    pub nip05_request: Option<bool>,
    pub nip05_key: Option<String>,
}

impl Default for RadrootsAppSettings {
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
pub struct RadrootsAppState {
    pub active_key: String,
    pub role: RadrootsAppRole,
    pub eula_date: String,
    pub nip05_key: Option<String>,
    pub notifications_permission: Option<String>,
}

impl Default for RadrootsAppState {
    fn default() -> Self {
        Self {
            active_key: String::new(),
            role: RadrootsAppRole::default(),
            eula_date: String::new(),
            nip05_key: None,
            notifications_permission: None,
        }
    }
}

pub fn app_state_is_initialized(state: &RadrootsAppState) -> bool {
    !state.active_key.is_empty() && !state.eula_date.is_empty()
}

#[cfg(test)]
mod tests {
    use super::{app_state_is_initialized, RadrootsAppRole, RadrootsAppSettings, RadrootsAppState};

    #[test]
    fn role_defaults_to_public() {
        assert_eq!(RadrootsAppRole::default(), RadrootsAppRole::Public);
    }

    #[test]
    fn settings_defaults_empty() {
        let data = RadrootsAppSettings::default();
        assert!(data.nostr_public_key.is_none());
        assert!(data.nostr_profile.is_none());
        assert!(data.role.is_none());
        assert!(data.nip05_request.is_none());
        assert!(data.nip05_key.is_none());
    }

    #[test]
    fn state_defaults_empty() {
        let data = RadrootsAppState::default();
        assert_eq!(data.active_key, "");
        assert_eq!(data.role, RadrootsAppRole::Public);
        assert_eq!(data.eula_date, "");
        assert!(data.nip05_key.is_none());
        assert!(data.notifications_permission.is_none());
    }

    #[test]
    fn state_initialized_requires_key_and_eula() {
        let data = RadrootsAppState::default();
        assert!(!app_state_is_initialized(&data));
        let mut data = RadrootsAppState::default();
        data.active_key = "pub".to_string();
        assert!(!app_state_is_initialized(&data));
        data.eula_date = "2025-01-01T00:00:00Z".to_string();
        assert!(app_state_is_initialized(&data));
    }
}
