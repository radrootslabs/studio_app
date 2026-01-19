#![forbid(unsafe_code)]

pub mod crypto;
pub mod cipher;
pub mod backup;
pub mod datastore;
pub mod fs;
pub mod geolocation;
pub mod idb;
pub mod keystore;
pub mod notifications;
pub mod radroots;
#[cfg(not(target_arch = "wasm32"))]
pub mod sql;
#[cfg(not(target_arch = "wasm32"))]
pub mod tangle;
