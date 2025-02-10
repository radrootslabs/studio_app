use futures::TryStreamExt;
use radroots_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct LocationGcs {
    id: String,
    created_at: String,
    updated_at: String,
    lat: f64,
    lng: f64,
    geohash: String,
    kind: String,
    label: Option<String>,
    area: Option<f64>,
    elevation: Option<u32>,
    soil: Option<String>,
    climate: Option<String>,
    gc_id: Option<String>,
    gc_name: Option<String>,
    gc_admin1_id: Option<String>,
    gc_admin1_name: Option<String>,
    gc_country_id: Option<String>,
    gc_country_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsFields {
    pub lat: f64,
    pub lng: f64,
    pub geohash: String,
    pub kind: String,
    pub label: Option<String>,
    pub area: Option<f64>,
    pub elevation: Option<u32>,
    pub soil: Option<String>,
    pub climate: Option<String>,
    pub gc_id: Option<String>,
    pub gc_name: Option<String>,
    pub gc_admin1_id: Option<String>,
    pub gc_admin1_name: Option<String>,
    pub gc_country_id: Option<String>,
    pub gc_country_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsFieldsPartial {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub geohash: Option<String>,
    pub kind: Option<String>,
    pub label: Option<String>,
    pub area: Option<f64>,
    pub elevation: Option<u32>,
    pub soil: Option<String>,
    pub climate: Option<String>,
    pub gc_id: Option<String>,
    pub gc_name: Option<String>,
    pub gc_admin1_id: Option<String>,
    pub gc_admin1_name: Option<String>,
    pub gc_country_id: Option<String>,
    pub gc_country_name: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub lat: Option<String>,
    pub lng: Option<String>,
    pub geohash: Option<String>,
    pub kind: Option<String>,
    pub label: Option<String>,
    pub area: Option<String>,
    pub elevation: Option<String>,
    pub soil: Option<String>,
    pub climate: Option<String>,
    pub gc_id: Option<String>,
    pub gc_name: Option<String>,
    pub gc_admin1_id: Option<String>,
    pub gc_admin1_name: Option<String>,
    pub gc_country_id: Option<String>,
    pub gc_country_name: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum LocationGcsQueryBindValues {
    Id { id: String },
    Geohash { geohash: String },
}

pub fn location_gcs_query_bind_values(opts: LocationGcsQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        LocationGcsQueryBindValues::Id { id } => ("id".to_string(), id),
        LocationGcsQueryBindValues::Geohash { geohash } => ("geohash".to_string(), geohash),
    }
}

fn lib_table_location_gcs_parse_fields(
    opts: ILocationGcsFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type ILocationGcsCreate = ILocationGcsFields;
pub type ILocationGcsCreateResolve = IResult<String>;

pub async fn lib_model_location_gcs_create(
    db: &DatabaseConnection,
    opts: ILocationGcsCreate,
) -> Result<ILocationGcsCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_location_gcs_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO location_gcs (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct ILocationGcsQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ILocationGcsRead = ILocationGcsQueryRead;
pub type ILocationGcsReadResolve = IResultList<LocationGcs>;

pub async fn lib_model_location_gcs_read(
    db: &DatabaseConnection,
    opts: ILocationGcsRead,
) -> Result<ILocationGcsReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, LocationGcs>(&opts.query);
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
pub struct ILocationGcsQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type ILocationGcsReadList = ILocationGcsQueryReadList;
pub type ILocationGcsReadListResolve = IResultList<LocationGcs>;

pub async fn lib_model_location_gcs_read_list(
    db: &DatabaseConnection,
    opts: ILocationGcsReadList,
) -> Result<ILocationGcsReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, LocationGcs>(&opts.query);
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

pub type ILocationGcsUpdate = LocationGcsQueryBindValues;
pub type ILocationGcsUpdateResolve = ();

pub async fn lib_model_location_gcs_update(
    _db: &sqlx::Pool<sqlx::Sqlite>,
    _opts: ILocationGcsUpdate,
) -> Result<ILocationGcsUpdateResolve, ModelError> {
    Ok(())
}
pub type ILocationGcsDelete = LocationGcsQueryBindValues;
pub type ILocationGcsDeleteResolve = IResultPass;

pub async fn lib_model_location_gcs_delete(
    db: &DatabaseConnection,
    opts: ILocationGcsDelete,
) -> Result<ILocationGcsDeleteResolve, ModelError> {
    let (bv_k, bv) = location_gcs_query_bind_values(opts);
    let query = format!("DELETE FROM location_gcs WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.location_gcs.name".to_string()))
    }
}
