#![forbid(unsafe_code)]

use radroots_studio_app_core::datastore::RadrootsClientDatastore;

use crate::{
    app_datastore_obj_key_cfg_data,
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

#[cfg(test)]
mod tests {
    use super::app_datastore_write_config;
    use crate::{app_key_maps_default, AppConfigData, AppInitError};
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
}
