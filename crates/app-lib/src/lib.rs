#![forbid(unsafe_code)]

pub mod browser;
pub mod fetch;
pub mod geo;
pub mod locale;
pub mod path;
pub mod query;
pub mod sleep;
pub mod storage;
pub mod symbols;
pub mod theme;

pub use browser::{browser_platform, BrowserPlatformInfo};
pub use fetch::{fetch_json, FetchJsonError, FetchJsonErrorKind, FetchJsonResult};
pub use geo::{geop_init, geop_is_valid, AppGeolocationPoint};
pub use locale::{get_locale, resolve_locale};
pub use path::{normalize_path, sanitize_path, trim_slashes};
pub use query::{encode_query_params, encode_route};
pub use sleep::sleep;
pub use storage::{build_storage_key, build_storage_key_with_prefix, fmt_id, fmt_id_from_path};
pub use symbols::{
    fmt_cl, value_constrain, SYMBOL_BULLET, SYMBOL_DASH, SYMBOL_DOWN, SYMBOL_PERCENT, SYMBOL_UP,
};
pub use theme::{
    get_system_theme, parse_layer, theme_set, ThemeError, ThemeLayer, ThemeMode,
};
