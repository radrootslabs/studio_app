#![forbid(unsafe_code)]

pub mod error;
pub mod errors;
pub mod r#async;
pub mod binary;
pub mod id;
pub mod numbers;
pub mod object;
pub mod path;
pub mod text;
pub mod time;
pub mod types;

pub use r#async::exe_iter;
pub use binary::{as_array_buffer, RadrootsAppArrayBuffer};
pub use id::{d_tag_create, uuidv4, uuidv4_b64url, uuidv7, uuidv7_b64url};
pub use errors::{err_msg, handle_err, throw_err, ERR_PREFIX_APP, ERR_PREFIX_UTILS};
pub use numbers::{num_interval_range, num_str, parse_float, parse_int};
pub use object::{obj_en, obj_result, obj_results_str, obj_truthy_fields};
pub use path::{
    parse_route_path,
    resolve_route_path,
    resolve_wasm_path,
    RadrootsAppRoutePathParts,
};
pub use text::{str_cap, str_cap_words, text_dec, text_enc, ROOT_SYMBOL};
pub use time::{time_now_ms, time_now_s};
pub use types::{resolve_err, resolve_ok, ResolveError, ResultPass};
