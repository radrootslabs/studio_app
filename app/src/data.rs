#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use sha2::{Digest, Sha256};

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

pub const APP_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppStateError {
    Missing,
    Corrupt,
    InvalidChecksum,
    UnsupportedVersion(u32),
    AlreadyExists,
}

impl RadrootsAppStateError {
    pub const fn message(&self) -> &'static str {
        match self {
            RadrootsAppStateError::Missing => "error.app.state.missing",
            RadrootsAppStateError::Corrupt => "error.app.state.corrupt",
            RadrootsAppStateError::InvalidChecksum => "error.app.state.checksum_invalid",
            RadrootsAppStateError::UnsupportedVersion(_) => "error.app.state.schema_unsupported",
            RadrootsAppStateError::AlreadyExists => "error.app.state.already_exists",
        }
    }
}

impl std::fmt::Display for RadrootsAppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppStateError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppStateRecord {
    pub schema_version: u32,
    pub revision: u64,
    pub updated_at_ms: i64,
    pub checksum: String,
    pub state: RadrootsAppState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RadrootsAppStateChecksumPayload {
    schema_version: u32,
    revision: u64,
    updated_at_ms: i64,
    state: RadrootsAppState,
}

pub fn app_state_timestamp_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_millis() as i64)
            .unwrap_or(0)
    }
}

fn app_state_record_checksum(payload: &RadrootsAppStateChecksumPayload) -> String {
    let serialized =
        serde_json::to_vec(payload).unwrap_or_else(|_| Vec::new());
    let hash = Sha256::digest(&serialized);
    hex::encode(hash)
}

pub fn app_state_record_new(
    state: RadrootsAppState,
    revision: u64,
    updated_at_ms: i64,
) -> RadrootsAppStateRecord {
    let payload = RadrootsAppStateChecksumPayload {
        schema_version: APP_STATE_SCHEMA_VERSION,
        revision,
        updated_at_ms,
        state: state.clone(),
    };
    let checksum = app_state_record_checksum(&payload);
    RadrootsAppStateRecord {
        schema_version: APP_STATE_SCHEMA_VERSION,
        revision,
        updated_at_ms,
        checksum,
        state,
    }
}

pub fn app_state_record_validate(
    record: &RadrootsAppStateRecord,
) -> Result<(), RadrootsAppStateError> {
    if record.schema_version != APP_STATE_SCHEMA_VERSION {
        return Err(RadrootsAppStateError::UnsupportedVersion(
            record.schema_version,
        ));
    }
    let payload = RadrootsAppStateChecksumPayload {
        schema_version: record.schema_version,
        revision: record.revision,
        updated_at_ms: record.updated_at_ms,
        state: record.state.clone(),
    };
    let expected = app_state_record_checksum(&payload);
    if record.checksum != expected {
        return Err(RadrootsAppStateError::InvalidChecksum);
    }
    Ok(())
}

pub fn app_state_is_initialized(state: &RadrootsAppState) -> bool {
    !state.active_key.is_empty() && !state.eula_date.is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        app_state_is_initialized,
        app_state_record_new,
        app_state_record_validate,
        app_state_timestamp_ms,
        RadrootsAppRole,
        RadrootsAppState,
        RadrootsAppStateError,
        APP_STATE_SCHEMA_VERSION,
    };

    #[test]
    fn role_defaults_to_public() {
        assert_eq!(RadrootsAppRole::default(), RadrootsAppRole::Public);
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

    #[test]
    fn state_record_validates_checksum() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        let record = app_state_record_new(state, 1, app_state_timestamp_ms());
        assert!(app_state_record_validate(&record).is_ok());
    }

    #[test]
    fn state_record_detects_checksum_mismatch() {
        let state = RadrootsAppState::default();
        let mut record = app_state_record_new(state, 1, app_state_timestamp_ms());
        record.checksum = "bad".to_string();
        let err = app_state_record_validate(&record).expect_err("checksum");
        assert_eq!(err, RadrootsAppStateError::InvalidChecksum);
    }

    #[test]
    fn state_record_rejects_unsupported_version() {
        let state = RadrootsAppState::default();
        let mut record = app_state_record_new(state, 1, app_state_timestamp_ms());
        record.schema_version = APP_STATE_SCHEMA_VERSION + 1;
        let err = app_state_record_validate(&record).expect_err("version");
        assert_eq!(
            err,
            RadrootsAppStateError::UnsupportedVersion(APP_STATE_SCHEMA_VERSION + 1)
        );
    }
}
