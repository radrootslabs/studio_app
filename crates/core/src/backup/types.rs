use async_trait::async_trait;

use crate::crypto::RadrootsClientCryptoRegistryExport;

pub type RadrootsClientBackupBundleVersion = u8;

pub const RADROOTS_CLIENT_BACKUP_BUNDLE_VERSION: RadrootsClientBackupBundleVersion = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientBackupBundleStoreType {
    Sql,
    Keystore,
    Datastore,
}

impl RadrootsClientBackupBundleStoreType {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsClientBackupBundleStoreType::Sql => "sql",
            RadrootsClientBackupBundleStoreType::Keystore => "keystore",
            RadrootsClientBackupBundleStoreType::Datastore => "datastore",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "sql" => Some(RadrootsClientBackupBundleStoreType::Sql),
            "keystore" => Some(RadrootsClientBackupBundleStoreType::Keystore),
            "datastore" => Some(RadrootsClientBackupBundleStoreType::Datastore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupSqlPayload {
    pub bytes_b64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupKeystoreEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupKeystorePayload {
    pub entries: Vec<RadrootsClientBackupKeystoreEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupDatastoreEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupDatastorePayload {
    pub entries: Vec<RadrootsClientBackupDatastoreEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsClientBackupBundlePayload {
    Sql {
        store_id: String,
        data: RadrootsClientBackupSqlPayload,
    },
    Keystore {
        store_id: String,
        data: RadrootsClientBackupKeystorePayload,
    },
    Datastore {
        store_id: String,
        data: RadrootsClientBackupDatastorePayload,
    },
}

impl RadrootsClientBackupBundlePayload {
    pub fn store_type(&self) -> RadrootsClientBackupBundleStoreType {
        match self {
            RadrootsClientBackupBundlePayload::Sql { .. } => {
                RadrootsClientBackupBundleStoreType::Sql
            }
            RadrootsClientBackupBundlePayload::Keystore { .. } => {
                RadrootsClientBackupBundleStoreType::Keystore
            }
            RadrootsClientBackupBundlePayload::Datastore { .. } => {
                RadrootsClientBackupBundleStoreType::Datastore
            }
        }
    }

    pub fn store_id(&self) -> &str {
        match self {
            RadrootsClientBackupBundlePayload::Sql { store_id, .. } => store_id,
            RadrootsClientBackupBundlePayload::Keystore { store_id, .. } => store_id,
            RadrootsClientBackupBundlePayload::Datastore { store_id, .. } => store_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupStoreRef {
    pub store_id: String,
    pub store_type: RadrootsClientBackupBundleStoreType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupBundleManifest {
    pub version: RadrootsClientBackupBundleVersion,
    pub created_at: u64,
    pub app_version: Option<String>,
    pub stores: Vec<RadrootsClientBackupStoreRef>,
    pub crypto_registry: RadrootsClientCryptoRegistryExport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupBundle {
    pub manifest: RadrootsClientBackupBundleManifest,
    pub payloads: Vec<RadrootsClientBackupBundlePayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsClientBackupBundleEnvelope {
    pub version: u8,
    pub created_at: u64,
    pub kdf_salt_b64: String,
    pub kdf_iterations: u32,
    pub iv_b64: String,
    pub ciphertext_b64: String,
}

#[async_trait(?Send)]
pub trait RadrootsClientBackupSqlStore {
    type Error;

    async fn export_backup(&self) -> Result<RadrootsClientBackupSqlPayload, Self::Error>;
    async fn import_backup(&self, payload: RadrootsClientBackupSqlPayload)
        -> Result<(), Self::Error>;
    fn store_id(&self) -> &str;
}

#[async_trait(?Send)]
pub trait RadrootsClientBackupKeystoreStore {
    type Error;

    async fn export_backup(&self) -> Result<RadrootsClientBackupKeystorePayload, Self::Error>;
    async fn import_backup(
        &self,
        payload: RadrootsClientBackupKeystorePayload,
    ) -> Result<(), Self::Error>;
    fn store_id(&self) -> &str;
}

#[async_trait(?Send)]
pub trait RadrootsClientBackupDatastoreStore {
    type Error;

    async fn export_backup(&self) -> Result<RadrootsClientBackupDatastorePayload, Self::Error>;
    async fn import_backup(
        &self,
        payload: RadrootsClientBackupDatastorePayload,
    ) -> Result<(), Self::Error>;
    fn store_id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::{
        RadrootsClientBackupBundlePayload,
        RadrootsClientBackupBundleStoreType,
    };

    #[test]
    fn store_type_roundtrip() {
        let sql = RadrootsClientBackupBundleStoreType::Sql;
        let keystore = RadrootsClientBackupBundleStoreType::Keystore;
        let datastore = RadrootsClientBackupBundleStoreType::Datastore;

        assert_eq!(sql.as_str(), "sql");
        assert_eq!(keystore.as_str(), "keystore");
        assert_eq!(datastore.as_str(), "datastore");
        assert_eq!(RadrootsClientBackupBundleStoreType::parse("sql"), Some(sql));
        assert_eq!(
            RadrootsClientBackupBundleStoreType::parse("keystore"),
            Some(keystore)
        );
        assert_eq!(
            RadrootsClientBackupBundleStoreType::parse("datastore"),
            Some(datastore)
        );
        assert_eq!(RadrootsClientBackupBundleStoreType::parse("other"), None);
    }

    #[test]
    fn payload_store_helpers() {
        let payload = RadrootsClientBackupBundlePayload::Sql {
            store_id: "store".to_string(),
            data: super::RadrootsClientBackupSqlPayload {
                bytes_b64: "bytes".to_string(),
            },
        };
        assert_eq!(payload.store_id(), "store");
        assert_eq!(
            payload.store_type(),
            RadrootsClientBackupBundleStoreType::Sql
        );
    }
}
