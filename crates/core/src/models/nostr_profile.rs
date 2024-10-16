use crate::{
    error::ModelError,
    types::{IModelsId, IModelsQueryBindValue, IModelsQueryBindValueTuple, IModelsResults},
    utils::{time_created_on, uuidv4},
};
use futures::TryStreamExt;

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
pub struct INostrProfileFieldsUpdate {
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
#[serde(rename_all = "snake_case")]
pub enum NostrProfileSort {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NostrProfileQueryBindValues {
    Id(IModelsQueryBindValue),
    PublicKey(IModelsQueryBindValue),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileQueryGetList {
    pub of: Vec<String>,
    pub sort: Option<NostrProfileSort>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileQueryGet {
    pub on: Option<NostrProfileQueryBindValues>,
    pub list: Option<INostrProfileQueryGetList>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileQueryUpdate {
    pub on: NostrProfileQueryBindValues,
    pub fields: INostrProfileFieldsUpdate,
}

impl INostrProfileQueryGet {
    pub fn new_on(values: NostrProfileQueryBindValues) -> Self {
        INostrProfileQueryGet {
            on: Some(values),
            list: None,
        }
    }
    pub fn new_list(list: INostrProfileQueryGetList) -> Self {
        INostrProfileQueryGet {
            on: None,
            list: Some(list),
        }
    }
}

pub type INostrProfileAdd = INostrProfileFields;
pub type INostrProfileAddResolve = IModelsId;
pub type INostrProfileGet = INostrProfileQueryGet;
pub type INostrProfileGetResolve = IModelsResults<NostrProfile>;
pub type INostrProfileDelete = NostrProfileQueryBindValues;
pub type INostrProfileDeleteResolve = ();
pub type INostrProfileUpdate = INostrProfileQueryUpdate;
pub type INostrProfileUpdateResolve = ();

fn nostr_profile_query_bind_values(opts: NostrProfileQueryBindValues) -> IModelsQueryBindValueTuple {
    match opts {
        NostrProfileQueryBindValues::Id(id) => ("id".to_string(), id),
        NostrProfileQueryBindValues::PublicKey(public_key) => ("public_key".to_string(), public_key),
    }
}

fn nostr_profile_fields_bind_values(
    opts: INostrProfileFields,
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

fn nostr_profile_fields_update_bind_values(
    opts: INostrProfileFieldsUpdate,
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

pub async fn lib_model_nostr_profile_add(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileAdd,
) -> Result<INostrProfileAddResolve, ModelError> {
    let id = uuidv4();
    let created_at = time_created_on();
    let updated_at = created_at.clone();
    let bind_values = nostr_profile_fields_bind_values(opts)
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
        "INSERT INTO nostr_profile ({}) VALUES ({});",
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

fn nostr_profile_query_get(
    opts: INostrProfileGet,
) -> Result<(String, Vec<IModelsQueryBindValue>), ModelError> {
    match opts {
        INostrProfileQueryGet {
            list: Some(opts_list),
            ..
        } => {
            let sort = match opts_list.sort {
                Some(NostrProfileSort::Newest) => "created_at DESC",
                Some(NostrProfileSort::Oldest) => "created_at ASC",
                None => "created_at DESC",
            };
            let query = format!("SELECT * FROM nostr_profile ORDER BY {};", sort);
            Ok((query, vec![]))
        }
        INostrProfileQueryGet {
            on: Some(opts_on), ..
        } => {
            let (bv_k, bv) = nostr_profile_query_bind_values(opts_on);
            let query = format!("SELECT * FROM nostr_profile WHERE {} = ?1;", bv_k);
            Ok((query, vec![bv]))
        }
        _ => Err(ModelError::InvalidQuery(
            "model.nostr_profile.error.query_invalid".to_string(),
        )),
    }
}

pub async fn lib_model_nostr_profile_get(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileQueryGet,
) -> Result<INostrProfileGetResolve, ModelError> {
    let (query, bind_values) = nostr_profile_query_get(opts)?;
    let mut query_builder = sqlx::query_as::<_, NostrProfile>(&query);
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

pub async fn lib_model_nostr_profile_delete(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileDelete,
) -> Result<INostrProfileDeleteResolve, ModelError> {
    let (bv_k, bv) = nostr_profile_query_bind_values(opts);
    let query = format!("DELETE FROM nostr_profile WHERE {} = ?1;", bv_k);
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

pub async fn lib_model_nostr_profile_update(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileUpdate,
) -> Result<INostrProfileUpdateResolve, ModelError> {
    let (bv_k, bv) = nostr_profile_query_bind_values(opts.on);
    let bind_values = nostr_profile_fields_update_bind_values(opts.fields)
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
        "UPDATE nostr_profile SET {} WHERE {} = ?1;",
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
