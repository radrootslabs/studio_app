#![forbid(unsafe_code)]

pub mod browser;
pub mod fetch;
pub mod geo;
pub mod path;
pub mod sleep;
pub mod symbols;

pub use browser::{browser_platform, BrowserPlatformInfo};
pub use fetch::{fetch_json, FetchJsonError, FetchJsonErrorKind, FetchJsonResult};
pub use geo::{geop_init, geop_is_valid, AppGeolocationPoint};
pub use path::{normalize_path, sanitize_path, trim_slashes};
pub use sleep::sleep;
pub use symbols::{
    fmt_cl, value_constrain, SYMBOL_BULLET, SYMBOL_DASH, SYMBOL_DOWN, SYMBOL_PERCENT, SYMBOL_UP,
};
