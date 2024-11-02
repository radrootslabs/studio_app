use crate::{
    error::ModelError,
    models::nostr_profile::{nostr_profile_query_bind_values, NostrProfileQueryBindValues},
    models::nostr_relay::{nostr_relay_query_bind_values, NostrRelayQueryBindValues},
};

pub type INostrProfileRelayRelationResolve = bool;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct INostrProfileRelayRelation {
    pub nostr_profile: NostrProfileQueryBindValues,
    pub nostr_relay: NostrRelayQueryBindValues,
}

pub async fn lib_model_nostr_profile_relay_set(
    db: &sqlx::Pool<sqlx::Sqlite>,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayRelationResolve, ModelError> {
    let (bv_np_k, bv_np) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_nr_k, bv_nr) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_np, bv_nr];
    let query = format!("INSERT INTO nostr_profile_relay (tb_pr_rl_0, tb_pr_rl_1) VALUES ((SELECT id FROM nostr_profile WHERE {} = ?1), (SELECT id FROM nostr_relay WHERE {} = ?2));", bv_np_k, bv_nr_k);
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
    let (bv_np_k, bv_np) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_nr_k, bv_nr) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_np, bv_nr];
    let query = format!("DELETE FROM nostr_profile_relay WHERE tb_pr_rl_0 = (SELECT id FROM nostr_profile WHERE {} = ?1) AND tb_pr_rl_1 = (SELECT id FROM nostr_relay WHERE {} = ?2);", bv_np_k, bv_nr_k);
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
