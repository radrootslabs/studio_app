use crate::radroots::Radroots;
use radroots_core::{
    models::trade_product_location::{lib_model_trade_product_location_set, lib_model_trade_product_location_unset, lib_model_trade_product_location_get_all, ITradeProductLocationRelation, ITradeProductLocationRelationResolve, ITradeProductLocationRelationResolveGetAll},
};

#[tauri::command]
pub async fn model_trade_product_location_set(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationRelationResolve, String> {
    match lib_model_trade_product_location_set(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_location_unset(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationRelationResolve, String> {
    match lib_model_trade_product_location_unset(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_location_get_all(
    state: tauri::State<'_, Radroots>,
) -> Result<ITradeProductLocationRelationResolveGetAll, String> {
    match lib_model_trade_product_location_get_all(&state.db).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
