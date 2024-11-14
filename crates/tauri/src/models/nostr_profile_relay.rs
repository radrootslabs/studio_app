use crate::radroots::Radroots;
use radroots_core::{
    models::nostr_profile_relay::{lib_model_nostr_profile_relay_set, lib_model_nostr_profile_relay_unset, lib_model_nostr_profile_relay_get_all, INostrProfileRelayRelation, INostrProfileRelayRelationResolve, INostrProfileRelayRelationResolveGetAll},
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

#[tauri::command]
pub async fn model_nostr_profile_relay_get_all(
    state: tauri::State<'_, Radroots>,
) -> Result<INostrProfileRelayRelationResolveGetAll, String> {
    match lib_model_nostr_profile_relay_get_all(&state.db).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
