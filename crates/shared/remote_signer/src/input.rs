use crate::error::RadrootsAppRemoteSignerError;
use radroots_identity::RadrootsIdentityPublic;
use radroots_nostr_connect::prelude::{
    RadrootsNostrConnectMethod, RadrootsNostrConnectPermission, RadrootsNostrConnectPermissions,
    RadrootsNostrConnectUri,
};
use radroots_nostr_connect::uri::RADROOTS_NOSTR_CONNECT_BUNKER_URI_SCHEME;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppRemoteSignerSource {
    BunkerUri,
    DiscoveryUrl,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppRemoteSignerTarget {
    pub source: RadrootsAppRemoteSignerSource,
    pub signer_identity: RadrootsIdentityPublic,
    pub relays: Vec<String>,
    pub connect_secret: Option<String>,
    pub requested_permissions: RadrootsNostrConnectPermissions,
}

impl RadrootsAppRemoteSignerTarget {
    pub fn source_label(&self) -> &'static str {
        match self.source {
            RadrootsAppRemoteSignerSource::BunkerUri => "bunker uri",
            RadrootsAppRemoteSignerSource::DiscoveryUrl => "discovery url",
        }
    }

    pub fn requested_permission_labels(&self) -> Vec<String> {
        self.requested_permissions
            .as_slice()
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

pub fn radroots_studio_app_remote_signer_requested_permissions() -> RadrootsNostrConnectPermissions {
    vec![
        RadrootsNostrConnectPermission::with_parameter(
            RadrootsNostrConnectMethod::SignEvent,
            "kind:1",
        ),
        RadrootsNostrConnectPermission::new(RadrootsNostrConnectMethod::SwitchRelays),
    ]
    .into()
}

pub fn radroots_studio_app_remote_signer_preview(
    input: &str,
) -> Result<RadrootsAppRemoteSignerTarget, RadrootsAppRemoteSignerError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RadrootsAppRemoteSignerError::EmptyInput);
    }

    if trimmed.starts_with(&format!("{RADROOTS_NOSTR_CONNECT_BUNKER_URI_SCHEME}://")) {
        return parse_bunker_uri(trimmed, RadrootsAppRemoteSignerSource::BunkerUri);
    }

    if trimmed.starts_with("nostrconnect://") {
        return Err(RadrootsAppRemoteSignerError::UnsupportedClientUri);
    }

    parse_discovery_url(trimmed)
}

fn parse_discovery_url(
    value: &str,
) -> Result<RadrootsAppRemoteSignerTarget, RadrootsAppRemoteSignerError> {
    let url = Url::parse(value)
        .map_err(|error| RadrootsAppRemoteSignerError::InvalidDiscoveryUrl(error.to_string()))?;
    let Some((_, bunker_uri)) = url.query_pairs().find(|(key, _)| key == "uri") else {
        return Err(RadrootsAppRemoteSignerError::MissingDiscoveryUri);
    };
    parse_bunker_uri(
        bunker_uri.as_ref(),
        RadrootsAppRemoteSignerSource::DiscoveryUrl,
    )
}

fn parse_bunker_uri(
    value: &str,
    source: RadrootsAppRemoteSignerSource,
) -> Result<RadrootsAppRemoteSignerTarget, RadrootsAppRemoteSignerError> {
    let uri = RadrootsNostrConnectUri::parse(value)?;
    let RadrootsNostrConnectUri::Bunker(bunker_uri) = uri else {
        return Err(RadrootsAppRemoteSignerError::UnsupportedClientUri);
    };
    Ok(RadrootsAppRemoteSignerTarget {
        source,
        signer_identity: RadrootsIdentityPublic::new(bunker_uri.remote_signer_public_key),
        relays: bunker_uri
            .relays
            .into_iter()
            .map(|relay| relay.to_string())
            .collect(),
        connect_secret: bunker_uri.secret,
        requested_permissions: radroots_studio_app_remote_signer_requested_permissions(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use radroots_identity::RadrootsIdentity;

    const RELAY_PRIMARY_WSS: &str = "wss://relay.example.com";
    const SIGNER_SECRET_KEY_HEX: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    fn signer_identity() -> RadrootsIdentity {
        RadrootsIdentity::from_secret_key_str(SIGNER_SECRET_KEY_HEX).expect("identity")
    }

    fn bunker_uri() -> String {
        format!(
            "bunker://{}?relay={}",
            signer_identity().public_key_hex(),
            urlencoding(RELAY_PRIMARY_WSS)
        )
    }

    fn discovery_url() -> String {
        format!(
            "http://localhost/connect?uri={}",
            urlencoding(bunker_uri().as_str())
        )
    }

    fn urlencoding(value: &str) -> String {
        url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
    }

    #[test]
    fn parses_direct_bunker_uri() {
        let preview = radroots_studio_app_remote_signer_preview(bunker_uri().as_str()).expect("preview");

        assert_eq!(preview.source, RadrootsAppRemoteSignerSource::BunkerUri);
        assert_eq!(
            preview.signer_identity.public_key_hex,
            signer_identity().public_key_hex()
        );
        assert_eq!(preview.relays, vec![RELAY_PRIMARY_WSS.to_owned()]);
        assert_eq!(preview.connect_secret, None);
        assert_eq!(
            preview.requested_permission_labels(),
            vec!["sign_event:kind:1".to_owned(), "switch_relays".to_owned()]
        );
    }

    #[test]
    fn parses_discovery_url_with_bunker_uri() {
        let preview =
            radroots_studio_app_remote_signer_preview(discovery_url().as_str()).expect("preview");

        assert_eq!(preview.source, RadrootsAppRemoteSignerSource::DiscoveryUrl);
        assert_eq!(
            preview.signer_identity.public_key_hex,
            signer_identity().public_key_hex()
        );
        assert_eq!(preview.relays, vec![RELAY_PRIMARY_WSS.to_owned()]);
        assert_eq!(
            preview.requested_permission_labels(),
            vec!["sign_event:kind:1".to_owned(), "switch_relays".to_owned()]
        );
    }

    #[test]
    fn rejects_client_side_nostrconnect_uri_input() {
        let err = radroots_studio_app_remote_signer_preview(
            "nostrconnect://npub1test?relay=wss%3A%2F%2Frelay.example.com&secret=test",
        )
        .expect_err("client uri rejected");

        assert_eq!(err, RadrootsAppRemoteSignerError::UnsupportedClientUri);
    }
}
