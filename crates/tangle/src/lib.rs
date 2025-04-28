pub mod app;
pub mod commands;
pub mod util;

use app::Tangle;
use commands::keys::{
    keys_nostr_add, keys_nostr_delete, keys_nostr_gen, keys_nostr_keystore_reset, keys_nostr_read,
    keys_nostr_read_all,
};
use commands::model::farm::{
        model_farm_create, model_farm_delete, model_farm_read,
        model_farm_read_list, model_farm_update,
    };
use commands::model::farm_location::{
        model_farm_location_set, model_farm_location_unset,
    };
use commands::model::location_gcs::{
        model_location_gcs_create, model_location_gcs_delete, model_location_gcs_read,
        model_location_gcs_read_list, model_location_gcs_update,
    };
use commands::model::log_error::{
        model_log_error_create, model_log_error_delete, model_log_error_read,
        model_log_error_read_list, model_log_error_update,
    };
use commands::model::media_image::{
        model_media_image_create, model_media_image_delete, model_media_image_read,
        model_media_image_read_list, model_media_image_update,
    };
use commands::model::model_tables_reset;
use commands::model::nostr_profile::{
        model_nostr_profile_create, model_nostr_profile_delete, model_nostr_profile_read,
        model_nostr_profile_read_list, model_nostr_profile_update,
    };
use commands::model::nostr_profile_relay::{
        model_nostr_profile_relay_set, model_nostr_profile_relay_unset,
    };
use commands::model::nostr_relay::{
        model_nostr_relay_create, model_nostr_relay_delete, model_nostr_relay_read,
        model_nostr_relay_read_list, model_nostr_relay_update,
    };
use commands::model::trade_product::{
        model_trade_product_create, model_trade_product_delete, model_trade_product_read,
        model_trade_product_read_list, model_trade_product_update,
    };
use commands::model::trade_product_location::{
        model_trade_product_location_set, model_trade_product_location_unset,
    };
use commands::model::trade_product_media::{
        model_trade_product_media_set, model_trade_product_media_unset,
    };
use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let data_dir = app
                .handle()
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data dir");

            let logs_dir = app.handle().path().app_log_dir().unwrap();

            let fmt_data_dir = if cfg!(dev) {
                PathBuf::from(format!("{}/dev", data_dir.to_string_lossy()))
            } else {
                PathBuf::from(format!("{}/release", data_dir.to_string_lossy()))
            };

            let fmt_logs_dir = if cfg!(dev) {
                PathBuf::from(format!("{}/dev", logs_dir.to_string_lossy()))
            } else {
                PathBuf::from(format!("{}/release", logs_dir.to_string_lossy()))
            };

            tauri::async_runtime::block_on(async move {
                let tangle = Tangle::new(fmt_data_dir, fmt_logs_dir).await;
                app.manage(tangle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // %model%
            model_farm_create,
            model_farm_delete,
            model_farm_location_set,
            model_farm_location_unset,
            model_farm_read,
            model_farm_read_list,
            model_farm_update,
            model_location_gcs_create,
            model_location_gcs_delete,
            model_location_gcs_read,
            model_location_gcs_read_list,
            model_location_gcs_update,
            model_log_error_create,
            model_log_error_delete,
            model_log_error_read,
            model_log_error_read_list,
            model_log_error_update,
            model_media_image_create,
            model_media_image_delete,
            model_media_image_read,
            model_media_image_read_list,
            model_media_image_update,
            model_nostr_profile_create,
            model_nostr_profile_delete,
            model_nostr_profile_read,
            model_nostr_profile_read_list,
            model_nostr_profile_relay_set,
            model_nostr_profile_relay_unset,
            model_nostr_profile_update,
            model_nostr_relay_create,
            model_nostr_relay_delete,
            model_nostr_relay_read,
            model_nostr_relay_read_list,
            model_nostr_relay_update,
            model_tables_reset,
            model_trade_product_create,
            model_trade_product_delete,
            model_trade_product_location_set,
            model_trade_product_location_unset,
            model_trade_product_media_set,
            model_trade_product_media_unset,
            model_trade_product_read,
            model_trade_product_read_list,
            model_trade_product_update,
            // %model%
            keys_nostr_gen,
            keys_nostr_add,
            keys_nostr_read,
            keys_nostr_read_all,
            keys_nostr_delete,
            keys_nostr_keystore_reset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
