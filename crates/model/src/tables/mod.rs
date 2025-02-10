use crate::{error::ModelError, types::DatabaseConnection};
use radroots_core::types::IResultPass;

pub mod location_gcs;
pub mod log_error;
pub mod media_image;
pub mod nostr_profile;
pub mod nostr_profile_relay;
pub mod nostr_relay;
pub mod trade_product;
pub mod trade_product_location;
pub mod trade_product_media;

pub async fn lib_model_tables_reset(db: &DatabaseConnection) -> Result<IResultPass, ModelError> {
    let query = format!("DELETE FROM location_gcs; DELETE FROM trade_product; DELETE FROM nostr_profile; DELETE FROM nostr_relay; DELETE FROM media_image; DELETE FROM log_error;DELETE FROM nostr_profile_relay; DELETE FROM trade_product_location; DELETE FROM trade_product_media;");
    sqlx::query(&query)
        .execute(db)
        .await
        .map_err(|e: sqlx::Error| ModelError::InvalidQuery(e.to_string()))?;
    Ok(IResultPass { pass: true })
}
