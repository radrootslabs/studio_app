use super::{
    location_gcs::{location_gcs_query_bind_values, LocationGcsQueryBindValues},
    trade_product::{trade_product_query_bind_values, TradeProductQueryBindValues},
};
use crate::{error::ModelError, types::DatabaseConnection};
use radroots_core::types::IResultPass;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TradeProductLocation {
    tb_tp: String,
    tb_lg: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ITradeProductLocationTables {
    pub trade_product: TradeProductQueryBindValues,
    pub location_gcs: LocationGcsQueryBindValues,
}

pub type ITradeProductLocationRelation = ITradeProductLocationTables;
pub type ITradeProductLocationResolve = IResultPass;

pub async fn lib_model_trade_product_location_set(
    db: &DatabaseConnection,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_tp, bv_lg];
    let query = format!("INSERT INTO trade_product_location (tb_tp, tb_lg) VALUES ((SELECT id FROM trade_product WHERE {} = ?), (SELECT id FROM location_gcs WHERE {} = ?));", bv_tp_k, bv_lg_k);
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
pub async fn lib_model_trade_product_location_unset(
    db: &DatabaseConnection,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_tp, bv_lg];
    let query = format!("DELETE FROM trade_product_location WHERE tb_tp = (SELECT id FROM trade_product WHERE {} = ?) AND tb_lg = (SELECT id FROM location_gcs WHERE {} = ?);", bv_tp_k, bv_lg_k);
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
