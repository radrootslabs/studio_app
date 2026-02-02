#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};

use crate::{
    app_datastore_key_setup_lock,
    RadrootsAppInitError,
    RadrootsAppInitResult,
    RadrootsAppKeyMapConfig,
    RadrootsAppStateError,
};

pub const APP_SETUP_LOCK_TTL_MS: u64 = 10 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppSetupLock {
    pub owner: String,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppSetupLockStatus {
    Acquired(RadrootsAppSetupLock),
    Locked(RadrootsAppSetupLock),
}

pub const fn app_setup_lock_enabled() -> bool {
    cfg!(target_arch = "wasm32")
}

pub const fn app_setup_lock_ttl_ms() -> u64 {
    APP_SETUP_LOCK_TTL_MS
}

pub fn app_setup_lock_is_expired(lock: &RadrootsAppSetupLock, now_ms: u64) -> bool {
    lock.expires_at_ms <= now_ms
}

fn app_setup_lock_new(owner: &str, now_ms: u64, ttl_ms: u64) -> RadrootsAppSetupLock {
    RadrootsAppSetupLock {
        owner: owner.to_string(),
        expires_at_ms: now_ms.saturating_add(ttl_ms),
    }
}

pub async fn app_setup_lock_acquire<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    owner: &str,
    now_ms: u64,
    ttl_ms: u64,
) -> RadrootsAppInitResult<RadrootsAppSetupLockStatus> {
    let key = app_datastore_key_setup_lock(key_maps).map_err(RadrootsAppInitError::Config)?;
    let existing = match datastore.get(key).await {
        Ok(value) => serde_json::from_str::<RadrootsAppSetupLock>(&value).ok(),
        Err(RadrootsClientDatastoreError::NoResult) => None,
        Err(err) => return Err(RadrootsAppInitError::Datastore(err)),
    };
    if let Some(lock) = existing.as_ref() {
        if !app_setup_lock_is_expired(lock, now_ms) && lock.owner != owner {
            return Ok(RadrootsAppSetupLockStatus::Locked(lock.clone()));
        }
    }
    let lock = app_setup_lock_new(owner, now_ms, ttl_ms);
    let encoded = serde_json::to_string(&lock)
        .map_err(|_| RadrootsAppInitError::State(RadrootsAppStateError::Corrupt))?;
    datastore
        .set(key, &encoded)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    Ok(RadrootsAppSetupLockStatus::Acquired(lock))
}

pub async fn app_setup_lock_release<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<()> {
    let key = app_datastore_key_setup_lock(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.del(key).await {
        Ok(_) => Ok(()),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(()),
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_setup_lock_acquire,
        app_setup_lock_enabled,
        app_setup_lock_is_expired,
        app_setup_lock_release,
        app_setup_lock_ttl_ms,
        RadrootsAppSetupLock,
        RadrootsAppSetupLockStatus,
        APP_SETUP_LOCK_TTL_MS,
    };
    use crate::{app_key_maps_default, RadrootsAppKeyMapConfig};
    use async_trait::async_trait;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
    };
    use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_DATASTORE};
    use serde::{de::DeserializeOwned, Serialize};
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    #[test]
    fn lock_enabled_matches_target_arch() {
        assert_eq!(app_setup_lock_enabled(), cfg!(target_arch = "wasm32"));
    }

    #[test]
    fn lock_ttl_defaults_to_constant() {
        assert_eq!(app_setup_lock_ttl_ms(), APP_SETUP_LOCK_TTL_MS);
    }

    #[test]
    fn lock_expired_checks_timestamp() {
        let lock = RadrootsAppSetupLock {
            owner: "owner".to_string(),
            expires_at_ms: 10,
        };
        assert!(!app_setup_lock_is_expired(&lock, 5));
        assert!(app_setup_lock_is_expired(&lock, 10));
    }

    struct LockDatastore {
        values: RefCell<BTreeMap<String, String>>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for LockDatastore {
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
            _value: &T,
        ) -> RadrootsClientDatastoreResult<T>
        where
            T: Serialize + DeserializeOwned + Clone,
        {
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
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn del(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
            let removed = self.values.borrow_mut().remove(key);
            match removed {
                Some(value) => Ok(value),
                None => Err(RadrootsClientDatastoreError::NoResult),
            }
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

    fn lock_datastore() -> LockDatastore {
        LockDatastore {
            values: RefCell::new(BTreeMap::new()),
        }
    }

    fn lock_key_maps() -> RadrootsAppKeyMapConfig {
        app_key_maps_default()
    }

    #[test]
    fn acquire_returns_locked_for_other_owner() {
        let datastore = lock_datastore();
        let key_maps = lock_key_maps();
        let acquired = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-a",
            100,
            50,
        ))
        .expect("acquire");
        assert!(matches!(acquired, RadrootsAppSetupLockStatus::Acquired(_)));
        let locked = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-b",
            120,
            50,
        ))
        .expect("acquire");
        assert!(matches!(locked, RadrootsAppSetupLockStatus::Locked(_)));
    }

    #[test]
    fn acquire_refreshes_for_same_owner() {
        let datastore = lock_datastore();
        let key_maps = lock_key_maps();
        let _ = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-a",
            100,
            50,
        ))
        .expect("acquire");
        let refreshed = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-a",
            140,
            50,
        ))
        .expect("refresh");
        assert!(matches!(refreshed, RadrootsAppSetupLockStatus::Acquired(_)));
    }

    #[test]
    fn release_clears_lock() {
        let datastore = lock_datastore();
        let key_maps = lock_key_maps();
        let _ = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-a",
            100,
            50,
        ))
        .expect("acquire");
        futures::executor::block_on(app_setup_lock_release(&datastore, &key_maps))
            .expect("release");
        let acquired = futures::executor::block_on(app_setup_lock_acquire(
            &datastore,
            &key_maps,
            "owner-b",
            200,
            50,
        ))
        .expect("acquire");
        assert!(matches!(acquired, RadrootsAppSetupLockStatus::Acquired(_)));
    }
}
