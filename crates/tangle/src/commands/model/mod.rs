use crate::app::Tangle;
use tangle_core::types::{IError, IResultPass};
use tangle_model::tables::lib_model_tables_reset;

pub(crate) mod location_gcs;
pub(crate) mod log_error;
pub(crate) mod media_image;
pub(crate) mod nostr_profile;
pub(crate) mod nostr_profile_relay;
pub(crate) mod nostr_relay;
pub(crate) mod trade_product;
pub(crate) mod trade_product_location;
pub(crate) mod trade_product_media;
    
#[tauri::command(rename_all = "snake_case")]
pub async fn model_tables_reset(state: tauri::State<'_, Tangle>) -> Result<IResultPass, IError> {
    match lib_model_tables_reset(&state.db).await {
        Ok(result) => Ok(result),
        Err(e) => Err(IError { err: e.to_string() }),
    }
}
