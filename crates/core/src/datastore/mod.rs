pub mod error;
pub mod types;
pub mod web;

pub use error::{RadrootsClientDatastoreError, RadrootsClientDatastoreErrorMessage};
pub use types::{
    RadrootsClientDatastore,
    RadrootsClientDatastoreEntries,
    RadrootsClientDatastoreEntry,
    RadrootsClientDatastoreResult,
    RadrootsClientDatastoreValue,
};
pub use web::RadrootsClientWebDatastore;
