use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppSqliteError {
    #[error("failed to create sqlite parent directory `{path}`")]
    CreateParentDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to open sqlite database at `{path}`")]
    OpenPath {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to open in-memory sqlite database")]
    OpenInMemory {
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to configure sqlite busy timeout")]
    ConfigureBusyTimeout {
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to apply sqlite pragma `{pragma}`")]
    ApplyPragma {
        pragma: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to read sqlite schema version")]
    ReadSchemaVersion {
        #[source]
        source: rusqlite::Error,
    },
    #[error(
        "sqlite schema version {current} is newer than supported version {latest}; manual migration is required"
    )]
    UnsupportedSchemaVersion { current: u32, latest: u32 },
    #[error("failed to begin sqlite migration transaction for version {version}")]
    BeginMigration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to execute sqlite migration {version}")]
    ExecuteMigration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to record sqlite schema version {version}")]
    RecordSchemaVersion {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to commit sqlite migration {version}")]
    CommitMigration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },
    #[error("failed to execute sqlite query for {operation}")]
    Query {
        operation: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error("invalid sqlite id in `{field}`: `{value}`")]
    DecodeId { field: &'static str, value: String },
    #[error("invalid sqlite enum value in `{field}`: `{value}`")]
    DecodeEnum { field: &'static str, value: String },
}
