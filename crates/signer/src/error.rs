use radroots_nostr_connect::prelude::{RadrootsNostrConnectError, RadrootsNostrConnectMethod};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppRemoteSignerError {
    EmptyInput,
    UnsupportedClientUri,
    MissingDiscoveryUri,
    InvalidDiscoveryUrl(String),
    InvalidBunkerUri(String),
    InvalidSessionStore(String),
    SessionStoreIo(String),
    PendingSessionExists,
    MissingClientSecret,
    ConnectFailed(String),
    RequestTimedOut {
        method: RadrootsNostrConnectMethod,
    },
    UnexpectedResponse {
        method: RadrootsNostrConnectMethod,
        response: String,
    },
}

impl fmt::Display for RadrootsAppRemoteSignerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => f.write_str("enter a bunker or discovery url to continue"),
            Self::UnsupportedClientUri => f.write_str(
                "enter a bunker or discovery url from the signer; raw nostrconnect client uris are signer-side only",
            ),
            Self::MissingDiscoveryUri => {
                f.write_str("discovery url does not contain a remote signer uri")
            }
            Self::InvalidDiscoveryUrl(reason) => write!(f, "invalid discovery url: {reason}"),
            Self::InvalidBunkerUri(reason) => write!(f, "invalid remote signer uri: {reason}"),
            Self::InvalidSessionStore(reason) => write!(f, "invalid remote signer store: {reason}"),
            Self::SessionStoreIo(reason) => write!(f, "remote signer storage failed: {reason}"),
            Self::PendingSessionExists => {
                f.write_str("a remote signer connection is already pending approval")
            }
            Self::MissingClientSecret => f.write_str("remote signer session secret is missing"),
            Self::ConnectFailed(reason) => write!(f, "remote signer connection failed: {reason}"),
            Self::RequestTimedOut { method } => {
                write!(f, "remote signer request `{method}` timed out")
            }
            Self::UnexpectedResponse { method, response } => {
                write!(f, "remote signer returned an unexpected `{method}` response: {response}")
            }
        }
    }
}

impl std::error::Error for RadrootsAppRemoteSignerError {}

impl From<RadrootsNostrConnectError> for RadrootsAppRemoteSignerError {
    fn from(value: RadrootsNostrConnectError) -> Self {
        Self::InvalidBunkerUri(value.to_string())
    }
}
