use futures::TryStreamExt;
use tangle_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NostrProfile {
    id: String,
    created_at: String,
    updated_at: String,
    public_key: String,
    name: Option<String>,
    display_name: Option<String>,
    about: Option<String>,
    website: Option<String>,
    picture: Option<String>,
    banner: Option<String>,
    nip05: Option<String>,
    lud06: Option<String>,
    lud16: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileFields {
    pub public_key: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub website: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud06: Option<String>,
    pub lud16: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileFieldsPartial {
    pub public_key: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub website: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud06: Option<String>,
    pub lud16: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub public_key: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub about: Option<String>,
    pub website: Option<String>,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud06: Option<String>,
    pub lud16: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum NostrProfileQueryBindValues {
    Id { id: String },
    PublicKey { public_key: String },
}

pub fn nostr_profile_query_bind_values(opts: NostrProfileQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        NostrProfileQueryBindValues::Id { id } => ("id".to_string(), id),
        NostrProfileQueryBindValues::PublicKey { public_key } => ("public_key".to_string(), public_key),
    }
}

fn lib_table_nostr_profile_parse_fields(
    opts: INostrProfileFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type INostrProfileCreate = INostrProfileFields;
pub type INostrProfileCreateResolve = IResult<String>;

pub async fn lib_model_nostr_profile_create(
    db: &DatabaseConnection,
    opts: INostrProfileCreate,
) -> Result<INostrProfileCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_nostr_profile_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO nostr_profile (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct INostrProfileQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type INostrProfileRead = INostrProfileQueryRead;
pub type INostrProfileReadResolve = IResultList<NostrProfile>;

pub async fn lib_model_nostr_profile_read(
    db: &DatabaseConnection,
    opts: INostrProfileRead,
) -> Result<INostrProfileReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, NostrProfile>(&opts.query);
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
pub struct INostrProfileQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type INostrProfileReadList = INostrProfileQueryReadList;
pub type INostrProfileReadListResolve = IResultList<NostrProfile>;

pub async fn lib_model_nostr_profile_read_list(
    db: &DatabaseConnection,
    opts: INostrProfileReadList,
) -> Result<INostrProfileReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, NostrProfile>(&opts.query);
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
pub struct INostrProfileQueryUpdate {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type INostrProfileUpdate = INostrProfileQueryUpdate;
pub type INostrProfileUpdateResolve = IResultPass;

pub async fn lib_model_nostr_profile_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileUpdate,
) -> Result<INostrProfileUpdateResolve, ModelError> {
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

pub type INostrProfileDelete = NostrProfileQueryBindValues;
pub type INostrProfileDeleteResolve = IResultPass;

pub async fn lib_model_nostr_profile_delete(
    db: &DatabaseConnection,
    opts: INostrProfileDelete,
) -> Result<INostrProfileDeleteResolve, ModelError> {
    let (bv_k, bv) = nostr_profile_query_bind_values(opts);
    let query = format!("DELETE FROM nostr_profile WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.nostr_profile.name".to_string()))
    }
}
