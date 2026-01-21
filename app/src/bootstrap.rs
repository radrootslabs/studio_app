#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};
use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;

use crate::{
    app_datastore_obj_key_settings,
    app_datastore_obj_key_state,
    app_log_debug_emit,
    RadrootsAppState,
    RadrootsAppSettings,
    RadrootsAppInitError,
    RadrootsAppInitResult,
    RadrootsAppKeyMapConfig,
};

pub async fn app_datastore_write_settings<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    data: &RadrootsAppSettings,
) -> RadrootsAppInitResult<RadrootsAppSettings> {
    let key = app_datastore_obj_key_settings(key_maps).map_err(RadrootsAppInitError::Config)?;
    let value = datastore
        .set_obj(key, data)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.config", "write", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_has_settings<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<bool> {
    let key = app_datastore_obj_key_settings(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppSettings>(key).await {
        Ok(_) => Ok(true),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(false),
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_write_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    data: &RadrootsAppState,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    let value = datastore
        .set_obj(key, data)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.state", "write", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_read_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    let value = datastore
        .get_obj::<RadrootsAppState>(key)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.state", "read", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_has_state<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<bool> {
    let key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppState>(key).await {
        Ok(_) => Ok(true),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(false),
        Err(err) => Err(RadrootsAppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_clear_bootstrap<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppInitResult<()> {
    let cfg_key = app_datastore_obj_key_settings(key_maps).map_err(RadrootsAppInitError::Config)?;
    datastore
        .del_obj(cfg_key)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let app_key = app_datastore_obj_key_state(key_maps).map_err(RadrootsAppInitError::Config)?;
    datastore
        .del_obj(app_key)
        .await
        .map_err(RadrootsAppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.reset", "clear", None);
    Ok(())
}

pub async fn app_state_set_notifications_permission<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    permission: &str,
) -> RadrootsAppInitResult<RadrootsAppState> {
    let mut data = match app_datastore_has_state(datastore, key_maps).await? {
        true => app_datastore_read_state(datastore, key_maps).await?,
        false => RadrootsAppState::default(),
    };
    data.notifications_permission = Some(permission.to_string());
    let value = app_datastore_write_state(datastore, key_maps, &data).await?;
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
        app_datastore_has_state,
        app_datastore_has_settings,
        app_datastore_read_state,
        app_state_set_notifications_permission,
        app_state_set_notifications_permission_value,
        app_state_notifications_permission_value,
        app_datastore_write_state,
        app_datastore_write_settings,
    };
    use crate::{app_key_maps_default, RadrootsAppState, RadrootsAppSettings, RadrootsAppInitError};
    use radroots_studio_app_core::datastore::{RadrootsClientDatastoreError, RadrootsClientWebDatastore};
    use radroots_studio_app_core::notifications::RadrootsClientNotificationsPermission;

    #[test]
    fn settings_write_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let data = RadrootsAppSettings::default();
        let err = futures::executor::block_on(app_datastore_write_settings(
            &datastore,
            &key_maps,
            &data,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, RadrootsAppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
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
    fn has_settings_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_has_settings(&datastore, &key_maps))
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
}
