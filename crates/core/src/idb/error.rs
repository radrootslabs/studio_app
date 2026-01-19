use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientIdbStoreError {
    IdbUndefined,
    OperationFailure,
    VersionError,
}

pub type RadrootsClientIdbStoreErrorMessage = &'static str;

impl RadrootsClientIdbStoreError {
    pub const fn message(self) -> RadrootsClientIdbStoreErrorMessage {
        match self {
            RadrootsClientIdbStoreError::IdbUndefined => "error.client.idb.idb_undefined",
            RadrootsClientIdbStoreError::OperationFailure => {
                "error.client.idb.operation_failure"
            }
            RadrootsClientIdbStoreError::VersionError => "error.client.idb.version_error",
        }
    }
}

impl fmt::Display for RadrootsClientIdbStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientIdbStoreError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientIdbStoreError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientIdbStoreError::IdbUndefined,
                "error.client.idb.idb_undefined",
            ),
            (
                RadrootsClientIdbStoreError::OperationFailure,
                "error.client.idb.operation_failure",
            ),
            (
                RadrootsClientIdbStoreError::VersionError,
                "error.client.idb.version_error",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
