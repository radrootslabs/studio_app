#![forbid(unsafe_code)]

pub mod browser;
pub mod fetch;
pub mod geo;

pub use browser::{browser_platform, BrowserPlatformInfo};
pub use fetch::{fetch_json, FetchJsonError, FetchJsonErrorKind, FetchJsonResult};
pub use geo::{geop_init, geop_is_valid, AppGeolocationPoint};
