use crate::{
    error::ModelError,
    types::IModelsResults,
    models::trade_product::{trade_product_query_bind_values, TradeProductQueryBindValues},
    models::media_upload::{media_upload_query_bind_values, MediaUploadQueryBindValues},
};
use futures::TryStreamExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TradeProductMedia {
    tb_tp: String,
    tb_mu: String,
}

pub type ITradeProductMediaRelationResolve = bool;
pub type ITradeProductMediaRelationResolveGetAll = IModelsResults<TradeProductMedia>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductMediaRelation {
    pub trade_product: TradeProductQueryBindValues,
    pub media_upload: MediaUploadQueryBindValues,
}

pub async fn lib_model_trade_product_media_set(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaRelationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_mu_k, bv_mu) = media_upload_query_bind_values(opts.media_upload);
    let query_vals = vec![bv_tp, bv_mu];
    let query = format!("INSERT INTO trade_product_media (tb_tp, tb_mu) VALUES ((SELECT id FROM trade_product WHERE {} = ?1), (SELECT id FROM media_upload WHERE {} = ?2));", bv_tp_k, bv_mu_k);
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

pub async fn lib_model_trade_product_media_unset(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductMediaRelation,
) -> Result<ITradeProductMediaRelationResolve, ModelError> {
    let (bv_tp_k, bv_tp) = trade_product_query_bind_values(opts.trade_product);
    let (bv_mu_k, bv_mu) = media_upload_query_bind_values(opts.media_upload);
    let query_vals = vec![bv_tp, bv_mu];
    let query = format!("DELETE FROM trade_product_media WHERE tb_tp = (SELECT id FROM trade_product WHERE {} = ?1) AND tb_mu = (SELECT id FROM media_upload WHERE {} = ?2);", bv_tp_k, bv_mu_k);
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

pub async fn lib_model_trade_product_media_get_all(
    db: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<ITradeProductMediaRelationResolveGetAll, ModelError> {
    let query = format!("SELECT * FROM trade_product_media;");
    let query_builder = sqlx::query_as::<_, TradeProductMedia>(&query);
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IModelsResults { results })
}
