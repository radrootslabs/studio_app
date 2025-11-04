use crate::ffi::Db;
use wasm_bindgen::prelude::*;

const MIG_0001: &str = include_str!("../tangle_migrations/0001_location_gcs.sql");
const MIG_0002: &str = include_str!("../tangle_migrations/0002_trade_product.sql");
const MIG_0003: &str = include_str!("../tangle_migrations/0003_nostr_profile.sql");
const MIG_0004: &str = include_str!("../tangle_migrations/0004_nostr_relay.sql");
const MIG_0005: &str = include_str!("../tangle_migrations/0005_media_image.sql");
const MIG_0006: &str = include_str!("../tangle_migrations/0006_log_error.sql");
const MIG_0007: &str = include_str!("../tangle_migrations/0007_farm.sql");
const MIG_0008: &str = include_str!("../tangle_migrations/0008_nostr_profile_relay.sql");
const MIG_0009: &str = include_str!("../tangle_migrations/0009_farm_location.sql");
const MIG_0010: &str = include_str!("../tangle_migrations/0010_trade_product_location.sql");
const MIG_0011: &str = include_str!("../tangle_migrations/0011_trade_product_media.sql");

#[wasm_bindgen]
pub async fn apply_migrations(db: Db) -> Result<bool, JsValue> {
    let steps = [
        MIG_0001, MIG_0002, MIG_0003, MIG_0004, MIG_0005, MIG_0006, MIG_0007, MIG_0008, MIG_0009,
        MIG_0010, MIG_0011,
    ];
    for sql in steps.iter() {
        db.execute(sql, &JsValue::UNDEFINED)?;
    }
    Ok(true)
}
