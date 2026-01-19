use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::types::{Value as SqlValue, ValueRef as SqlValueRef};
use rusqlite::{params_from_iter, Connection, DatabaseName};
use serde_json::Value;

use crate::backup::{backup_b64_to_bytes, backup_bytes_to_b64, RadrootsClientBackupSqlPayload};
#[cfg(target_arch = "wasm32")]
use crate::crypto::RadrootsClientCryptoError;
use crate::crypto::RadrootsClientLegacyKeyConfig;
use crate::idb::{IDB_CONFIG_CIPHER_SQL, RadrootsClientIdbConfig};
#[cfg(target_arch = "wasm32")]
use crate::idb::RadrootsClientIdbStoreError;
use crate::idb::{RadrootsClientWebEncryptedStore, RadrootsClientWebEncryptedStoreConfig};

use super::{
    RadrootsClientSqlCipherConfig,
    RadrootsClientSqlEncryptedStore,
    RadrootsClientSqlEngine,
    RadrootsClientSqlEngineConfig,
    RadrootsClientSqlError,
    RadrootsClientSqlExecOutcome,
    RadrootsClientSqlParams,
    RadrootsClientSqlResult,
    RadrootsClientSqlResultRow,
    RadrootsClientSqlValue,
};

const SQL_STORE_PREFIX: &str = "sql";
const DEFAULT_IV_LENGTH: u32 = 12;

pub struct RadrootsClientWebSqlEncryptedStore {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    store_key: String,
    store_id: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    encrypted_store: RadrootsClientWebEncryptedStore,
}

impl RadrootsClientWebSqlEncryptedStore {
    pub fn new(config: &RadrootsClientSqlEngineConfig) -> Self {
        let store_key = config.store_key.clone();
        let store_id = format!("{SQL_STORE_PREFIX}:{store_key}");
        let legacy_idb_config = resolve_cipher_config(&config.cipher_config);
        let legacy_key = legacy_idb_config.map(|idb_config| RadrootsClientLegacyKeyConfig {
            idb_config,
            key_name: format!("radroots.sql.{store_key}.aes-gcm.key"),
            iv_length: DEFAULT_IV_LENGTH,
            algorithm: String::from("AES-GCM"),
        });
        let encrypted_store = RadrootsClientWebEncryptedStore::new(
            RadrootsClientWebEncryptedStoreConfig {
                idb_config: config.idb_config,
                store_id: store_id.clone(),
                legacy_key,
                iv_length: Some(DEFAULT_IV_LENGTH),
                crypto_service: None,
            },
        );
        Self {
            store_key,
            store_id,
            encrypted_store,
        }
    }

    pub fn get_store_id(&self) -> &str {
        &self.store_id
    }
}

#[async_trait(?Send)]
impl RadrootsClientSqlEncryptedStore for RadrootsClientWebSqlEncryptedStore {
    async fn load(&self) -> RadrootsClientSqlResult<Option<Vec<u8>>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientSqlError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.encrypted_store
                .ensure_store()
                .await
                .map_err(map_crypto_error)?;
            let stored = crate::idb::idb_get(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                &self.store_key,
            )
            .await
            .map_err(map_idb_error)?;
            let Some(stored) = stored else {
                return Ok(None);
            };
            let Some(bytes) = crate::idb::idb_value_as_bytes(&stored) else {
                return Ok(None);
            };
            let outcome = self
                .encrypted_store
                .decrypt_record(&bytes)
                .await
                .map_err(map_crypto_error)?;
            if let Some(reencrypted) = outcome.reencrypted {
                let value = js_sys::Uint8Array::from(&reencrypted[..]);
                crate::idb::idb_set(
                    self.encrypted_store.get_config().database,
                    self.encrypted_store.get_config().store,
                    &self.store_key,
                    &value.into(),
                )
                .await
                .map_err(map_idb_error)?;
            }
            Ok(Some(outcome.plaintext))
        }
    }

    async fn save(&self, bytes: &[u8]) -> RadrootsClientSqlResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = bytes;
            return Err(RadrootsClientSqlError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.encrypted_store
                .ensure_store()
                .await
                .map_err(map_crypto_error)?;
            let encrypted = self
                .encrypted_store
                .encrypt_bytes(bytes)
                .await
                .map_err(map_crypto_error)?;
            let value = js_sys::Uint8Array::from(&encrypted[..]);
            crate::idb::idb_set(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                &self.store_key,
                &value.into(),
            )
            .await
            .map_err(map_idb_error)?;
            Ok(())
        }
    }

    async fn remove(&self) -> RadrootsClientSqlResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            return Err(RadrootsClientSqlError::IdbUndefined);
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::idb::idb_del(
                self.encrypted_store.get_config().database,
                self.encrypted_store.get_config().store,
                &self.store_key,
            )
            .await
            .map_err(map_idb_error)?;
            Ok(())
        }
    }
}

pub struct RadrootsClientWebSqlEngine {
    store_id: String,
    store: Arc<RadrootsClientWebSqlEncryptedStore>,
    conn: Arc<Mutex<Connection>>,
}

impl RadrootsClientWebSqlEngine {
    pub async fn create(
        config: RadrootsClientSqlEngineConfig,
    ) -> RadrootsClientSqlResult<Self> {
        let store = Arc::new(RadrootsClientWebSqlEncryptedStore::new(&config));
        let conn = Connection::open_in_memory().map_err(map_rusqlite_error)?;
        let engine = Self {
            store_id: store.get_store_id().to_string(),
            store,
            conn: Arc::new(Mutex::new(conn)),
        };
        match engine.store.load().await {
            Ok(Some(bytes)) => {
                let _ = engine.import_bytes(&bytes).await?;
            }
            Ok(None) => {}
            Err(RadrootsClientSqlError::IdbUndefined) => {}
            Err(err) => return Err(err),
        }
        Ok(engine)
    }

    pub fn get_store_id(&self) -> &str {
        &self.store_id
    }

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn shared_connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    fn exec_statement(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<RadrootsClientSqlExecOutcome> {
        let conn = self.conn.lock().map_err(|_| RadrootsClientSqlError::EngineUnavailable)?;
        let mut stmt = conn.prepare(sql).map_err(map_rusqlite_error)?;
        let changes = match params {
            RadrootsClientSqlParams::Named(named) => {
                let named = build_named_params(&named)?;
                let mut refs = Vec::with_capacity(named.len());
                for (key, value) in &named {
                    refs.push((key.as_str(), value as &dyn rusqlite::ToSql));
                }
                stmt.execute(refs.as_slice()).map_err(map_rusqlite_error)?
            }
            RadrootsClientSqlParams::Positional(values) => {
                let values = build_positional_params(&values)?;
                stmt.execute(params_from_iter(values.into_iter()))
                    .map_err(map_rusqlite_error)?
            }
        };
        let last_insert_id = conn.last_insert_rowid();
        Ok(RadrootsClientSqlExecOutcome {
            changes: changes as i64,
            last_insert_id,
        })
    }

    fn query_statement(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<Vec<RadrootsClientSqlResultRow>> {
        let conn = self.conn.lock().map_err(|_| RadrootsClientSqlError::EngineUnavailable)?;
        let mut stmt = conn.prepare(sql).map_err(map_rusqlite_error)?;
        let rows = match params {
            RadrootsClientSqlParams::Named(named) => {
                let named = build_named_params(&named)?;
                let mut refs = Vec::with_capacity(named.len());
                for (key, value) in &named {
                    refs.push((key.as_str(), value as &dyn rusqlite::ToSql));
                }
                let mapped = stmt
                    .query_map(refs.as_slice(), row_to_map)
                    .map_err(map_rusqlite_error)?;
                mapped.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite_error)?
            }
            RadrootsClientSqlParams::Positional(values) => {
                let values = build_positional_params(&values)?;
                let mapped = stmt
                    .query_map(params_from_iter(values.into_iter()), row_to_map)
                    .map_err(map_rusqlite_error)?;
                mapped.collect::<Result<Vec<_>, _>>().map_err(map_rusqlite_error)?
            }
        };
        Ok(rows)
    }

    fn export_bytes_inner(&self) -> RadrootsClientSqlResult<Vec<u8>> {
        let conn = self.conn.lock().map_err(|_| RadrootsClientSqlError::EngineUnavailable)?;
        let data = conn
            .serialize(DatabaseName::Main)
            .map_err(|_| RadrootsClientSqlError::ExportFailure)?;
        Ok(data.to_vec())
    }

    async fn persist(&self) -> RadrootsClientSqlResult<()> {
        let bytes = self.export_bytes_inner()?;
        match self.store.save(&bytes).await {
            Ok(()) => Ok(()),
            Err(RadrootsClientSqlError::IdbUndefined) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

#[async_trait(?Send)]
impl RadrootsClientSqlEngine for RadrootsClientWebSqlEngine {
    async fn close(&self) -> RadrootsClientSqlResult<()> {
        self.persist().await
    }

    async fn purge_storage(&self) -> RadrootsClientSqlResult<()> {
        match self.store.remove().await {
            Ok(()) => Ok(()),
            Err(RadrootsClientSqlError::IdbUndefined) => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn exec(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<RadrootsClientSqlExecOutcome> {
        self.exec_statement(sql, params)
    }

    fn query(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<Vec<RadrootsClientSqlResultRow>> {
        self.query_statement(sql, params)
    }

    fn export_bytes(&self) -> RadrootsClientSqlResult<Vec<u8>> {
        self.export_bytes_inner()
    }

    async fn import_bytes(&self, _bytes: &[u8]) -> RadrootsClientSqlResult<()> {
        Err(RadrootsClientSqlError::ImportFailure)
    }

    async fn export_backup(&self) -> RadrootsClientSqlResult<RadrootsClientBackupSqlPayload> {
        let bytes = self.export_bytes_inner()?;
        let bytes_b64 = backup_bytes_to_b64(&bytes)
            .map_err(|_| RadrootsClientSqlError::BackupFailure)?;
        Ok(RadrootsClientBackupSqlPayload { bytes_b64 })
    }

    async fn import_backup(
        &self,
        payload: RadrootsClientBackupSqlPayload,
    ) -> RadrootsClientSqlResult<()> {
        let bytes = backup_b64_to_bytes(&payload.bytes_b64)
            .map_err(|_| RadrootsClientSqlError::BackupFailure)?;
        self.import_bytes(&bytes).await
    }

    fn get_store_id(&self) -> &str {
        &self.store_id
    }
}

fn resolve_cipher_config(
    cipher_config: &RadrootsClientSqlCipherConfig,
) -> Option<RadrootsClientIdbConfig> {
    match cipher_config {
        RadrootsClientSqlCipherConfig::Default => Some(IDB_CONFIG_CIPHER_SQL),
        RadrootsClientSqlCipherConfig::Disabled => None,
        RadrootsClientSqlCipherConfig::Custom(config) => Some(*config),
    }
}

fn build_positional_params(
    values: &[RadrootsClientSqlValue],
) -> RadrootsClientSqlResult<Vec<SqlValue>> {
    let mut binds = Vec::with_capacity(values.len());
    for value in values {
        binds.push(map_param_value(value)?);
    }
    Ok(binds)
}

fn build_named_params(
    values: &BTreeMap<String, RadrootsClientSqlValue>,
) -> RadrootsClientSqlResult<Vec<(String, SqlValue)>> {
    let mut binds = Vec::with_capacity(values.len());
    for (key, value) in values {
        let key = if key.starts_with(':') || key.starts_with('@') || key.starts_with('$') {
            key.clone()
        } else {
            format!(":{key}")
        };
        binds.push((key, map_param_value(value)?));
    }
    Ok(binds)
}

fn map_param_value(value: &RadrootsClientSqlValue) -> RadrootsClientSqlResult<SqlValue> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::Bool(value) => Ok(SqlValue::Integer(i64::from(*value))),
        Value::Number(value) => {
            if let Some(v) = value.as_i64() {
                Ok(SqlValue::Integer(v))
            } else if let Some(v) = value.as_u64() {
                Ok(SqlValue::Integer(v as i64))
            } else if let Some(v) = value.as_f64() {
                Ok(SqlValue::Real(v))
            } else {
                Err(RadrootsClientSqlError::InvalidParams)
            }
        }
        Value::String(value) => Ok(SqlValue::Text(value.clone())),
        _ => Err(RadrootsClientSqlError::InvalidParams),
    }
}

fn row_to_map(row: &rusqlite::Row) -> rusqlite::Result<RadrootsClientSqlResultRow> {
    let stmt = row.as_ref();
    let mut map = BTreeMap::new();
    for i in 0..stmt.column_count() {
        let name = stmt.column_name(i).unwrap_or("").to_string();
        let value = row.get_ref(i)?;
        let json_value = match value {
            SqlValueRef::Null => Value::Null,
            SqlValueRef::Integer(i) => Value::from(i),
            SqlValueRef::Real(f) => Value::from(f),
            SqlValueRef::Text(s) => {
                let s = std::str::from_utf8(s).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        i,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Value::from(s.to_string())
            }
            SqlValueRef::Blob(_) => Value::Null,
        };
        map.insert(name, json_value);
    }
    Ok(map)
}

#[cfg(target_arch = "wasm32")]
fn map_crypto_error(err: RadrootsClientCryptoError) -> RadrootsClientSqlError {
    match err {
        RadrootsClientCryptoError::IdbUndefined => RadrootsClientSqlError::IdbUndefined,
        RadrootsClientCryptoError::CryptoUndefined => RadrootsClientSqlError::EngineUnavailable,
        _ => RadrootsClientSqlError::QueryFailure,
    }
}

#[cfg(target_arch = "wasm32")]
fn map_idb_error(err: RadrootsClientIdbStoreError) -> RadrootsClientSqlError {
    match err {
        RadrootsClientIdbStoreError::IdbUndefined => RadrootsClientSqlError::IdbUndefined,
        _ => RadrootsClientSqlError::QueryFailure,
    }
}

fn map_rusqlite_error(_err: rusqlite::Error) -> RadrootsClientSqlError {
    RadrootsClientSqlError::QueryFailure
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::RadrootsClientWebSqlEngine;
    use crate::idb::RadrootsClientIdbConfig;
    use crate::sql::{
        RadrootsClientSqlCipherConfig,
        RadrootsClientSqlEngine,
        RadrootsClientSqlEngineConfig,
        RadrootsClientSqlParams,
        RadrootsClientSqlValue,
    };

    #[test]
    fn sql_exec_query_roundtrip() {
        let config = RadrootsClientSqlEngineConfig {
            store_key: "test-store".to_string(),
            idb_config: RadrootsClientIdbConfig::new("db", "store"),
            cipher_config: RadrootsClientSqlCipherConfig::Disabled,
            sql_wasm_path: None,
        };
        let engine = futures::executor::block_on(RadrootsClientWebSqlEngine::create(config))
            .expect("engine");
        let _ = engine.exec(
            "CREATE TABLE test_items (id INTEGER PRIMARY KEY, name TEXT)",
            RadrootsClientSqlParams::Positional(Vec::new()),
        );
        let _ = engine.exec(
            "INSERT INTO test_items (name) VALUES (?)",
            RadrootsClientSqlParams::Positional(vec![RadrootsClientSqlValue::from("rad")]),
        );
        let rows = engine
            .query(
                "SELECT name FROM test_items WHERE id = ?",
                RadrootsClientSqlParams::Positional(vec![RadrootsClientSqlValue::from(1)]),
            )
            .expect("query");
        let name = rows
            .first()
            .and_then(|row| row.get("name"))
            .and_then(|value| value.as_str())
            .expect("name");
        assert_eq!(name, "rad");
    }

    #[test]
    fn sql_named_params_execute() {
        let config = RadrootsClientSqlEngineConfig {
            store_key: "test-store".to_string(),
            idb_config: RadrootsClientIdbConfig::new("db", "store"),
            cipher_config: RadrootsClientSqlCipherConfig::Disabled,
            sql_wasm_path: None,
        };
        let engine = futures::executor::block_on(RadrootsClientWebSqlEngine::create(config))
            .expect("engine");
        let _ = engine.exec(
            "CREATE TABLE named_items (id INTEGER PRIMARY KEY, name TEXT)",
            RadrootsClientSqlParams::Positional(Vec::new()),
        );
        let mut named = BTreeMap::new();
        named.insert("name".to_string(), RadrootsClientSqlValue::from("rad"));
        let outcome = engine
            .exec(
                "INSERT INTO named_items (name) VALUES (:name)",
                RadrootsClientSqlParams::Named(named),
            )
            .expect("insert");
        assert_eq!(outcome.changes, 1);
    }
}
