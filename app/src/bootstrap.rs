#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};
use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;

use crate::{
    app_datastore_obj_key_state,
    app_datastore_obj_key_setup_draft,
    app_datastore_param_key,
    app_datastore_key_eula_date,
    app_datastore_key_nostr_key,
    app_log_debug_emit,
    app_setup_state_new,
    app_state_record_new,
    app_state_record_validate,
    app_state_timestamp_ms,
    RadrootsAppProfileSeed,
    RadrootsAppRole,
    RadrootsAppState,
    RadrootsAppSetupDraft,
    RadrootsAppStateError,
    RadrootsAppStateRecord,
    RadrootsAppInitError,
    RadrootsAppInitResult,
    RadrootsAppKeyMapConfig,
};

pub async fn app_datastore_write_state_record<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    record: &RadrootsAppStateRecord,
) -> RadrootsAppInitResult<RadrootsAppStateRecord> {
    let key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    let value = datastore
        .set_obj(key, record)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.state", "write", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_read_state_record<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<RadrootsAppStateRecord> {
    let key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppStateRecord>(key).await {
        Ok(record) => {
            app_state_record_validate(&record).map_err(RadrootsAppInitError::State)?;
            let _ =
                app_log_debug_emit("log.app.bootstrap.state", "read", Some(key.to_string()));
            Ok(record)
        }
        Err(RadrootsClientDatastoreError::NoResult) => {
            match datastore.get_obj::<RadrootsAppState>(key).await {
                Ok(state) => {
                    let record = app_state_record_new(state, 1, app_state_timestamp_ms());
                    let value = app_datastore_write_state_record(datastore, key_maps, &record)
                        .await?;
                    Ok(value)
                }
                Err(RadrootsClientDatastoreError::NoResult) => {
                    if let Some(record) = app_datastore_migrate_legacy_state(datastore, key_maps).await? {
                        return Ok(record);
                    }
                    Err(RadrootsAppInitError::State(RadrootsAppStateError::Missing))
                }
                Err(err) => Err(RadrootsAppInitError::Datastore(err)),
            }
        }
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

async fn app_datastore_migrate_legacy_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<Option<RadrootsAppStateRecord>> {
    let key_nostr = app_datastore_key_nostr_key(key_maps).map_err(RadrootsAppInitError::Config)?;
    let key_eula = app_datastore_key_eula_date(key_maps).map_err(RadrootsAppInitError::Config)?;
    let active_key = match datastore.get(key_nostr).await {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let eula_date = match datastore.get(key_eula).await {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let state = app_setup_state_new(active_key.clone(), eula_date, RadrootsAppRole::default());
    let record = app_state_record_new(state, 1, app_state_timestamp_ms());
    let stored = app_datastore_write_state_record(datastore, key_maps, &record).await?;
    let _ = datastore.del(key_nostr).await;
    let _ = datastore.del(key_eula).await;
    Ok(Some(stored))
}

pub async fn app_datastore_write_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    data: &RadrootsAppState,
) -> RadrootsAppInitResult<RadrootsAppState> {
    app_datastore_update_state(datastore, key_maps, data).await
}

pub async fn app_datastore_create_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    data: &RadrootsAppState,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let now_ms = app_state_timestamp_ms();
    match app_datastore_read_state_record(datastore, key_maps).await {
        Ok(_) => Err(RadrootsAppInitError::State(RadrootsAppStateError::AlreadyExists)),
        Err(RadrootsAppInitError::State(RadrootsAppStateError::Missing)) => {
            let record = app_state_record_new(data.clone(), 1, now_ms);
            let value = app_datastore_write_state_record(datastore, key_maps, &record).await?;
            Ok(value.state)
        }
        Err(err) => Err(err),
    }
}

pub async fn app_datastore_update_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    data: &RadrootsAppState,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let now_ms = app_state_timestamp_ms();
    let record = match app_datastore_read_state_record(datastore, key_maps).await {
        Ok(existing) => app_state_record_new(data.clone(), existing.revision + 1, now_ms),
        Err(RadrootsAppInitError::State(RadrootsAppStateError::Missing)) => {
            return Err(RadrootsAppInitError::State(RadrootsAppStateError::Missing));
        }
        Err(err) => return Err(err),
    };
    let value = app_datastore_write_state_record(datastore, key_maps, &record).await?;
    Ok(value.state)
}

pub async fn app_datastore_read_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let record = app_datastore_read_state_record(datastore, key_maps).await?;
    Ok(record.state)
}

pub async fn app_datastore_has_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<bool> {
    match app_datastore_read_state_record(datastore, key_maps).await {
        Ok(_) => Ok(true),
        Err(RadrootsAppInitError::State(RadrootsAppStateError::Missing)) => Ok(false),
        Err(err) => Err(err),
    }
}

pub async fn app_datastore_clear_bootstrap<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<()> {
    let app_key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    datastore
        .del_obj(app_key)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.reset", "clear", None);
    Ok(())
}

pub async fn app_datastore_read_setup_draft<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<Option<RadrootsAppSetupDraft>> {
    let key = app_datastore_obj_key_setup_draft(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppSetupDraft>(key).await {
        Ok(draft) => Ok(Some(draft)),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(None),
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_write_setup_draft<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    draft: &RadrootsAppSetupDraft,
) -> RadrootsAppInitResult<RadrootsAppSetupDraft> {
    let key = app_datastore_obj_key_setup_draft(key_maps).map_err(RadrootsAppInitError::Config)?;
    let value = datastore
        .set_obj(key, draft)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    Ok(value)
}

pub async fn app_datastore_clear_setup_draft<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<()> {
    let key = app_datastore_obj_key_setup_draft(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.del_obj(key).await {
        Ok(_) => Ok(()),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(()),
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_write_profile_seed<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    profile: &RadrootsAppProfileSeed,
) -> RadrootsAppInitResult<RadrootsAppProfileSeed> {
    let param = app_datastore_param_key(key_maps, "nostr_profile")
        .map_err(RadrootsAppInitError::Config)?;
    let key = param(&profile.public_key);
    let stored = datastore
        .set_obj(&key, profile)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.profile", "write", Some(key));
    Ok(stored)
}

pub async fn app_state_set_notifications_permission<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    permission: &str,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let mut data = app_datastore_read_state(datastore, key_maps).await?;
    data.notifications_permission = Some(permission.to_string());
    let value = app_datastore_update_state(datastore, key_maps, &data).await?;
    Ok(value)
}

pub fn app_state_notifications_permission_value(
    data: &RadrootsAppState,
) -> Option<RadrootsClientNotificationsPermission> {
    data.notifications_permission
        .as_deref()
        .and_then(RadrootsClientNotificationsPermission::parse)
}

pub async fn app_state_set_notifications_permission_value<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    permission: RadrootsClientNotificationsPermission,
) -> RadrootsAppInitResult<RadrootsAppState> {
    app_state_set_notifications_permission(datastore, key_maps, permission.as_str()).await
}

#[cfg(test)]
mod tests {
    use super::{
        app_datastore_clear_bootstrap,
        app_datastore_clear_setup_draft,
        app_datastore_create_state,
        app_datastore_has_state,
        app_datastore_read_state_record,
        app_datastore_read_state,
        app_datastore_read_setup_draft,
        app_datastore_update_state,
        app_datastore_write_setup_draft,
        app_datastore_write_profile_seed,
        app_state_set_notifications_permission,
        app_state_set_notifications_permission_value,
        app_state_notifications_permission_value,
        app_datastore_write_state,
    };
    use crate::{
        app_datastore_key_eula_date,
        app_datastore_key_nostr_key,
        app_key_maps_default,
        RadrootsAppInitError,
        RadrootsAppProfileSeed,
        RadrootsAppRole,
        RadrootsAppState,
        RadrootsAppStateError,
        RadrootsAppStateRecord,
        RadrootsAppSetupDraft,
    };
    use async_trait::async_trait;
    use radroots_studio_app_core::backup::RadrootsClientBackupDatastorePayload;
    use radroots_studio_app_core::datastore::{
        RadrootsClientDatastore,
        RadrootsClientDatastoreEntries,
        RadrootsClientDatastoreError,
        RadrootsClientDatastoreResult,
        RadrootsClientWebDatastore,
    };
    use radroots_studio_app_core::idb::{RadrootsClientIdbConfig, IDB_CONFIG_DATASTORE};
    use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct SetupDraftDatastore {
        draft: RefCell<Option<RadrootsAppSetupDraft>>,
    }

    struct ProfileSeedDatastore {
        profile: RefCell<Option<RadrootsAppProfileSeed>>,
    }

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for SetupDraftDatastore {
        fn get_config(&self) -> RadrootsClientIdbConfig {
            IDB_CONFIG_DATASTORE
        }

        fn get_store_id(&self) -> &str {
            "test"
        }

        async fn init(&self) -> RadrootsClientDatastoreResult<()> {
            Ok(())
        }

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
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
            if let Ok(parsed) = serde_json::from_str::<RadrootsAppSetupDraft>(&encoded) {
                *self.draft.borrow_mut() = Some(parsed);
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
            if let Some(draft) = self.draft.borrow().as_ref() {
                let encoded = serde_json::to_string(draft)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
                return serde_json::from_str(&encoded)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult);
            }
            Err(RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            *self.draft.borrow_mut() = None;
            Ok("cleared".to_string())
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

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for ProfileSeedDatastore {
        fn get_config(&self) -> RadrootsClientIdbConfig {
            IDB_CONFIG_DATASTORE
        }

        fn get_store_id(&self) -> &str {
            "test"
        }

        async fn init(&self) -> RadrootsClientDatastoreResult<()> {
            Ok(())
        }

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
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
            if let Ok(parsed) = serde_json::from_str::<RadrootsAppProfileSeed>(&encoded) {
                *self.profile.borrow_mut() = Some(parsed);
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
            if let Some(profile) = self.profile.borrow().as_ref() {
                let encoded = serde_json::to_string(profile)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
                return serde_json::from_str(&encoded)
                    .map_err(|_| RadrootsClientDatastoreError::NoResult);
            }
            Err(RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            *self.profile.borrow_mut() = None;
            Ok("cleared".to_string())
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

    struct TestDatastore {
        state: Option<RadrootsAppState>,
        record: RefCell<Option<RadrootsAppStateRecord>>,
    }

    struct LegacyKeyDatastore {
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

        async fn set(&self, _key: &str, _value: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
        }

        async fn get(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            Err(RadrootsClientDatastoreError::IdbUndefined)
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
            let Some(state) = self.state.as_ref() else {
                return Err(RadrootsClientDatastoreError::NoResult);
            };
            let encoded = serde_json::to_string(state)
                .map_err(|_| RadrootsClientDatastoreError::NoResult)?;
            serde_json::from_str(&encoded).map_err(|_| RadrootsClientDatastoreError::NoResult)
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

    #[async_trait(?Send)]
    impl RadrootsClientDatastore for LegacyKeyDatastore {
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
            self.values
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
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
            }
            Err(RadrootsClientDatastoreError::NoResult)
        }

        async fn del_obj(&self, _key: &str) -> RadrootsClientDatastoreResult<String> {
            *self.record.borrow_mut() = None;
            Ok("cleared".to_string())
        }

        async fn del(&self, key: &str) -> RadrootsClientDatastoreResult<String> {
            self.values.borrow_mut().remove(key);
            Ok(key.to_string())
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
    fn state_write_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let data = RadrootsAppState::default();
        let err = futures::executor::block_on(app_datastore_write_state(
            &datastore,
            &key_maps,
            &data,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn state_read_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_read_state(
            &datastore,
            &key_maps,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn clear_bootstrap_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_clear_bootstrap(
            &datastore,
            &key_maps,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn has_state_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_has_state(&datastore, &key_maps))
            .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn set_notifications_permission_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_state_set_notifications_permission(
            &datastore,
            &key_maps,
            "granted",
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn notifications_permission_value_parses_state() {
        let mut state = RadrootsAppState::default();
        assert!(app_state_notifications_permission_value(&state).is_none());
        state.notifications_permission = Some(String::from("granted"));
        assert_eq!(
            app_state_notifications_permission_value(&state),
            Some(RadrootsClientNotificationsPermission::Granted)
        );
    }

    #[test]
    fn notifications_permission_value_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_state_set_notifications_permission_value(
            &datastore,
            &key_maps,
            RadrootsClientNotificationsPermission::Granted,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn set_notifications_permission_requires_state() {
        let datastore = TestDatastore {
            state: None,
            record: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_state_set_notifications_permission(
            &datastore,
            &key_maps,
            "granted",
        ))
        .expect_err("missing state");
        assert_eq!(err, RadrootsAppInitError::State(RadrootsAppStateError::Missing));
    }

    #[test]
    fn set_notifications_permission_updates_state() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        let datastore = TestDatastore {
            state: Some(state),
            record: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let updated = futures::executor::block_on(app_state_set_notifications_permission(
            &datastore,
            &key_maps,
            "granted",
        ))
        .expect("updated");
        assert_eq!(updated.notifications_permission.as_deref(), Some("granted"));
        let record = datastore.record.borrow();
        let record = record.as_ref().expect("record");
        assert_eq!(record.state.notifications_permission.as_deref(), Some("granted"));
    }

    #[test]
    fn create_state_writes_record() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        let datastore = TestDatastore {
            state: None,
            record: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let created = futures::executor::block_on(app_datastore_create_state(
            &datastore,
            &key_maps,
            &state,
        ))
        .expect("created");
        assert_eq!(created.active_key, "pub");
        let record = datastore.record.borrow();
        let record = record.as_ref().expect("record");
        assert_eq!(record.state.active_key, "pub");
    }

    #[test]
    fn create_state_reports_existing() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        let datastore = TestDatastore {
            state: Some(state.clone()),
            record: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_create_state(
            &datastore,
            &key_maps,
            &state,
        ))
        .expect_err("exists");
        assert_eq!(err, RadrootsAppInitError::State(RadrootsAppStateError::AlreadyExists));
    }

    #[test]
    fn update_state_requires_existing_record() {
        let mut state = RadrootsAppState::default();
        state.active_key = "pub".to_string();
        state.eula_date = "2025-01-01T00:00:00Z".to_string();
        let datastore = TestDatastore {
            state: None,
            record: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_update_state(
            &datastore,
            &key_maps,
            &state,
        ))
        .expect_err("missing");
        assert_eq!(err, RadrootsAppInitError::State(RadrootsAppStateError::Missing));
    }

    #[test]
    fn setup_draft_roundtrip() {
        let datastore = SetupDraftDatastore {
            draft: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let draft = RadrootsAppSetupDraft {
            nostr_public_key: Some("pub".to_string()),
            profile_name: Some("radroots".to_string()),
            role: Some(RadrootsAppRole::Individual),
            nip05_request: Some(true),
        };
        let stored = futures::executor::block_on(app_datastore_write_setup_draft(
            &datastore,
            &key_maps,
            &draft,
        ))
        .expect("store draft");
        assert_eq!(stored, draft);
        let loaded = futures::executor::block_on(app_datastore_read_setup_draft(
            &datastore,
            &key_maps,
        ))
        .expect("read draft");
        assert_eq!(loaded, Some(draft));
        let cleared = futures::executor::block_on(app_datastore_clear_setup_draft(
            &datastore,
            &key_maps,
        ));
        assert!(cleared.is_ok());
        let loaded = futures::executor::block_on(app_datastore_read_setup_draft(
            &datastore,
            &key_maps,
        ))
        .expect("read draft");
        assert!(loaded.is_none());
    }

    #[test]
    fn profile_seed_write_persists_data() {
        let datastore = ProfileSeedDatastore {
            profile: RefCell::new(None),
        };
        let key_maps = app_key_maps_default();
        let profile = RadrootsAppProfileSeed {
            public_key: "pub".to_string(),
            name: "radroots".to_string(),
            display_name: Some("Radroots".to_string()),
            nip05_request: true,
        };
        let stored = futures::executor::block_on(app_datastore_write_profile_seed(
            &datastore,
            &key_maps,
            &profile,
        ))
        .expect("profile seed");
        assert_eq!(stored, profile);
        let stored_profile = datastore.profile.borrow().clone();
        assert_eq!(stored_profile, Some(profile));
    }

    #[test]
    fn state_record_migrates_legacy_keys() {
        let key_maps = app_key_maps_default();
        let key_nostr = app_datastore_key_nostr_key(&key_maps).expect("nostr key");
        let key_eula = app_datastore_key_eula_date(&key_maps).expect("eula key");
        let mut values = BTreeMap::new();
        values.insert(key_nostr.to_string(), "pub".to_string());
        values.insert(key_eula.to_string(), "2025-01-01T00:00:00Z".to_string());
        let datastore = LegacyKeyDatastore {
            record: RefCell::new(None),
            values: RefCell::new(values),
        };
        let record = futures::executor::block_on(app_datastore_read_state_record(
            &datastore,
            &key_maps,
        ))
        .expect("record");
        assert_eq!(record.state.active_key, "pub");
        assert_eq!(record.state.eula_date, "2025-01-01T00:00:00Z");
        assert!(datastore.values.borrow().get(key_nostr).is_none());
        assert!(datastore.values.borrow().get(key_eula).is_none());
    }
}
