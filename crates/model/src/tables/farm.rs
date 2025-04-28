use futures::TryStreamExt;
use tangle_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Farm {
    id: String,
    created_at: String,
    updated_at: String,
    name: String,
    area: Option<String>,
    area_unit: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IFarmFields {
    pub name: String,
    pub area: Option<String>,
    pub area_unit: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IFarmFieldsPartial {
    pub name: Option<String>,
    pub area: Option<String>,
    pub area_unit: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IFarmFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub name: Option<String>,
    pub area: Option<String>,
    pub area_unit: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum FarmQueryBindValues {
    Id { id: String },
}

pub fn farm_query_bind_values(opts: FarmQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        FarmQueryBindValues::Id { id } => ("id".to_string(), id),
    }
}

fn lib_table_farm_parse_fields(
    opts: IFarmFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type IFarmCreate = IFarmFields;
pub type IFarmCreateResolve = IResult<String>;

pub async fn lib_model_farm_create(
    db: &DatabaseConnection,
    opts: IFarmCreate,
) -> Result<IFarmCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_farm_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO farm (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct IFarmQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IFarmRead = IFarmQueryRead;
pub type IFarmReadResolve = IResultList<Farm>;

pub async fn lib_model_farm_read(
    db: &DatabaseConnection,
    opts: IFarmRead,
) -> Result<IFarmReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, Farm>(&opts.query);
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
pub struct IFarmQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IFarmReadList = IFarmQueryReadList;
pub type IFarmReadListResolve = IResultList<Farm>;

pub async fn lib_model_farm_read_list(
    db: &DatabaseConnection,
    opts: IFarmReadList,
) -> Result<IFarmReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, Farm>(&opts.query);
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
pub struct IFarmQueryUpdate {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IFarmUpdate = IFarmQueryUpdate;
pub type IFarmUpdateResolve = IResultPass;

pub async fn lib_model_farm_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IFarmUpdate,
) -> Result<IFarmUpdateResolve, ModelError> {
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

pub type IFarmDelete = FarmQueryBindValues;
pub type IFarmDeleteResolve = IResultPass;

pub async fn lib_model_farm_delete(
    db: &DatabaseConnection,
    opts: IFarmDelete,
) -> Result<IFarmDeleteResolve, ModelError> {
    let (bv_k, bv) = farm_query_bind_values(opts);
    let query = format!("DELETE FROM farm WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.farm.name".to_string()))
    }
}
