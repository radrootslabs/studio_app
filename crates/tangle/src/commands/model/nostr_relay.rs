use crate::app::Tangle;
use tangle_core::types::IError;
use tangle_model::{
    tables::nostr_relay::{lib_model_nostr_relay_create, INostrRelayCreate, INostrRelayCreateResolve, lib_model_nostr_relay_read, INostrRelayRead, INostrRelayReadResolve, lib_model_nostr_relay_read_list, INostrRelayReadList, INostrRelayReadListResolve, lib_model_nostr_relay_delete, INostrRelayDelete, INostrRelayDeleteResolve, lib_model_nostr_relay_update, INostrRelayUpdate, INostrRelayUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_relay_create(
    state: tauri::State<'_, Tangle>,
    args: INostrRelayCreate,
) -> Result<INostrRelayCreateResolve, IError> {
    match lib_model_nostr_relay_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_relay_read(
    state: tauri::State<'_, Tangle>,
    args: INostrRelayRead,
) -> Result<INostrRelayReadResolve, IError> {
    match lib_model_nostr_relay_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_relay_read_list(
    state: tauri::State<'_, Tangle>,
    args: INostrRelayReadList,
) -> Result<INostrRelayReadListResolve, IError> {
    match lib_model_nostr_relay_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_relay_update(
    state: tauri::State<'_, Tangle>,
    args: INostrRelayUpdate,
) -> Result<INostrRelayUpdateResolve, IError> {
    match lib_model_nostr_relay_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_nostr_relay_delete(
    state: tauri::State<'_, Tangle>,
    args: INostrRelayDelete,
) -> Result<INostrRelayDeleteResolve, IError> {
    match lib_model_nostr_relay_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}