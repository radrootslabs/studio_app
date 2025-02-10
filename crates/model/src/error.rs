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
