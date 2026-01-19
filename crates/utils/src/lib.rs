#![forbid(unsafe_code)]

pub mod error;
pub mod errors;
pub mod numbers;
pub mod path;
pub mod text;
pub mod time;
pub mod types;

pub use errors::{err_msg, handle_err, throw_err, ERR_PREFIX_APP, ERR_PREFIX_UTILS};
pub use numbers::{num_str, parse_float, parse_int};
pub use path::{
    parse_route_path,
    resolve_route_path,
    resolve_wasm_path,
    RadrootsAppRoutePathParts,
};
pub use text::{str_cap, str_cap_words, text_dec, text_enc, ROOT_SYMBOL};
pub use time::{time_now_ms, time_now_s};
pub use types::{resolve_err, resolve_ok, ResolveError, ResultPass};
