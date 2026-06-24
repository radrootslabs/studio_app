use crate::error::RadrootsAppRemoteSignerError;
use crate::input::{RadrootsAppRemoteSignerTarget, radroots_studio_app_remote_signer_preview};
use crate::session::RadrootsAppRemoteSignerSessionRecord;
use nostr::JsonUtil;
use nostr::{EventBuilder, RelayUrl, UnsignedEvent};
use radroots_identity::{RadrootsIdentity, RadrootsIdentityPublic};
use radroots_nostr::prelude::{
    RadrootsNostrClient, RadrootsNostrEvent, RadrootsNostrFilter, RadrootsNostrKind,
    RadrootsNostrRelayPoolNotification, RadrootsNostrTimestamp, radroots_nostr_filter_tag,
};
use radroots_nostr_connect::prelude::{
    RADROOTS_NOSTR_CONNECT_RPC_KIND, RadrootsNostrConnectClientProgress,
    RadrootsNostrConnectClientRequest, RadrootsNostrConnectClientTarget,
    RadrootsNostrConnectClientTransport, RadrootsNostrConnectClientTransportFuture,
    RadrootsNostrConnectError, RadrootsNostrConnectMethod,
    RadrootsNostrConnectPendingConnectionPollOutcome, RadrootsNostrConnectPermissions,
    RadrootsNostrConnectRequest, RadrootsNostrConnectResponse, execute_request_with_transport,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const GET_SESSION_CAPABILITY_TIMEOUT: Duration = Duration::from_secs(60);
const SWITCH_RELAYS_TIMEOUT: Duration = Duration::from_secs(30);
const SIGN_EVENT_TIMEOUT: Duration = Duration::from_secs(60);
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct RadrootsAppRemoteSignerPendingSession {
    pub record: RadrootsAppRemoteSignerSessionRecord,
    pub client_secret_key_hex: String,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppRemoteSignerApprovedSession {
    pub user_identity: RadrootsIdentityPublic,
    pub relays: Vec<String>,
    pub approved_permissions: RadrootsNostrConnectPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppRemoteSignerSignedEvent {
    pub event_id_hex: String,
    pub event_json: String,
    pub relays: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppRemoteSignerProgressUpdate {
    AuthChallenge { url: String },
}

#[derive(Debug, Clone)]
pub enum RadrootsAppRemoteSignerPendingPollOutcome {
    PendingApproval,
    Approved(RadrootsAppRemoteSignerApprovedSession),
    TransportFailure { message: String },
    Rejected { message: String },
    FatalError { message: String },
}

pub(crate) struct RadrootsAppRemoteSignerPendingPoller {
    client: ConnectedRemoteSignerSessionClient,
}

struct ConnectedRemoteSignerSessionClient {
    client_identity: RadrootsIdentity,
    target: RadrootsAppRemoteSignerTarget,
    client_target: RadrootsNostrConnectClientTarget,
    transport: ConnectedRemoteSignerTransport,
}

struct ConnectedRemoteSignerTransport {
    client: RadrootsNostrClient,
    notifications: broadcast::Receiver<RadrootsNostrRelayPoolNotification>,
}

pub async fn radroots_studio_app_remote_signer_connect_pending(
    input: &str,
) -> Result<RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerError> {
    let target = radroots_studio_app_remote_signer_preview(input)?;
    connect_pending_session(target).await
}

pub async fn radroots_studio_app_remote_signer_poll_pending_session(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError> {
    radroots_studio_app_remote_signer_poll_pending_session_with_progress(
        record,
        client_secret_key_hex,
        |_| {},
    )
    .await
}

pub async fn radroots_studio_app_remote_signer_poll_pending_session_with_progress<F>(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    mut progress: F,
) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    let mut poller =
        radroots_studio_app_remote_signer_open_pending_poller(record, client_secret_key_hex).await?;
    radroots_studio_app_remote_signer_poll_pending_poller_with_progress(&mut poller, &mut progress).await
}

pub(crate) async fn radroots_studio_app_remote_signer_open_pending_poller(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
) -> Result<RadrootsAppRemoteSignerPendingPoller, RadrootsAppRemoteSignerError> {
    let client_identity = load_client_identity(client_secret_key_hex)?;
    let target = target_for_record(record);
    Ok(RadrootsAppRemoteSignerPendingPoller {
        client: ConnectedRemoteSignerSessionClient::connect(client_identity, target).await?,
    })
}

pub(crate) async fn radroots_studio_app_remote_signer_poll_pending_poller_with_progress<F>(
    poller: &mut RadrootsAppRemoteSignerPendingPoller,
    progress: &mut F,
) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    poller.poll_with_progress(progress).await
}

pub async fn radroots_studio_app_remote_signer_sign_kind1_note(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    content: &str,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError> {
    radroots_studio_app_remote_signer_sign_kind1_note_with_progress(
        record,
        client_secret_key_hex,
        content,
        |_| {},
    )
    .await
}

pub async fn radroots_studio_app_remote_signer_sign_kind1_note_with_progress<F>(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    content: &str,
    mut progress: F,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    sign_kind1_note(record, client_secret_key_hex, content, &mut progress).await
}

pub async fn radroots_studio_app_remote_signer_sign_unsigned_event(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    unsigned_event: UnsignedEvent,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError> {
    radroots_studio_app_remote_signer_sign_unsigned_event_with_progress(
        record,
        client_secret_key_hex,
        unsigned_event,
        |_| {},
    )
    .await
}

pub async fn radroots_studio_app_remote_signer_sign_unsigned_event_with_progress<F>(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    unsigned_event: UnsignedEvent,
    mut progress: F,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    sign_unsigned_event(record, client_secret_key_hex, unsigned_event, &mut progress).await
}

async fn connect_pending_session(
    target: RadrootsAppRemoteSignerTarget,
) -> Result<RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerError> {
    let client_identity = RadrootsIdentity::generate();
    let connect_request = connect_request_for_target(&target);
    let response = execute_request(
        &client_identity,
        &target,
        RadrootsNostrConnectMethod::Connect,
        connect_request,
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

fn connect_request_for_target(
    target: &RadrootsAppRemoteSignerTarget,
) -> RadrootsNostrConnectRequest {
    RadrootsNostrConnectRequest::Connect {
        remote_signer_public_key: parse_public_key_hex(
            target.signer_identity.public_key_hex.as_str(),
        )
        .expect("signer public key is derived from a validated identity"),
        secret: target.connect_secret.clone(),
        requested_permissions: target.requested_permissions.clone(),
    }
}

async fn sign_kind1_note<F>(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    content: &str,
    progress: &mut F,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    if !record.allows_sign_event_kind1() {
        return Err(RadrootsAppRemoteSignerError::ConnectFailed(
            "remote signer has not approved sign_event:kind:1".to_owned(),
        ));
    }
    let user_identity = record.user_identity.as_ref().ok_or_else(|| {
        RadrootsAppRemoteSignerError::ConnectFailed(
            "remote signer session is missing the approved user identity".to_owned(),
        )
    })?;
    let unsigned_event = EventBuilder::text_note(content.trim())
        .build(parse_public_key_hex(user_identity.public_key_hex.as_str())?);
    sign_unsigned_event(record, client_secret_key_hex, unsigned_event, progress).await
}

async fn sign_unsigned_event<F>(
    record: &RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: &str,
    unsigned_event: UnsignedEvent,
    progress: &mut F,
) -> Result<RadrootsAppRemoteSignerSignedEvent, RadrootsAppRemoteSignerError>
where
    F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
{
    let client_identity = load_client_identity(client_secret_key_hex)?;
    let target = target_for_record(record);
    let mut client = ConnectedRemoteSignerSessionClient::connect(client_identity, target).await?;
    let relays = client.sync_relays_if_allowed(record, progress).await?;
    let response = client
        .execute_request_with_progress(
            RadrootsNostrConnectMethod::SignEvent,
            RadrootsNostrConnectRequest::SignEvent(unsigned_event),
            SIGN_EVENT_TIMEOUT,
            progress,
        )
        .await?;

    match response {
        RadrootsNostrConnectResponse::SignedEvent(event) => {
            Ok(RadrootsAppRemoteSignerSignedEvent {
                event_id_hex: event.id.to_hex(),
                event_json: event.as_json(),
                relays,
            })
        }
        RadrootsNostrConnectResponse::Error { error, .. } => {
            Err(RadrootsAppRemoteSignerError::ConnectFailed(error))
        }
        other => Err(RadrootsAppRemoteSignerError::UnexpectedResponse {
            method: RadrootsNostrConnectMethod::SignEvent,
            response: format!("{other:?}"),
        }),
    }
}

async fn execute_request(
    client_identity: &RadrootsIdentity,
    target: &RadrootsAppRemoteSignerTarget,
    method: RadrootsNostrConnectMethod,
    request: RadrootsNostrConnectRequest,
    request_timeout: Duration,
) -> Result<RadrootsNostrConnectResponse, RadrootsAppRemoteSignerError> {
    let mut client =
        ConnectedRemoteSignerSessionClient::connect(client_identity.clone(), target.clone())
            .await?;
    client
        .execute_request_with_progress(method, request, request_timeout, &mut |_| {})
        .await
}

impl RadrootsAppRemoteSignerPendingPoller {
    async fn poll_with_progress<F>(
        &mut self,
        progress: &mut F,
    ) -> Result<RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerError>
    where
        F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
    {
        match self
            .client
            .execute_request_with_progress(
                RadrootsNostrConnectMethod::GetSessionCapability,
                RadrootsNostrConnectRequest::GetSessionCapability,
                GET_SESSION_CAPABILITY_TIMEOUT,
                progress,
            )
            .await
        {
            Ok(response) => Ok(classify_pending_poll_response(response)),
            Err(error) => Ok(classify_pending_poll_error(error)),
        }
    }
}

impl ConnectedRemoteSignerSessionClient {
    async fn connect(
        client_identity: RadrootsIdentity,
        target: RadrootsAppRemoteSignerTarget,
    ) -> Result<Self, RadrootsAppRemoteSignerError> {
        let client_target = client_target_for_app_target(&target)?;
        let client = RadrootsNostrClient::from_identity(&client_identity);
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
        let notifications = client.notifications();
        client
            .subscribe(filter, None)
            .await
            .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))?;

        Ok(Self {
            client_identity,
            target,
            client_target,
            transport: ConnectedRemoteSignerTransport {
                client,
                notifications,
            },
        })
    }

    async fn sync_relays_if_allowed<F>(
        &mut self,
        record: &RadrootsAppRemoteSignerSessionRecord,
        progress: &mut F,
    ) -> Result<Vec<String>, RadrootsAppRemoteSignerError>
    where
        F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
    {
        if !record.allows_switch_relays() {
            return Ok(self.target.relays.clone());
        }

        match self
            .execute_request_with_progress(
                RadrootsNostrConnectMethod::SwitchRelays,
                RadrootsNostrConnectRequest::SwitchRelays,
                SWITCH_RELAYS_TIMEOUT,
                progress,
            )
            .await?
        {
            RadrootsNostrConnectResponse::RelayList(relays) => {
                let relays: Vec<String> = relays.iter().map(ToString::to_string).collect();
                self.client_target.relays = relays
                    .iter()
                    .map(|relay| parse_relay_url(relay))
                    .collect::<Result<Vec<_>, _>>()?;
                self.target.relays = relays.clone();
                Ok(relays)
            }
            RadrootsNostrConnectResponse::RelayListUnchanged => Ok(self.target.relays.clone()),
            RadrootsNostrConnectResponse::Error { error, .. } => {
                Err(RadrootsAppRemoteSignerError::ConnectFailed(format!(
                    "remote signer rejected relay update: {error}"
                )))
            }
            other => Err(RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: RadrootsNostrConnectMethod::SwitchRelays,
                response: format!("{other:?}"),
            }),
        }
    }

    async fn execute_request_with_progress<F>(
        &mut self,
        method: RadrootsNostrConnectMethod,
        request: RadrootsNostrConnectRequest,
        request_timeout: Duration,
        progress: &mut F,
    ) -> Result<RadrootsNostrConnectResponse, RadrootsAppRemoteSignerError>
    where
        F: FnMut(RadrootsAppRemoteSignerProgressUpdate),
    {
        let request_id = next_request_id(method.to_string().as_str());
        let response_method = method.clone();
        let client_keys = self.client_identity.keys().clone();
        let client_target = self.client_target.clone();
        let request = RadrootsNostrConnectClientRequest::new(request_id, request);
        let response = timeout(
            request_timeout,
            execute_request_with_transport(
                &client_keys,
                &client_target,
                request,
                &mut self.transport,
                |event| {
                    match event {
                        RadrootsNostrConnectClientProgress::AuthChallenge { url } => {
                            progress(RadrootsAppRemoteSignerProgressUpdate::AuthChallenge { url });
                        }
                    }
                    Ok(())
                },
            ),
        )
        .await
        .map_err(|_| RadrootsAppRemoteSignerError::RequestTimedOut {
            method: response_method.clone(),
        })?;
        response.map_err(|error| app_error_from_nostr_connect_error(&response_method, error))
    }
}

impl RadrootsNostrConnectClientTransport for ConnectedRemoteSignerTransport {
    fn publish_request_event<'a>(
        &'a mut self,
        event: RadrootsNostrEvent,
    ) -> RadrootsNostrConnectClientTransportFuture<'a, ()> {
        Box::pin(async move {
            self.client
                .send_event(&event)
                .await
                .map(|_| ())
                .map_err(|error| RadrootsNostrConnectError::Transport {
                    reason: error.to_string(),
                })
        })
    }

    fn next_response_event<'a>(
        &'a mut self,
    ) -> RadrootsNostrConnectClientTransportFuture<'a, RadrootsNostrEvent> {
        Box::pin(async move {
            loop {
                let notification = match self.notifications.recv().await {
                    Ok(notification) => notification,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(RadrootsNostrConnectError::Transport {
                            reason: "remote signer notification stream closed".to_owned(),
                        });
                    }
                };
                let RadrootsNostrRelayPoolNotification::Event { event, .. } = notification else {
                    continue;
                };
                let event = *event;
                if event.kind != RadrootsNostrKind::Custom(RADROOTS_NOSTR_CONNECT_RPC_KIND) {
                    continue;
                }
                return Ok(event);
            }
        })
    }
}

fn client_target_for_app_target(
    target: &RadrootsAppRemoteSignerTarget,
) -> Result<RadrootsNostrConnectClientTarget, RadrootsAppRemoteSignerError> {
    Ok(RadrootsNostrConnectClientTarget::new(
        parse_public_key_hex(target.signer_identity.public_key_hex.as_str())?,
        target
            .relays
            .iter()
            .map(|relay| parse_relay_url(relay))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_relay_url(value: &str) -> Result<RelayUrl, RadrootsAppRemoteSignerError> {
    RelayUrl::parse(value).map_err(|error| {
        RadrootsAppRemoteSignerError::ConnectFailed(format!(
            "invalid remote signer relay `{value}`: {error}"
        ))
    })
}

fn app_error_from_nostr_connect_error(
    method: &RadrootsNostrConnectMethod,
    error: RadrootsNostrConnectError,
) -> RadrootsAppRemoteSignerError {
    match error {
        RadrootsNostrConnectError::RequestTimedOut => {
            RadrootsAppRemoteSignerError::RequestTimedOut {
                method: method.clone(),
            }
        }
        RadrootsNostrConnectError::Transport { reason }
        | RadrootsNostrConnectError::Encrypt { reason }
        | RadrootsNostrConnectError::Sign { reason } => {
            RadrootsAppRemoteSignerError::ConnectFailed(reason)
        }
        RadrootsNostrConnectError::Decrypt { reason }
        | RadrootsNostrConnectError::Json(reason)
        | RadrootsNostrConnectError::InvalidResponsePayload { reason, .. } => {
            RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: method.clone(),
                response: reason,
            }
        }
        other => RadrootsAppRemoteSignerError::UnexpectedResponse {
            method: method.clone(),
            response: other.to_string(),
        },
    }
}

fn classify_pending_poll_response(
    response: RadrootsNostrConnectResponse,
) -> RadrootsAppRemoteSignerPendingPollOutcome {
    match response.into_pending_connection_poll_outcome() {
        RadrootsNostrConnectPendingConnectionPollOutcome::Approved(public_key) => {
            RadrootsAppRemoteSignerPendingPollOutcome::Approved(
                RadrootsAppRemoteSignerApprovedSession {
                    user_identity: RadrootsIdentityPublic::new(public_key),
                    relays: Vec::new(),
                    approved_permissions: RadrootsNostrConnectPermissions::default(),
                },
            )
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::ApprovedCapability(capability) => {
            RadrootsAppRemoteSignerPendingPollOutcome::Approved(
                RadrootsAppRemoteSignerApprovedSession {
                    user_identity: RadrootsIdentityPublic::new(capability.user_public_key),
                    relays: capability
                        .relays
                        .into_iter()
                        .map(|relay| relay.to_string())
                        .collect(),
                    approved_permissions: capability.permissions,
                },
            )
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::PendingApproval => {
            RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::Rejected { message } => {
            RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message }
        }
        RadrootsNostrConnectPendingConnectionPollOutcome::AuthChallenge { url } => {
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError {
                message: format!("unexpected remote signer authorization challenge: {url}"),
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

fn load_client_identity(
    client_secret_key_hex: &str,
) -> Result<RadrootsIdentity, RadrootsAppRemoteSignerError> {
    RadrootsIdentity::from_secret_key_str(client_secret_key_hex)
        .map_err(|error| RadrootsAppRemoteSignerError::ConnectFailed(error.to_string()))
}

fn target_for_record(
    record: &RadrootsAppRemoteSignerSessionRecord,
) -> RadrootsAppRemoteSignerTarget {
    RadrootsAppRemoteSignerTarget {
        source: crate::RadrootsAppRemoteSignerSource::BunkerUri,
        signer_identity: record.signer_identity.clone(),
        relays: record.relays.clone(),
        connect_secret: None,
        requested_permissions: if record.approved_permissions.is_empty() {
            crate::radroots_studio_app_remote_signer_requested_permissions()
        } else {
            record.approved_permissions.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radroots_studio_app_remote_signer_preview;
    use radroots_identity::RadrootsIdentity;
    use radroots_nostr_connect::prelude::{
        RadrootsNostrConnectPermission, RadrootsNostrConnectRemoteSessionCapability,
    };

    const RELAY_PRIMARY_WSS: &str = "wss://relay.example.com";
    const SIGNER_SECRET_KEY_HEX: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const CLIENT_SECRET_KEY_HEX: &str =
        "2222222222222222222222222222222222222222222222222222222222222222";

    fn fixture_identity(secret_key_hex: &str) -> RadrootsIdentity {
        RadrootsIdentity::from_secret_key_str(secret_key_hex).expect("identity")
    }

    fn fixture_public_key() -> nostr::PublicKey {
        fixture_identity(SIGNER_SECRET_KEY_HEX).public_key()
    }

    fn fixture_discovery_url() -> String {
        format!(
            "http://localhost/connect?uri={}",
            url::form_urlencoded::byte_serialize(
                format!(
                    "bunker://{}?relay={RELAY_PRIMARY_WSS}",
                    fixture_identity(SIGNER_SECRET_KEY_HEX).public_key_hex()
                )
                .as_bytes()
            )
            .collect::<String>()
        )
    }

    #[test]
    fn pending_connection_response_is_classified_as_pending_approval() {
        let outcome =
            classify_pending_poll_response(RadrootsNostrConnectResponse::PendingConnection);

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval
        ));
    }

    #[test]
    fn signer_error_response_is_classified_as_rejected() {
        let outcome = classify_pending_poll_response(RadrootsNostrConnectResponse::Error {
            result: None,
            error: "unauthorized".to_owned(),
        });

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message }
                if message == "unauthorized"
        ));
    }

    #[test]
    fn session_capability_success_is_classified_as_approved() {
        let outcome =
            classify_pending_poll_response(RadrootsNostrConnectResponse::RemoteSessionCapability(
                RadrootsNostrConnectRemoteSessionCapability {
                    user_public_key: fixture_public_key(),
                    relays: vec![nostr::RelayUrl::parse(RELAY_PRIMARY_WSS).expect("relay")],
                    permissions: vec![
                        RadrootsNostrConnectPermission::with_parameter(
                            RadrootsNostrConnectMethod::SignEvent,
                            "kind:1",
                        ),
                        RadrootsNostrConnectPermission::new(
                            RadrootsNostrConnectMethod::SwitchRelays,
                        ),
                    ]
                    .into(),
                },
            ));

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::Approved(
                RadrootsAppRemoteSignerApprovedSession { user_identity, approved_permissions, .. }
            ) if user_identity.public_key_hex == fixture_public_key().to_hex()
                && approved_permissions.to_string() == "sign_event:kind:1,switch_relays"
        ));
    }

    #[test]
    fn timeout_error_is_classified_as_transport_failure() {
        let outcome = classify_pending_poll_error(RadrootsAppRemoteSignerError::RequestTimedOut {
            method: RadrootsNostrConnectMethod::GetSessionCapability,
        });

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { message }
                if message == "remote signer did not respond yet"
        ));
    }

    #[test]
    fn unexpected_response_error_is_fatal() {
        let outcome =
            classify_pending_poll_error(RadrootsAppRemoteSignerError::UnexpectedResponse {
                method: RadrootsNostrConnectMethod::GetSessionCapability,
                response: "failed to decode signer response envelope: bad".to_owned(),
            });

        assert!(matches!(
            outcome,
            RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message }
                if message.contains("unexpected `get_session_capability` response")
        ));
    }

    #[test]
    fn connect_request_uses_explicit_requested_permissions() {
        let target =
            radroots_studio_app_remote_signer_preview(fixture_discovery_url().as_str()).expect("preview");

        let request = connect_request_for_target(&target);

        match request {
            RadrootsNostrConnectRequest::Connect {
                requested_permissions,
                ..
            } => assert_eq!(
                requested_permissions.to_string(),
                "sign_event:kind:1,switch_relays"
            ),
            other => panic!("unexpected request: {other:?}"),
        }
    }

    #[test]
    fn sign_kind1_note_output_carries_signed_relay_state() {
        let signed_event = RadrootsAppRemoteSignerSignedEvent {
            event_id_hex: "deadbeef".to_owned(),
            event_json: "{\"id\":\"deadbeef\"}".to_owned(),
            relays: vec!["ws://localhost:8080".to_owned()],
        };

        assert_eq!(signed_event.event_id_hex, "deadbeef");
        assert_eq!(signed_event.relays, vec!["ws://localhost:8080".to_owned()]);
    }

    #[test]
    fn target_for_record_uses_approved_permissions_when_available() {
        let client_identity = fixture_identity(CLIENT_SECRET_KEY_HEX).to_public();
        let signer_identity = fixture_identity(SIGNER_SECRET_KEY_HEX).to_public();
        let mut record = RadrootsAppRemoteSignerSessionRecord::pending(
            client_identity,
            signer_identity,
            vec![RELAY_PRIMARY_WSS.to_owned()],
        );
        record.approved_permissions = vec![RadrootsNostrConnectPermission::new(
            RadrootsNostrConnectMethod::SwitchRelays,
        )]
        .into();

        let target = target_for_record(&record);

        assert_eq!(target.requested_permissions.to_string(), "switch_relays");
    }
}
