#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::RadrootsClientDatastore;
use radroots_studio_app_core::keystore::{RadrootsClientKeystoreError, RadrootsClientKeystoreNostr};

#[cfg(target_arch = "wasm32")]
use js_sys::Date;

#[cfg(not(target_arch = "wasm32"))]
use chrono::{SecondsFormat, Utc};

use crate::{
    app_datastore_create_state,
    app_datastore_key_nostr_key,
    app_default_relays,
    app_keystore_nostr_ensure_key,
    app_keystore_nostr_verify_key,
    app_log_debug_emit,
    RadrootsAppInitError,
    RadrootsAppInitResult,
    RadrootsAppKeyMapConfig,
    RadrootsAppKeystoreError,
    RadrootsAppRole,
    RadrootsAppState,
};

#[cfg(target_arch = "wasm32")]
pub fn app_setup_eula_date() -> String {
    Date::new_0().to_iso_string().into()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn app_setup_eula_date() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn app_setup_state_new(
    active_key: String,
    eula_date: String,
    role: RadrootsAppRole,
) -> RadrootsAppState {
    RadrootsAppState {
        active_key,
        role,
        eula_date,
        eula_version: String::from("0.1.0"),
        eula_hash: String::from("unknown"),
        relays: app_default_relays(),
        nip05_key: None,
        notifications_permission: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppSetupStep {
    Intro,
    KeyChoice,
    KeyAddExisting,
    Profile,
    FarmerSetup,
    BusinessSetup,
    Eula,
}

impl RadrootsAppSetupStep {
    pub const fn next(self) -> Self {
        match self {
            RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::KeyChoice,
            RadrootsAppSetupStep::KeyChoice => RadrootsAppSetupStep::KeyAddExisting,
            RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::Profile,
            RadrootsAppSetupStep::Profile => RadrootsAppSetupStep::FarmerSetup,
            RadrootsAppSetupStep::FarmerSetup => RadrootsAppSetupStep::BusinessSetup,
            RadrootsAppSetupStep::BusinessSetup => RadrootsAppSetupStep::Eula,
            RadrootsAppSetupStep::Eula => RadrootsAppSetupStep::Eula,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::Intro,
            RadrootsAppSetupStep::KeyChoice => RadrootsAppSetupStep::Intro,
            RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::KeyChoice,
            RadrootsAppSetupStep::Profile => RadrootsAppSetupStep::KeyAddExisting,
            RadrootsAppSetupStep::FarmerSetup => RadrootsAppSetupStep::Profile,
            RadrootsAppSetupStep::BusinessSetup => RadrootsAppSetupStep::FarmerSetup,
            RadrootsAppSetupStep::Eula => RadrootsAppSetupStep::BusinessSetup,
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, RadrootsAppSetupStep::Eula)
    }
}

pub const fn app_setup_step_default() -> RadrootsAppSetupStep {
    RadrootsAppSetupStep::Intro
}

pub async fn app_setup_initialize<T: RadrootsClientDatastore, K: RadrootsClientKeystoreNostr>(
    datastore: &T,
    keystore: &K,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let active_key = app_keystore_nostr_ensure_key(keystore)
        .await
        .map_err(|err| match err {
            RadrootsAppKeystoreError::Keystore(inner) => RadrootsAppInitError::Keystore(inner),
            RadrootsAppKeystoreError::KeyMismatch => {
                RadrootsAppInitError::Keystore(RadrootsClientKeystoreError::NostrInvalidSecretKey)
            }
        })?;
    app_keystore_nostr_verify_key(keystore, &active_key)
        .await
        .map_err(|err| match err {
            RadrootsAppKeystoreError::Keystore(inner) => RadrootsAppInitError::Keystore(inner),
            RadrootsAppKeystoreError::KeyMismatch => {
                RadrootsAppInitError::Keystore(RadrootsClientKeystoreError::NostrInvalidSecretKey)
            }
        })?;
    app_setup_finalize_with_key(
        datastore,
        key_maps,
        active_key,
        app_setup_eula_date(),
        None,
        RadrootsAppRole::default(),
    )
    .await
}

pub async fn app_setup_finalize_with_key<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    active_key: String,
    eula_date: String,
    nip05_key: Option<String>,
    role: RadrootsAppRole,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let mut state = app_setup_state_new(active_key.clone(), eula_date, role);
    state.nip05_key = nip05_key;
    let stored_state = app_datastore_create_state(datastore, key_maps, &state).await?;
    let key_name = app_datastore_key_nostr_key(key_maps).map_err(RadrootsAppInitError::Config)?;
    datastore
        .set(key_name, &active_key)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.setup", "created", Some(format!("key={active_key}")));
    Ok(stored_state)
}

#[cfg(test)]
mod tests {
    use super::{
        app_setup_eula_date,
        app_setup_finalize_with_key,
        app_setup_initialize,
        app_setup_state_new,
        app_setup_step_default,
        RadrootsAppSetupStep,
    };
    use crate::{app_datastore_key_nostr_key, app_key_maps_default, RadrootsAppRole, RadrootsAppStateRecord};
    use async_trait::async_trait;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
    };
    use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_DATASTORE};
    use radroots_studio_app_core::keystore::{
        RadrootsClientKeystoreError,
        RadrootsClientKeystoreNostr,
        RadrootsClientKeystoreResult,
    };
    use radroots_nostr::prelude::{RadrootsNostrKeys, RadrootsNostrSecretKey};
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct TestKeystore {
        keys_result: RadrootsClientKeystoreResult<Vec<String>>,
        generate_result: RadrootsClientKeystoreResult<String>,
        read_result: RadrootsClientKeystoreResult<String>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientKeystoreNostr for TestKeystore {
        async fn generate(&self) -> RadrootsClientKeystoreResult<String> {
            self.generate_result.clone()
        }

        async fn add(&self, _secret_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn read(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            self.read_result.clone()
        }

        async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>> {
            self.keys_result.clone()
        }

        async fn remove(&self, _public_key: &str) -> RadrootsClientKeystoreResult<String> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }

        async fn reset(&self) -> RadrootsClientKeystoreResult<()> {
            Err(RadrootsClientKeystoreError::IdbUndefined)
        }
    }

    struct TestDatastore {
        record: RefCell<Option<RadrootsAppStateRecord>>,
        values: RefCell<BTreeMap<String, String>>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for TestDatastore {
        fn get_config(&self) -> RadrootsClientIdbConfig {
            IDB_CONFIG_DATASTORE
        }

        fn get_store_id(&self) -> &str {
            "test"
        }

        async fn init(&self) -> RadrootsClientDatastoreResult<()> {
            Ok(())
        }

        async fn set(&self, key: &str, value: &str) -> RadrootsClientDatastoreResult<String> {
            self.values.borrow_mut().insert(key.to_string(), value.to_string());
            Ok(value.to_string())
        }

        async fn get(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
            self.values
                .borrow()
                .get(key)
                .cloned()
                .ok_or(RadrootsClientDatastoreError::NoResult)
        }

        async fn set_obj<T>(
            &self,
            _key: &str,
            value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
            let encoded = serde_json::to_string(value)
                .map_err(|_| RadrootsClientDatastoreError::IdbUndefined)?;
            if let Ok(parsed) = serde_json::from_str::<RadrootsAppStateRecord>(&encoded) {
                *self.record.borrow_mut() = Some(parsed);
                return Ok(value.clone());
            }
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn update_obj<T>(
            &self,
            _key: &str,
            _value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get_obj<T>(&self, _key: &str) -> RadrootsClientDatastoreResult<T>
        where
            T: DeserializeOwned,
        {
            if let Some(record) = self.record.borrow().as_ref() {
                let encoded = serde_json::to_string(record)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
                if let Ok(parsed) = serde_json::from_str(&encoded) {
                    return Ok(parsed);
                }
            };
            Err(RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del_pref(&self, _key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn set_param(
            &self,
            _key: &str,
            _key_param: &str,
            _value: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get_param(
            &self,
            _key: &str,
            _key_param: &str,
        ) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn keys(&self) -> RadrootsClientDatastoreResult<Vec<String>> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn entries(&self) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn entries_pref(
            &self,
            _key_prefix: &str,
        ) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn reset(&self) -> RadrootsClientDatastoreResult<()> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn export_backup(
            &self,
        ) -> RadrootsClientDatastoreResult<RadrootsClientBackupDatastorePayload> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn import_backup(
            &self,
            _payload: RadrootsClientBackupDatastorePayload,
        ) -> RadrootsClientDatastoreResult<()> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }
    }

    #[test]
    fn setup_state_new_populates_defaults() {
        let state = app_setup_state_new(
            "pub".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
            RadrootsAppRole::default(),
        );
        assert_eq!(state.active_key, "pub");
        assert_eq!(state.role, RadrootsAppRole::Individual);
        assert_eq!(state.eula_date, "2025-01-01T00:00:00Z");
        assert!(!state.relays.is_empty());
        assert!(state.nip05_key.is_none());
        assert!(state.notifications_permission.is_none());
    }

    #[test]
    fn setup_eula_date_is_non_empty() {
        let value = app_setup_eula_date();
        assert!(!value.is_empty());
    }

    #[test]
    fn setup_step_default_is_intro() {
        assert_eq!(app_setup_step_default(), RadrootsAppSetupStep::Intro);
    }

    #[test]
    fn setup_step_next_advances_once() {
        assert_eq!(
            RadrootsAppSetupStep::Intro.next(),
            RadrootsAppSetupStep::KeyChoice
        );
        assert_eq!(
            RadrootsAppSetupStep::KeyChoice.next(),
            RadrootsAppSetupStep::KeyAddExisting
        );
        assert_eq!(
            RadrootsAppSetupStep::KeyAddExisting.next(),
            RadrootsAppSetupStep::Profile
        );
        assert_eq!(
            RadrootsAppSetupStep::Profile.next(),
            RadrootsAppSetupStep::FarmerSetup
        );
        assert_eq!(
            RadrootsAppSetupStep::FarmerSetup.next(),
            RadrootsAppSetupStep::BusinessSetup
        );
        assert_eq!(
            RadrootsAppSetupStep::BusinessSetup.next(),
            RadrootsAppSetupStep::Eula
        );
        assert_eq!(
            RadrootsAppSetupStep::Eula.next(),
            RadrootsAppSetupStep::Eula
        );
    }

    #[test]
    fn setup_step_prev_rewinds_once() {
        assert_eq!(
            RadrootsAppSetupStep::Intro.prev(),
            RadrootsAppSetupStep::Intro
        );
        assert_eq!(
            RadrootsAppSetupStep::KeyChoice.prev(),
            RadrootsAppSetupStep::Intro
        );
        assert_eq!(
            RadrootsAppSetupStep::KeyAddExisting.prev(),
            RadrootsAppSetupStep::KeyChoice
        );
        assert_eq!(
            RadrootsAppSetupStep::Profile.prev(),
            RadrootsAppSetupStep::KeyAddExisting
        );
        assert_eq!(
            RadrootsAppSetupStep::FarmerSetup.prev(),
            RadrootsAppSetupStep::Profile
        );
        assert_eq!(
            RadrootsAppSetupStep::BusinessSetup.prev(),
            RadrootsAppSetupStep::FarmerSetup
        );
        assert_eq!(
            RadrootsAppSetupStep::Eula.prev(),
            RadrootsAppSetupStep::BusinessSetup
        );
    }

    #[test]
    fn setup_step_terminal_matches_eula() {
        assert!(!RadrootsAppSetupStep::Intro.is_terminal());
        assert!(!RadrootsAppSetupStep::KeyChoice.is_terminal());
        assert!(!RadrootsAppSetupStep::KeyAddExisting.is_terminal());
        assert!(!RadrootsAppSetupStep::Profile.is_terminal());
        assert!(!RadrootsAppSetupStep::FarmerSetup.is_terminal());
        assert!(!RadrootsAppSetupStep::BusinessSetup.is_terminal());
        assert!(RadrootsAppSetupStep::Eula.is_terminal());
    }

    #[test]
    fn setup_initialize_creates_state_and_key() {
        let secret_key = RadrootsNostrSecretKey::generate();
        let secret_hex = secret_key.to_secret_hex();
        let keys = RadrootsNostrKeys::new(secret_key);
        let public_key = keys.public_key().to_hex();
        let datastore = TestDatastore {
            record: RefCell::new(None),
            values: RefCell::new(BTreeMap::new()),
        };
        let keystore = TestKeystore {
            keys_result: Err(RadrootsClientKeystoreError::NostrNoResults),
            generate_result: Ok(public_key.clone()),
            read_result: Ok(secret_hex),
        };
        let key_maps = app_key_maps_default();
        let state = futures::executor::block_on(app_setup_initialize(
            &datastore,
            &keystore,
            &key_maps,
        ))
        .expect("setup");
        assert_eq!(state.active_key, public_key);
        let key_name = app_datastore_key_nostr_key(&key_maps).expect("key name");
        let stored = futures::executor::block_on(datastore.get(key_name)).expect("stored");
        assert_eq!(stored, public_key);
        assert!(datastore.record.borrow().is_some());
    }

    #[test]
    fn setup_finalize_with_key_writes_state() {
        let datastore = TestDatastore {
            record: RefCell::new(None),
            values: RefCell::new(BTreeMap::new()),
        };
        let key_maps = app_key_maps_default();
        let state = futures::executor::block_on(app_setup_finalize_with_key(
            &datastore,
            &key_maps,
            "pub".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
            None,
            RadrootsAppRole::default(),
        ))
        .expect("finalize");
        assert_eq!(state.active_key, "pub");
        let key_name = app_datastore_key_nostr_key(&key_maps).expect("key name");
        let stored = futures::executor::block_on(datastore.get(key_name)).expect("stored");
        assert_eq!(stored, "pub");
        assert!(datastore.record.borrow().is_some());
    }
}
