#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};

use crate::{
    app_datastore_obj_key_cfg_data,
    app_datastore_obj_key_app_data,
    app_log_debug_emit,
    RadrootsAppState,
    RadrootsAppSettings,
    AppInitError,
    AppInitResult,
    AppKeyMapConfig,
};

pub async fn app_datastore_write_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    data: &RadrootsAppSettings,
) -> AppInitResult<RadrootsAppSettings> {
    let key = app_datastore_obj_key_cfg_data(key_maps).map_err(AppInitError::Config)?;
    let value = datastore
        .set_obj(key, data)
        .await
        .map_err(AppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.config", "write", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_has_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<bool> {
    let key = app_datastore_obj_key_cfg_data(key_maps).map_err(AppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppSettings>(key).await {
        Ok(_) => Ok(true),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(false),
        Err(err) => Err(AppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_write_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    data: &RadrootsAppState,
) -> AppInitResult<RadrootsAppState> {
    let key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    let value = datastore
        .set_obj(key, data)
        .await
        .map_err(AppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.app_data", "write", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_read_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<RadrootsAppState> {
    let key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    let value = datastore
        .get_obj::<RadrootsAppState>(key)
        .await
        .map_err(AppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.app_data", "read", Some(key.to_string()));
    Ok(value)
}

pub async fn app_datastore_has_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<bool> {
    let key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    match datastore.get_obj::<RadrootsAppState>(key).await {
        Ok(_) => Ok(true),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(false),
        Err(err) => Err(AppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_clear_bootstrap<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<()> {
    let cfg_key = app_datastore_obj_key_cfg_data(key_maps).map_err(AppInitError::Config)?;
    datastore
        .del_obj(cfg_key)
        .await
        .map_err(AppInitError::Datastore)?;
    let app_key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    datastore
        .del_obj(app_key)
        .await
        .map_err(AppInitError::Datastore)?;
    let _ = app_log_debug_emit("log.app.bootstrap.reset", "clear", None);
    Ok(())
}

pub async fn app_datastore_set_notifications_permission<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    permission: &str,
) -> AppInitResult<RadrootsAppState> {
    let mut data = match app_datastore_has_app_data(datastore, key_maps).await? {
        true => app_datastore_read_app_data(datastore, key_maps).await?,
        false => RadrootsAppState::default(),
    };
    data.notifications_permission = Some(permission.to_string());
    let value = app_datastore_write_app_data(datastore, key_maps, &data).await?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        app_datastore_clear_bootstrap,
        app_datastore_has_app_data,
        app_datastore_has_config,
        app_datastore_read_app_data,
        app_datastore_set_notifications_permission,
        app_datastore_write_app_data,
        app_datastore_write_config,
    };
    use crate::{app_key_maps_default, RadrootsAppState, RadrootsAppSettings, AppInitError};
    use radroots_studio_app_core::datastore::{RadrootsClientDatastoreError, RadrootsClientWebDatastore};

    #[test]
    fn config_write_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let data = RadrootsAppSettings::default();
        let err = futures::executor::block_on(app_datastore_write_config(
            &datastore,
            &key_maps,
            &data,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn app_data_write_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let data = RadrootsAppState::default();
        let err = futures::executor::block_on(app_datastore_write_app_data(
            &datastore,
            &key_maps,
            &data,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn app_data_read_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_read_app_data(
            &datastore,
            &key_maps,
        ))
        .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
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
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn has_config_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_has_config(&datastore, &key_maps))
            .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn has_app_data_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_has_app_data(&datastore, &key_maps))
            .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }

    #[test]
    fn set_notifications_permission_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let err = futures::executor::block_on(app_datastore_set_notifications_permission(
            &datastore,
            &key_maps,
            "granted",
        ))
        .expect_err("idb undefined");
        assert_eq!(err, AppInitError::Datastore(RadrootsClientDatastoreError::IdbUndefined));
    }
}
