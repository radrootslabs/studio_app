#![forbid(unsafe_code)]

pub mod error;
pub mod types;

pub use types::{resolve_err, resolve_ok, ResolveError, ResultPass};
