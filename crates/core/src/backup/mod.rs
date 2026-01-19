pub mod error;
pub mod types;
pub mod codec;
pub mod bundle;

pub use error::{RadrootsClientBackupError, RadrootsClientBackupErrorMessage};
pub use types::{
    RadrootsClientBackupBundle,
    RadrootsClientBackupBundleEnvelope,
    RadrootsClientBackupBundleManifest,
    RadrootsClientBackupBundlePayload,
    RadrootsClientBackupBundleStoreType,
    RadrootsClientBackupBundleVersion,
    RadrootsClientBackupDatastoreEntry,
    RadrootsClientBackupDatastorePayload,
    RadrootsClientBackupDatastoreStore,
    RadrootsClientBackupKeystoreEntry,
    RadrootsClientBackupKeystorePayload,
    RadrootsClientBackupKeystoreStore,
    RadrootsClientBackupSqlPayload,
    RadrootsClientBackupSqlStore,
    RadrootsClientBackupStoreRef,
    RADROOTS_CLIENT_BACKUP_BUNDLE_VERSION,
};
pub use bundle::{
    backup_bundle_build,
    backup_bundle_export,
    backup_bundle_import,
    RadrootsClientBackupBundleBuildOpts,
    RadrootsClientBackupBundleError,
    RadrootsClientBackupBundleImportOpts,
    RadrootsClientBackupBundleResult,
};
pub use codec::{
    backup_b64_to_bytes,
    backup_bytes_to_b64,
    backup_bundle_decode,
    backup_bundle_encode,
};
