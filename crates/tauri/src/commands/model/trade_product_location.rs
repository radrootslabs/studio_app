use crate::radroots::Radroots;
use radroots_core::types::IError;
use radroots_model::tables::trade_product_location::{
    lib_model_trade_product_location_set, lib_model_trade_product_location_unset,
    ITradeProductLocationRelation, ITradeProductLocationResolve,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_location_set(
    state: tauri::State<'_, Radroots>,
    args: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationResolve, IError> {
    match lib_model_trade_product_location_set(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_location_unset(
    state: tauri::State<'_, Radroots>,
    args: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationResolve, IError> {
    match lib_model_trade_product_location_unset(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}