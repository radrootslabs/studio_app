use super::{
    nostr_profile::{nostr_profile_query_bind_values, NostrProfileQueryBindValues},
    nostr_relay::{nostr_relay_query_bind_values, NostrRelayQueryBindValues},
};
use crate::{error::ModelError, types::DatabaseConnection};
use radroots_core::types::IResultPass;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NostrProfileRelay {
    tb_pr: String,
    tb_rl: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct INostrProfileRelayTables {
    pub nostr_profile: NostrProfileQueryBindValues,
    pub nostr_relay: NostrRelayQueryBindValues,
}

pub type INostrProfileRelayRelation = INostrProfileRelayTables;
pub type INostrProfileRelayResolve = IResultPass;

pub async fn lib_model_nostr_profile_relay_set(
    db: &DatabaseConnection,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayResolve, ModelError> {
    let (bv_pr_k, bv_pr) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_rl_k, bv_rl) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_pr, bv_rl];
    let query = format!("INSERT INTO nostr_profile_relay (tb_pr, tb_rl) VALUES ((SELECT id FROM nostr_profile WHERE {} = ?), (SELECT id FROM nostr_relay WHERE {} = ?));", bv_pr_k, bv_rl_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
pub async fn lib_model_nostr_profile_relay_unset(
    db: &DatabaseConnection,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayResolve, ModelError> {
    let (bv_pr_k, bv_pr) = nostr_profile_query_bind_values(opts.nostr_profile);
    let (bv_rl_k, bv_rl) = nostr_relay_query_bind_values(opts.nostr_relay);
    let query_vals = vec![bv_pr, bv_rl];
    let query = format!("DELETE FROM nostr_profile_relay WHERE tb_pr = (SELECT id FROM nostr_profile WHERE {} = ?) AND tb_rl = (SELECT id FROM nostr_relay WHERE {} = ?);", bv_pr_k, bv_rl_k);
    let mut query_builder = sqlx::query(&query);
    for value in query_vals.iter() {
        query_builder = query_builder.bind(value);
    }
    query_builder
        .execute(db)
        .await
        .map_err(|e| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
