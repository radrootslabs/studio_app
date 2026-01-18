use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsClientSqlError {
    IdbUndefined,
}

pub type RadrootsClientSqlErrorMessage = &'static str;

impl RadrootsClientSqlError {
    pub const fn message(self) -> RadrootsClientSqlErrorMessage {
        match self {
            RadrootsClientSqlError::IdbUndefined => "error.client.sql.idb_undefined",
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
        let cases = [(RadrootsClientSqlError::IdbUndefined, "error.client.sql.idb_undefined")];
        for (err, expected) in cases {
            assert_eq!(err.message(), expected);
            assert_eq!(err.to_string(), expected);
        }
    }
}
