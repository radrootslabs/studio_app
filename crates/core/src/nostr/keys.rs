use nostr_sdk::prelude::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NostrKeyError {
    #[error("Key error: {0}")]
    KeyError(#[from] nostr_sdk::key::Error),
}

pub type NostrKeyResult<T> = std::result::Result<T, NostrKeyError>;

pub fn lib_nostr_keys_gen() -> nostr_sdk::Keys {
    Keys::generate()
}

pub fn lib_nostr_keys_parse(secret_key: String) -> NostrKeyResult<nostr_sdk::Keys> {
    Keys::parse(&secret_key).map_err(NostrKeyError::KeyError)
}

pub fn lib_nostr_secret_key_hex(keys: nostr_sdk::Keys) -> String {
    keys.secret_key().to_secret_hex()
}

pub fn lib_nostr_public_key_hex(keys: nostr_sdk::Keys) -> String {
    keys.public_key().to_hex()
}
