use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientSqlError {
    IdbUndefined,
    EngineUnavailable,
    InvalidParams,
    QueryFailure,
    ExportFailure,
    ImportFailure,
    BackupFailure,
}

pub type RadrootsClientSqlErrorMessage = &'static str;

impl RadrootsClientSqlError {
    pub const fn message(self) -> RadrootsClientSqlErrorMessage {
        match self {
            RadrootsClientSqlError::IdbUndefined => "error.client.sql.idb_undefined",
            RadrootsClientSqlError::EngineUnavailable => {
                "error.client.sql.engine_unavailable"
            }
            RadrootsClientSqlError::InvalidParams => "error.client.sql.invalid_params",
            RadrootsClientSqlError::QueryFailure => "error.client.sql.query_failure",
            RadrootsClientSqlError::ExportFailure => "error.client.sql.export_failure",
            RadrootsClientSqlError::ImportFailure => "error.client.sql.import_failure",
            RadrootsClientSqlError::BackupFailure => "error.client.sql.backup_failure",
        }
    }
}

impl fmt::Display for RadrootsClientSqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientSqlError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientSqlError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientSqlError::IdbUndefined,
                "error.client.sql.idb_undefined",
            ),
            (
                RadrootsClientSqlError::EngineUnavailable,
                "error.client.sql.engine_unavailable",
            ),
            (
                RadrootsClientSqlError::InvalidParams,
                "error.client.sql.invalid_params",
            ),
            (
                RadrootsClientSqlError::QueryFailure,
                "error.client.sql.query_failure",
            ),
            (
                RadrootsClientSqlError::ExportFailure,
                "error.client.sql.export_failure",
            ),
            (
                RadrootsClientSqlError::ImportFailure,
                "error.client.sql.import_failure",
            ),
            (
                RadrootsClientSqlError::BackupFailure,
                "error.client.sql.backup_failure",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
