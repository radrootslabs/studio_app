use futures::TryStreamExt;
use tangle_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct LogError {
    id: String,
    created_at: String,
    updated_at: String,
    error: String,
    message: String,
    stack_trace: Option<String>,
    cause: Option<String>,
    app_system: String,
    app_version: String,
    nostr_pubkey: String,
    data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILogErrorFields {
    pub error: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub cause: Option<String>,
    pub app_system: String,
    pub app_version: String,
    pub nostr_pubkey: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILogErrorFieldsPartial {
    pub error: Option<String>,
    pub message: Option<String>,
    pub stack_trace: Option<String>,
    pub cause: Option<String>,
    pub app_system: Option<String>,
    pub app_version: Option<String>,
    pub nostr_pubkey: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILogErrorFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
    pub stack_trace: Option<String>,
    pub cause: Option<String>,
    pub app_system: Option<String>,
    pub app_version: Option<String>,
    pub nostr_pubkey: Option<String>,
    pub data: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum LogErrorQueryBindValues {
    Id { id: String },
    NostrPubkey { nostr_pubkey: String },
}

pub fn log_error_query_bind_values(opts: LogErrorQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        LogErrorQueryBindValues::Id { id } => ("id".to_string(), id),
        LogErrorQueryBindValues::NostrPubkey { nostr_pubkey } => ("nostr_pubkey".to_string(), nostr_pubkey),
    }
}

fn lib_table_log_error_parse_fields(
    opts: ILogErrorFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type ILogErrorCreate = ILogErrorFields;
pub type ILogErrorCreateResolve = IResult<String>;

pub async fn lib_model_log_error_create(
    db: &DatabaseConnection,
    opts: ILogErrorCreate,
) -> Result<ILogErrorCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_log_error_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO log_error (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct ILogErrorQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ILogErrorRead = ILogErrorQueryRead;
pub type ILogErrorReadResolve = IResultList<LogError>;

pub async fn lib_model_log_error_read(
    db: &DatabaseConnection,
    opts: ILogErrorRead,
) -> Result<ILogErrorReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, LogError>(&opts.query);
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
pub struct ILogErrorQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ILogErrorReadList = ILogErrorQueryReadList;
pub type ILogErrorReadListResolve = IResultList<LogError>;

pub async fn lib_model_log_error_read_list(
    db: &DatabaseConnection,
    opts: ILogErrorReadList,
) -> Result<ILogErrorReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, LogError>(&opts.query);
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
pub struct ILogErrorQueryUpdate {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ILogErrorUpdate = ILogErrorQueryUpdate;
pub type ILogErrorUpdateResolve = IResultPass;

pub async fn lib_model_log_error_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ILogErrorUpdate,
) -> Result<ILogErrorUpdateResolve, ModelError> {
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

pub type ILogErrorDelete = LogErrorQueryBindValues;
pub type ILogErrorDeleteResolve = IResultPass;

pub async fn lib_model_log_error_delete(
    db: &DatabaseConnection,
    opts: ILogErrorDelete,
) -> Result<ILogErrorDeleteResolve, ModelError> {
    let (bv_k, bv) = log_error_query_bind_values(opts);
    let query = format!("DELETE FROM log_error WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.log_error.name".to_string()))
    }
}
