use crate::{
    error::ModelError,
    types::{IModelsId, IModelsQueryBindValue, IModelsQueryBindValueTuple, IModelsResults},
    utils::{time_created_on, uuidv4},
};
use futures::TryStreamExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct MediaUpload {
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
pub struct IMediaUploadFields {
    pub file_path: String,
    pub mime_type: String,
    pub res_base: String,
    pub res_path: String,
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaUploadFieldsUpdate {
    pub file_path: Option<String>,
    pub mime_type: Option<String>,
    pub res_base: Option<String>,
    pub res_path: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaUploadSort {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaUploadQueryBindValues {
    Id(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaUploadQueryListOf {
    All(IModelsQueryBindValue),
    OnTradeProduct(IModelsQueryBindValue),
    OffTradeProduct(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaUploadQueryGetList {
    pub of: MediaUploadQueryListOf,
    pub sort: Option<MediaUploadSort>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaUploadQueryGet {
    pub on: Option<MediaUploadQueryBindValues>,
    pub list: Option<IMediaUploadQueryGetList>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IMediaUploadQueryUpdate {
    pub on: MediaUploadQueryBindValues,
    pub fields: IMediaUploadFieldsUpdate,
}

pub type IMediaUploadAdd = IMediaUploadFields;
pub type IMediaUploadAddResolve = IModelsId;
pub type IMediaUploadGet = IMediaUploadQueryGet;
pub type IMediaUploadGetResolve = IModelsResults<MediaUpload>;
pub type IMediaUploadDelete = MediaUploadQueryBindValues;
pub type IMediaUploadDeleteResolve = ();
pub type IMediaUploadUpdate = IMediaUploadQueryUpdate;
pub type IMediaUploadUpdateResolve = ();

pub fn media_upload_query_bind_values(opts: MediaUploadQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        MediaUploadQueryBindValues::Id(id) => ("id".to_string(), id),
    }
}

pub fn media_upload_query_get_list(opts: IMediaUploadQueryGetList) -> IModelsQueryBindValueTuple {
    let query_sort = match opts.sort {
        Some(MediaUploadSort::Newest) => " ORDER BY mu.created_at DESC",
        Some(MediaUploadSort::Oldest) => " ORDER BY mu.created_at ASC",
        None => "",
    };
    match opts.of {
        MediaUploadQueryListOf::All(_) => (format!("SELECT mu.* FROM media_upload mu{}", query_sort), "".to_string()),
        MediaUploadQueryListOf::OnTradeProduct(id) => (format!("SELECT mu.* FROM media_upload mu JOIN trade_product_media tp_lg ON mu.id = tp_lg.tb_mu WHERE tp_lg.tb_tp = ?1{}", query_sort), id),
        MediaUploadQueryListOf::OffTradeProduct(id) => (format!("SELECT mu.* FROM media_upload mu WHERE NOT EXISTS (SELECT 1 FROM trade_product_media tp_lg WHERE tp_lg.tb_mu = mu.id AND tp_lg.tb_tp = ?1){}", query_sort), id),
    }
}

fn media_upload_fields_bind_values(
    opts: IMediaUploadFields,
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

fn media_upload_fields_update_bind_values(
    opts: IMediaUploadFieldsUpdate,
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

pub async fn lib_model_media_upload_add(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IMediaUploadAdd,
) -> Result<IMediaUploadAddResolve, ModelError> {
    let id = uuidv4();
    let created_at = time_created_on();
    let updated_at = created_at.clone();
    let bind_values = media_upload_fields_bind_values(opts)
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
        "INSERT INTO media_upload ({}) VALUES ({});",
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

fn media_upload_query_get(
    opts: IMediaUploadGet,
) -> Result<(String, Vec<IModelsQueryBindValue>), ModelError> {
    match opts {
        IMediaUploadQueryGet {
            list: Some(opts_list),
            ..
        } => {
            let (query, bv) = media_upload_query_get_list(opts_list);
            Ok((query, vec![bv]))
        }
        IMediaUploadQueryGet {
            on: Some(opts_on), ..
        } => {
            let (bv_k, bv) = media_upload_query_bind_values(opts_on);
            let query = format!("SELECT * FROM media_upload WHERE {} = ?1;", bv_k);
            Ok((query, vec![bv]))
        }
        _ => Err(ModelError::InvalidQuery(
            "model.media_upload.error.query_invalid".to_string(),
        )),
    }
}

pub async fn lib_model_media_upload_get(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IMediaUploadQueryGet,
) -> Result<IMediaUploadGetResolve, ModelError> {
    let (query, bind_values) = media_upload_query_get(opts)?;
    let mut query_builder = sqlx::query_as::<_, MediaUpload>(&query);
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

pub async fn lib_model_media_upload_delete(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IMediaUploadDelete,
) -> Result<IMediaUploadDeleteResolve, ModelError> {
    let (bv_k, bv) = media_upload_query_bind_values(opts);
    let query = format!("DELETE FROM media_upload WHERE {} = ?1;", bv_k);
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

pub async fn lib_model_media_upload_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: IMediaUploadUpdate,
) -> Result<IMediaUploadUpdateResolve, ModelError> {
    let (bv_k, bv) = media_upload_query_bind_values(opts.on);
    let bind_values = media_upload_fields_update_bind_values(opts.fields)
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
        "UPDATE media_upload SET {} WHERE {} = ?1;",
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
