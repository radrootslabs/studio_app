use std::collections::BTreeMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::backup::RadrootsClientBackupSqlPayload;
use crate::idb::RadrootsClientIdbConfig;

use super::RadrootsClientSqlError;

pub type RadrootsClientSqlResult<T> = Result<T, RadrootsClientSqlError>;
pub type RadrootsClientSqlValue = Value;
pub type RadrootsClientSqlResultRow = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq)]
pub struct RadrootsClientSqlExecOutcome {
    pub changes: i64,
    pub last_insert_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientSqlMigrationRow {
    pub id: i64,
    pub name: String,
    pub applied_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientSqlMigrationState {
    pub applied_names: Vec<String>,
    pub applied_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RadrootsClientSqlParams {
    Named(BTreeMap<String, Value>),
    Positional(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsClientSqlCipherConfig {
    Default,
    Disabled,
    Custom(RadrootsClientIdbConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientSqlEngineConfig {
    pub store_key: String,
    pub idb_config: RadrootsClientIdbConfig,
    pub cipher_config: RadrootsClientSqlCipherConfig,
    pub sql_wasm_path: Option<String>,
}

#[async_trait(?Send)]
pub trait RadrootsClientSqlEncryptedStore {
    async fn load(&self) -> RadrootsClientSqlResult<Option<Vec<u8>>>;
    async fn save(&self, bytes: &[u8]) -> RadrootsClientSqlResult<()>;
    async fn remove(&self) -> RadrootsClientSqlResult<()>;
}

#[async_trait(?Send)]
pub trait RadrootsClientSqlEngine {
    async fn close(&self) -> RadrootsClientSqlResult<()>;
    async fn purge_storage(&self) -> RadrootsClientSqlResult<()>;
    fn exec(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<RadrootsClientSqlExecOutcome>;
    fn query(
        &self,
        sql: &str,
        params: RadrootsClientSqlParams,
    ) -> RadrootsClientSqlResult<Vec<RadrootsClientSqlResultRow>>;
    fn export_bytes(&self) -> RadrootsClientSqlResult<Vec<u8>>;
    async fn import_bytes(&self, bytes: &[u8]) -> RadrootsClientSqlResult<()>;
    async fn export_backup(
        &self,
    ) -> RadrootsClientSqlResult<RadrootsClientBackupSqlPayload>;
    async fn import_backup(
        &self,
        payload: RadrootsClientBackupSqlPayload,
    ) -> RadrootsClientSqlResult<()>;
    fn get_store_id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::{RadrootsClientSqlParams, RadrootsClientSqlValue};

    #[test]
    fn params_accept_positional_values() {
        let params = RadrootsClientSqlParams::Positional(vec![
            RadrootsClientSqlValue::from(1),
            RadrootsClientSqlValue::from("two"),
        ]);
        match params {
            RadrootsClientSqlParams::Positional(values) => {
                assert_eq!(values.len(), 2);
            }
            RadrootsClientSqlParams::Named(_) => panic!("expected positional params"),
        }
    }
}
