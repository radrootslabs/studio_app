mod database;
pub mod models;
mod radroots;

use database::setup_db;
use models::{
    location_gcs::{
        model_location_gcs_add, model_location_gcs_delete, model_location_gcs_get,
        model_location_gcs_update,
    },
    log_error::{model_log_error_add, model_log_error_delete, model_log_error_get},
    media_upload::{
        model_media_upload_add, model_media_upload_delete, model_media_upload_get,
        model_media_upload_update,
    },
    nostr_profile::{
        model_nostr_profile_add, model_nostr_profile_delete, model_nostr_profile_get,
        model_nostr_profile_update,
    },
    nostr_profile_relay::{
        model_nostr_profile_relay_get_all, model_nostr_profile_relay_set,
        model_nostr_profile_relay_unset,
    },
    nostr_relay::{
        model_nostr_relay_add, model_nostr_relay_delete, model_nostr_relay_get,
        model_nostr_relay_update,
    },
    trade_product::{
        model_trade_product_add, model_trade_product_delete, model_trade_product_get,
        model_trade_product_update,
    },
    trade_product_location::{
        model_trade_product_location_get_all, model_trade_product_location_set,
        model_trade_product_location_unset,
    },
    trade_product_media::{
        model_trade_product_media_get_all, model_trade_product_media_set,
        model_trade_product_media_unset,
    },
};
use radroots::Radroots;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let data_dir = app
                    .handle()
                    .path()
                    .app_data_dir()
                    .expect("Failed to resolve app data dir");

                let db = setup_db(&data_dir).await;
                app.manage(Radroots { db });

                Ok(())
            })
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_map_display::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            model_location_gcs_add,
            model_location_gcs_get,
            model_location_gcs_delete,
            model_location_gcs_update,
            model_trade_product_add,
            model_trade_product_get,
            model_trade_product_delete,
            model_trade_product_update,
            model_nostr_profile_add,
            model_nostr_profile_get,
            model_nostr_profile_delete,
            model_nostr_profile_update,
            model_nostr_relay_add,
            model_nostr_relay_get,
            model_nostr_relay_delete,
            model_nostr_relay_update,
            model_nostr_profile_relay_set,
            model_nostr_profile_relay_unset,
            model_nostr_profile_relay_get_all,
            model_trade_product_location_set,
            model_trade_product_location_unset,
            model_trade_product_location_get_all,
            model_media_upload_add,
            model_media_upload_get,
            model_media_upload_delete,
            model_media_upload_update,
            model_trade_product_media_set,
            model_trade_product_media_unset,
            model_trade_product_media_get_all,
            model_log_error_add,
            model_log_error_get,
            model_log_error_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
