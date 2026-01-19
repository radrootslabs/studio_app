#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};

use crate::{
    app_datastore_obj_key_cfg_data,
    app_datastore_obj_key_app_data,
    AppAppData,
    AppConfigData,
    AppInitError,
    AppInitResult,
    AppKeyMapConfig,
};

pub async fn app_datastore_write_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    data: &AppConfigData,
) -> AppInitResult<AppConfigData> {
    let key = app_datastore_obj_key_cfg_data(key_maps).map_err(AppInitError::Config)?;
    datastore
        .set_obj(key, data)
        .await
        .map_err(AppInitError::Datastore)
}

pub async fn app_datastore_has_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<bool> {
    let key = app_datastore_obj_key_cfg_data(key_maps).map_err(AppInitError::Config)?;
    match datastore.get_obj::<AppConfigData>(key).await {
        Ok(_) => Ok(true),
        Err(RadrootsClientDatastoreError::NoResult) => Ok(false),
        Err(err) => Err(AppInitError::Datastore(err)),
    }
}

pub async fn app_datastore_write_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
    data: &AppAppData,
) -> AppInitResult<AppAppData> {
    let key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    datastore
        .set_obj(key, data)
        .await
        .map_err(AppInitError::Datastore)
}

pub async fn app_datastore_has_app_data<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &AppKeyMapConfig,
) -> AppInitResult<bool> {
    let key = app_datastore_obj_key_app_data(key_maps).map_err(AppInitError::Config)?;
    match datastore.get_obj::<AppAppData>(key).await {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        app_datastore_clear_bootstrap,
        app_datastore_has_app_data,
        app_datastore_has_config,
        app_datastore_write_app_data,
        app_datastore_write_config,
    };
    use crate::{app_key_maps_default, AppAppData, AppConfigData, AppInitError};
    use radroots_studio_app_core::datastore::{RadrootsClientDatastoreError, RadrootsClientWebDatastore};

    #[test]
    fn config_write_maps_idb_errors() {
        let datastore = RadrootsClientWebDatastore::new(None);
        let key_maps = app_key_maps_default();
        let data = AppConfigData::default();
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
        let data = AppAppData::default();
        let err = futures::executor::block_on(app_datastore_write_app_data(
            &datastore,
            &key_maps,
            &data,
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
}
