use crate::radroots::Radroots;
use radroots_core::{
    models::nostr_relay::{lib_model_nostr_relay_add, INostrRelayAdd, INostrRelayAddResolve, lib_model_nostr_relay_get, INostrRelayGet, INostrRelayGetResolve, lib_model_nostr_relay_delete, INostrRelayDelete, INostrRelayDeleteResolve, lib_model_nostr_relay_update, INostrRelayUpdate, INostrRelayUpdateResolve},
};

#[tauri::command]
pub async fn model_nostr_relay_add(
    state: tauri::State<'_, Radroots>,
    opts: INostrRelayAdd,
) -> Result<INostrRelayAddResolve, String> {
    match lib_model_nostr_relay_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_relay_get(
    state: tauri::State<'_, Radroots>,
    opts: INostrRelayGet,
) -> Result<INostrRelayGetResolve, String> {
    match lib_model_nostr_relay_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_relay_delete(
    state: tauri::State<'_, Radroots>,
    opts: INostrRelayDelete,
) -> Result<INostrRelayDeleteResolve, String> {
    match lib_model_nostr_relay_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_relay_update(
    state: tauri::State<'_, Radroots>,
    opts: INostrRelayUpdate,
) -> Result<INostrRelayUpdateResolve, String> {
    match lib_model_nostr_relay_update(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
