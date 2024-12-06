use crate::radroots::Radroots;
use radroots_core::{
    models::log_error::{lib_model_log_error_add, ILogErrorAdd, ILogErrorAddResolve, lib_model_log_error_get, ILogErrorGet, ILogErrorGetResolve, lib_model_log_error_delete, ILogErrorDelete, ILogErrorDeleteResolve},
};

#[tauri::command]
pub async fn model_log_error_add(
    state: tauri::State<'_, Radroots>,
    opts: ILogErrorAdd,
) -> Result<ILogErrorAddResolve, String> {
    match lib_model_log_error_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_log_error_get(
    state: tauri::State<'_, Radroots>,
    opts: ILogErrorGet,
) -> Result<ILogErrorGetResolve, String> {
    match lib_model_log_error_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_log_error_delete(
    state: tauri::State<'_, Radroots>,
    opts: ILogErrorDelete,
) -> Result<ILogErrorDeleteResolve, String> {
    match lib_model_log_error_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
