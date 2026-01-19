use async_trait::async_trait;

use crate::backup::RadrootsClientBackupKeystorePayload;

use super::RadrootsClientKeystoreError;

pub type RadrootsClientKeystoreValue = Option<String>;
pub type RadrootsClientKeystoreResult<T> = Result<T, RadrootsClientKeystoreError>;

#[async_trait(?Send)]
pub trait RadrootsClientKeystore {
    async fn add(&self, key: &str, value: &str) -> RadrootsClientKeystoreResult<String>;
    async fn remove(&self, key: &str) -> RadrootsClientKeystoreResult<String>;
    async fn read(&self, key: Option<&str>) -> RadrootsClientKeystoreResult<RadrootsClientKeystoreValue>;
    async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>>;
    async fn reset(&self) -> RadrootsClientKeystoreResult<()>;
    fn get_store_id(&self) -> &str;
    async fn export_backup(
        &self,
    ) -> RadrootsClientKeystoreResult<RadrootsClientBackupKeystorePayload>;
    async fn import_backup(
        &self,
        payload: RadrootsClientBackupKeystorePayload,
    ) -> RadrootsClientKeystoreResult<()>;
}

#[async_trait(?Send)]
pub trait RadrootsClientKeystoreNostr {
    async fn generate(&self) -> RadrootsClientKeystoreResult<String>;
    async fn add(&self, secret_key: &str) -> RadrootsClientKeystoreResult<String>;
    async fn read(&self, public_key: &str) -> RadrootsClientKeystoreResult<String>;
    async fn keys(&self) -> RadrootsClientKeystoreResult<Vec<String>>;
    async fn remove(&self, public_key: &str) -> RadrootsClientKeystoreResult<String>;
    async fn reset(&self) -> RadrootsClientKeystoreResult<()>;
}

#[cfg(test)]
mod tests {
    use super::RadrootsClientKeystoreValue;

    #[test]
    fn keystore_value_allows_none() {
        let value: RadrootsClientKeystoreValue = None;
        assert!(value.is_none());
    }
}
