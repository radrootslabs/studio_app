use crate::app::Tangle;
use radroots_core::types::IError;
use radroots_model::{
    tables::location_gcs::{lib_model_location_gcs_create, ILocationGcsCreate, ILocationGcsCreateResolve, lib_model_location_gcs_read, ILocationGcsRead, ILocationGcsReadResolve, lib_model_location_gcs_read_list, ILocationGcsReadList, ILocationGcsReadListResolve, lib_model_location_gcs_delete, ILocationGcsDelete, ILocationGcsDeleteResolve, lib_model_location_gcs_update, ILocationGcsUpdate, ILocationGcsUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_location_gcs_create(
    state: tauri::State<'_, Tangle>,
    args: ILocationGcsCreate,
) -> Result<ILocationGcsCreateResolve, IError> {
    match lib_model_location_gcs_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_location_gcs_read(
    state: tauri::State<'_, Tangle>,
    args: ILocationGcsRead,
) -> Result<ILocationGcsReadResolve, IError> {
    match lib_model_location_gcs_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_location_gcs_read_list(
    state: tauri::State<'_, Tangle>,
    args: ILocationGcsReadList,
) -> Result<ILocationGcsReadListResolve, IError> {
    match lib_model_location_gcs_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_location_gcs_update(
    state: tauri::State<'_, Tangle>,
    args: ILocationGcsUpdate,
) -> Result<ILocationGcsUpdateResolve, IError> {
    match lib_model_location_gcs_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_location_gcs_delete(
    state: tauri::State<'_, Tangle>,
    args: ILocationGcsDelete,
) -> Result<ILocationGcsDeleteResolve, IError> {
    match lib_model_location_gcs_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}