#![allow(unsafe_code)]

mod security;
mod vault;

pub use security::{APPLE_NOSTR_NAMESPACE, APPLE_NOSTR_SERVICE};
pub use vault::RadrootsAppleKeychainVault;
