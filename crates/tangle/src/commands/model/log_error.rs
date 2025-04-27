use crate::app::Tangle;
use tangle_core::types::IError;
use tangle_model::{
    tables::log_error::{lib_model_log_error_create, ILogErrorCreate, ILogErrorCreateResolve, lib_model_log_error_read, ILogErrorRead, ILogErrorReadResolve, lib_model_log_error_read_list, ILogErrorReadList, ILogErrorReadListResolve, lib_model_log_error_delete, ILogErrorDelete, ILogErrorDeleteResolve, lib_model_log_error_update, ILogErrorUpdate, ILogErrorUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_log_error_create(
    state: tauri::State<'_, Tangle>,
    args: ILogErrorCreate,
) -> Result<ILogErrorCreateResolve, IError> {
    match lib_model_log_error_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_log_error_read(
    state: tauri::State<'_, Tangle>,
    args: ILogErrorRead,
) -> Result<ILogErrorReadResolve, IError> {
    match lib_model_log_error_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_log_error_read_list(
    state: tauri::State<'_, Tangle>,
    args: ILogErrorReadList,
) -> Result<ILogErrorReadListResolve, IError> {
    match lib_model_log_error_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_log_error_update(
    state: tauri::State<'_, Tangle>,
    args: ILogErrorUpdate,
) -> Result<ILogErrorUpdateResolve, IError> {
    match lib_model_log_error_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_log_error_delete(
    state: tauri::State<'_, Tangle>,
    args: ILogErrorDelete,
) -> Result<ILogErrorDeleteResolve, IError> {
    match lib_model_log_error_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}