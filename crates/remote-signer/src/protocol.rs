use crate::error::RadrootsAppRemoteSignerError;
use crate::input::{RadrootsAppRemoteSignerTarget, radroots_studio_app_remote_signer_preview};
use crate::session::RadrootsAppRemoteSignerSessionRecord;
use nostr::nips::nip44;
use nostr::nips::nip44::Version;
use radroots_identity::{RadrootsIdentity, RadrootsIdentityPublic};
use radroots_nostr::prelude::{
    RadrootsNostrClient, RadrootsNostrEvent, RadrootsNostrEventBuilder, RadrootsNostrFilter,
    RadrootsNostrKind, RadrootsNostrRelayPoolNotification, RadrootsNostrTag,
    RadrootsNostrTimestamp, radroots_nostr_filter_tag, radroots_nostr_kind,
};
use radroots_nostr_connect::message::RADROOTS_NOSTR_CONNECT_RPC_KIND;
use radroots_nostr_connect::prelude::{
    RadrootsNostrConnectMethod, RadrootsNostrConnectPendingConnectionPollOutcome,
    RadrootsNostrConnectRequest, RadrootsNostrConnectRequestMessage, RadrootsNostrConnectResponse,
    RadrootsNostrConnectResponseEnvelope,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use tokio::time::timeout;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GET_PUBLIC_KEY_TIMEOUT: Duration = Duration::from_secs(10);
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct RadrootsAppRemoteSignerPendingSession {
    pub record: RadrootsAppRemoteSignerSessionRecord,
    pub client_secret_key_hex: String,
}

#[derive(Debug, Clone)]
pub enum RadrootsAppRemoteSignerPendingPollOutcome {
    PendingApproval,
    Approved(RadrootsIdentityPublic),
    TransportFailure { message: String },
    Rejected { message: String },
    FatalError { message: String },
}

pub fn radroots_studio_app_remote_signer_connect_pending(
    input: &str,
) -> Result<RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerError> {
    let target = radroots_studio_app_remote_signer_preview(input)?;
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    runtime.block_on(connect_pending_session(target))
}

pub fn radroots_studio_app_remote_signer_poll_pending_session(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError> {
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    runtime.block_on(poll_pending_session(record, client_secret_key_hex))
}

async fn connect_pending_session(
    target: RadrootsAppRemoteSignerTarget,
) -> Result<RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerError> {
    let client_identity = RadrootsIdentity::generate();
    let response = execute_request(
        &client_identity,
        &target,
        RadrootsNostrConnectMethod::Connect,
        RadrootsNostrConnectRequest::Connect {
            remote_signer_public_key: parse_public_key_hex(
                target.signer_identity.public_key_hex.as_str(),
            )?,
            secret: target.connect_secret.clone(),
            requested_permissions: Default::default(),
        },
        CONNECT_TIMEOUT,
    )
    .await?;

    match response {
        RadrootsNostrConnectResponse::ConnectAcknowledged
        | RadrootsNostrConnectResponse::ConnectSecretEcho(_) => {
            Ok(RadrootsAppRemoteSignerPendingSession {
                record: RadrootsAppRemoteSignerSessionRecord::pending(
                    client_identity.to_public(),
                    target.signer_identity,
                    target.relays,
                ),
                client_secret_key_hex: client_identity.secret_key_hex(),
            })
        }
        other => Err(RadrootsAppRemoteSignerError::UnexpectedResponse {
            method: RadrootsNostrConnectMethod::Connect,
            response: format!("{other:?}"),
        }),
    }
}

async fn poll_pending_session(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError> {
    let client_identity = RadrootsIdentity::from_secret_key_str(client_secret_key_hex)
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    let target = RadrootsAppRemoteSignerTarget {
        source: crate::RadrootsAppRemoteSignerSource::BunkerUri,
        signer_identity: record.signer_identity.clone(),
        relays: record.relays.clone(),
        connect_secret: None,
    };

    match execute_request(
        &client_identity,
        &target,
        RadrootsNostrConnectMethod::GetPublicKey,
        RadrootsNostrConnectRequest::GetPublicKey,
        GET_PUBLIC_KEY_TIMEOUT,
    )
    .await
    {
        Ok(response) => Ok(classify_pending_poll_response(response)),
        Err(error) => Ok(classify_pending_poll_error(error)),
    }
}

async fn execute_request(
    client_identity: &RadrootsIdentity,
    target: &RadrootsAppRemoteSignerTarget,
    method: RadrootsNostrConnectMethod,
    request: RadrootsNostrConnectRequest,
    request_timeout: Duration,
) -> Result<RadrootsNostrConnectResponse, RadrootsAppRemoteSignerError> {
    let client = RadrootsNostrClient::from_identity(client_identity);
    for relay in &target.relays {
        client
            .add_relay(relay)
            .await
            .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    }
    client.connect().await;

    let filter = radroots_nostr_filter_tag(
        RadrootsNostrFilter::new()
            .kind(RadrootsNostrKind::Custom(RADROOTS_NOSTR_CONNECT_RPC_KIND))
            .since(RadrootsNostrTimestamp::now()),
        "p",
        vec![client_identity.public_key_hex()],
    )
    .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    let mut notifications = client.notifications();
    client
        .subscribe(filter, None)
        .await
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;

    let request_id = next_request_id(method.to_string().as_str());
    let event_builder = build_request_event(
        client_identity,
        &target.signer_identity,
        request_id.as_str(),
        request.clone(),
    )?;
    client
        .send_event_builder(event_builder)
        .await
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;

    let response_method = method.clone();
    let response = timeout(request_timeout, async move {
        loop {
            let notification = match notifications.recv().await {
                Ok(notification) => notification,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(RadrootsAppRemoteSignerError::ConnectFailed(
                        "remote signer notification stream closed".to_owned(),
                    ));
                }
            };
            let RadrootsNostrRelayPoolNotification::Event { event, .. } = notification else {
                continue;
            };
            let event = *event;
            if event.kind != RadrootsNostrKind::Custom(RADROOTS_NOSTR_CONNECT_RPC_KIND) {
                continue;
            }
            if event.pubkey.to_hex() != target.signer_identity.public_key_hex {
                continue;
            }
            match parse_response_event(
                client_identity,
                &event,
                &response_method,
                request_id.as_str(),
            )? {
                Some(response) => return Ok(response),
                None => continue,
            }
        }
    })
    .await
    .map_err(|_| RadrootsAppRemoteSignerError::RequestTimedOut {
        method: method.clone(),
    })??;

    Ok(response)
}

fn build_request_event(
    client_identity: &RadrootsIdentity,
    signer_identity: &RadrootsIdentityPublic,
    request_id: &str,
    request: RadrootsNostrConnectRequest,
) -> Result<RadrootsNostrEventBuilder, RadrootsAppRemoteSignerError> {
    let payload = serde_json::to_string(&RadrootsNostrConnectRequestMessage::new(
        request_id.to_owned(),
        request,
    ))
    .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    let signer_public_key = parse_public_key_hex(signer_identity.public_key_hex.as_str())?;
    let ciphertext = nip44::encrypt(
        client_identity.keys().secret_key(),
        &signer_public_key,
        payload,
        Version::V2,
    )
    .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;
    Ok(RadrootsNostrEventBuilder::new(
        radroots_nostr_kind(RADROOTS_NOSTR_CONNECT_RPC_KIND),
        ciphertext,
    )
    .tags(vec![RadrootsNostrTag::public_key(signer_public_key)]))
}

fn parse_response_event(
    client_identity: &RadrootsIdentity,
    event: &RadrootsNostrEvent,
    method: &RadrootsNostrConnectMethod,
    request_id: &str,
) -> Result<Option<RadrootsNostrConnectResponse>, RadrootsAppRemoteSignerError> {
    let decrypted = nip44::decrypt(
        client_identity.keys().secret_key(),
        &event.pubkey,
        &event.content,
    )
    .map_err(|error| RadrootsAppRemoteSignerError::UnexpectedResponse {
        method: method.clone(),
        response: format!("failed to decrypt signer response: {error}"),
    })?;
    let envelope: RadrootsNostrConnectResponseEnvelope =
        serde_json::from_str(&decrypted).map_err(|error| {
            RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: method.clone(),
                response: format!("failed to decode signer response envelope: {error}"),
            }
        })?;
    if envelope.id != request_id {
        return Ok(None);
    }
    let response =
        RadrootsNostrConnectResponse::from_envelope(method, envelope).map_err(|error| {
            RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: method.clone(),
                response: format!("failed to decode signer response payload: {error}"),
            }
        })?;
    Ok(Some(response))
}

fn classify_pending_poll_response(
    response: RadrootsNostrConnectResponse,
) -> RadrootsAppRemoteSignerPendingPollOutcome {
    match response.into_pending_connection_poll_outcome() {
        RadrootsNostrConnectPendingConnectionPollOutcome::Approved(public_key) => {
            RadrootsAppRemoteSignerPendingPollOutcome::Approved(RadrootsIdentityPublic::new(
                public_key,
            ))
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::PendingApproval => {
            RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::Rejected { message } => {
            RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message }
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::AuthChallenge { url } => {
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError {
                message: format!(
                    "remote signer requires an unsupported authorization challenge: {url}"
                ),
            }
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::UnexpectedResponse { response } => {
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError {
                message: format!("unexpected remote signer response: {response}"),
            }
        }
    }
}

fn classify_pending_poll_error(
    error: RadrootsAppRemoteSignerError,
) -> RadrootsAppRemoteSignerPendingPollOutcome {
    match error {
        RadrootsAppRemoteSignerError::RequestTimedOut { .. } => {
            RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure {
                message: "remote signer did not respond yet".to_owned(),
            }
        }
        RadrootsAppRemoteSignerError::ConnectFailed(message) => {
            RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { message }
        }
        RadrootsAppRemoteSignerError::UnexpectedResponse { .. } => {
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError {
                message: error.to_string(),
            }
        }
        other => RadrootsAppRemoteSignerPendingPollOutcome::FatalError {
            message: other.to_string(),
        },
    }
}

fn next_request_id(prefix: &str) -> String {
    let tick = REQUEST_COUNTER.fetch_add(1, Ordering::AcqRel);
    format!("{prefix}-{tick}")
}

fn parse_public_key_hex(value: &str) -> Result<nostr::PublicKey, RadrootsAppRemoteSignerError> {
    nostr::PublicKey::parse(value)
        .or_else(|_| nostr::PublicKey::from_hex(value))
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nostr::PublicKey;
    use radroots_studio_app_test_support::{FIXTURE_ALICE, fixture_identity};

    fn fixture_public_key() -> PublicKey {
        fixture_identity(&FIXTURE_ALICE)
            .expect("identity")
            .public_key()
    }

    #[test]
    fn pending_error_response_is_classified_as_pending_approval() {
        let outcome = classify_pending_poll_response(RadrootsNostrConnectResponse::Error {
            result: None,
            error: radroots_nostr_connect::prelude::RADROOTS_NOSTR_CONNECT_PENDING_CONNECTION_ERROR
                .to_owned(),
        });

        assert_eq!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval
        );
    }

    #[test]
    fn signer_error_response_is_classified_as_rejected() {
        let outcome = classify_pending_poll_response(RadrootsNostrConnectResponse::Error {
            result: None,
            error: "unauthorized".to_owned(),
        });

        assert_eq!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::Rejected {
                message: "unauthorized".to_owned(),
            }
        );
    }

    #[test]
    fn get_public_key_success_is_classified_as_approved() {
        let outcome = classify_pending_poll_response(RadrootsNostrConnectResponse::UserPublicKey(
            fixture_public_key(),
        ));

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::Approved(identity)
                if identity.public_key_hex == fixture_public_key().to_hex()
        ));
    }

    #[test]
    fn timeout_error_is_classified_as_transport_failure() {
        let outcome = classify_pending_poll_error(RadrootsAppRemoteSignerError::RequestTimedOut {
            method: RadrootsNostrConnectMethod::GetPublicKey,
        });

        assert_eq!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure {
                message: "remote signer did not respond yet".to_owned(),
            }
        );
    }

    #[test]
    fn unexpected_response_error_is_fatal() {
        let outcome =
            classify_pending_poll_error(RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: RadrootsNostrConnectMethod::GetPublicKey,
                response: "failed to decode signer response envelope: bad".to_owned(),
            });

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message }
                if message.contains("unexpected `get_public_key` response")
        ));
    }
}
