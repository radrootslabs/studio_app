use crate::app::Tangle;
use tangle_core::types::IError;
use tangle_model::{
    tables::trade_product::{lib_model_trade_product_create, ITradeProductCreate, ITradeProductCreateResolve, lib_model_trade_product_read, ITradeProductRead, ITradeProductReadResolve, lib_model_trade_product_read_list, ITradeProductReadList, ITradeProductReadListResolve, lib_model_trade_product_delete, ITradeProductDelete, ITradeProductDeleteResolve, lib_model_trade_product_update, ITradeProductUpdate, ITradeProductUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_create(
    state: tauri::State<'_, Tangle>,
    args: ITradeProductCreate,
) -> Result<ITradeProductCreateResolve, IError> {
    match lib_model_trade_product_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_read(
    state: tauri::State<'_, Tangle>,
    args: ITradeProductRead,
) -> Result<ITradeProductReadResolve, IError> {
    match lib_model_trade_product_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_read_list(
    state: tauri::State<'_, Tangle>,
    args: ITradeProductReadList,
) -> Result<ITradeProductReadListResolve, IError> {
    match lib_model_trade_product_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_update(
    state: tauri::State<'_, Tangle>,
    args: ITradeProductUpdate,
) -> Result<ITradeProductUpdateResolve, IError> {
    match lib_model_trade_product_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_trade_product_delete(
    state: tauri::State<'_, Tangle>,
    args: ITradeProductDelete,
) -> Result<ITradeProductDeleteResolve, IError> {
    match lib_model_trade_product_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}