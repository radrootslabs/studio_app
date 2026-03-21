#![allow(unsafe_code)]

mod security;
mod vault;

pub use security::{APPLE_NOSTR_NAMESPACE, APPLE_NOSTR_SERVICE, verify_user_presence};
pub use vault::RadrootsAppleKeychainVault;
