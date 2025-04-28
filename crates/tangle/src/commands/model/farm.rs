use crate::app::Tangle;
use tangle_core::types::IError;
use tangle_model::{
    tables::farm::{lib_model_farm_create, IFarmCreate, IFarmCreateResolve, lib_model_farm_read, IFarmRead, IFarmReadResolve, lib_model_farm_read_list, IFarmReadList, IFarmReadListResolve, lib_model_farm_delete, IFarmDelete, IFarmDeleteResolve, lib_model_farm_update, IFarmUpdate, IFarmUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_create(
    state: tauri::State<'_, Tangle>,
    args: IFarmCreate,
) -> Result<IFarmCreateResolve, IError> {
    match lib_model_farm_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_read(
    state: tauri::State<'_, Tangle>,
    args: IFarmRead,
) -> Result<IFarmReadResolve, IError> {
    match lib_model_farm_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_read_list(
    state: tauri::State<'_, Tangle>,
    args: IFarmReadList,
) -> Result<IFarmReadListResolve, IError> {
    match lib_model_farm_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_update(
    state: tauri::State<'_, Tangle>,
    args: IFarmUpdate,
) -> Result<IFarmUpdateResolve, IError> {
    match lib_model_farm_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_delete(
    state: tauri::State<'_, Tangle>,
    args: IFarmDelete,
) -> Result<IFarmDeleteResolve, IError> {
    match lib_model_farm_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}