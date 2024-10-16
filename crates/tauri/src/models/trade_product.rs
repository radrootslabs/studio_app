use crate::radroots::Radroots;
use radroots_core::{
    models::trade_product::{lib_model_trade_product_add, ITradeProductAdd, ITradeProductAddResolve, lib_model_trade_product_get, ITradeProductGet, ITradeProductGetResolve, lib_model_trade_product_delete, ITradeProductDelete, ITradeProductDeleteResolve, lib_model_trade_product_update, ITradeProductUpdate, ITradeProductUpdateResolve},
};

#[tauri::command]
pub async fn model_trade_product_add(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductAdd,
) -> Result<ITradeProductAddResolve, String> {
    match lib_model_trade_product_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_get(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductGet,
) -> Result<ITradeProductGetResolve, String> {
    match lib_model_trade_product_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_delete(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductDelete,
) -> Result<ITradeProductDeleteResolve, String> {
    match lib_model_trade_product_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_update(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductUpdate,
) -> Result<ITradeProductUpdateResolve, String> {
    match lib_model_trade_product_update(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
