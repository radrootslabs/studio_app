use crate::{
    error::ModelError,
    types::IModelsResults,
    models::nostr_profile::{nostr_profile_query_bind_values, NostrProfileQueryBindValues},
    models::nostr_relay::{nostr_relay_query_bind_values, NostrRelayQueryBindValues},
};
use futures::TryStreamExt;

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NostrProfileRelay {
    tb_pr: String,
    tb_rl: String,
}

pub type INostrProfileRelayRelationResolve = bool;
pub type INostrProfileRelayRelationResolveGetAll = IModelsResults<NostrProfileRelay>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileRelayRelation {
    pub nostr_profile: NostrProfileQueryBindValues,
    pub nostr_relay: NostrRelayQueryBindValues,
}

pub async fn lib_model_nostr_profile_relay_set(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayRelationResolve, ModelError> {
    let (bv_pr_k, bv_pr) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_rl_k, bv_rl) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_pr, bv_rl];
    let query = format!("INSERT INTO nostr_profile_relay (tb_pr, tb_rl) VALUES ((SELECT id FROM nostr_profile WHERE {} = ?1), (SELECT id FROM nostr_relay WHERE {} = ?2));", bv_pr_k, bv_rl_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(true)
}

pub async fn lib_model_nostr_profile_relay_unset(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayRelationResolve, ModelError> {
    let (bv_pr_k, bv_pr) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_rl_k, bv_rl) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_pr, bv_rl];
    let query = format!("DELETE FROM nostr_profile_relay WHERE tb_pr = (SELECT id FROM nostr_profile WHERE {} = ?1) AND tb_rl = (SELECT id FROM nostr_relay WHERE {} = ?2);", bv_pr_k, bv_rl_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(true)
}

pub async fn lib_model_nostr_profile_relay_get_all(
    db: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<INostrProfileRelayRelationResolveGetAll, ModelError> {
    let query = format!("SELECT * FROM nostr_profile_relay;");
    let query_builder = sqlx::query_as::<_, NostrProfileRelay>(&query);
    let results = query_builder
        .fetch(db)
        .try_collect()
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IModelsResults { results })
}
