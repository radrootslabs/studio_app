use crate::radroots::Radroots;
use radroots_core::{
    models::media_upload::{lib_model_media_upload_add, IMediaUploadAdd, IMediaUploadAddResolve, lib_model_media_upload_get, IMediaUploadGet, IMediaUploadGetResolve, lib_model_media_upload_delete, IMediaUploadDelete, IMediaUploadDeleteResolve, lib_model_media_upload_update, IMediaUploadUpdate, IMediaUploadUpdateResolve},
};

#[tauri::command]
pub async fn model_media_upload_add(
    state: tauri::State<'_, Radroots>,
    opts: IMediaUploadAdd,
) -> Result<IMediaUploadAddResolve, String> {
    match lib_model_media_upload_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_media_upload_get(
    state: tauri::State<'_, Radroots>,
    opts: IMediaUploadGet,
) -> Result<IMediaUploadGetResolve, String> {
    match lib_model_media_upload_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_media_upload_delete(
    state: tauri::State<'_, Radroots>,
    opts: IMediaUploadDelete,
) -> Result<IMediaUploadDeleteResolve, String> {
    match lib_model_media_upload_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_media_upload_update(
    state: tauri::State<'_, Radroots>,
    opts: IMediaUploadUpdate,
) -> Result<IMediaUploadUpdateResolve, String> {
    match lib_model_media_upload_update(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
