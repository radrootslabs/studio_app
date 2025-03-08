use futures::TryStreamExt;
use radroots_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

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
    year: i64,
    qty_amt: i64,
    qty_unit: String,
    qty_label: Option<String>,
    qty_avail: Option<i64>,
    price_amt: f64,
    price_currency: String,
    price_qty_amt: u32,
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
    pub year: i64,
    pub qty_amt: i64,
    pub qty_unit: String,
    pub qty_label: Option<String>,
    pub qty_avail: Option<i64>,
    pub price_amt: f64,
    pub price_currency: String,
    pub price_qty_amt: u32,
    pub price_qty_unit: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductFieldsPartial {
    pub key: Option<String>,
    pub category: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub process: Option<String>,
    pub lot: Option<String>,
    pub profile: Option<String>,
    pub year: Option<i64>,
    pub qty_amt: Option<i64>,
    pub qty_unit: Option<String>,
    pub qty_label: Option<String>,
    pub qty_avail: Option<i64>,
    pub price_amt: Option<f64>,
    pub price_currency: Option<String>,
    pub price_qty_amt: Option<u32>,
    pub price_qty_unit: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ITradeProductFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum TradeProductQueryBindValues {
    Id { id: String },
}

pub fn trade_product_query_bind_values(opts: TradeProductQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        TradeProductQueryBindValues::Id { id } => ("id".to_string(), id),
    }
}

fn lib_table_trade_product_parse_fields(
    opts: ITradeProductFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type ITradeProductCreate = ITradeProductFields;
pub type ITradeProductCreateResolve = IResult<String>;

pub async fn lib_model_trade_product_create(
    db: &DatabaseConnection,
    opts: ITradeProductCreate,
) -> Result<ITradeProductCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_trade_product_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO trade_product (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
        fields
            .keys()
            .map(|k| k.to_string())
            .collect::<Vec<String>>()
            .join(","),
        (0..fields.len())
            .map(|_| "?")
            .collect::<Vec<&str>>()
            .join(",")
    );
    
    let mut query_builder = sqlx::query(&query);
    query_builder = query_builder.bind(&id);
    query_builder = query_builder.bind(&created_at);
    query_builder = query_builder.bind(&updated_at);
    for (_, value) in fields.iter() {
        match value {
            serde_json::Value::Bool(b) => {
                let bool_str = if *b { "1" } else { "0" };
                query_builder = query_builder.bind(bool_str);
            }
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    query_builder = query_builder.bind(f);
                } else if let Some(i64) = n.as_i64() {
                    query_builder = query_builder.bind(i64);
                } else if let Some(u64) = n.as_u64() {
                    if u64 <= u32::MAX as u64 {
                        query_builder = query_builder.bind(u64 as u32);
                    }
                }
            }
            serde_json::Value::String(s) => {
                query_builder = query_builder.bind(s);
            }
            _ => {
                query_builder = query_builder.bind::<Option<serde_json::Value>>(None);
            }
        }
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResult { result: id })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ITradeProductQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ITradeProductRead = ITradeProductQueryRead;
pub type ITradeProductReadResolve = IResultList<TradeProduct>;

pub async fn lib_model_trade_product_read(
    db: &DatabaseConnection,
    opts: ITradeProductRead,
) -> Result<ITradeProductReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, TradeProduct>(&opts.query);
    for value in opts.bind_values.iter() {
        query_builder = query_builder.bind(parse_query_value(value)?);
    }
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultList { results })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ITradeProductQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ITradeProductReadList = ITradeProductQueryReadList;
pub type ITradeProductReadListResolve = IResultList<TradeProduct>;

pub async fn lib_model_trade_product_read_list(
    db: &DatabaseConnection,
    opts: ITradeProductReadList,
) -> Result<ITradeProductReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, TradeProduct>(&opts.query);
    for value in opts.bind_values.iter() {
        query_builder = query_builder.bind(parse_query_value(value)?);
    }
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultList { results })
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ITradeProductQueryUpdate {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ITradeProductUpdate = ITradeProductQueryUpdate;
pub type ITradeProductUpdateResolve = IResultPass;

pub async fn lib_model_trade_product_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ITradeProductUpdate,
) -> Result<ITradeProductUpdateResolve, ModelError> {
    let mut query_builder = sqlx::query(&opts.query);
    for value in opts.bind_values.iter() {
        query_builder = query_builder.bind(parse_query_value(value)?);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}

pub type ITradeProductDelete = TradeProductQueryBindValues;
pub type ITradeProductDeleteResolve = IResultPass;

pub async fn lib_model_trade_product_delete(
    db: &DatabaseConnection,
    opts: ITradeProductDelete,
) -> Result<ITradeProductDeleteResolve, ModelError> {
    let (bv_k, bv) = trade_product_query_bind_values(opts);
    let query = format!("DELETE FROM trade_product WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.trade_product.name".to_string()))
    }
}
