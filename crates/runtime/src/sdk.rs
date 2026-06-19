use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use radroots_authority::{RadrootsActorContext, RadrootsLocalEventSigner};
use radroots_events::{
    RadrootsNostrEvent, RadrootsNostrEventPtr,
    contract::RadrootsActorRole,
    farm::RadrootsFarm,
    listing::RadrootsListing,
    order::{
        RadrootsOrderCancellation, RadrootsOrderDecision, RadrootsOrderFulfillmentUpdate,
        RadrootsOrderReceipt, RadrootsOrderRequest, RadrootsOrderRevisionDecision,
        RadrootsOrderRevisionProposal,
    },
};
use radroots_nostr::prelude::RadrootsNostrKeys;
use radroots_sdk::{
    FARM_PUBLISH_OPERATION_KIND, FarmEnqueuePublishRequest, FarmEnqueueReceipt, IntegrityReceipt,
    IntegrityRequest, LISTING_PUBLISH_OPERATION_KIND, ListingEnqueuePublishRequest,
    ListingEnqueueReceipt, ORDER_CANCELLATION_OPERATION_KIND, ORDER_DECISION_OPERATION_KIND,
    ORDER_FULFILLMENT_UPDATE_OPERATION_KIND, ORDER_RECEIPT_RECORD_OPERATION_KIND,
    ORDER_REVISION_DECISION_OPERATION_KIND, ORDER_REVISION_PROPOSAL_OPERATION_KIND,
    ORDER_SUBMIT_OPERATION_KIND, OrderCancellationEnqueueRequest, OrderCancellationReceipt,
    OrderDecisionEnqueueRequest, OrderDecisionReceipt, OrderEvidenceIngestRequest,
    OrderFulfillmentUpdateEnqueueRequest, OrderFulfillmentUpdateReceipt,
    OrderReceiptRecordEnqueueRequest, OrderReceiptRecordReceipt, OrderRequestEvidenceIngestRequest,
    OrderRevisionDecisionEnqueueRequest, OrderRevisionDecisionReceipt,
    OrderRevisionProposalEnqueueRequest, OrderRevisionProposalReceipt, OrderSubmitEnqueueRequest,
    OrderSubmitReceipt, RadrootsSdk, RadrootsSdkError, RadrootsSdkStoragePaths, RestoreReceipt,
    RestoreRequest, SdkBackupVerification, SdkRelayUrlPolicy as SdkRuntimeRelayUrlPolicy,
    StorageStatusReceipt, StorageStatusRequest, SyncStatusReceipt, SyncStatusRequest,
};
use radroots_sdk::{SdkMutationState, SdkRelayTargetPolicy};
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;
use tokio::runtime::Builder as TokioRuntimeBuilder;

use crate::AppDesktopRuntimePaths;

pub const APP_SDK_STORAGE_DIR_NAME: &str = "sdk";
pub const APP_SDK_DEFAULT_COMMAND_QUEUE_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkRelayUrlPolicy {
    Public,
    Localhost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkLifecycleState {
    Starting,
    Ready,
    Degraded,
    Pausing,
    Paused,
    Restoring,
    RebuildingProjections,
    ShuttingDown,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkConfig {
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub command_queue_capacity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkStoragePaths {
    pub event_store_path: PathBuf,
    pub outbox_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRuntimeIssue {
    pub code: String,
    pub class: String,
    pub retryable: bool,
    pub message: String,
    pub recovery_actions: Vec<String>,
    pub detail_json: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRuntimeStatus {
    pub state: AppSdkLifecycleState,
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub storage_paths: Option<AppSdkStoragePaths>,
    pub last_issue: Option<AppSdkRuntimeIssue>,
    pub projection_lifecycle: AppSdkProjectionLifecycleStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkDiagnostics {
    pub runtime: AppSdkRuntimeStatus,
    pub storage: AppSdkStorageDiagnostics,
    pub integrity: AppSdkIntegrityDiagnostics,
    pub sync: AppSdkSyncDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkStorageDiagnostics {
    pub storage_kind: String,
    pub paths: Option<AppSdkStoragePaths>,
    pub event_store: AppSdkEventStoreDiagnostics,
    pub outbox: AppSdkOutboxDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkSqliteStoreDiagnostics {
    pub schema_version: i64,
    pub journal_mode: String,
    pub foreign_keys_enabled: bool,
    pub busy_timeout_ms: i64,
    pub integrity_ok: bool,
    pub integrity_result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkEventStoreDiagnostics {
    pub store: AppSdkSqliteStoreDiagnostics,
    pub total_events: i64,
    pub projection_eligible_events: i64,
    pub relay_observations: i64,
    pub last_event_seq: Option<i64>,
    pub last_event_updated_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkOutboxDiagnostics {
    pub store: AppSdkSqliteStoreDiagnostics,
    pub total_events: i64,
    pub pending_events: i64,
    pub retryable_events: i64,
    pub terminal_events: i64,
    pub failed_terminal_events: i64,
    pub ready_signed_events: i64,
    pub publishing_events: i64,
    pub last_attempt_at_ms: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkIntegrityDiagnostics {
    pub checked_paths: Vec<PathBuf>,
    pub event_store_ok: bool,
    pub outbox_ok: bool,
    pub event_store_result: String,
    pub outbox_result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkSyncDiagnostics {
    pub source: String,
    pub observed_at_ms: i64,
    pub event_store: AppSdkSyncEventStoreDiagnostics,
    pub outbox: AppSdkSyncOutboxDiagnostics,
    pub relay_targets: AppSdkSyncRelayTargetDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkSyncEventStoreDiagnostics {
    pub total_events: i64,
    pub projection_eligible_events: i64,
    pub relay_observations: i64,
    pub last_event_seq: Option<i64>,
    pub last_event_updated_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkSyncOutboxDiagnostics {
    pub total_events: i64,
    pub pending_events: i64,
    pub retryable_events: i64,
    pub terminal_events: i64,
    pub failed_terminal_events: i64,
    pub ready_signed_events: i64,
    pub publishing_events: i64,
    pub last_attempt_at_ms: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkSyncRelayTargetDiagnostics {
    pub configured_count: usize,
    pub configured_relays: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRestorePreflightRequest {
    pub source: PathBuf,
    pub overwrite_existing_sdk_storage: bool,
}

pub struct AppSdkFarmPublishRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub farm: RadrootsFarm,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkListingPublishRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub listing: RadrootsListing,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderSubmitRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub listing_event: RadrootsNostrEventPtr,
    pub order: RadrootsOrderRequest,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderDecisionRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub request_event: RadrootsNostrEvent,
    pub request_event_ptr: RadrootsNostrEventPtr,
    pub decision: RadrootsOrderDecision,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderRevisionProposalRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub evidence_events: Vec<RadrootsNostrEvent>,
    pub root_event: RadrootsNostrEventPtr,
    pub previous_event: RadrootsNostrEventPtr,
    pub proposal: RadrootsOrderRevisionProposal,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderRevisionDecisionRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub evidence_events: Vec<RadrootsNostrEvent>,
    pub root_event: RadrootsNostrEventPtr,
    pub previous_event: RadrootsNostrEventPtr,
    pub decision: RadrootsOrderRevisionDecision,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderCancellationRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub evidence_events: Vec<RadrootsNostrEvent>,
    pub root_event: RadrootsNostrEventPtr,
    pub previous_event: RadrootsNostrEventPtr,
    pub cancellation: RadrootsOrderCancellation,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderFulfillmentUpdateRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub evidence_events: Vec<RadrootsNostrEvent>,
    pub root_event: RadrootsNostrEventPtr,
    pub previous_event: RadrootsNostrEventPtr,
    pub fulfillment: RadrootsOrderFulfillmentUpdate,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct AppSdkOrderReceiptRecordRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer_keys: RadrootsNostrKeys,
    pub evidence_events: Vec<RadrootsNostrEvent>,
    pub root_event: RadrootsNostrEventPtr,
    pub previous_event: RadrootsNostrEventPtr,
    pub receipt: RadrootsOrderReceipt,
    pub target_relays: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkWorkflowReceipt {
    pub operation_kind: String,
    pub expected_event_id: String,
    pub signed_event_id: String,
    pub outbox_operation_id: i64,
    pub outbox_event_id: i64,
    pub state: String,
    pub idempotency_digest_prefix: Option<String>,
    pub actor_pubkey: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRestorePreflightReceipt {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub state: String,
    pub destination_paths: Option<AppSdkStoragePaths>,
    pub restored_paths: Option<AppSdkStoragePaths>,
    pub event_store_path: PathBuf,
    pub outbox_path: PathBuf,
    pub manifest_path: PathBuf,
    pub verification: AppSdkBackupVerificationDiagnostics,
    pub source_storage: AppSdkStorageDiagnostics,
    pub projection_lifecycle: AppSdkProjectionLifecycleStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkBackupVerificationDiagnostics {
    pub event_store_ok: bool,
    pub outbox_ok: bool,
    pub event_store_events: i64,
    pub outbox_events: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkProjectionLifecycleStatus {
    pub state: AppSdkProjectionLifecycleState,
    pub reason: Option<String>,
    pub restore_source: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkProjectionLifecycleState {
    Current,
    Stale,
    Rebuilding,
}

#[derive(Debug, Error)]
pub enum AppSdkRuntimeError {
    #[error("app sdk command queue capacity must be greater than zero")]
    CommandQueueCapacityZero,
    #[error("failed to start app sdk worker: {0}")]
    WorkerSpawn(#[from] io::Error),
    #[error("app sdk command queue is full")]
    CommandQueueFull,
    #[error("app sdk command queue is closed")]
    CommandQueueClosed,
    #[error("app sdk command response channel is closed")]
    CommandResponseClosed,
    #[error("app sdk command failed: {0}")]
    CommandFailed(AppSdkRuntimeIssue),
    #[error("app sdk shutdown acknowledgement failed")]
    ShutdownAck,
    #[error("app sdk worker failed to join")]
    WorkerJoin,
}

#[derive(Debug)]
pub struct AppSdkRuntime {
    command_sender: Mutex<Option<SyncSender<AppSdkWorkerCommand>>>,
    shared: Arc<AppSdkRuntimeShared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug)]
struct AppSdkRuntimeShared {
    status: Mutex<AppSdkRuntimeStatus>,
    status_changed: Condvar,
    shutdown_requested: AtomicBool,
}

enum AppSdkWorkerCommand {
    StorageStatus(mpsc::Sender<Result<AppSdkStorageDiagnostics, AppSdkRuntimeIssue>>),
    IntegrityStatus(mpsc::Sender<Result<AppSdkIntegrityDiagnostics, AppSdkRuntimeIssue>>),
    SyncStatus(mpsc::Sender<Result<AppSdkSyncDiagnostics, AppSdkRuntimeIssue>>),
    Diagnostics(mpsc::Sender<Result<AppSdkDiagnostics, AppSdkRuntimeIssue>>),
    RestorePreflight(
        AppSdkRestorePreflightRequest,
        mpsc::Sender<Result<AppSdkRestorePreflightReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueFarmPublish(
        AppSdkFarmPublishRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueListingPublish(
        AppSdkListingPublishRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderSubmit(
        AppSdkOrderSubmitRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderDecision(
        AppSdkOrderDecisionRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderRevisionProposal(
        AppSdkOrderRevisionProposalRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderRevisionDecision(
        AppSdkOrderRevisionDecisionRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderCancellation(
        AppSdkOrderCancellationRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderFulfillmentUpdate(
        AppSdkOrderFulfillmentUpdateRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    EnqueueOrderReceiptRecord(
        AppSdkOrderReceiptRecordRequest,
        mpsc::Sender<Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue>>,
    ),
    BeginProjectionRebuild(
        mpsc::Sender<Result<AppSdkProjectionLifecycleStatus, AppSdkRuntimeIssue>>,
    ),
    CompleteProjectionRebuild(
        mpsc::Sender<Result<AppSdkProjectionLifecycleStatus, AppSdkRuntimeIssue>>,
    ),
}

impl fmt::Debug for AppSdkWorkerCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageStatus(_) => formatter.write_str("StorageStatus"),
            Self::IntegrityStatus(_) => formatter.write_str("IntegrityStatus"),
            Self::SyncStatus(_) => formatter.write_str("SyncStatus"),
            Self::Diagnostics(_) => formatter.write_str("Diagnostics"),
            Self::RestorePreflight(_, _) => formatter.write_str("RestorePreflight"),
            Self::EnqueueFarmPublish(_, _) => formatter.write_str("EnqueueFarmPublish"),
            Self::EnqueueListingPublish(_, _) => formatter.write_str("EnqueueListingPublish"),
            Self::EnqueueOrderSubmit(_, _) => formatter.write_str("EnqueueOrderSubmit"),
            Self::EnqueueOrderDecision(_, _) => formatter.write_str("EnqueueOrderDecision"),
            Self::EnqueueOrderRevisionProposal(_, _) => {
                formatter.write_str("EnqueueOrderRevisionProposal")
            }
            Self::EnqueueOrderRevisionDecision(_, _) => {
                formatter.write_str("EnqueueOrderRevisionDecision")
            }
            Self::EnqueueOrderCancellation(_, _) => formatter.write_str("EnqueueOrderCancellation"),
            Self::EnqueueOrderFulfillmentUpdate(_, _) => {
                formatter.write_str("EnqueueOrderFulfillmentUpdate")
            }
            Self::EnqueueOrderReceiptRecord(_, _) => {
                formatter.write_str("EnqueueOrderReceiptRecord")
            }
            Self::BeginProjectionRebuild(_) => formatter.write_str("BeginProjectionRebuild"),
            Self::CompleteProjectionRebuild(_) => formatter.write_str("CompleteProjectionRebuild"),
        }
    }
}

impl AppSdkConfig {
    pub fn from_desktop_paths(paths: &AppDesktopRuntimePaths, relay_urls: Vec<String>) -> Self {
        Self::from_app_data_root(paths.app.data.as_path(), relay_urls)
    }

    pub fn from_app_data_root(data_root: &Path, relay_urls: Vec<String>) -> Self {
        Self {
            storage_root: app_sdk_storage_root_from_data_root(data_root),
            relay_url_policy: app_sdk_relay_url_policy(relay_urls.as_slice()),
            relay_urls,
            command_queue_capacity: APP_SDK_DEFAULT_COMMAND_QUEUE_CAPACITY,
        }
    }

    pub fn with_command_queue_capacity(mut self, capacity: usize) -> Self {
        self.command_queue_capacity = capacity;
        self
    }
}

impl AppSdkRestorePreflightRequest {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            overwrite_existing_sdk_storage: false,
        }
    }

    pub fn with_overwrite_existing_sdk_storage(mut self, overwrite: bool) -> Self {
        self.overwrite_existing_sdk_storage = overwrite;
        self
    }
}

impl AppSdkProjectionLifecycleStatus {
    pub fn current() -> Self {
        Self {
            state: AppSdkProjectionLifecycleState::Current,
            reason: None,
            restore_source: None,
        }
    }

    fn stale(reason: impl Into<String>, restore_source: Option<PathBuf>) -> Self {
        Self {
            state: AppSdkProjectionLifecycleState::Stale,
            reason: Some(reason.into()),
            restore_source,
        }
    }

    fn rebuilding(reason: impl Into<String>, restore_source: Option<PathBuf>) -> Self {
        Self {
            state: AppSdkProjectionLifecycleState::Rebuilding,
            reason: Some(reason.into()),
            restore_source,
        }
    }
}

impl AppSdkRuntime {
    pub fn start(config: AppSdkConfig) -> Result<Self, AppSdkRuntimeError> {
        if config.command_queue_capacity == 0 {
            return Err(AppSdkRuntimeError::CommandQueueCapacityZero);
        }

        let initial_status =
            AppSdkRuntimeStatus::from_config(&config, AppSdkLifecycleState::Starting, None, None);
        let shared = Arc::new(AppSdkRuntimeShared {
            status: Mutex::new(initial_status),
            status_changed: Condvar::new(),
            shutdown_requested: AtomicBool::new(false),
        });
        let (command_sender, command_receiver) = mpsc::sync_channel(config.command_queue_capacity);
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("radroots-app-sdk-runtime".to_owned())
            .spawn(move || run_app_sdk_worker(config, worker_shared, command_receiver))?;

        Ok(Self {
            command_sender: Mutex::new(Some(command_sender)),
            shared,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn status(&self) -> AppSdkRuntimeStatus {
        lock_status(&self.shared).clone()
    }

    pub fn storage_status(&self) -> Result<AppSdkStorageDiagnostics, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::StorageStatus)
    }

    pub fn integrity_status(&self) -> Result<AppSdkIntegrityDiagnostics, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::IntegrityStatus)
    }

    pub fn sync_status(&self) -> Result<AppSdkSyncDiagnostics, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::SyncStatus)
    }

    pub fn diagnostics(&self) -> Result<AppSdkDiagnostics, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::Diagnostics)
    }

    pub fn restore_preflight(
        &self,
        request: AppSdkRestorePreflightRequest,
    ) -> Result<AppSdkRestorePreflightReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::RestorePreflight(request, response_sender)
        })
    }

    pub fn enqueue_farm_publish(
        &self,
        request: AppSdkFarmPublishRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueFarmPublish(request, response_sender)
        })
    }

    pub fn enqueue_listing_publish(
        &self,
        request: AppSdkListingPublishRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueListingPublish(request, response_sender)
        })
    }

    pub fn enqueue_order_submit(
        &self,
        request: AppSdkOrderSubmitRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderSubmit(request, response_sender)
        })
    }

    pub fn enqueue_order_decision(
        &self,
        request: AppSdkOrderDecisionRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderDecision(request, response_sender)
        })
    }

    pub fn enqueue_order_revision_proposal(
        &self,
        request: AppSdkOrderRevisionProposalRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderRevisionProposal(request, response_sender)
        })
    }

    pub fn enqueue_order_revision_decision(
        &self,
        request: AppSdkOrderRevisionDecisionRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderRevisionDecision(request, response_sender)
        })
    }

    pub fn enqueue_order_cancellation(
        &self,
        request: AppSdkOrderCancellationRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderCancellation(request, response_sender)
        })
    }

    pub fn enqueue_order_fulfillment_update(
        &self,
        request: AppSdkOrderFulfillmentUpdateRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderFulfillmentUpdate(request, response_sender)
        })
    }

    pub fn enqueue_order_receipt_record(
        &self,
        request: AppSdkOrderReceiptRecordRequest,
    ) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeError> {
        self.run_command(|response_sender| {
            AppSdkWorkerCommand::EnqueueOrderReceiptRecord(request, response_sender)
        })
    }

    pub fn begin_projection_rebuild(
        &self,
    ) -> Result<AppSdkProjectionLifecycleStatus, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::BeginProjectionRebuild)
    }

    pub fn complete_projection_rebuild(
        &self,
    ) -> Result<AppSdkProjectionLifecycleStatus, AppSdkRuntimeError> {
        self.run_command(AppSdkWorkerCommand::CompleteProjectionRebuild)
    }

    pub fn wait_for_startup(&self, timeout: Duration) -> AppSdkRuntimeStatus {
        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(Instant::now);
        let mut status = lock_status(&self.shared);
        loop {
            if !matches!(status.state, AppSdkLifecycleState::Starting) {
                return status.clone();
            }
            let now = Instant::now();
            if now >= deadline {
                return status.clone();
            }
            let remaining = deadline.saturating_duration_since(now);
            let wait_result = self.shared.status_changed.wait_timeout(status, remaining);
            let (next_status, timeout_result) = wait_result.unwrap_or_else(|poisoned| {
                let (guard, timeout_result) = poisoned.into_inner();
                (guard, timeout_result)
            });
            status = next_status;
            if timeout_result.timed_out() {
                return status.clone();
            }
        }
    }

    pub fn shutdown(&self) -> Result<(), AppSdkRuntimeError> {
        if matches!(self.status().state, AppSdkLifecycleState::Stopped) {
            return self.join_worker();
        }

        self.shared.shutdown_requested.store(true, Ordering::SeqCst);
        transition_status_state(&self.shared, AppSdkLifecycleState::ShuttingDown);
        let command_sender = self
            .command_sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        drop(command_sender);
        self.join_worker()
    }

    fn join_worker(&self) -> Result<(), AppSdkRuntimeError> {
        let mut worker = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(worker) = worker.take() else {
            return Ok(());
        };
        worker.join().map_err(|_| AppSdkRuntimeError::WorkerJoin)
    }

    fn run_command<T>(
        &self,
        command: impl FnOnce(mpsc::Sender<Result<T, AppSdkRuntimeIssue>>) -> AppSdkWorkerCommand,
    ) -> Result<T, AppSdkRuntimeError> {
        let (response_sender, response_receiver) = mpsc::channel();
        let command_sender = {
            let command_sender = self
                .command_sender
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if self.shared.shutdown_requested.load(Ordering::SeqCst) {
                return Err(AppSdkRuntimeError::CommandQueueClosed);
            }
            command_sender
                .as_ref()
                .cloned()
                .ok_or(AppSdkRuntimeError::CommandQueueClosed)?
        };
        match command_sender.try_send(command(response_sender)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => return Err(AppSdkRuntimeError::CommandQueueFull),
            Err(TrySendError::Disconnected(_)) => {
                return Err(AppSdkRuntimeError::CommandQueueClosed);
            }
        }
        response_receiver
            .recv()
            .map_err(|_| AppSdkRuntimeError::CommandResponseClosed)?
            .map_err(AppSdkRuntimeError::CommandFailed)
    }
}

impl Drop for AppSdkRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl From<AppSdkRelayUrlPolicy> for SdkRuntimeRelayUrlPolicy {
    fn from(policy: AppSdkRelayUrlPolicy) -> Self {
        match policy {
            AppSdkRelayUrlPolicy::Public => Self::Public,
            AppSdkRelayUrlPolicy::Localhost => Self::Localhost,
        }
    }
}

impl From<&RadrootsSdkStoragePaths> for AppSdkStoragePaths {
    fn from(paths: &RadrootsSdkStoragePaths) -> Self {
        Self {
            event_store_path: paths.event_store_path.clone(),
            outbox_path: paths.outbox_path.clone(),
        }
    }
}

impl AppSdkRuntimeIssue {
    fn from_sdk_error(error: &RadrootsSdkError) -> Self {
        Self {
            code: error.code().to_owned(),
            class: sdk_error_class_label(error),
            retryable: error.retryable(),
            message: error.to_string(),
            recovery_actions: error
                .recovery_actions()
                .into_iter()
                .filter_map(|action| serde_json::to_value(action).ok())
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect(),
            detail_json: error.detail_json(),
        }
    }

    fn runtime_error(code: &'static str, message: String) -> Self {
        Self {
            code: code.to_owned(),
            class: "runtime".to_owned(),
            retryable: true,
            message: message.clone(),
            recovery_actions: vec!["retry_startup".to_owned()],
            detail_json: json!({
                "code": code,
                "class": "runtime",
                "retryable": true,
                "message": message,
                "recovery_actions": ["retry_startup"],
                "detail": {}
            }),
        }
    }

    fn lifecycle_blocked(state: AppSdkLifecycleState) -> Self {
        Self {
            code: "sdk_lifecycle_busy".to_owned(),
            class: "runtime".to_owned(),
            retryable: true,
            message: format!("app sdk runtime is {:?}", state),
            recovery_actions: vec!["wait_for_sdk_lifecycle".to_owned()],
            detail_json: json!({
                "code": "sdk_lifecycle_busy",
                "class": "runtime",
                "retryable": true,
                "state": format!("{state:?}"),
                "recovery_actions": ["wait_for_sdk_lifecycle"]
            }),
        }
    }
}

impl fmt::Display for AppSdkRuntimeIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl AppSdkRuntimeStatus {
    fn from_config(
        config: &AppSdkConfig,
        state: AppSdkLifecycleState,
        storage_paths: Option<AppSdkStoragePaths>,
        last_issue: Option<AppSdkRuntimeIssue>,
    ) -> Self {
        Self {
            state,
            storage_root: config.storage_root.clone(),
            relay_urls: config.relay_urls.clone(),
            relay_url_policy: config.relay_url_policy,
            storage_paths,
            last_issue,
            projection_lifecycle: AppSdkProjectionLifecycleStatus::current(),
        }
    }
}

impl From<StorageStatusReceipt> for AppSdkStorageDiagnostics {
    fn from(receipt: StorageStatusReceipt) -> Self {
        Self {
            storage_kind: serialized_label(&receipt.storage),
            paths: receipt.paths.as_ref().map(AppSdkStoragePaths::from),
            event_store: AppSdkEventStoreDiagnostics {
                store: receipt.event_store.store.into(),
                total_events: receipt.event_store.total_events,
                projection_eligible_events: receipt.event_store.projection_eligible_events,
                relay_observations: receipt.event_store.relay_observations,
                last_event_seq: receipt.event_store.last_event_seq,
                last_event_updated_at_ms: receipt.event_store.last_event_updated_at_ms,
            },
            outbox: AppSdkOutboxDiagnostics {
                store: receipt.outbox.store.into(),
                total_events: receipt.outbox.total_events,
                pending_events: receipt.outbox.pending_events,
                retryable_events: receipt.outbox.retryable_events,
                terminal_events: receipt.outbox.terminal_events,
                failed_terminal_events: receipt.outbox.failed_terminal_events,
                ready_signed_events: receipt.outbox.ready_signed_events,
                publishing_events: receipt.outbox.publishing_events,
                last_attempt_at_ms: receipt.outbox.last_attempt_at_ms,
                last_error: receipt.outbox.last_error,
            },
        }
    }
}

impl From<radroots_sdk::SdkSqliteStoreStatus> for AppSdkSqliteStoreDiagnostics {
    fn from(status: radroots_sdk::SdkSqliteStoreStatus) -> Self {
        Self {
            schema_version: status.schema_version,
            journal_mode: status.journal_mode,
            foreign_keys_enabled: status.foreign_keys_enabled,
            busy_timeout_ms: status.busy_timeout_ms,
            integrity_ok: status.integrity_ok,
            integrity_result: status.integrity_result,
        }
    }
}

impl From<IntegrityReceipt> for AppSdkIntegrityDiagnostics {
    fn from(receipt: IntegrityReceipt) -> Self {
        Self {
            checked_paths: receipt.checked_paths,
            event_store_ok: receipt.event_store_ok,
            outbox_ok: receipt.outbox_ok,
            event_store_result: receipt.event_store_result,
            outbox_result: receipt.outbox_result,
        }
    }
}

impl From<SyncStatusReceipt> for AppSdkSyncDiagnostics {
    fn from(receipt: SyncStatusReceipt) -> Self {
        Self {
            source: serialized_label(&receipt.source),
            observed_at_ms: receipt.observed_at_ms,
            event_store: AppSdkSyncEventStoreDiagnostics {
                total_events: receipt.event_store.total_events,
                projection_eligible_events: receipt.event_store.projection_eligible_events,
                relay_observations: receipt.event_store.relay_observations,
                last_event_seq: receipt.event_store.last_event_seq,
                last_event_updated_at_ms: receipt.event_store.last_event_updated_at_ms,
            },
            outbox: AppSdkSyncOutboxDiagnostics {
                total_events: receipt.outbox.total_events,
                pending_events: receipt.outbox.pending_events,
                retryable_events: receipt.outbox.retryable_events,
                terminal_events: receipt.outbox.terminal_events,
                failed_terminal_events: receipt.outbox.failed_terminal_events,
                ready_signed_events: receipt.outbox.ready_signed_events,
                publishing_events: receipt.outbox.publishing_events,
                last_attempt_at_ms: receipt.outbox.last_attempt_at_ms,
                last_error: receipt.outbox.last_error,
            },
            relay_targets: AppSdkSyncRelayTargetDiagnostics {
                configured_count: receipt.relay_targets.configured_count,
                configured_relays: receipt.relay_targets.configured_relays,
            },
        }
    }
}

impl From<SdkBackupVerification> for AppSdkBackupVerificationDiagnostics {
    fn from(verification: SdkBackupVerification) -> Self {
        Self {
            event_store_ok: verification.event_store_ok,
            outbox_ok: verification.outbox_ok,
            event_store_events: verification.event_store_events,
            outbox_events: verification.outbox_events,
        }
    }
}

impl AppSdkRestorePreflightReceipt {
    fn from_restore_receipt(
        receipt: RestoreReceipt,
        destination: PathBuf,
        projection_lifecycle: AppSdkProjectionLifecycleStatus,
    ) -> Self {
        Self {
            source: receipt.source,
            destination: receipt.destination.unwrap_or(destination),
            state: serialized_label(&receipt.state),
            destination_paths: receipt
                .destination_paths
                .as_ref()
                .map(AppSdkStoragePaths::from),
            restored_paths: receipt
                .restored_paths
                .as_ref()
                .map(AppSdkStoragePaths::from),
            event_store_path: receipt.event_store_path,
            outbox_path: receipt.outbox_path,
            manifest_path: receipt.manifest_path,
            verification: receipt.verification.into(),
            source_storage: receipt.manifest.source_status.into(),
            projection_lifecycle,
        }
    }
}

pub fn app_sdk_storage_root_from_data_root(data_root: &Path) -> PathBuf {
    data_root.join(APP_SDK_STORAGE_DIR_NAME)
}

fn app_sdk_relay_url_policy(relay_urls: &[String]) -> AppSdkRelayUrlPolicy {
    if relay_urls
        .iter()
        .any(|relay_url| relay_url.trim().to_ascii_lowercase().starts_with("ws://"))
    {
        AppSdkRelayUrlPolicy::Localhost
    } else {
        AppSdkRelayUrlPolicy::Public
    }
}

fn run_app_sdk_worker(
    config: AppSdkConfig,
    shared: Arc<AppSdkRuntimeShared>,
    command_receiver: Receiver<AppSdkWorkerCommand>,
) {
    let runtime = match TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Degraded,
                    None,
                    Some(AppSdkRuntimeIssue::runtime_error(
                        "tokio_runtime_init",
                        error.to_string(),
                    )),
                ),
            );
            run_degraded_worker(config, shared, command_receiver);
            return;
        }
    };

    let mut sdk = match runtime.block_on(build_sdk_runtime(&config)) {
        Ok(sdk) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Ready,
                    sdk.storage_paths().map(AppSdkStoragePaths::from),
                    None,
                ),
            );
            Some(sdk)
        }
        Err(error) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Degraded,
                    None,
                    Some(AppSdkRuntimeIssue::from_sdk_error(&error)),
                ),
            );
            None
        }
    };

    while let Ok(command) = command_receiver.recv() {
        if shared.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        match command {
            AppSdkWorkerCommand::StorageStatus(response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => runtime
                            .block_on(sdk.storage_status(StorageStatusRequest::new()))
                            .map(AppSdkStorageDiagnostics::from)
                            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error)),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::IntegrityStatus(response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => runtime
                            .block_on(sdk.integrity(IntegrityRequest::new()))
                            .map(AppSdkIntegrityDiagnostics::from)
                            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error)),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::SyncStatus(response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => runtime
                            .block_on(sdk.sync().status(SyncStatusRequest::new()))
                            .map(AppSdkSyncDiagnostics::from)
                            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error)),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::Diagnostics(response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => {
                            let mut runtime_status = lock_status(&shared).clone();
                            runtime_status.last_issue = None;
                            runtime
                                .block_on(collect_sdk_diagnostics(sdk, runtime_status))
                                .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))
                        }
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::RestorePreflight(request, response_sender) => {
                let result = match sdk.as_ref() {
                    Some(_) => run_restore_preflight(&runtime, &shared, &config, request),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueFarmPublish(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_farm_publish_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueListingPublish(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_listing_publish_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderSubmit(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_order_submit_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderDecision(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_order_decision_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderRevisionProposal(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => {
                            enqueue_order_revision_proposal_with_sdk(&runtime, sdk, request)
                        }
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderRevisionDecision(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => {
                            enqueue_order_revision_decision_with_sdk(&runtime, sdk, request)
                        }
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderCancellation(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_order_cancellation_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderFulfillmentUpdate(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => {
                            enqueue_order_fulfillment_update_with_sdk(&runtime, sdk, request)
                        }
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::EnqueueOrderReceiptRecord(request, response_sender) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_order_receipt_record_with_sdk(&runtime, sdk, request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::BeginProjectionRebuild(response_sender) => {
                let result = match sdk.as_ref() {
                    Some(_) => Ok(begin_projection_rebuild(&shared)),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                send_worker_result(&shared, response_sender, result);
            }
            AppSdkWorkerCommand::CompleteProjectionRebuild(response_sender) => {
                let result = match sdk.as_ref() {
                    Some(_) => complete_projection_rebuild(&shared),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                send_worker_result(&shared, response_sender, result);
            }
        }
    }

    drop(sdk.take());
    transition_status_state(&shared, AppSdkLifecycleState::Stopped);
}

fn run_degraded_worker(
    config: AppSdkConfig,
    shared: Arc<AppSdkRuntimeShared>,
    command_receiver: Receiver<AppSdkWorkerCommand>,
) {
    while let Ok(command) = command_receiver.recv() {
        if shared.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        match command {
            AppSdkWorkerCommand::StorageStatus(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::IntegrityStatus(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::SyncStatus(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::Diagnostics(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::RestorePreflight(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueFarmPublish(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueListingPublish(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderSubmit(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderDecision(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderRevisionProposal(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderRevisionDecision(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderCancellation(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderFulfillmentUpdate(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::EnqueueOrderReceiptRecord(_, response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::BeginProjectionRebuild(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
            AppSdkWorkerCommand::CompleteProjectionRebuild(response_sender) => {
                send_worker_result(
                    &shared,
                    response_sender,
                    Err(runtime_unavailable_issue(&shared)),
                );
            }
        }
    }

    let last_issue = lock_status(&shared).last_issue.clone();
    replace_status(
        &shared,
        AppSdkRuntimeStatus::from_config(&config, AppSdkLifecycleState::Stopped, None, last_issue),
    );
}

async fn build_sdk_runtime(config: &AppSdkConfig) -> Result<RadrootsSdk, RadrootsSdkError> {
    let mut builder = RadrootsSdk::builder()
        .directory_storage(config.storage_root.clone())
        .relay_url_policy(config.relay_url_policy.into());
    for relay_url in &config.relay_urls {
        builder = builder.relay_url(relay_url.clone());
    }
    builder.build().await
}

fn run_restore_preflight(
    runtime: &tokio::runtime::Runtime,
    shared: &AppSdkRuntimeShared,
    config: &AppSdkConfig,
    request: AppSdkRestorePreflightRequest,
) -> Result<AppSdkRestorePreflightReceipt, AppSdkRuntimeIssue> {
    if let Some(issue) = lifecycle_busy_issue(shared) {
        return Err(issue);
    }
    transition_status_state(shared, AppSdkLifecycleState::Pausing);
    transition_status_state(shared, AppSdkLifecycleState::Paused);
    transition_status_state(shared, AppSdkLifecycleState::Restoring);

    let restore_request = RestoreRequest::new(request.source.clone())
        .with_destination(config.storage_root.clone())
        .with_overwrite(request.overwrite_existing_sdk_storage)
        .dry_run();
    let result = runtime
        .block_on(RadrootsSdk::restore(restore_request))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))
        .map(|receipt| {
            let projection_lifecycle = mark_projections_stale(
                shared,
                "sdk_restore_preflight",
                Some(request.source.clone()),
            );
            AppSdkRestorePreflightReceipt::from_restore_receipt(
                receipt,
                config.storage_root.clone(),
                projection_lifecycle,
            )
        });
    if result.is_err() {
        transition_status_state(shared, AppSdkLifecycleState::Ready);
    }
    result
}

async fn collect_sdk_diagnostics(
    sdk: &RadrootsSdk,
    runtime: AppSdkRuntimeStatus,
) -> Result<AppSdkDiagnostics, RadrootsSdkError> {
    let storage = sdk.storage_status(StorageStatusRequest::new()).await?;
    let integrity = sdk.integrity(IntegrityRequest::new()).await?;
    let sync = sdk.sync().status(SyncStatusRequest::new()).await?;
    Ok(AppSdkDiagnostics {
        runtime,
        storage: storage.into(),
        integrity: integrity.into(),
        sync: sync.into(),
    })
}

fn enqueue_farm_publish_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkFarmPublishRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Farmer,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    let mut enqueue = FarmEnqueuePublishRequest::new(actor, request.farm, target_relays);
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.farms().enqueue_publish(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_farm_receipt(receipt, request.actor_pubkey))
}

fn enqueue_listing_publish_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkListingPublishRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    let mut enqueue = ListingEnqueuePublishRequest::new(actor, request.listing, target_relays);
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.listings().enqueue_publish(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_listing_receipt(receipt, request.actor_pubkey))
}

fn enqueue_order_submit_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderSubmitRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    let mut enqueue =
        OrderSubmitEnqueueRequest::new(actor, request.listing_event, request.order, target_relays);
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_submit(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_receipt(receipt, request.actor_pubkey))
}

fn enqueue_order_decision_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderDecisionRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    runtime
        .block_on(
            sdk.orders()
                .ingest_request_evidence(OrderRequestEvidenceIngestRequest::new(
                    request.request_event,
                )),
        )
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    let mut enqueue = OrderDecisionEnqueueRequest::new(
        actor,
        request.request_event_ptr,
        request.decision,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_decision(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_decision_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn enqueue_order_revision_proposal_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderRevisionProposalRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    ingest_order_evidence_with_sdk(runtime, sdk, request.evidence_events)?;
    let mut enqueue = OrderRevisionProposalEnqueueRequest::new(
        actor,
        request.root_event,
        request.previous_event,
        request.proposal,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_revision_proposal(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_revision_proposal_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn enqueue_order_revision_decision_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderRevisionDecisionRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    ingest_order_evidence_with_sdk(runtime, sdk, request.evidence_events)?;
    let mut enqueue = OrderRevisionDecisionEnqueueRequest::new(
        actor,
        request.root_event,
        request.previous_event,
        request.decision,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_revision_decision(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_revision_decision_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn enqueue_order_cancellation_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderCancellationRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    ingest_order_evidence_with_sdk(runtime, sdk, request.evidence_events)?;
    let mut enqueue = OrderCancellationEnqueueRequest::new(
        actor,
        request.root_event,
        request.previous_event,
        request.cancellation,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_cancellation(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_cancellation_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn enqueue_order_fulfillment_update_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderFulfillmentUpdateRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    ingest_order_evidence_with_sdk(runtime, sdk, request.evidence_events)?;
    let mut enqueue = OrderFulfillmentUpdateEnqueueRequest::new(
        actor,
        request.root_event,
        request.previous_event,
        request.fulfillment,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_fulfillment_update(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_fulfillment_update_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn enqueue_order_receipt_record_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    request: AppSdkOrderReceiptRecordRequest,
) -> Result<AppSdkWorkflowReceipt, AppSdkRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let signer = sdk_local_signer(request.signer_keys)?;
    let target_relays = sdk_relay_targets(request.target_relays, request.relay_url_policy)?;
    ingest_order_evidence_with_sdk(runtime, sdk, request.evidence_events)?;
    let mut enqueue = OrderReceiptRecordEnqueueRequest::new(
        actor,
        request.root_event,
        request.previous_event,
        request.receipt,
        target_relays,
    );
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = runtime
        .block_on(sdk.orders().enqueue_receipt_record(enqueue, &signer))
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    Ok(app_sdk_order_receipt_record_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn ingest_order_evidence_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsSdk,
    evidence_events: Vec<RadrootsNostrEvent>,
) -> Result<(), AppSdkRuntimeIssue> {
    for event in evidence_events {
        runtime
            .block_on(
                sdk.orders()
                    .ingest_evidence(OrderEvidenceIngestRequest::new(event)),
            )
            .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))?;
    }
    Ok(())
}

fn sdk_actor_context(
    actor_pubkey: &str,
    actor_account_id: &str,
    role: RadrootsActorRole,
) -> Result<RadrootsActorContext, AppSdkRuntimeIssue> {
    RadrootsActorContext::local_account(actor_pubkey, actor_account_id.to_owned(), [role]).map_err(
        |error| AppSdkRuntimeIssue::runtime_error("sdk_actor_context_invalid", error.to_string()),
    )
}

fn sdk_local_signer(
    keys: RadrootsNostrKeys,
) -> Result<RadrootsLocalEventSigner, AppSdkRuntimeIssue> {
    RadrootsLocalEventSigner::new(keys).map_err(|error| {
        AppSdkRuntimeIssue::runtime_error("sdk_signer_init_failed", error.to_string())
    })
}

fn sdk_relay_targets(
    relays: Vec<String>,
    policy: AppSdkRelayUrlPolicy,
) -> Result<SdkRelayTargetPolicy, AppSdkRuntimeIssue> {
    SdkRelayTargetPolicy::try_explicit(relays, policy.into())
        .map_err(|error| AppSdkRuntimeIssue::from_sdk_error(&error))
}

fn app_sdk_farm_receipt(
    receipt: FarmEnqueueReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: FARM_PUBLISH_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_listing_receipt(
    receipt: ListingEnqueueReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: LISTING_PUBLISH_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_receipt(
    receipt: OrderSubmitReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_SUBMIT_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_decision_receipt(
    receipt: OrderDecisionReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_DECISION_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_revision_proposal_receipt(
    receipt: OrderRevisionProposalReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_REVISION_PROPOSAL_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_revision_decision_receipt(
    receipt: OrderRevisionDecisionReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_REVISION_DECISION_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_cancellation_receipt(
    receipt: OrderCancellationReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_CANCELLATION_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_fulfillment_update_receipt(
    receipt: OrderFulfillmentUpdateReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_FULFILLMENT_UPDATE_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn app_sdk_order_receipt_record_receipt(
    receipt: OrderReceiptRecordReceipt,
    actor_pubkey: String,
) -> AppSdkWorkflowReceipt {
    AppSdkWorkflowReceipt {
        operation_kind: ORDER_RECEIPT_RECORD_OPERATION_KIND.to_owned(),
        expected_event_id: receipt.expected_event_id.as_str().to_owned(),
        signed_event_id: receipt.signed_event_id.as_str().to_owned(),
        outbox_operation_id: receipt.outbox_operation_id,
        outbox_event_id: receipt.outbox_event_id,
        state: sdk_mutation_state_key(receipt.state).to_owned(),
        idempotency_digest_prefix: receipt.idempotency_digest_prefix,
        actor_pubkey,
    }
}

fn sdk_mutation_state_key(state: SdkMutationState) -> &'static str {
    match state {
        SdkMutationState::StoredAndQueued => "enqueued",
        SdkMutationState::AlreadyQueued => "already_queued",
        _ => "unknown",
    }
}

fn send_worker_result<T>(
    shared: &AppSdkRuntimeShared,
    response_sender: mpsc::Sender<Result<T, AppSdkRuntimeIssue>>,
    result: Result<T, AppSdkRuntimeIssue>,
) {
    set_last_issue(
        shared,
        match &result {
            Ok(_) => None,
            Err(issue) => Some(issue.clone()),
        },
    );
    let _ = response_sender.send(result);
}

fn lifecycle_busy_issue(shared: &AppSdkRuntimeShared) -> Option<AppSdkRuntimeIssue> {
    let state = lock_status(shared).state;
    if matches!(
        state,
        AppSdkLifecycleState::Pausing
            | AppSdkLifecycleState::Paused
            | AppSdkLifecycleState::Restoring
            | AppSdkLifecycleState::RebuildingProjections
            | AppSdkLifecycleState::ShuttingDown
    ) {
        Some(AppSdkRuntimeIssue::lifecycle_blocked(state))
    } else {
        None
    }
}

fn runtime_unavailable_issue(shared: &AppSdkRuntimeShared) -> AppSdkRuntimeIssue {
    let status = lock_status(shared).clone();
    if let Some(issue) = status.last_issue {
        issue
    } else {
        AppSdkRuntimeIssue::runtime_error(
            "sdk_runtime_not_ready",
            format!("app sdk runtime is {:?}", status.state),
        )
    }
}

fn replace_status(shared: &AppSdkRuntimeShared, status: AppSdkRuntimeStatus) {
    *lock_status(shared) = status;
    shared.status_changed.notify_all();
}

fn set_last_issue(shared: &AppSdkRuntimeShared, issue: Option<AppSdkRuntimeIssue>) {
    lock_status(shared).last_issue = issue;
    shared.status_changed.notify_all();
}

fn transition_status_state(shared: &AppSdkRuntimeShared, state: AppSdkLifecycleState) {
    lock_status(shared).state = state;
    shared.status_changed.notify_all();
}

fn mark_projections_stale(
    shared: &AppSdkRuntimeShared,
    reason: impl Into<String>,
    restore_source: Option<PathBuf>,
) -> AppSdkProjectionLifecycleStatus {
    let mut status = lock_status(shared);
    status.projection_lifecycle = AppSdkProjectionLifecycleStatus::stale(reason, restore_source);
    status.state = AppSdkLifecycleState::Ready;
    let projection_lifecycle = status.projection_lifecycle.clone();
    shared.status_changed.notify_all();
    projection_lifecycle
}

fn begin_projection_rebuild(shared: &AppSdkRuntimeShared) -> AppSdkProjectionLifecycleStatus {
    let restore_source = lock_status(shared)
        .projection_lifecycle
        .restore_source
        .clone();
    let mut status = lock_status(shared);
    status.state = AppSdkLifecycleState::RebuildingProjections;
    status.projection_lifecycle =
        AppSdkProjectionLifecycleStatus::rebuilding("sdk_projection_rebuild", restore_source);
    let projection_lifecycle = status.projection_lifecycle.clone();
    shared.status_changed.notify_all();
    projection_lifecycle
}

fn complete_projection_rebuild(
    shared: &AppSdkRuntimeShared,
) -> Result<AppSdkProjectionLifecycleStatus, AppSdkRuntimeIssue> {
    let mut status = lock_status(shared);
    if !matches!(status.state, AppSdkLifecycleState::RebuildingProjections) {
        return Err(AppSdkRuntimeIssue::lifecycle_blocked(status.state));
    }
    status.state = AppSdkLifecycleState::Ready;
    status.projection_lifecycle = AppSdkProjectionLifecycleStatus::current();
    let projection_lifecycle = status.projection_lifecycle.clone();
    shared.status_changed.notify_all();
    Ok(projection_lifecycle)
}

fn lock_status(shared: &AppSdkRuntimeShared) -> MutexGuard<'_, AppSdkRuntimeStatus> {
    shared
        .status
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sdk_error_class_label(error: &RadrootsSdkError) -> String {
    serde_json::to_value(error.class())
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{:?}", error.class()))
}

fn serialized_label(value: &(impl Serialize + fmt::Debug)) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use radroots_core::{
        RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreQuantity,
        RadrootsCoreQuantityPrice, RadrootsCoreUnit,
    };
    use radroots_events::{
        farm::RadrootsFarmRef,
        ids::{RadrootsDTag, RadrootsInventoryBinId},
        listing::{
            RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
            RadrootsListingDeliveryMethod, RadrootsListingLocation, RadrootsListingProduct,
            RadrootsListingStatus,
        },
    };
    use radroots_nostr::prelude::{RadrootsNostrKeys, RadrootsNostrSecretKey};
    use radroots_sdk::{
        BackupRequest, LISTING_PUBLISH_OPERATION_KIND, RadrootsSdk,
        SdkRelayUrlPolicy as SdkRuntimeRelayUrlPolicy,
    };

    use crate::{
        APP_RUNTIME_NAMESPACE, AppDesktopRuntimePaths, AppRuntimeHostEnvironment,
        AppRuntimePlatform,
    };

    use super::{
        APP_SDK_STORAGE_DIR_NAME, AppSdkConfig, AppSdkLifecycleState, AppSdkListingPublishRequest,
        AppSdkProjectionLifecycleState, AppSdkRelayUrlPolicy, AppSdkRestorePreflightRequest,
        AppSdkRuntime, AppSdkRuntimeError, AppSdkRuntimeShared, AppSdkRuntimeStatus,
        AppSdkWorkerCommand, app_sdk_storage_root_from_data_root, transition_status_state,
    };

    const SDK_TEST_SELLER_SECRET_KEY_HEX: &str =
        "10c5304d6c9ae3a1a16f7860f1cc8f5e3a76225a2663b3a989a0d775919b7df5";

    #[test]
    fn sdk_config_uses_app_data_sdk_storage_root() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some("/Users/treesap".into()),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("desktop paths should resolve");
        let config =
            AppSdkConfig::from_desktop_paths(&paths, vec!["wss://relay.example".to_owned()]);

        assert_eq!(
            config.storage_root,
            paths.app.data.join(APP_SDK_STORAGE_DIR_NAME)
        );
        assert_eq!(
            config.storage_root,
            app_sdk_storage_root_from_data_root(paths.app.data.as_path())
        );
        assert_eq!(config.storage_root.parent(), Some(paths.app.data.as_path()));
        assert!(paths.app.data.ends_with(APP_RUNTIME_NAMESPACE));
        assert_eq!(config.relay_url_policy, AppSdkRelayUrlPolicy::Public);
    }

    #[test]
    fn sdk_config_uses_localhost_policy_for_ws_relay_urls() {
        let config = AppSdkConfig::from_app_data_root(
            "/tmp/radroots-app-data".as_ref(),
            vec![
                "wss://relay.example".to_owned(),
                "ws://127.0.0.1:8080".to_owned(),
            ],
        );

        assert_eq!(config.relay_url_policy, AppSdkRelayUrlPolicy::Localhost);
    }

    #[test]
    fn sdk_runtime_reaches_ready_with_directory_storage() {
        let storage_root = temp_storage_root("ready");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");

        let status = runtime.wait_for_startup(Duration::from_secs(5));

        assert_eq!(status.state, AppSdkLifecycleState::Ready);
        assert_eq!(status.storage_root, storage_root);
        assert_eq!(status.relay_url_policy, AppSdkRelayUrlPolicy::Localhost);
        let storage_paths = status
            .storage_paths
            .expect("storage paths should be present");
        assert_eq!(
            storage_paths.event_store_path,
            storage_root.join("event_store.sqlite")
        );
        assert_eq!(
            storage_paths.outbox_path,
            storage_root.join("outbox.sqlite")
        );
        let storage = runtime
            .storage_status()
            .expect("storage diagnostics should load");
        assert_eq!(storage.storage_kind, "directory");
        assert!(storage.event_store.store.integrity_ok);
        assert!(storage.outbox.store.integrity_ok);
        let integrity = runtime
            .integrity_status()
            .expect("integrity diagnostics should load");
        assert!(integrity.event_store_ok);
        assert!(integrity.outbox_ok);
        let sync = runtime.sync_status().expect("sync diagnostics should load");
        assert_eq!(sync.source, "sdk_canonical_stores");
        assert_eq!(sync.relay_targets.configured_count, 1);
        let diagnostics = runtime.diagnostics().expect("diagnostics should load");
        assert_eq!(diagnostics.runtime.state, AppSdkLifecycleState::Ready);
        assert_eq!(diagnostics.storage.storage_kind, "directory");
        assert_eq!(diagnostics.sync.relay_targets.configured_count, 1);
        runtime.shutdown().expect("sdk runtime should shut down");
        assert_eq!(runtime.status().state, AppSdkLifecycleState::Stopped);
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn sdk_runtime_enqueues_listing_publish_work() {
        let storage_root = temp_storage_root("listing_enqueue");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");
        assert_eq!(
            runtime.wait_for_startup(Duration::from_secs(5)).state,
            AppSdkLifecycleState::Ready
        );
        let secret_key = RadrootsNostrSecretKey::from_hex(SDK_TEST_SELLER_SECRET_KEY_HEX)
            .expect("secret key should parse");
        let signer_keys = RadrootsNostrKeys::new(secret_key);
        let seller_pubkey = signer_keys.public_key().to_hex();

        let receipt = runtime
            .enqueue_listing_publish(AppSdkListingPublishRequest {
                actor_account_id: "seller-account".to_owned(),
                actor_pubkey: seller_pubkey.clone(),
                signer_keys,
                listing: test_listing(seller_pubkey.as_str()),
                target_relays: vec!["ws://127.0.0.1:8080".to_owned()],
                relay_url_policy: AppSdkRelayUrlPolicy::Localhost,
                idempotency_key: Some("listing-enqueue-idempotency".to_owned()),
            })
            .expect("listing publish should enqueue");

        assert_eq!(receipt.operation_kind, LISTING_PUBLISH_OPERATION_KIND);
        assert_eq!(receipt.actor_pubkey, seller_pubkey);
        assert_eq!(receipt.state, "enqueued");
        assert!(!receipt.expected_event_id.is_empty());
        assert_eq!(receipt.expected_event_id, receipt.signed_event_id);
        assert!(receipt.outbox_operation_id > 0);
        assert!(receipt.outbox_event_id > 0);
        assert!(receipt.idempotency_digest_prefix.is_some());
        let sync = runtime.sync_status().expect("sync diagnostics should load");
        assert_eq!(sync.outbox.ready_signed_events, 1);
        runtime.shutdown().expect("sdk runtime should shut down");
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn sdk_runtime_degrades_with_structured_sdk_error() {
        let storage_root = temp_storage_root("invalid_relay");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://relay.example".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");

        let status = runtime.wait_for_startup(Duration::from_secs(5));

        assert_eq!(status.state, AppSdkLifecycleState::Degraded);
        let issue = status
            .last_issue
            .expect("degraded status should include issue");
        assert_eq!(issue.code, "invalid_relay_url");
        assert_eq!(issue.class, "configuration");
        assert!(!issue.retryable);
        assert!(
            issue
                .recovery_actions
                .contains(&"configure_relay_targets".to_owned())
        );
        assert_eq!(issue.detail_json["code"], "invalid_relay_url");
        let error = runtime
            .diagnostics()
            .expect_err("degraded diagnostics should fail");
        match error {
            AppSdkRuntimeError::CommandFailed(issue) => {
                assert_eq!(issue.code, "invalid_relay_url");
                assert_eq!(issue.class, "configuration");
                assert_eq!(issue.detail_json["code"], "invalid_relay_url");
            }
            unexpected => panic!("unexpected degraded diagnostics error: {unexpected:?}"),
        }
        runtime.shutdown().expect("sdk runtime should shut down");
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn sdk_shutdown_joins_when_normal_command_queue_is_full() {
        let config = AppSdkConfig::from_app_data_root(
            "/tmp/radroots-app-sdk-full-queue".as_ref(),
            vec!["ws://127.0.0.1:8080".to_owned()],
        )
        .with_command_queue_capacity(1);
        let shared = Arc::new(AppSdkRuntimeShared {
            status: Mutex::new(AppSdkRuntimeStatus::from_config(
                &config,
                AppSdkLifecycleState::Ready,
                None,
                None,
            )),
            status_changed: Condvar::new(),
            shutdown_requested: AtomicBool::new(false),
        });
        let (command_sender, command_receiver) = mpsc::sync_channel(config.command_queue_capacity);
        let worker_shared = Arc::clone(&shared);
        let worker = thread::spawn(move || {
            while !worker_shared.shutdown_requested.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
            drop(command_receiver);
            transition_status_state(&worker_shared, AppSdkLifecycleState::Stopped);
        });
        let runtime = AppSdkRuntime {
            command_sender: Mutex::new(Some(command_sender)),
            shared,
            worker: Mutex::new(Some(worker)),
        };
        let (response_sender, _response_receiver) = mpsc::channel();
        runtime
            .command_sender
            .lock()
            .expect("command sender lock")
            .as_ref()
            .expect("command sender")
            .try_send(AppSdkWorkerCommand::Diagnostics(response_sender))
            .expect("normal command queue should fill");

        assert!(matches!(
            runtime.sync_status(),
            Err(AppSdkRuntimeError::CommandQueueFull)
        ));
        assert_eq!(runtime.status().state, AppSdkLifecycleState::Ready);

        runtime
            .shutdown()
            .expect("shutdown should not depend on normal command queue capacity");

        assert_eq!(runtime.status().state, AppSdkLifecycleState::Stopped);
    }

    #[test]
    fn sdk_restore_preflight_marks_projections_stale_without_writing_destination() {
        let backup_source_root = temp_storage_root("restore_backup_source");
        let backup_archive = backup_source_root
            .parent()
            .expect("backup source should have parent")
            .join("backup_archive");
        let tokio = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let sdk = tokio
            .block_on(
                RadrootsSdk::builder()
                    .directory_storage(backup_source_root.clone())
                    .relay_url_policy(SdkRuntimeRelayUrlPolicy::Localhost)
                    .relay_url("ws://127.0.0.1:8080")
                    .build(),
            )
            .expect("source sdk should build");
        tokio
            .block_on(sdk.backup(BackupRequest::new(backup_archive.clone())))
            .expect("backup should complete");

        let app_storage_root = temp_storage_root("restore_preflight_destination");
        let app_data_root = app_storage_root
            .parent()
            .expect("app storage root should have parent")
            .to_path_buf();
        let config = AppSdkConfig::from_app_data_root(
            app_data_root.as_path(),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");
        assert_eq!(
            runtime.wait_for_startup(Duration::from_secs(5)).state,
            AppSdkLifecycleState::Ready
        );
        let sentinel = app_storage_root.join("restore-preflight-sentinel");
        fs::write(&sentinel, "existing destination").expect("sentinel should write");

        let receipt = runtime
            .restore_preflight(
                AppSdkRestorePreflightRequest::new(backup_archive.clone())
                    .with_overwrite_existing_sdk_storage(true),
            )
            .expect("restore preflight should succeed");

        assert_eq!(receipt.state, "dry_run");
        assert_eq!(receipt.destination, app_storage_root);
        assert_eq!(receipt.restored_paths, None);
        assert!(sentinel.exists());
        assert_eq!(
            receipt.projection_lifecycle.state,
            AppSdkProjectionLifecycleState::Stale
        );
        assert_eq!(
            receipt.projection_lifecycle.reason.as_deref(),
            Some("sdk_restore_preflight")
        );
        assert_eq!(
            runtime.status().projection_lifecycle.state,
            AppSdkProjectionLifecycleState::Stale
        );
        assert_eq!(runtime.status().state, AppSdkLifecycleState::Ready);
        runtime.shutdown().expect("sdk runtime should shut down");
        let _ = fs::remove_dir_all(
            backup_source_root
                .parent()
                .expect("backup source should have parent"),
        );
        let _ = fs::remove_dir_all(app_data_root);
    }

    #[test]
    fn sdk_projection_rebuild_state_rejects_conflicting_commands() {
        let storage_root = temp_storage_root("projection_rebuild");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");
        assert_eq!(
            runtime.wait_for_startup(Duration::from_secs(5)).state,
            AppSdkLifecycleState::Ready
        );

        let rebuilding = runtime
            .begin_projection_rebuild()
            .expect("projection rebuild should start");

        assert_eq!(rebuilding.state, AppSdkProjectionLifecycleState::Rebuilding);
        assert_eq!(
            runtime.status().state,
            AppSdkLifecycleState::RebuildingProjections
        );
        let error = runtime
            .sync_status()
            .expect_err("sync status should wait for rebuild completion");
        match error {
            AppSdkRuntimeError::CommandFailed(issue) => {
                assert_eq!(issue.code, "sdk_lifecycle_busy");
                assert_eq!(issue.detail_json["state"], "RebuildingProjections");
            }
            unexpected => panic!("unexpected lifecycle error: {unexpected:?}"),
        }

        let complete = runtime
            .complete_projection_rebuild()
            .expect("projection rebuild should complete");

        assert_eq!(complete.state, AppSdkProjectionLifecycleState::Current);
        assert_eq!(runtime.status().state, AppSdkLifecycleState::Ready);
        runtime
            .sync_status()
            .expect("sync status should work after rebuild");
        runtime.shutdown().expect("sdk runtime should shut down");
        let _ = fs::remove_dir_all(storage_root);
    }

    fn temp_storage_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("radroots_studio_app_sdk_runtime_{label}_{nanos}"))
            .join(APP_SDK_STORAGE_DIR_NAME)
    }

    fn test_listing(seller_pubkey: &str) -> RadrootsListing {
        let bin_id = RadrootsInventoryBinId::parse("bin-1").expect("bin id");
        RadrootsListing {
            d_tag: RadrootsDTag::parse("AAAAAAAAAAAAAAAAAAAAAQ").expect("d tag"),
            published_at: None,
            farm: RadrootsFarmRef {
                pubkey: seller_pubkey.to_owned(),
                d_tag: "AAAAAAAAAAAAAAAAAAAAAA".to_owned(),
            },
            product: RadrootsListingProduct {
                key: "coffee".to_owned(),
                title: "Coffee".to_owned(),
                category: "coffee".to_owned(),
                summary: Some("Single origin coffee".to_owned()),
                process: None,
                lot: None,
                location: None,
                profile: None,
                year: None,
            },
            primary_bin_id: bin_id.clone(),
            bins: vec![RadrootsListingBin {
                bin_id,
                quantity: RadrootsCoreQuantity::new(
                    RadrootsCoreDecimal::from(1000u32),
                    RadrootsCoreUnit::MassG,
                ),
                price_per_canonical_unit: RadrootsCoreQuantityPrice {
                    amount: RadrootsCoreMoney::new(
                        RadrootsCoreDecimal::from(20u32),
                        RadrootsCoreCurrency::USD,
                    ),
                    quantity: RadrootsCoreQuantity::new(
                        RadrootsCoreDecimal::from(1u32),
                        RadrootsCoreUnit::MassG,
                    ),
                },
                display_amount: None,
                display_unit: None,
                display_label: None,
                display_price: None,
                display_price_unit: None,
            }],
            resource_area: None,
            plot: None,
            discounts: None,
            inventory_available: Some(RadrootsCoreDecimal::from(5u32)),
            availability: Some(RadrootsListingAvailability::Status {
                status: RadrootsListingStatus::Active,
            }),
            delivery_method: Some(RadrootsListingDeliveryMethod::Pickup),
            location: Some(RadrootsListingLocation {
                primary: "North Farm".to_owned(),
                city: None,
                region: None,
                country: Some("US".to_owned()),
                lat: None,
                lng: None,
                geohash: None,
            }),
            images: None,
        }
    }
}
