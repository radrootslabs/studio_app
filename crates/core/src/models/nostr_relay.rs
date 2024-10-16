use crate::{
    error::ModelError,
    types::{IModelsId, IModelsQueryBindValue, IModelsQueryBindValueTuple, IModelsResults},
    utils::{time_created_on, uuidv4},
};
use futures::TryStreamExt;

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
pub struct INostrRelayFieldsUpdate {
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
#[serde(rename_all = "snake_case")]
pub enum NostrRelaySort {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NostrRelayQueryBindValues {
    Id(IModelsQueryBindValue),
    Url(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayQueryGetList {
    pub of: Vec<String>,
    pub sort: Option<NostrRelaySort>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayQueryGet {
    pub on: Option<NostrRelayQueryBindValues>,
    pub list: Option<INostrRelayQueryGetList>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrRelayQueryUpdate {
    pub on: NostrRelayQueryBindValues,
    pub fields: INostrRelayFieldsUpdate,
}

impl INostrRelayQueryGet {
    pub fn new_on(values: NostrRelayQueryBindValues) -> Self {
        INostrRelayQueryGet {
            on: Some(values),
            list: None,
        }
    }
    pub fn new_list(list: INostrRelayQueryGetList) -> Self {
        INostrRelayQueryGet {
            on: None,
            list: Some(list),
        }
    }
}

pub type INostrRelayAdd = INostrRelayFields;
pub type INostrRelayAddResolve = IModelsId;
pub type INostrRelayGet = INostrRelayQueryGet;
pub type INostrRelayGetResolve = IModelsResults<NostrRelay>;
pub type INostrRelayDelete = NostrRelayQueryBindValues;
pub type INostrRelayDeleteResolve = ();
pub type INostrRelayUpdate = INostrRelayQueryUpdate;
pub type INostrRelayUpdateResolve = ();

fn nostr_relay_query_bind_values(opts: NostrRelayQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        NostrRelayQueryBindValues::Id(id) => ("id".to_string(), id),
        NostrRelayQueryBindValues::Url(url) => ("url".to_string(), url),
    }
}

fn nostr_relay_fields_bind_values(
    opts: INostrRelayFields,
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

fn nostr_relay_fields_update_bind_values(
    opts: INostrRelayFieldsUpdate,
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

pub async fn lib_model_nostr_relay_add(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrRelayAdd,
) -> Result<INostrRelayAddResolve, ModelError> {
    let id = uuidv4();
    let created_at = time_created_on();
    let updated_at = created_at.clone();
    let bind_values = nostr_relay_fields_bind_values(opts)
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
        "INSERT INTO nostr_relay ({}) VALUES ({});",
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

fn nostr_relay_query_get(
    opts: INostrRelayGet,
) -> Result<(String, Vec<IModelsQueryBindValue>), ModelError> {
    match opts {
        INostrRelayQueryGet {
            list: Some(opts_list),
            ..
        } => {
            let sort = match opts_list.sort {
                Some(NostrRelaySort::Newest) => "created_at DESC",
                Some(NostrRelaySort::Oldest) => "created_at ASC",
                None => "created_at DESC",
            };
            let query = format!("SELECT * FROM nostr_relay ORDER BY {};", sort);
            Ok((query, vec![]))
        }
        INostrRelayQueryGet {
            on: Some(opts_on), ..
        } => {
            let (bv_k, bv) = nostr_relay_query_bind_values(opts_on);
            let query = format!("SELECT * FROM nostr_relay WHERE {} = ?1;", bv_k);
            Ok((query, vec![bv]))
        }
        _ => Err(ModelError::InvalidQuery(
            "model.nostr_relay.error.query_invalid".to_string(),
        )),
    }
}

pub async fn lib_model_nostr_relay_get(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrRelayQueryGet,
) -> Result<INostrRelayGetResolve, ModelError> {
    let (query, bind_values) = nostr_relay_query_get(opts)?;
    let mut query_builder = sqlx::query_as::<_, NostrRelay>(&query);
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

pub async fn lib_model_nostr_relay_delete(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrRelayDelete,
) -> Result<INostrRelayDeleteResolve, ModelError> {
    let (bv_k, bv) = nostr_relay_query_bind_values(opts);
    let query = format!("DELETE FROM nostr_relay WHERE {} = ?1;", bv_k);
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

pub async fn lib_model_nostr_relay_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrRelayUpdate,
) -> Result<INostrRelayUpdateResolve, ModelError> {
    let (bv_k, bv) = nostr_relay_query_bind_values(opts.on);
    let bind_values = nostr_relay_fields_update_bind_values(opts.fields)
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
        "UPDATE nostr_relay SET {} WHERE {} = ?1;",
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
