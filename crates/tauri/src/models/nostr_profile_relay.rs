use crate::radroots::Radroots;
use radroots_core::{
    models::nostr_profile_relay::{lib_model_nostr_profile_relay_set, lib_model_nostr_profile_relay_unset, INostrProfileRelayRelation, INostrProfileRelayRelationResolve},
};

#[tauri::command]
pub async fn model_nostr_profile_relay_set(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayRelationResolve, String> {
    match lib_model_nostr_profile_relay_set(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_profile_relay_unset(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileRelayRelation,
) -> Result<INostrProfileRelayRelationResolve, String> {
    match lib_model_nostr_profile_relay_unset(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
