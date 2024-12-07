use crate::{
    error::ModelError,
    types::{IModelsId, IModelsQueryBindValue, IModelsQueryBindValueTuple, IModelsResults},
    utils::{time_created_on, uuidv4},
};
use futures::TryStreamExt;

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
    elevation: Option<i32>,
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
    pub lat: String,
    pub lng: String,
    pub geohash: String,
    pub kind: String,
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsFieldsUpdate {
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationGcsSort {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationGcsQueryBindValues {
    Id(IModelsQueryBindValue),
    Geohash(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationGcsQueryListOf {
    All(IModelsQueryBindValue),
    OnTradeProduct(IModelsQueryBindValue),
    OffTradeProduct(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsQueryGetList {
    pub of: LocationGcsQueryListOf,
    pub sort: Option<LocationGcsSort>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsQueryGet {
    pub on: Option<LocationGcsQueryBindValues>,
    pub list: Option<ILocationGcsQueryGetList>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ILocationGcsQueryUpdate {
    pub on: LocationGcsQueryBindValues,
    pub fields: ILocationGcsFieldsUpdate,
}

pub type ILocationGcsAdd = ILocationGcsFields;
pub type ILocationGcsAddResolve = IModelsId;
pub type ILocationGcsGet = ILocationGcsQueryGet;
pub type ILocationGcsGetResolve = IModelsResults<LocationGcs>;
pub type ILocationGcsDelete = LocationGcsQueryBindValues;
pub type ILocationGcsDeleteResolve = ();
pub type ILocationGcsUpdate = ILocationGcsQueryUpdate;
pub type ILocationGcsUpdateResolve = ();

pub fn location_gcs_query_bind_values(opts: LocationGcsQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        LocationGcsQueryBindValues::Id(id) => ("id".to_string(), id),
        LocationGcsQueryBindValues::Geohash(geohash) => ("geohash".to_string(), geohash),
    }
}

pub fn location_gcs_query_get_list(opts: ILocationGcsQueryGetList) -> IModelsQueryBindValueTuple {
    let query_sort = match opts.sort {
        Some(LocationGcsSort::Newest) => " ORDER BY lg.created_at DESC",
        Some(LocationGcsSort::Oldest) => " ORDER BY lg.created_at ASC",
        None => "",
    };
    match opts.of {
        LocationGcsQueryListOf::All(_) => (format!("SELECT lg.* FROM location_gcs lg{}", query_sort), "".to_string()),
        LocationGcsQueryListOf::OnTradeProduct(id) => (format!("SELECT lg.* FROM location_gcs lg JOIN trade_product_location tp_lg ON lg.id = tp_lg.tb_lg WHERE tp_lg.tb_tp = ?1{}", query_sort), id),
        LocationGcsQueryListOf::OffTradeProduct(id) => (format!("SELECT lg.* FROM location_gcs lg WHERE NOT EXISTS (SELECT 1 FROM trade_product_location tp_lg WHERE tp_lg.tb_lg = lg.id AND tp_lg.tb_tp = ?1){}", query_sort), id),
    }
}

fn location_gcs_fields_bind_values(
    opts: ILocationGcsFields,
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

fn location_gcs_fields_update_bind_values(
    opts: ILocationGcsFieldsUpdate,
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

pub async fn lib_model_location_gcs_add(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ILocationGcsAdd,
) -> Result<ILocationGcsAddResolve, ModelError> {
    let id = uuidv4();
    let created_at = time_created_on();
    let updated_at = created_at.clone();
    let bind_values = location_gcs_fields_bind_values(opts)
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
        "INSERT INTO location_gcs ({}) VALUES ({});",
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

fn location_gcs_query_get(
    opts: ILocationGcsGet,
) -> Result<(String, Vec<IModelsQueryBindValue>), ModelError> {
    match opts {
        ILocationGcsQueryGet {
            list: Some(opts_list),
            ..
        } => {
            let (query, bv) = location_gcs_query_get_list(opts_list);
            Ok((query, vec![bv]))
        }
        ILocationGcsQueryGet {
            on: Some(opts_on), ..
        } => {
            let (bv_k, bv) = location_gcs_query_bind_values(opts_on);
            let query = format!("SELECT * FROM location_gcs WHERE {} = ?1;", bv_k);
            Ok((query, vec![bv]))
        }
        _ => Err(ModelError::InvalidQuery(
            "model.location_gcs.error.query_invalid".to_string(),
        )),
    }
}

pub async fn lib_model_location_gcs_get(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ILocationGcsQueryGet,
) -> Result<ILocationGcsGetResolve, ModelError> {
    let (query, bind_values) = location_gcs_query_get(opts)?;
    let mut query_builder = sqlx::query_as::<_, LocationGcs>(&query);
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

pub async fn lib_model_location_gcs_delete(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ILocationGcsDelete,
) -> Result<ILocationGcsDeleteResolve, ModelError> {
    let (bv_k, bv) = location_gcs_query_bind_values(opts);
    let query = format!("DELETE FROM location_gcs WHERE {} = ?1;", bv_k);
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

pub async fn lib_model_location_gcs_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: ILocationGcsUpdate,
) -> Result<ILocationGcsUpdateResolve, ModelError> {
    let (bv_k, bv) = location_gcs_query_bind_values(opts.on);
    let bind_values = location_gcs_fields_update_bind_values(opts.fields)
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
        "UPDATE location_gcs SET {} WHERE {} = ?1;",
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
