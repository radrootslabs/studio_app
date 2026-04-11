use crate::error::RadrootsAppRemoteSignerError;
use radroots_identity::RadrootsIdentityPublic;
use radroots_nostr_connect::prelude::{
    RadrootsNostrConnectMethod, RadrootsNostrConnectPermission, RadrootsNostrConnectPermissions,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RadrootsAppRemoteSignerSessionStatus {
    PendingApproval,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadrootsAppRemoteSignerSessionRecord {
    pub client_identity: RadrootsIdentityPublic,
    pub signer_identity: RadrootsIdentityPublic,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_identity: Option<RadrootsIdentityPublic>,
    pub relays: Vec<String>,
    #[serde(default)]
    pub approved_permissions: RadrootsNostrConnectPermissions,
    pub status: RadrootsAppRemoteSignerSessionStatus,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadrootsAppRemoteSignerSessionStoreState {
    pub version: u32,
    pub sessions: Vec<RadrootsAppRemoteSignerSessionRecord>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppRemoteSignerSessionStoreLoadResult {
    pub state: RadrootsAppRemoteSignerSessionStoreState,
    pub recovered_from_corruption: bool,
}

impl Default for RadrootsAppRemoteSignerSessionStoreState {
    fn default() -> Self {
        Self {
            version: RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION,
            sessions: Vec::new(),
        }
    }
}

impl RadrootsAppRemoteSignerSessionRecord {
    pub fn pending(
        client_identity: RadrootsIdentityPublic,
        signer_identity: RadrootsIdentityPublic,
        relays: Vec<String>,
    ) -> Self {
        let now = now_unix_secs();
        Self {
            client_identity,
            signer_identity,
            user_identity: None,
            relays,
            approved_permissions: RadrootsNostrConnectPermissions::default(),
            status: RadrootsAppRemoteSignerSessionStatus::PendingApproval,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn account_id(&self) -> Option<&str> {
        self.user_identity
            .as_ref()
            .map(|identity| identity.id.as_str())
    }

    pub fn client_account_id(&self) -> &str {
        self.client_identity.id.as_str()
    }

    pub fn approved_permission_labels(&self) -> Vec<String> {
        self.approved_permissions
            .as_slice()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    pub fn allows_sign_event_kind1(&self) -> bool {
        self.approved_permissions
            .as_slice()
            .iter()
            .any(|permission| {
                permission_matches(
                    permission,
                    &RadrootsNostrConnectPermission::with_parameter(
                        RadrootsNostrConnectMethod::SignEvent,
                        "kind:1",
                    ),
                )
            })
    }

    pub fn allows_switch_relays(&self) -> bool {
        self.approved_permissions
            .as_slice()
            .iter()
            .any(|permission| {
                permission_matches(
                    permission,
                    &RadrootsNostrConnectPermission::new(RadrootsNostrConnectMethod::SwitchRelays),
                )
            })
    }
}

impl RadrootsAppRemoteSignerSessionStoreState {
    pub fn load(path: &Path) -> Result<Self, RadrootsAppRemoteSignerError> {
        Ok(Self::load_with_recovery(path)?.state)
    }

    pub fn load_with_recovery(
        path: &Path,
    ) -> Result<RadrootsAppRemoteSignerSessionStoreLoadResult, RadrootsAppRemoteSignerError> {
        match std::fs::read(path) {
            Ok(contents) => Self::load_bytes(path, contents),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(RadrootsAppRemoteSignerSessionStoreLoadResult {
                    state: Self::default(),
                    recovered_from_corruption: false,
                })
            }
            Err(error) => Err(RadrootsAppRemoteSignerError::SessionStoreIo(
                error.to_string(),
            )),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), RadrootsAppRemoteSignerError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))?;
        let temp_path = temporary_store_path(path);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path.as_path())
            .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))?;
        if let Err(error) = (|| -> Result<(), std::io::Error> {
            file.write_all(json.as_bytes())?;
            file.flush()?;
            file.sync_all()
        })() {
            let _ = std::fs::remove_file(temp_path.as_path());
            return Err(RadrootsAppRemoteSignerError::SessionStoreIo(
                error.to_string(),
            ));
        }

        #[cfg(windows)]
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))?;
        }

        std::fs::rename(temp_path.as_path(), path)
            .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))
    }

    pub fn pending_session(&self) -> Option<&RadrootsAppRemoteSignerSessionRecord> {
        self.sessions
            .iter()
            .find(|record| record.status == RadrootsAppRemoteSignerSessionStatus::PendingApproval)
    }

    pub fn active_session_for_account_id(
        &self,
        account_id: &str,
    ) -> Option<&RadrootsAppRemoteSignerSessionRecord> {
        self.sessions.iter().find(|record| {
            record.status == RadrootsAppRemoteSignerSessionStatus::Active
                && record.account_id() == Some(account_id)
        })
    }

    pub fn upsert_pending(
        &mut self,
        pending: RadrootsAppRemoteSignerSessionRecord,
    ) -> Result<(), RadrootsAppRemoteSignerError> {
        if self.pending_session().is_some() {
            return Err(RadrootsAppRemoteSignerError::PendingSessionExists);
        }
        self.sessions
            .retain(|record| record.client_account_id() != pending.client_account_id());
        self.sessions.push(pending);
        Ok(())
    }

    pub fn activate_session(
        &mut self,
        client_account_id: &str,
        user_identity: RadrootsIdentityPublic,
        relays: Vec<String>,
        approved_permissions: RadrootsNostrConnectPermissions,
    ) -> Option<RadrootsAppRemoteSignerSessionRecord> {
        let now = now_unix_secs();
        self.sessions.retain(|record| {
            !(record.status == RadrootsAppRemoteSignerSessionStatus::Active
                && record.account_id() == Some(user_identity.id.as_str()))
        });
        let record = self
            .sessions
            .iter_mut()
            .find(|record| record.client_account_id() == client_account_id)?;
        record.user_identity = Some(user_identity);
        record.relays = relays;
        record.approved_permissions = approved_permissions;
        record.status = RadrootsAppRemoteSignerSessionStatus::Active;
        record.updated_at_unix = now;
        Some(record.clone())
    }

    pub fn remove_pending_session(&mut self) -> Option<RadrootsAppRemoteSignerSessionRecord> {
        let index = self.sessions.iter().position(|record| {
            record.status == RadrootsAppRemoteSignerSessionStatus::PendingApproval
        })?;
        Some(self.sessions.remove(index))
    }

    pub fn remove_active_session_for_account_id(
        &mut self,
        account_id: &str,
    ) -> Option<RadrootsAppRemoteSignerSessionRecord> {
        let index = self.sessions.iter().position(|record| {
            record.status == RadrootsAppRemoteSignerSessionStatus::Active
                && record.account_id() == Some(account_id)
        })?;
        Some(self.sessions.remove(index))
    }

    fn load_bytes(
        path: &Path,
        contents: Vec<u8>,
    ) -> Result<RadrootsAppRemoteSignerSessionStoreLoadResult, RadrootsAppRemoteSignerError> {
        let contents = String::from_utf8(contents).map_err(|error| {
            RadrootsAppRemoteSignerError::InvalidSessionStore(format!(
                "session store was not valid utf-8: {error}"
            ))
        });

        let contents = match contents {
            Ok(contents) => contents,
            Err(error) => {
                quarantine_invalid_store(path)?;
                let _ = error;
                return Ok(RadrootsAppRemoteSignerSessionStoreLoadResult {
                    state: Self::default(),
                    recovered_from_corruption: true,
                });
            }
        };

        let state = match serde_json::from_str::<Self>(&contents) {
            Ok(state) => state,
            Err(error) => {
                quarantine_invalid_store(path)?;
                let _ = error;
                return Ok(RadrootsAppRemoteSignerSessionStoreLoadResult {
                    state: Self::default(),
                    recovered_from_corruption: true,
                });
            }
        };

        if state.version != RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION {
            quarantine_invalid_store(path)?;
            return Ok(RadrootsAppRemoteSignerSessionStoreLoadResult {
                state: Self::default(),
                recovered_from_corruption: true,
            });
        }

        Ok(RadrootsAppRemoteSignerSessionStoreLoadResult {
            state,
            recovered_from_corruption: false,
        })
    }
}

fn permission_matches(
    granted_permission: &RadrootsNostrConnectPermission,
    required_permission: &RadrootsNostrConnectPermission,
) -> bool {
    if granted_permission.method != required_permission.method {
        return false;
    }

    match (
        &granted_permission.method,
        granted_permission.parameter.as_deref(),
        required_permission.parameter.as_deref(),
    ) {
        (RadrootsNostrConnectMethod::SignEvent, None, _) => true,
        (RadrootsNostrConnectMethod::SignEvent, Some(parameter), Some(required)) => {
            parameter == required || parameter == sign_event_kind_suffix(required)
        }
        (_, None, _) => true,
        (_, Some(parameter), Some(required)) => parameter == required,
        (_, Some(_), None) => false,
    }
}

fn sign_event_kind_suffix(value: &str) -> &str {
    value.strip_prefix("kind:").unwrap_or(value)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn temporary_store_path(path: &Path) -> std::path::PathBuf {
    let process_id = std::process::id();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    path.with_extension(format!("json.tmp-{process_id}-{timestamp}"))
}

fn quarantine_invalid_store(path: &Path) -> Result<(), RadrootsAppRemoteSignerError> {
    let process_id = std::process::id();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("remote-signer-sessions.json");
    let quarantine_path =
        path.with_file_name(format!("{file_name}.corrupt-{timestamp}-{process_id}"));
    std::fs::rename(path, quarantine_path.as_path())
        .map_err(|error| RadrootsAppRemoteSignerError::SessionStoreIo(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, FIXTURE_BOB, fixture_identity};
    use radroots_nostr_connect::prelude::{
        RadrootsNostrConnectMethod, RadrootsNostrConnectPermission,
    };

    fn fixture_public(
        label: &radroots_studio_app_test_support::RadrootsAppApprovedFixtureIdentity,
    ) -> RadrootsIdentityPublic {
        fixture_identity(label).expect("identity").to_public()
    }

    fn pending_record() -> RadrootsAppRemoteSignerSessionRecord {
        RadrootsAppRemoteSignerSessionRecord::pending(
            fixture_public(&FIXTURE_ALICE),
            fixture_public(&FIXTURE_BOB),
            vec!["wss://relay.example.com".to_owned()],
        )
    }

    #[test]
    fn pending_store_round_trips() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        state.upsert_pending(pending_record()).expect("pending");
        state.save(path.as_path()).expect("save");

        let loaded = RadrootsAppRemoteSignerSessionStoreState::load(path.as_path()).expect("load");

        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(
            loaded.sessions[0].status,
            RadrootsAppRemoteSignerSessionStatus::PendingApproval
        );
    }

    #[test]
    fn activate_session_replaces_pending_with_active_user_identity() {
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        let pending = pending_record();
        let client_account_id = pending.client_account_id().to_owned();
        state.upsert_pending(pending).expect("pending");

        let alice_public = fixture_public(&FIXTURE_ALICE);
        let active = state
            .activate_session(
                client_account_id.as_str(),
                alice_public.clone(),
                vec!["wss://relay.updated.example".to_owned()],
                vec![
                    RadrootsNostrConnectPermission::with_parameter(
                        RadrootsNostrConnectMethod::SignEvent,
                        "kind:1",
                    ),
                    RadrootsNostrConnectPermission::new(RadrootsNostrConnectMethod::SwitchRelays),
                ]
                .into(),
            )
            .expect("active");

        assert_eq!(active.status, RadrootsAppRemoteSignerSessionStatus::Active);
        assert_eq!(active.account_id(), Some(alice_public.id.as_str()));
        assert_eq!(
            active.relays,
            vec!["wss://relay.updated.example".to_owned()]
        );
        assert_eq!(
            active.approved_permission_labels(),
            vec!["sign_event:kind:1".to_owned(), "switch_relays".to_owned()]
        );
        assert!(active.allows_sign_event_kind1());
        assert!(active.allows_switch_relays());
        assert!(state.pending_session().is_none());
    }

    #[test]
    fn remove_active_session_matches_user_account_id() {
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        let pending = pending_record();
        let client_account_id = pending.client_account_id().to_owned();
        state.upsert_pending(pending).expect("pending");
        let alice_public = fixture_public(&FIXTURE_ALICE);
        state.activate_session(
            client_account_id.as_str(),
            alice_public.clone(),
            vec!["wss://relay.updated.example".to_owned()],
            RadrootsNostrConnectPermissions::default(),
        );

        let removed = state
            .remove_active_session_for_account_id(alice_public.id.as_str())
            .expect("removed");

        assert_eq!(removed.account_id(), Some(alice_public.id.as_str()));
        assert!(state.sessions.is_empty());
    }

    #[test]
    fn load_recovers_from_invalid_json_by_quarantining_store() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        std::fs::write(path.as_path(), "{invalid").expect("write invalid");

        let loaded = RadrootsAppRemoteSignerSessionStoreState::load(path.as_path()).expect("load");

        assert!(loaded.sessions.is_empty());
        assert!(!path.exists());
        let quarantined = std::fs::read_dir(temp.path())
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .any(|entry| entry.file_name().to_string_lossy().contains("corrupt"));
        assert!(quarantined);
    }

    #[test]
    fn load_recovers_from_unsupported_schema_version() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("sessions.json");
        std::fs::write(path.as_path(), r#"{"version":999,"sessions":[]}"#).expect("write invalid");

        let loaded = RadrootsAppRemoteSignerSessionStoreState::load(path.as_path()).expect("load");

        assert_eq!(
            loaded.version,
            RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION
        );
        assert!(loaded.sessions.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn active_session_permission_helpers_respect_sign_event_and_switch_relays() {
        let mut record = pending_record();
        record.user_identity = Some(fixture_public(&FIXTURE_ALICE));
        record.status = RadrootsAppRemoteSignerSessionStatus::Active;
        record.approved_permissions = vec![RadrootsNostrConnectPermission::with_parameter(
            RadrootsNostrConnectMethod::SignEvent,
            "1",
        )]
        .into();

        assert!(record.allows_sign_event_kind1());
        assert!(!record.allows_switch_relays());
    }
}
