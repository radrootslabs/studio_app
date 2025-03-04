use futures::TryStreamExt;
use radroots_core::{
    types::{IModelsQueryBindValueTuple, IResult, IResultList, IResultPass},
    util::{time_created_on, uuidv4},
};

use crate::{error::ModelError, types::DatabaseConnection, util::parse_query_value};

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct MediaImage {
    id: String,
    created_at: String,
    updated_at: String,
    file_path: String,
    mime_type: String,
    res_base: String,
    res_path: String,
    label: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaImageFields {
    pub file_path: String,
    pub mime_type: String,
    pub res_base: String,
    pub res_path: String,
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaImageFieldsPartial {
    pub file_path: Option<String>,
    pub mime_type: Option<String>,
    pub res_base: Option<String>,
    pub res_path: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaImageFieldsFilter {
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub file_path: Option<String>,
    pub mime_type: Option<String>,
    pub res_base: Option<String>,
    pub res_path: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MediaImageQueryBindValues {
    Id { id: String },
    FilePath { file_path: String },
}

pub fn media_image_query_bind_values(opts: MediaImageQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        MediaImageQueryBindValues::Id { id } => ("id".to_string(), id),
        MediaImageQueryBindValues::FilePath { file_path } => ("file_path".to_string(), file_path),
    }
}

fn lib_table_media_image_parse_fields(
    opts: IMediaImageFields,
) -> Result<serde_json::Map<String, serde_json::Value>, ModelError> {
    let fields = serde_json::to_value(opts)
        .map_err(|err| ModelError::SerializationError(err.to_string()))?
        .as_object()
        .ok_or_else(|| ModelError::SerializationError("Expected an object".to_string()))?
        .clone();
    Ok(fields)
}

pub type IMediaImageCreate = IMediaImageFields;
pub type IMediaImageCreateResolve = IResult<String>;

pub async fn lib_model_media_image_create(
    db: &DatabaseConnection,
    opts: IMediaImageCreate,
) -> Result<IMediaImageCreateResolve, ModelError> {
    let id: String = uuidv4();
    let created_at: String = time_created_on();
    let updated_at: String = created_at.clone();
    let fields = lib_table_media_image_parse_fields(opts)?;

    let query = format!(
        "INSERT INTO media_image (id, created_at, updated_at, {}) VALUES (?, ?, ?, {});",
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
pub struct IMediaImageQueryRead {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IMediaImageRead = IMediaImageQueryRead;
pub type IMediaImageReadResolve = IResultList<MediaImage>;

pub async fn lib_model_media_image_read(
    db: &DatabaseConnection,
    opts: IMediaImageRead,
) -> Result<IMediaImageReadResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, MediaImage>(&opts.query);
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
pub struct IMediaImageQueryReadList {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IMediaImageReadList = IMediaImageQueryReadList;
pub type IMediaImageReadListResolve = IResultList<MediaImage>;

pub async fn lib_model_media_image_read_list(
    db: &DatabaseConnection,
    opts: IMediaImageReadList,
) -> Result<IMediaImageReadListResolve, ModelError> {
    let mut query_builder = sqlx::query_as::<_, MediaImage>(&opts.query);
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
pub struct IMediaImageQueryUpdate {
    pub query: String,
    pub bind_values: Vec<serde_json::Value>,
}

pub type IMediaImageUpdate = IMediaImageQueryUpdate;
pub type IMediaImageUpdateResolve = ();

pub async fn lib_model_media_image_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IMediaImageUpdate,
) -> Result<IMediaImageUpdateResolve, ModelError> {
    let mut query_builder = sqlx::query(&opts.query);
    for value in opts.bind_values.iter() {
        query_builder = query_builder.bind(parse_query_value(value)?);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(())
}

pub type IMediaImageDelete = MediaImageQueryBindValues;
pub type IMediaImageDeleteResolve = IResultPass;

pub async fn lib_model_media_image_delete(
    db: &DatabaseConnection,
    opts: IMediaImageDelete,
) -> Result<IMediaImageDeleteResolve, ModelError> {
    let (bv_k, bv) = media_image_query_bind_values(opts);
    let query = format!("DELETE FROM media_image WHERE {} = ?1;", bv_k);
    let result = sqlx::query(&query)
        .bind(bv)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    if result.rows_affected() > 0 {
        Ok(IResultPass { pass: true })
    } else {
        Err(ModelError::NotFound("model.media_image.name".to_string()))
    }
}
