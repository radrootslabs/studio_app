use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientDatastoreError {
    IdbUndefined,
    NoResult,
}

pub type RadrootsClientDatastoreErrorMessage = &'static str;

impl RadrootsClientDatastoreError {
    pub const fn message(self) -> RadrootsClientDatastoreErrorMessage {
        match self {
            RadrootsClientDatastoreError::IdbUndefined => "error.client.datastore.idb_undefined",
            RadrootsClientDatastoreError::NoResult => "error.client.datastore.no_result",
        }
    }
}

impl fmt::Display for RadrootsClientDatastoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for RadrootsClientDatastoreError {}

#[cfg(test)]
mod tests {
    use super::RadrootsClientDatastoreError;

    #[test]
    fn message_matches_spec() {
        let cases = [
            (
                RadrootsClientDatastoreError::IdbUndefined,
                "error.client.datastore.idb_undefined",
            ),
            (
                RadrootsClientDatastoreError::NoResult,
                "error.client.datastore.no_result",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
