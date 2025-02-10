use crate::radroots::Radroots;
use radroots_core::types::IError;
use radroots_model::tables::trade_product_media::{
    lib_model_trade_product_media_set, lib_model_trade_product_media_unset,
    ITradeProductMediaRelation, ITradeProductMediaResolve,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_media_set(
    state: tauri::State<'_, Radroots>,
    args: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaResolve, IError> {
    match lib_model_trade_product_media_set(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_media_unset(
    state: tauri::State<'_, Radroots>,
    args: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaResolve, IError> {
    match lib_model_trade_product_media_unset(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}