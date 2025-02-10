use crate::radroots::Radroots;
use radroots_core::types::IError;
use radroots_model::{
    tables::media_image::{lib_model_media_image_create, IMediaImageCreate, IMediaImageCreateResolve, lib_model_media_image_read, IMediaImageRead, IMediaImageReadResolve, lib_model_media_image_read_list, IMediaImageReadList, IMediaImageReadListResolve, lib_model_media_image_delete, IMediaImageDelete, IMediaImageDeleteResolve, lib_model_media_image_update, IMediaImageUpdate, IMediaImageUpdateResolve},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn model_media_image_create(
    state: tauri::State<'_, Radroots>,
    args: IMediaImageCreate,
) -> Result<IMediaImageCreateResolve, IError> {
    match lib_model_media_image_create(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_media_image_read(
    state: tauri::State<'_, Radroots>,
    args: IMediaImageRead,
) -> Result<IMediaImageReadResolve, IError> {
    match lib_model_media_image_read(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_media_image_read_list(
    state: tauri::State<'_, Radroots>,
    args: IMediaImageReadList,
) -> Result<IMediaImageReadListResolve, IError> {
    match lib_model_media_image_read_list(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_media_image_update(
    state: tauri::State<'_, Radroots>,
    args: IMediaImageUpdate,
) -> Result<IMediaImageUpdateResolve, IError> {
    match lib_model_media_image_update(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn model_media_image_delete(
    state: tauri::State<'_, Radroots>,
    args: IMediaImageDelete,
) -> Result<IMediaImageDeleteResolve, IError> {
    match lib_model_media_image_delete(&state.db, args).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}