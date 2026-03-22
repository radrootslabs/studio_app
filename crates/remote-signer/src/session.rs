use crate::error::RadrootsAppRemoteSignerError;
use radroots_identity::RadrootsIdentityPublic;
use serde::{Deserialize, Serialize};
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
    pub status: RadrootsAppRemoteSignerSessionStatus,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadrootsAppRemoteSignerSessionStoreState {
    pub version: u32,
    pub sessions: Vec<RadrootsAppRemoteSignerSessionRecord>,
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
}

impl RadrootsAppRemoteSignerSessionStoreState {
    pub fn load(path: &Path) -> Result<Self, RadrootsAppRemoteSignerError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let state: Self = serde_json::from_str(&contents).map_err(|error| {
                    RadrootsAppRemoteSignerError::InvalidSessionStore(error.to_string())
                })?;
                if state.version != RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION {
                    return Err(RadrootsAppRemoteSignerError::InvalidSessionStore(format!(
                        "unsupported schema version {}",
                        state.version
                    )));
                }
                Ok(state)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
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
        std::fs::write(path, json)
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
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, FIXTURE_BOB, fixture_identity};

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
            .activate_session(client_account_id.as_str(), alice_public.clone())
            .expect("active");

        assert_eq!(active.status, RadrootsAppRemoteSignerSessionStatus::Active);
        assert_eq!(active.account_id(), Some(alice_public.id.as_str()));
        assert!(state.pending_session().is_none());
    }

    #[test]
    fn remove_active_session_matches_user_account_id() {
        let mut state = RadrootsAppRemoteSignerSessionStoreState::default();
        let pending = pending_record();
        let client_account_id = pending.client_account_id().to_owned();
        state.upsert_pending(pending).expect("pending");
        let alice_public = fixture_public(&FIXTURE_ALICE);
        state.activate_session(client_account_id.as_str(), alice_public.clone());

        let removed = state
            .remove_active_session_for_account_id(alice_public.id.as_str())
            .expect("removed");

        assert_eq!(removed.account_id(), Some(alice_public.id.as_str()));
        assert!(state.sessions.is_empty());
    }
}
