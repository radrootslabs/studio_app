use super::{
    media_image::{media_image_query_bind_values, MediaImageQueryBindValues},
    trade_product::{trade_product_query_bind_values, TradeProductQueryBindValues},
};
use crate::{error::ModelError, types::DatabaseConnection};
use radroots_core::types::IResultPass;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TradeProductMedia {
    tb_tp: String,
    tb_mu: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ITradeProductMediaTables {
    pub trade_product: TradeProductQueryBindValues,
    pub media_image: MediaImageQueryBindValues,
}

pub type ITradeProductMediaRelation = ITradeProductMediaTables;
pub type ITradeProductMediaResolve = IResultPass;

pub async fn lib_model_trade_product_media_set(
    db: &DatabaseConnection,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_mu_k, bv_mu) = media_image_query_bind_values(opts.media_image);
    let query_vals = vec![bv_tp, bv_mu];
    let query = format!("INSERT INTO trade_product_media (tb_tp, tb_mu) VALUES ((SELECT id FROM trade_product WHERE {} = ?), (SELECT id FROM media_image WHERE {} = ?));", bv_tp_k, bv_mu_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
pub async fn lib_model_trade_product_media_unset(
    db: &DatabaseConnection,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_mu_k, bv_mu) = media_image_query_bind_values(opts.media_image);
    let query_vals = vec![bv_tp, bv_mu];
    let query = format!("DELETE FROM trade_product_media WHERE tb_tp = (SELECT id FROM trade_product WHERE {} = ?) AND tb_mu = (SELECT id FROM media_image WHERE {} = ?);", bv_tp_k, bv_mu_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
