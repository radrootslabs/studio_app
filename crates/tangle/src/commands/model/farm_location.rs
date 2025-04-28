use crate::app::Tangle;
use tangle_core::types::IError;
use tangle_model::tables::farm_location::{
    lib_model_farm_location_set, lib_model_farm_location_unset,
    IFarmLocationRelation, IFarmLocationResolve,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_location_set(
    state: tauri::State<'_, Tangle>,
    args: IFarmLocationRelation,
) -> Result<IFarmLocationResolve, IError> {
    match lib_model_farm_location_set(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_farm_location_unset(
    state: tauri::State<'_, Tangle>,
    args: IFarmLocationRelation,
) -> Result<IFarmLocationResolve, IError> {
    match lib_model_farm_location_unset(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}