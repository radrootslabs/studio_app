#![forbid(unsafe_code)]

pub mod error;
pub mod errors;
pub mod text;
pub mod types;

pub use errors::{err_msg, handle_err, throw_err, ERR_PREFIX_APP, ERR_PREFIX_UTILS};
pub use text::ROOT_SYMBOL;
pub use types::{resolve_err, resolve_ok, ResolveError, ResultPass};
