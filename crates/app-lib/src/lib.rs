#![forbid(unsafe_code)]

pub mod browser;
pub mod geo;

pub use browser::{browser_platform, BrowserPlatformInfo};
pub use geo::{geop_init, geop_is_valid, AppGeolocationPoint};
