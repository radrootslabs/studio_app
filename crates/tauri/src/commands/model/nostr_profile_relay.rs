use crate::radroots::Radroots;
use radroots_core::types::IError;
use radroots_model::tables::nostr_profile_relay::{
    lib_model_nostr_profile_relay_set, lib_model_nostr_profile_relay_unset,
    INostrProfileRelayRelation, INostrProfileRelayResolve,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_relay_set(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayResolve, IError> {
    match lib_model_nostr_profile_relay_set(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_relay_unset(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayResolve, IError> {
    match lib_model_nostr_profile_relay_unset(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}