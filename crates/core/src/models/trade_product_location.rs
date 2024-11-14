use crate::{
    error::ModelError,
    types::IModelsResults,
    models::trade_product::{trade_product_query_bind_values, TradeProductQueryBindValues},
    models::location_gcs::{location_gcs_query_bind_values, LocationGcsQueryBindValues},
};
use futures::TryStreamExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TradeProductLocation {
    tb_tp: String,
    tb_lg: String,
}

pub type ITradeProductLocationRelationResolve = bool;
pub type ITradeProductLocationRelationResolveGetAll = IModelsResults<TradeProductLocation>;

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
    let query = format!("INSERT INTO trade_product_location (tb_tp, tb_lg) VALUES ((SELECT id FROM trade_product WHERE {} = ?1), (SELECT id FROM location_gcs WHERE {} = ?2));", bv_tp_k, bv_lg_k);
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
    let query = format!("DELETE FROM trade_product_location WHERE tb_tp = (SELECT id FROM trade_product WHERE {} = ?1) AND tb_lg = (SELECT id FROM location_gcs WHERE {} = ?2);", bv_tp_k, bv_lg_k);
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

pub async fn lib_model_trade_product_location_get_all(
    db: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<ITradeProductLocationRelationResolveGetAll, ModelError> {
    let query = format!("SELECT * FROM trade_product_location;");
    let query_builder = sqlx::query_as::<_, TradeProductLocation>(&query);
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IModelsResults { results })
}
