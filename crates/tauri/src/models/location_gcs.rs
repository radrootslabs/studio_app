use crate::radroots::Radroots;
use radroots_core::{
    models::location_gcs::{lib_model_location_gcs_add, ILocationGcsAdd, ILocationGcsAddResolve, lib_model_location_gcs_get, ILocationGcsGet, ILocationGcsGetResolve, lib_model_location_gcs_delete, ILocationGcsDelete, ILocationGcsDeleteResolve, lib_model_location_gcs_update, ILocationGcsUpdate, ILocationGcsUpdateResolve},
};

#[tauri::command]
pub async fn model_location_gcs_add(
    state: tauri::State<'_, Radroots>,
    opts: ILocationGcsAdd,
) -> Result<ILocationGcsAddResolve, String> {
    match lib_model_location_gcs_add(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_location_gcs_get(
    state: tauri::State<'_, Radroots>,
    opts: ILocationGcsGet,
) -> Result<ILocationGcsGetResolve, String> {
    match lib_model_location_gcs_get(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_location_gcs_delete(
    state: tauri::State<'_, Radroots>,
    opts: ILocationGcsDelete,
) -> Result<ILocationGcsDeleteResolve, String> {
    match lib_model_location_gcs_delete(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn model_location_gcs_update(
    state: tauri::State<'_, Radroots>,
    opts: ILocationGcsUpdate,
) -> Result<ILocationGcsUpdateResolve, String> {
    match lib_model_location_gcs_update(&state.db, opts).await {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("ERROR {}", e);
            Err(e.to_string())
        }
    }
}
