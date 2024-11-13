use crate::{
    error::ModelError,
    models::trade_product::{trade_product_query_bind_values, TradeProductQueryBindValues},
    models::location_gcs::{location_gcs_query_bind_values, LocationGcsQueryBindValues},
};

pub type ITradeProductLocationRelationResolve = bool;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductLocationRelation {
    pub trade_product: TradeProductQueryBindValues,
    pub location_gcs: LocationGcsQueryBindValues,
}

pub async fn lib_model_trade_product_location_set(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationRelationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_tp, bv_lg];
    let query = format!("INSERT INTO trade_product_location (tb_tp_lg_0, tb_tp_lg_1) VALUES ((SELECT id FROM trade_product WHERE {} = ?1), (SELECT id FROM location_gcs WHERE {} = ?2));", bv_tp_k, bv_lg_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(true)
}

pub async fn lib_model_trade_product_location_unset(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductLocationRelation,
) -> Result<ITradeProductLocationRelationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_lg_k, bv_lg) = location_gcs_query_bind_values(opts.location_gcs);
    let query_vals = vec![bv_tp, bv_lg];
    let query = format!("DELETE FROM trade_product_location WHERE tb_tp_lg_0 = (SELECT id FROM nostr_profile WHERE {} = ?1) AND tb_tp_lg_1 = (SELECT id FROM nostr_relay WHERE {} = ?2);", bv_tp_k, bv_lg_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(true)
}
