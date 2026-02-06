#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use radroots_studio_app_core::datastore::{RadrootsClientDatastore, RadrootsClientDatastoreError};

use crate::{
    app_datastore_obj_key_config,
    app_state_timestamp_ms,
    RadrootsAppConfigError,
    RadrootsAppKeyMapConfig,
    RadrootsAppRole,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigProfile {
    pub name: String,
    pub location: String,
}

impl Default for RadrootsAppConfigProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            location: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigPreferences {
    pub notifications_orders: bool,
    pub notifications_messages: bool,
    pub payment_method: Option<String>,
}

impl Default for RadrootsAppConfigPreferences {
    fn default() -> Self {
        Self {
            notifications_orders: true,
            notifications_messages: true,
            payment_method: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigFarmer {
    pub farm_name: String,
    pub farm_location: String,
    pub products_growing: Vec<String>,
}

impl Default for RadrootsAppConfigFarmer {
    fn default() -> Self {
        Self {
            farm_name: String::new(),
            farm_location: String::new(),
            products_growing: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigIndividual {
    pub name: String,
    pub location: String,
    pub products_interested: Vec<String>,
}

impl Default for RadrootsAppConfigIndividual {
    fn default() -> Self {
        Self {
            name: String::new(),
            location: String::new(),
            products_interested: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigBusiness {
    pub name: String,
    pub location: String,
    pub operations: String,
}

impl Default for RadrootsAppConfigBusiness {
    fn default() -> Self {
        Self {
            name: String::new(),
            location: String::new(),
            operations: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigData {
    pub profile: RadrootsAppConfigProfile,
    pub role: RadrootsAppRole,
    pub farmer: Option<RadrootsAppConfigFarmer>,
    pub business: Option<RadrootsAppConfigBusiness>,
    pub individual: Option<RadrootsAppConfigIndividual>,
    pub preferences: RadrootsAppConfigPreferences,
}

impl Default for RadrootsAppConfigData {
    fn default() -> Self {
        Self {
            profile: RadrootsAppConfigProfile::default(),
            role: RadrootsAppRole::default(),
            farmer: None,
            business: None,
            individual: None,
            preferences: RadrootsAppConfigPreferences::default(),
        }
    }
}

pub const APP_CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppConfigRecordError {
    Missing,
    Corrupt,
    InvalidChecksum,
    UnsupportedVersion(u32),
    AlreadyExists,
}

impl RadrootsAppConfigRecordError {
    pub const fn message(&self) -> &'static str {
        match self {
            RadrootsAppConfigRecordError::Missing => "error.app.config.missing",
            RadrootsAppConfigRecordError::Corrupt => "error.app.config.corrupt",
            RadrootsAppConfigRecordError::InvalidChecksum => "error.app.config.checksum_invalid",
            RadrootsAppConfigRecordError::UnsupportedVersion(_) => "error.app.config.schema_unsupported",
            RadrootsAppConfigRecordError::AlreadyExists => "error.app.config.already_exists",
        }
    }
}

impl std::fmt::Display for RadrootsAppConfigRecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsAppConfigRecordError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadrootsAppConfigRecord {
    pub schema_version: u32,
    pub revision: u64,
    pub updated_at_ms: i64,
    pub checksum: String,
    pub config: RadrootsAppConfigData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RadrootsAppConfigChecksumPayload {
    schema_version: u32,
    revision: u64,
    updated_at_ms: i64,
    config: RadrootsAppConfigData,
}

fn app_config_record_checksum(payload: &RadrootsAppConfigChecksumPayload) -> String {
    let serialized = serde_json::to_vec(payload).unwrap_or_else(|_| Vec::new());
    let hash = Sha256::digest(&serialized);
    hex::encode(hash)
}

pub fn app_config_record_new(
    config: RadrootsAppConfigData,
    revision: u64,
    updated_at_ms: i64,
) -> RadrootsAppConfigRecord {
    let payload = RadrootsAppConfigChecksumPayload {
        schema_version: APP_CONFIG_SCHEMA_VERSION,
        revision,
        updated_at_ms,
        config: config.clone(),
    };
    let checksum = app_config_record_checksum(&payload);
    RadrootsAppConfigRecord {
        schema_version: APP_CONFIG_SCHEMA_VERSION,
        revision,
        updated_at_ms,
        checksum,
        config,
    }
}

pub fn app_config_record_validate(
    record: &RadrootsAppConfigRecord,
) -> Result<(), RadrootsAppConfigRecordError> {
    if record.schema_version != APP_CONFIG_SCHEMA_VERSION {
        return Err(RadrootsAppConfigRecordError::UnsupportedVersion(
            record.schema_version,
        ));
    }
    let payload = RadrootsAppConfigChecksumPayload {
        schema_version: record.schema_version,
        revision: record.revision,
        updated_at_ms: record.updated_at_ms,
        config: record.config.clone(),
    };
    let expected = app_config_record_checksum(&payload);
    if record.checksum != expected {
        return Err(RadrootsAppConfigRecordError::InvalidChecksum);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppConfigStoreError {
    Datastore(RadrootsClientDatastoreError),
    Config(RadrootsAppConfigError),
    Record(RadrootsAppConfigRecordError),
}

impl std::fmt::Display for RadrootsAppConfigStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RadrootsAppConfigStoreError::Datastore(err) => write!(f, "{err}"),
            RadrootsAppConfigStoreError::Config(err) => write!(f, "{err}"),
            RadrootsAppConfigStoreError::Record(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RadrootsAppConfigStoreError {}

pub type RadrootsAppConfigStoreResult<T> = Result<T, RadrootsAppConfigStoreError>;

pub async fn app_datastore_write_config_record<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    record: &RadrootsAppConfigRecord,
) -> RadrootsAppConfigStoreResult<RadrootsAppConfigRecord> {
    let key = app_datastore_obj_key_config(key_maps).map_err(RadrootsAppConfigStoreError::Config)?;
    let value = datastore
        .set_obj(key, record)
        .await
        .map_err(RadrootsAppConfigStoreError::Datastore)?;
    Ok(value)
}

pub async fn app_datastore_read_config_record<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppConfigStoreResult<RadrootsAppConfigRecord> {
    let key = app_datastore_obj_key_config(key_maps).map_err(RadrootsAppConfigStoreError::Config)?;
    match datastore.get_obj::<RadrootsAppConfigRecord>(key).await {
        Ok(record) => {
            app_config_record_validate(&record).map_err(RadrootsAppConfigStoreError::Record)?;
            Ok(record)
        }
        Err(RadrootsClientDatastoreError::NoResult) => {
            Err(RadrootsAppConfigStoreError::Record(RadrootsAppConfigRecordError::Missing))
        }
        Err(err) => Err(RadrootsAppConfigStoreError::Datastore(err)),
    }
}

pub async fn app_datastore_create_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    config: &RadrootsAppConfigData,
) -> RadrootsAppConfigStoreResult<RadrootsAppConfigData> {
    let now_ms = app_state_timestamp_ms();
    match app_datastore_read_config_record(datastore, key_maps).await {
        Ok(_) => Err(RadrootsAppConfigStoreError::Record(
            RadrootsAppConfigRecordError::AlreadyExists,
        )),
        Err(RadrootsAppConfigStoreError::Record(RadrootsAppConfigRecordError::Missing)) => {
            let record = app_config_record_new(config.clone(), 1, now_ms);
            let value = app_datastore_write_config_record(datastore, key_maps, &record).await?;
            Ok(value.config)
        }
        Err(err) => Err(err),
    }
}

pub async fn app_datastore_update_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
    config: &RadrootsAppConfigData,
) -> RadrootsAppConfigStoreResult<RadrootsAppConfigData> {
    let now_ms = app_state_timestamp_ms();
    let record = match app_datastore_read_config_record(datastore, key_maps).await {
        Ok(existing) => app_config_record_new(config.clone(), existing.revision + 1, now_ms),
        Err(err) => return Err(err),
    };
    let value = app_datastore_write_config_record(datastore, key_maps, &record).await?;
    Ok(value.config)
}

pub async fn app_datastore_read_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppConfigStoreResult<RadrootsAppConfigData> {
    let record = app_datastore_read_config_record(datastore, key_maps).await?;
    Ok(record.config)
}

pub async fn app_datastore_has_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppConfigStoreResult<bool> {
    match app_datastore_read_config_record(datastore, key_maps).await {
        Ok(_) => Ok(true),
        Err(RadrootsAppConfigStoreError::Record(RadrootsAppConfigRecordError::Missing)) => Ok(false),
        Err(err) => Err(err),
    }
}

pub async fn app_datastore_clear_config<T: RadrootsClientDatastore>(
    datastore: &T,
    key_maps: &RadrootsAppKeyMapConfig,
) -> RadrootsAppConfigStoreResult<()> {
    let key = app_datastore_obj_key_config(key_maps).map_err(RadrootsAppConfigStoreError::Config)?;
    datastore
        .del_obj(key)
        .await
        .map_err(RadrootsAppConfigStoreError::Datastore)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        app_config_record_new,
        app_config_record_validate,
        RadrootsAppConfigData,
        RadrootsAppConfigRecordError,
        APP_CONFIG_SCHEMA_VERSION,
    };

    #[test]
    fn config_record_roundtrips() {
        let config = RadrootsAppConfigData::default();
        let record = app_config_record_new(config.clone(), 1, 1234);
        assert_eq!(record.schema_version, APP_CONFIG_SCHEMA_VERSION);
        assert_eq!(record.revision, 1);
        assert_eq!(record.updated_at_ms, 1234);
        assert_eq!(record.config, config);
        assert!(app_config_record_validate(&record).is_ok());
    }

    #[test]
    fn config_record_detects_invalid_checksum() {
        let config = RadrootsAppConfigData::default();
        let mut record = app_config_record_new(config, 1, 1234);
        record.checksum = String::from("invalid");
        let err = app_config_record_validate(&record).expect_err("checksum");
        assert_eq!(err, RadrootsAppConfigRecordError::InvalidChecksum);
    }
}
