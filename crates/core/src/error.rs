use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ModelError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Internal error")]
    Internal,
}

impl From<ModelError> for String {
    fn from(err: ModelError) -> Self {
        err.to_string()
    }
}

#[derive(Error, Debug, Clone)]
pub enum KeyringError {
    #[error("Internal error")]
    Internal,
    #[error("Keyring error: {0}")]
    KeyringError(String),
    #[error("Parsing error: {0}")]
    ParsingError(String),
    #[error("Missing secret key")]
    MissingSecretKey,
}

impl From<KeyringError> for String {
    fn from(err: KeyringError) -> Self {
        err.to_string()
    }
}
