use crate::radroots::Radroots;
use radroots_core::{
    models::trade_product_media::{lib_model_trade_product_media_set, lib_model_trade_product_media_unset, lib_model_trade_product_media_get_all, ITradeProductMediaRelation, ITradeProductMediaRelationResolve, ITradeProductMediaRelationResolveGetAll},
};

#[tauri::command]
pub async fn model_trade_product_media_set(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaRelationResolve, String> {
    match lib_model_trade_product_media_set(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_media_unset(
    state: tauri::State<'_, Radroots>,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaRelationResolve, String> {
    match lib_model_trade_product_media_unset(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_trade_product_media_get_all(
    state: tauri::State<'_, Radroots>,
) -> Result<ITradeProductMediaRelationResolveGetAll, String> {
    match lib_model_trade_product_media_get_all(&state.db).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
