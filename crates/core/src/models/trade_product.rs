use crate::{
    error::ModelError,
    types::{IModelsId, IModelsQueryBindValue, IModelsQueryBindValueTuple, IModelsResults},
    utils::{time_created_on, uuidv4},
};
use futures::TryStreamExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TradeProduct {
    id: String,
    created_at: String,
    updated_at: String,
    key: String,
    category: String,
    title: String,
    summary: String,
    process: String,
    lot: String,
    profile: String,
    year: i32,
    qty_amt: i32,
    qty_unit: String,
    qty_label: Option<String>,
    qty_avail: Option<i32>,
    price_amt: f64,
    price_currency: String,
    price_qty_amt: i32,
    price_qty_unit: String,
    notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductFields {
    pub key: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub process: String,
    pub lot: String,
    pub profile: String,
    pub year: String,
    pub qty_amt: String,
    pub qty_unit: String,
    pub qty_label: Option<String>,
    pub qty_avail: Option<String>,
    pub price_amt: String,
    pub price_currency: String,
    pub price_qty_amt: String,
    pub price_qty_unit: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductFieldsUpdate {
    pub key: Option<String>,
    pub category: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub process: Option<String>,
    pub lot: Option<String>,
    pub profile: Option<String>,
    pub year: Option<String>,
    pub qty_amt: Option<String>,
    pub qty_unit: Option<String>,
    pub qty_label: Option<String>,
    pub qty_avail: Option<String>,
    pub price_amt: Option<String>,
    pub price_currency: Option<String>,
    pub price_qty_amt: Option<String>,
    pub price_qty_unit: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeProductSort {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeProductQueryBindValues {
    Id(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeProductQueryListOf {
    All(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductQueryGetList {
    pub of: TradeProductQueryListOf,
    pub sort: Option<TradeProductSort>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductQueryGet {
    pub on: Option<TradeProductQueryBindValues>,
    pub list: Option<ITradeProductQueryGetList>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductQueryUpdate {
    pub on: TradeProductQueryBindValues,
    pub fields: ITradeProductFieldsUpdate,
}

pub type ITradeProductAdd = ITradeProductFields;
pub type ITradeProductAddResolve = IModelsId;
pub type ITradeProductGet = ITradeProductQueryGet;
pub type ITradeProductGetResolve = IModelsResults<TradeProduct>;
pub type ITradeProductDelete = TradeProductQueryBindValues;
pub type ITradeProductDeleteResolve = ();
pub type ITradeProductUpdate = ITradeProductQueryUpdate;
pub type ITradeProductUpdateResolve = ();

pub fn trade_product_query_bind_values(opts: TradeProductQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        TradeProductQueryBindValues::Id(id) => ("id".to_string(), id),
    }
}

pub fn trade_product_query_get_list(opts: ITradeProductQueryGetList) -> IModelsQueryBindValueTuple {
    let query_sort = match opts.sort {
        Some(TradeProductSort::Newest) => " ORDER BY tp.created_at DESC",
        Some(TradeProductSort::Oldest) => " ORDER BY tp.created_at ASC",
        None => "",
    };
    match opts.of {
        TradeProductQueryListOf::All(_) => (format!("SELECT tp.* FROM trade_product tp{}", query_sort), "".to_string()),
    }
}

fn trade_product_fields_bind_values(
    opts: ITradeProductFields,
) -> Result<Vec<IModelsQueryBindValueTuple>, ModelError> {
    let bind_values = serde_json::to_value(&opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::InvalidArgument("model.error.object_invalid".to_string()))?
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|v| (key.clone(), v.to_string())))
        .collect::<Vec<_>>();
    Ok(bind_values)
}

fn trade_product_fields_update_bind_values(
    opts: ITradeProductFieldsUpdate,
) -> Result<Vec<IModelsQueryBindValueTuple>, ModelError> {
    let bind_values = serde_json::to_value(&opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::InvalidArgument("model.error.object_invalid".to_string()))?
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|v| (key.clone(), v.to_string())))
        .collect::<Vec<_>>();
    Ok(bind_values)
}

pub async fn lib_model_trade_product_add(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductAdd,
) -> Result<ITradeProductAddResolve, ModelError> {
    let id = uuidv4();
    let created_at = time_created_on();
    let updated_at = created_at.clone();
    let bind_values = trade_product_fields_bind_values(opts)
        .map_err(|e| ModelError::InvalidArgument(e.to_string()))?;
    let mut query_col = vec![
        "id".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
    ];
    let mut query_pl = vec!["?1".to_string(), "?2".to_string(), "?3".to_string()];
    let mut query_vals: Vec<String> = vec![id.to_string(), created_at.clone(), updated_at.clone()];
    for (k, v) in bind_values.iter() {
        query_col.push(k.clone());
        query_pl.push(format!("?{}", query_col.len()));
        query_vals.push(v.clone());
    }
    let query = format!(
        "INSERT INTO trade_product ({}) VALUES ({});",
        query_col.join(", "),
        query_pl.join(", ")
    );
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IModelsId { id })
}

fn trade_product_query_get(
    opts: ITradeProductGet,
) -> Result<(String, Vec<IModelsQueryBindValue>), ModelError> {
    match opts {
        ITradeProductQueryGet {
            list: Some(opts_list),
            ..
        } => {
            let (query, bv) = trade_product_query_get_list(opts_list);
            Ok((query, vec![bv]))
        }
        ITradeProductQueryGet {
            on: Some(opts_on), ..
        } => {
            let (bv_k, bv) = trade_product_query_bind_values(opts_on);
            let query = format!("SELECT * FROM trade_product WHERE {} = ?1;", bv_k);
            Ok((query, vec![bv]))
        }
        _ => Err(ModelError::InvalidQuery(
            "model.trade_product.error.query_invalid".to_string(),
        )),
    }
}

pub async fn lib_model_trade_product_get(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductQueryGet,
) -> Result<ITradeProductGetResolve, ModelError> {
    let (query, bind_values) = trade_product_query_get(opts)?;
    let mut query_builder = sqlx::query_as::<_, TradeProduct>(&query);
    for value in bind_values.iter() {
        query_builder = query_builder.bind(value);
    }
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IModelsResults { results })
}

pub async fn lib_model_trade_product_delete(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductDelete,
) -> Result<ITradeProductDeleteResolve, ModelError> {
    let (bv_k, bv) = trade_product_query_bind_values(opts);
    let query = format!("DELETE FROM trade_product WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    println!("{:?}", result);
    if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(ModelError::InvalidQuery(
            "models.error.model_not_found".to_string(),
        ))
    }
}

pub async fn lib_model_trade_product_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductUpdate,
) -> Result<ITradeProductUpdateResolve, ModelError> {
    let (bv_k, bv) = trade_product_query_bind_values(opts.on);
    let bind_values = trade_product_fields_update_bind_values(opts.fields)
        .map_err(|e| ModelError::InvalidArgument(e.to_string()))?;
    let updated_at = time_created_on();
    let mut query_col = vec!["updated_at".to_string()];
    let mut query_pl = vec!["?2".to_string()];
    let mut query_vals = vec![bv, updated_at];
    for (k, v) in bind_values.iter() {
        query_col.push(k.clone());
        query_pl.push(format!("?{}", query_col.len() + 1));
        query_vals.push(v.clone());
    }
    let query = format!(
        "UPDATE trade_product SET {} WHERE {} = ?1;",
        query_col
            .iter()
            .enumerate()
            .map(|(i, col)| format!("{} = {}", col, query_pl[i]))
            .collect::<Vec<_>>()
            .join(", "),
        bv_k
    );
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    let result = query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    println!("{:?}", result);
    if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(ModelError::InvalidQuery(
            "models.error.model_not_found".to_string(),
        ))
    }
}
