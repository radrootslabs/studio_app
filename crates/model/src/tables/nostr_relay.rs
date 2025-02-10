use futures::TryStreamExt;
use radroots_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NostrRelay {
    id: String,
    created_at: String,
    updated_at: String,
    url: String,
    relay_id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    pubkey: Option<String>,
    contact: Option<String>,
    supported_nips: Option<String>,
    software: Option<String>,
    version: Option<String>,
    data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayFields {
    pub url: String,
    pub relay_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub pubkey: Option<String>,
    pub contact: Option<String>,
    pub supported_nips: Option<String>,
    pub software: Option<String>,
    pub version: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayFieldsPartial {
    pub url: Option<String>,
    pub relay_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub pubkey: Option<String>,
    pub contact: Option<String>,
    pub supported_nips: Option<String>,
    pub software: Option<String>,
    pub version: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
    pub relay_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub pubkey: Option<String>,
    pub contact: Option<String>,
    pub supported_nips: Option<String>,
    pub software: Option<String>,
    pub version: Option<String>,
    pub data: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NostrRelayQueryBindValues {
    Id { id: String },
    Url { url: String },
}

pub fn nostr_relay_query_bind_values(opts: NostrRelayQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        NostrRelayQueryBindValues::Id { id } => ("id".to_string(), id),
        NostrRelayQueryBindValues::Url { url } => ("url".to_string(), url),
    }
}

fn lib_table_nostr_relay_parse_fields(
    opts: INostrRelayFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type INostrRelayCreate = INostrRelayFields;
pub type INostrRelayCreateResolve = IResult<String>;

pub async fn lib_model_nostr_relay_create(
    db: &DatabaseConnection,
    opts: INostrRelayCreate,
) -> Result<INostrRelayCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_nostr_relay_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO nostr_relay (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct INostrRelayQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type INostrRelayRead = INostrRelayQueryRead;
pub type INostrRelayReadResolve = IResultList<NostrRelay>;

pub async fn lib_model_nostr_relay_read(
    db: &DatabaseConnection,
    opts: INostrRelayRead,
) -> Result<INostrRelayReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, NostrRelay>(&opts.query);
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
pub struct INostrRelayQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type INostrRelayReadList = INostrRelayQueryReadList;
pub type INostrRelayReadListResolve = IResultList<NostrRelay>;

pub async fn lib_model_nostr_relay_read_list(
    db: &DatabaseConnection,
    opts: INostrRelayReadList,
) -> Result<INostrRelayReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, NostrRelay>(&opts.query);
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

pub type INostrRelayUpdate = NostrRelayQueryBindValues;
pub type INostrRelayUpdateResolve = ();

pub async fn lib_model_nostr_relay_update(
    _db: &sqlx::Pool<sqlx::Sqlite>,
    _opts: INostrRelayUpdate,
) -> Result<INostrRelayUpdateResolve, ModelError> {
    Ok(())
}
pub type INostrRelayDelete = NostrRelayQueryBindValues;
pub type INostrRelayDeleteResolve = IResultPass;

pub async fn lib_model_nostr_relay_delete(
    db: &DatabaseConnection,
    opts: INostrRelayDelete,
) -> Result<INostrRelayDeleteResolve, ModelError> {
    let (bv_k, bv) = nostr_relay_query_bind_values(opts);
    let query = format!("DELETE FROM nostr_relay WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.nostr_relay.name".to_string()))
    }
}
