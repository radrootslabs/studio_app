use crate::radroots::Radroots;
use radroots_core::{
    models::nostr_profile::{lib_model_nostr_profile_add, INostrProfileAdd, INostrProfileAddResolve, lib_model_nostr_profile_get, INostrProfileGet, INostrProfileGetResolve, lib_model_nostr_profile_delete, INostrProfileDelete, INostrProfileDeleteResolve, lib_model_nostr_profile_update, INostrProfileUpdate, INostrProfileUpdateResolve},
};

#[tauri::command]
pub async fn model_nostr_profile_add(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileAdd,
) -> Result<INostrProfileAddResolve, String> {
    match lib_model_nostr_profile_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_profile_get(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileGet,
) -> Result<INostrProfileGetResolve, String> {
    match lib_model_nostr_profile_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_profile_delete(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileDelete,
) -> Result<INostrProfileDeleteResolve, String> {
    match lib_model_nostr_profile_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_nostr_profile_update(
    state: tauri::State<'_, Radroots>,
    opts: INostrProfileUpdate,
) -> Result<INostrProfileUpdateResolve, String> {
    match lib_model_nostr_profile_update(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
