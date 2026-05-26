#![forbid(unsafe_code)]

mod publish;

pub use publish::{
    AppFarmProfilePublishPayload, AppListingPublishPayload, AppOrderDecisionInventoryCommitment,
    AppOrderDecisionPayload, AppOrderDecisionPublishPayload, AppOrderRequestItemPayload,
    AppOrderRequestPublishPayload, AppPublishContext, AppPublishPayload,
    AppPublishPayloadJsonError, AppPublishValidationFailure, AppPublishValidationFailureSet,
    AppPublishWorkKind,
};

use radroots_studio_app_view::{FarmId, FulfillmentWindowId, OrderId, ProductId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "aggregate_kind",
    content = "aggregate_id",
    rename_all = "snake_case"
)]
pub enum SyncAggregateRef {
    Farm(FarmId),
    FulfillmentWindow(FulfillmentWindowId),
    Product(ProductId),
    Order(OrderId),
}

impl SyncAggregateRef {
    pub const fn aggregate_kind(&self) -> &'static str {
        match self {
            Self::Farm(_) => "farm",
            Self::FulfillmentWindow(_) => "fulfillment_window",
            Self::Product(_) => "product",
            Self::Order(_) => "order",
        }
    }

    pub fn aggregate_id(&self) -> String {
        match self {
            Self::Farm(farm_id) => farm_id.to_string(),
            Self::FulfillmentWindow(fulfillment_window_id) => fulfillment_window_id.to_string(),
            Self::Product(product_id) => product_id.to_string(),
            Self::Order(order_id) => order_id.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncTrigger {
    AppLaunch,
    ForegroundResume,
    #[default]
    ManualRefresh,
    LocalMutation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperationKind {
    Upsert,
    Delete,
}

impl SyncOperationKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Upsert => "upsert",
            Self::Delete => "delete",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingSyncOperationState {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Blocked,
    Retryable,
}

impl PendingSyncOperationState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Retryable => "retryable",
        }
    }

    pub const fn is_active(self) -> bool {
        !matches!(self, Self::Succeeded)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingSyncOperation {
    pub operation_key: String,
    pub aggregate: SyncAggregateRef,
    pub operation: SyncOperationKind,
    pub payload_json: String,
    pub created_at: String,
    pub available_at: String,
    pub attempt_count: u32,
    pub state: PendingSyncOperationState,
    pub last_error_message: Option<String>,
}

impl PendingSyncOperation {
    pub fn new(
        aggregate: SyncAggregateRef,
        operation: SyncOperationKind,
        payload_json: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        let operation_key = Self::deterministic_operation_key(&aggregate, operation);
        let created_at = created_at.into();
        Self {
            operation_key,
            aggregate,
            operation,
            payload_json: payload_json.into(),
            created_at: created_at.clone(),
            available_at: created_at,
            attempt_count: 0,
            state: PendingSyncOperationState::Pending,
            last_error_message: None,
        }
    }

    pub fn deterministic_operation_key(
        aggregate: &SyncAggregateRef,
        operation: SyncOperationKind,
    ) -> String {
        format!(
            "{}:{}:{}",
            aggregate.aggregate_kind(),
            aggregate.aggregate_id(),
            operation.storage_key()
        )
    }

    pub const fn is_retry(&self) -> bool {
        self.attempt_count > 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictKind {
    RevisionMismatch,
    RemoteDelete,
    RemoteValidationReject,
}

impl SyncConflictKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::RevisionMismatch => "revision_mismatch",
            Self::RemoteDelete => "remote_delete",
            Self::RemoteValidationReject => "remote_validation_reject",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictSeverity {
    ReviewRequired,
    Blocking,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncConflictResolutionStatus {
    Unresolved,
    AcceptedLocal,
    AcceptedRemote,
    Dismissed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub aggregate: SyncAggregateRef,
    pub kind: SyncConflictKind,
    pub severity: SyncConflictSeverity,
    pub resolution: SyncConflictResolutionStatus,
    pub local_payload_json: String,
    pub remote_payload_json: Option<String>,
    pub detected_at: String,
    pub resolved_at: Option<String>,
}

impl SyncConflict {
    pub const fn is_unresolved(&self) -> bool {
        matches!(self.resolution, SyncConflictResolutionStatus::Unresolved)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncConflictStatus {
    pub unresolved_count: usize,
    pub blocking_count: usize,
}

impl SyncConflictStatus {
    pub const fn clear() -> Self {
        Self {
            unresolved_count: 0,
            blocking_count: 0,
        }
    }

    pub fn from_conflicts(conflicts: &[SyncConflict]) -> Self {
        let unresolved_conflicts = conflicts.iter().filter(|conflict| conflict.is_unresolved());
        let unresolved_count = unresolved_conflicts.clone().count();
        let blocking_count = unresolved_conflicts
            .filter(|conflict| matches!(conflict.severity, SyncConflictSeverity::Blocking))
            .count();

        Self {
            unresolved_count,
            blocking_count,
        }
    }

    pub const fn is_clear(&self) -> bool {
        self.unresolved_count == 0
    }

    pub const fn requires_attention(&self) -> bool {
        self.unresolved_count > 0
    }

    pub const fn has_blocking_conflicts(&self) -> bool {
        self.blocking_count > 0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncCheckpointState {
    #[default]
    NeverSynced,
    Syncing,
    Current,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncCheckpointStatus {
    pub state: SyncCheckpointState,
    pub last_sync_started_at: Option<String>,
    pub last_sync_completed_at: Option<String>,
    pub last_remote_cursor: Option<String>,
    pub last_error_message: Option<String>,
}

impl Default for SyncCheckpointStatus {
    fn default() -> Self {
        Self::never_synced()
    }
}

impl SyncCheckpointStatus {
    pub const fn never_synced() -> Self {
        Self {
            state: SyncCheckpointState::NeverSynced,
            last_sync_started_at: None,
            last_sync_completed_at: None,
            last_remote_cursor: None,
            last_error_message: None,
        }
    }

    pub fn syncing(started_at: impl Into<String>, last_remote_cursor: Option<String>) -> Self {
        Self {
            state: SyncCheckpointState::Syncing,
            last_sync_started_at: Some(started_at.into()),
            last_sync_completed_at: None,
            last_remote_cursor,
            last_error_message: None,
        }
    }

    pub fn current(
        started_at: Option<String>,
        completed_at: impl Into<String>,
        last_remote_cursor: Option<String>,
    ) -> Self {
        Self {
            state: SyncCheckpointState::Current,
            last_sync_started_at: started_at,
            last_sync_completed_at: Some(completed_at.into()),
            last_remote_cursor,
            last_error_message: None,
        }
    }

    pub fn failed(
        started_at: Option<String>,
        completed_at: Option<String>,
        last_remote_cursor: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            state: SyncCheckpointState::Failed,
            last_sync_started_at: started_at,
            last_sync_completed_at: completed_at,
            last_remote_cursor,
            last_error_message: Some(message.into()),
        }
    }

    pub const fn is_failed(&self) -> bool {
        matches!(self.state, SyncCheckpointState::Failed)
    }

    pub const fn is_syncing(&self) -> bool {
        matches!(self.state, SyncCheckpointState::Syncing)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSyncRunStatus {
    #[default]
    Idle,
    Syncing,
    Succeeded,
    Conflicted,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppSyncProjection {
    pub run_status: AppSyncRunStatus,
    pub checkpoint: SyncCheckpointStatus,
    pub conflict_status: SyncConflictStatus,
}

impl Default for AppSyncProjection {
    fn default() -> Self {
        Self {
            run_status: AppSyncRunStatus::Idle,
            checkpoint: SyncCheckpointStatus::never_synced(),
            conflict_status: SyncConflictStatus::clear(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRelayIngestFreshnessState {
    Fresh,
    #[default]
    Stale,
    Failed,
}

impl AppRelayIngestFreshnessState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Stale => "stale",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRelayIngestScopeStatus {
    Fresh,
    #[default]
    Stale,
    Partial,
    Failed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppRelayIngestRelayFreshness {
    pub relay_url: String,
    pub state: AppRelayIngestFreshnessState,
    pub cursor_since_unix_seconds: Option<i64>,
    pub last_event_created_at_unix_seconds: Option<i64>,
    pub last_fetch_started_at: Option<String>,
    pub last_fetch_completed_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error_message: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppRelayIngestScopeFreshness {
    pub scope_key: String,
    pub status: AppRelayIngestScopeStatus,
    pub relays: Vec<AppRelayIngestRelayFreshness>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppSyncRequest {
    pub trigger: SyncTrigger,
    pub checkpoint: SyncCheckpointStatus,
    pub pending_operations: Vec<PendingSyncOperation>,
    pub known_conflicts: Vec<SyncConflict>,
}

impl AppSyncRequest {
    pub fn conflict_status(&self) -> SyncConflictStatus {
        SyncConflictStatus::from_conflicts(&self.known_conflicts)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppSyncResult {
    pub run_status: AppSyncRunStatus,
    pub checkpoint: SyncCheckpointStatus,
    pub pushed_operation_count: usize,
    pub pulled_record_count: usize,
    pub conflicts: Vec<SyncConflict>,
    #[serde(default)]
    pub published_receipts: Vec<AppPublishedOperationReceipt>,
}

impl AppSyncResult {
    pub fn projection(&self) -> AppSyncProjection {
        AppSyncProjection {
            run_status: self.run_status,
            checkpoint: self.checkpoint.clone(),
            conflict_status: SyncConflictStatus::from_conflicts(&self.conflicts),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppPublishedOperationReceipt {
    pub operation_key: String,
    pub source_account_id: String,
    pub source_local_event_id: Option<String>,
    #[serde(default)]
    pub listing_addr: Option<String>,
    pub event_id: String,
    pub event_kind: u32,
    pub event_pubkey: String,
    pub event_created_at: u32,
    pub event_tags_json: serde_json::Value,
    pub event_content: String,
    pub event_sig: String,
    pub raw_event_json: serde_json::Value,
    pub relay_set_fingerprint: String,
    pub relay_delivery_json: serde_json::Value,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AppSyncTransportError {
    #[error("app sync transport is unavailable: {message}")]
    Unavailable { message: String },
    #[error("app sync transport failed: {message}")]
    Failed { message: String },
}

impl AppSyncTransportError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable {
            message: message.into(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed {
            message: message.into(),
        }
    }
}

pub trait AppSyncTransport {
    fn sync(&mut self, request: AppSyncRequest) -> Result<AppSyncResult, AppSyncTransportError>;

    fn supports_empty_sync_request(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
pub struct RecordedAppSyncTransport {
    result: Result<AppSyncResult, AppSyncTransportError>,
    last_request: Option<AppSyncRequest>,
    call_count: usize,
}

impl RecordedAppSyncTransport {
    pub fn succeed(result: AppSyncResult) -> Self {
        Self {
            result: Ok(result),
            last_request: None,
            call_count: 0,
        }
    }

    pub fn fail(error: AppSyncTransportError) -> Self {
        Self {
            result: Err(error),
            last_request: None,
            call_count: 0,
        }
    }

    pub fn last_request(&self) -> Option<&AppSyncRequest> {
        self.last_request.as_ref()
    }

    pub const fn call_count(&self) -> usize {
        self.call_count
    }
}

impl AppSyncTransport for RecordedAppSyncTransport {
    fn sync(&mut self, request: AppSyncRequest) -> Result<AppSyncResult, AppSyncTransportError> {
        self.call_count += 1;
        self.last_request = Some(request);
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppSyncProjection, AppSyncRequest, AppSyncResult, AppSyncRunStatus, AppSyncTransport,
        AppSyncTransportError, PendingSyncOperation, RecordedAppSyncTransport, SyncAggregateRef,
        SyncCheckpointState, SyncCheckpointStatus, SyncConflict, SyncConflictKind,
        SyncConflictResolutionStatus, SyncConflictSeverity, SyncConflictStatus, SyncOperationKind,
        SyncTrigger,
    };
    use radroots_studio_app_view::{FarmId, ProductId};

    #[test]
    fn default_projection_starts_idle_and_clear() {
        let projection = AppSyncProjection::default();

        assert_eq!(projection.run_status, AppSyncRunStatus::Idle);
        assert_eq!(
            projection.checkpoint.state,
            SyncCheckpointState::NeverSynced
        );
        assert!(projection.conflict_status.is_clear());
    }

    #[test]
    fn checkpoint_constructors_keep_sync_and_failure_state_explicit() {
        let syncing =
            SyncCheckpointStatus::syncing("2026-04-17T19:30:00Z", Some("cursor-1".to_owned()));
        let failed = SyncCheckpointStatus::failed(
            Some("2026-04-17T19:30:00Z".to_owned()),
            Some("2026-04-17T19:30:30Z".to_owned()),
            Some("cursor-1".to_owned()),
            "relay timeout",
        );
        let current = SyncCheckpointStatus::current(
            Some("2026-04-17T19:30:00Z".to_owned()),
            "2026-04-17T19:30:30Z",
            Some("cursor-2".to_owned()),
        );

        assert!(syncing.is_syncing());
        assert_eq!(syncing.last_sync_completed_at, None);
        assert_eq!(syncing.last_error_message, None);

        assert!(failed.is_failed());
        assert_eq!(failed.last_error_message.as_deref(), Some("relay timeout"));

        assert_eq!(current.state, SyncCheckpointState::Current);
        assert_eq!(current.last_remote_cursor.as_deref(), Some("cursor-2"));
        assert_eq!(current.last_error_message, None);
    }

    #[test]
    fn conflict_status_counts_only_unresolved_conflicts() {
        let conflicts = vec![
            SyncConflict {
                aggregate: SyncAggregateRef::Product(ProductId::new()),
                kind: SyncConflictKind::RevisionMismatch,
                severity: SyncConflictSeverity::Blocking,
                resolution: SyncConflictResolutionStatus::Unresolved,
                local_payload_json: "{\"title\":\"carrots\"}".to_owned(),
                remote_payload_json: Some("{\"title\":\"rainbow carrots\"}".to_owned()),
                detected_at: "2026-04-17T19:31:00Z".to_owned(),
                resolved_at: None,
            },
            SyncConflict {
                aggregate: SyncAggregateRef::Farm(FarmId::new()),
                kind: SyncConflictKind::RemoteValidationReject,
                severity: SyncConflictSeverity::ReviewRequired,
                resolution: SyncConflictResolutionStatus::AcceptedRemote,
                local_payload_json: "{\"display_name\":\"Sunrise Farm\"}".to_owned(),
                remote_payload_json: Some("{\"display_name\":\"Sunrise Farm LLC\"}".to_owned()),
                detected_at: "2026-04-17T19:31:30Z".to_owned(),
                resolved_at: Some("2026-04-17T19:32:00Z".to_owned()),
            },
        ];

        let status = SyncConflictStatus::from_conflicts(&conflicts);

        assert_eq!(status.unresolved_count, 1);
        assert_eq!(status.blocking_count, 1);
        assert!(status.requires_attention());
        assert!(status.has_blocking_conflicts());
    }

    #[test]
    fn request_and_result_surface_conflict_status_through_typed_contracts() {
        let mut pending_operation = PendingSyncOperation::new(
            SyncAggregateRef::Product(ProductId::new()),
            SyncOperationKind::Upsert,
            "{\"title\":\"greens\"}",
            "2026-04-17T19:32:00Z",
        );
        pending_operation.attempt_count = 1;
        let conflict = SyncConflict {
            aggregate: SyncAggregateRef::Product(ProductId::new()),
            kind: SyncConflictKind::RevisionMismatch,
            severity: SyncConflictSeverity::ReviewRequired,
            resolution: SyncConflictResolutionStatus::Unresolved,
            local_payload_json: "{\"stock_count\":4}".to_owned(),
            remote_payload_json: Some("{\"stock_count\":6}".to_owned()),
            detected_at: "2026-04-17T19:33:00Z".to_owned(),
            resolved_at: None,
        };
        let request = AppSyncRequest {
            trigger: SyncTrigger::LocalMutation,
            checkpoint: SyncCheckpointStatus::current(
                Some("2026-04-17T19:30:00Z".to_owned()),
                "2026-04-17T19:32:30Z",
                Some("cursor-4".to_owned()),
            ),
            pending_operations: vec![pending_operation.clone()],
            known_conflicts: vec![conflict.clone()],
        };
        let result = AppSyncResult {
            run_status: AppSyncRunStatus::Conflicted,
            checkpoint: request.checkpoint.clone(),
            pushed_operation_count: 1,
            pulled_record_count: 3,
            conflicts: vec![conflict],
            published_receipts: Vec::new(),
        };

        assert_eq!(request.conflict_status().unresolved_count, 1);
        assert!(pending_operation.is_retry());
        assert_eq!(pending_operation.operation.storage_key(), "upsert");

        let projection = result.projection();
        assert_eq!(projection.run_status, AppSyncRunStatus::Conflicted);
        assert_eq!(
            projection.checkpoint.last_remote_cursor.as_deref(),
            Some("cursor-4")
        );
        assert_eq!(projection.conflict_status.unresolved_count, 1);
    }

    #[test]
    fn recorded_transport_is_mockable_and_records_requests() {
        let request = AppSyncRequest {
            trigger: SyncTrigger::ManualRefresh,
            checkpoint: SyncCheckpointStatus::never_synced(),
            pending_operations: vec![],
            known_conflicts: vec![],
        };
        let expected_result = AppSyncResult {
            run_status: AppSyncRunStatus::Succeeded,
            checkpoint: SyncCheckpointStatus::current(
                Some("2026-04-17T19:34:00Z".to_owned()),
                "2026-04-17T19:34:10Z",
                Some("cursor-9".to_owned()),
            ),
            pushed_operation_count: 0,
            pulled_record_count: 2,
            conflicts: vec![],
            published_receipts: Vec::new(),
        };
        let mut transport = RecordedAppSyncTransport::succeed(expected_result.clone());

        let actual_result = transport
            .sync(request.clone())
            .expect("recorded transport should succeed");

        assert_eq!(actual_result, expected_result);
        assert_eq!(transport.last_request(), Some(&request));
        assert_eq!(transport.call_count(), 1);
    }

    #[test]
    fn recorded_transport_can_fail_without_a_live_backend() {
        let mut transport =
            RecordedAppSyncTransport::fail(AppSyncTransportError::unavailable("offline"));

        let error = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::AppLaunch,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![],
                known_conflicts: vec![],
            })
            .expect_err("recorded transport should fail");

        assert_eq!(error, AppSyncTransportError::unavailable("offline"));
        assert_eq!(transport.call_count(), 1);
    }
}
