use crate::radroots::Radroots;
use radroots_core::types::IError;
use radroots_model::{
    tables::nostr_profile::{lib_model_nostr_profile_create, INostrProfileCreate, INostrProfileCreateResolve, lib_model_nostr_profile_read, INostrProfileRead, INostrProfileReadResolve, lib_model_nostr_profile_read_list, INostrProfileReadList, INostrProfileReadListResolve, lib_model_nostr_profile_delete, INostrProfileDelete, INostrProfileDeleteResolve, lib_model_nostr_profile_update, INostrProfileUpdate, INostrProfileUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_create(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileCreate,
) -> Result<INostrProfileCreateResolve, IError> {
    match lib_model_nostr_profile_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_read(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileRead,
) -> Result<INostrProfileReadResolve, IError> {
    match lib_model_nostr_profile_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_read_list(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileReadList,
) -> Result<INostrProfileReadListResolve, IError> {
    match lib_model_nostr_profile_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_update(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileUpdate,
) -> Result<INostrProfileUpdateResolve, IError> {
    match lib_model_nostr_profile_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_profile_delete(
    state: tauri::State<'_, Radroots>,
    args: INostrProfileDelete,
) -> Result<INostrProfileDeleteResolve, IError> {
    match lib_model_nostr_profile_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}