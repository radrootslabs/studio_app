use tangle_core::{
    keystore,
    nostr::keys::{
        lib_nostr_keys_gen, lib_nostr_keys_parse, lib_nostr_public_key_hex,
        lib_nostr_secret_key_hex,
    },
    types::{IResult, IResultList, IResultPass},
};

use crate::app::Tangle;

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_gen(rr: tauri::State<'_, Tangle>) -> Result<IResult<String>, String> {
    let keys = lib_nostr_keys_gen();
    keystore::key_add(&keys, &rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResult {
        result: keys.public_key.to_hex(),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_add(
    secret_key: String,
    rr: tauri::State<'_, Tangle>,
) -> Result<IResult<String>, String> {
    let keys = lib_nostr_keys_parse(secret_key).map_err(|e| e.to_string())?;
    keystore::key_add(&keys, &rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResult {
        result: lib_nostr_public_key_hex(keys),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_read(
    public_key: String,
    rr: tauri::State<'_, Tangle>,
) -> Result<IResult<String>, String> {
    let keys = keystore::key_read(&public_key, &rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResult {
        result: lib_nostr_secret_key_hex(keys),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_read_all(
    rr: tauri::State<'_, Tangle>,
) -> Result<IResultList<String>, String> {
    let keystore_keys = keystore::keys_read_all(&rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResultList {
        results: keystore_keys,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_delete(
    public_key: String,
    rr: tauri::State<'_, Tangle>,
) -> Result<IResultPass, String> {
    keystore::key_delete(&public_key, &rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResultPass { pass: true })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn keys_nostr_keystore_reset(
    rr: tauri::State<'_, Tangle>,
) -> Result<IResultPass, String> {
    keystore::reset(&rr.data_dir).map_err(|e| e.to_string())?;
    Ok(IResultPass { pass: true })
}
