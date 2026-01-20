use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::backup::RadrootsClientBackupDatastorePayload;
use crate::idb::RadrootsClientIdbConfig;

use super::RadrootsClientDatastoreError;

pub type RadrootsClientDatastoreValue = Option<String>;
pub type RadrootsClientDatastoreResult<T> = Result<T, RadrootsClientDatastoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientDatastoreEntry {
    pub key: String,
    pub value: RadrootsClientDatastoreValue,
}

impl RadrootsClientDatastoreEntry {
    pub fn new(key: impl Into<String>, value: RadrootsClientDatastoreValue) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

pub type RadrootsClientDatastoreEntries = Vec<RadrootsClientDatastoreEntry>;

#[async_trait(?Send)]
pub trait RadrootsClientDatastore {
    fn get_config(&self) -> RadrootsClientIdbConfig;
    fn get_store_id(&self) -> &str;

    async fn init(&self) -> RadrootsClientDatastoreResult<()>;
    async fn set(&self, key: &str, value: &str) -> RadrootsClientDatastoreResult<String>;
    async fn get(&self, key: &str) -> RadrootsClientDatastoreResult<String>;
    async fn set_obj<T>(&self, key: &str, value: &T) -> RadrootsClientDatastoreResult<T>
    where
        T: Serialize + DeserializeOwned + Clone;
    async fn update_obj<T>(&self, key: &str, value: &T) -> RadrootsClientDatastoreResult<T>
    where
        T: Serialize + DeserializeOwned + Clone;
    async fn get_obj<T>(&self, key: &str) -> RadrootsClientDatastoreResult<T>
    where
        T: DeserializeOwned;
    async fn del_obj(&self, key: &str) -> RadrootsClientDatastoreResult<String>;
    async fn del(&self, key: &str) -> RadrootsClientDatastoreResult<String>;
    async fn del_pref(&self, key_prefix: &str) -> RadrootsClientDatastoreResult<Vec<String>>;
    async fn set_param(
        &self,
        key: &str,
        key_param: &str,
        value: &str,
    ) -> RadrootsClientDatastoreResult<String>;
    async fn get_param(
        &self,
        key: &str,
        key_param: &str,
    ) -> RadrootsClientDatastoreResult<String>;
    async fn keys(&self) -> RadrootsClientDatastoreResult<Vec<String>>;
    async fn entries(&self) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries>;
    async fn entries_pref(
        &self,
        key_prefix: &str,
    ) -> RadrootsClientDatastoreResult<RadrootsClientDatastoreEntries>;
    async fn reset(&self) -> RadrootsClientDatastoreResult<()>;
    async fn export_backup(
        &self,
    ) -> RadrootsClientDatastoreResult<RadrootsClientBackupDatastorePayload>;
    async fn import_backup(
        &self,
        payload: RadrootsClientBackupDatastorePayload,
    ) -> RadrootsClientDatastoreResult<()>;
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientDatastoreEntry;

    #[test]
    fn entry_builder_preserves_values() {
        let entry = RadrootsClientDatastoreEntry::new("key", Some(String::from("value")));
        assert_eq!(entry.key, "key");
        assert_eq!(entry.value.as_deref(), Some("value"));
    }
}
