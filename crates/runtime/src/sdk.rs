use std::{
    fmt,
    future::Future,
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};

use radroots_authority::{RadrootsActorContext, RadrootsLocalEventSigner};
use radroots_event::{
    RadrootsEventPtr,
    contract::RadrootsActorRole,
    farm::{RadrootsFarm, RadrootsFarmPublicLocation},
    ids::{
        RadrootsAddressableCoordinate, RadrootsListingAddress, RadrootsOrderId, RadrootsPublicKey,
    },
    kinds::KIND_FARM,
    listing::{RadrootsListing, RadrootsListingPublicLocation},
    order::{
        RadrootsOrderEconomics, RadrootsOrderInventoryCommitment, RadrootsOrderItem,
        RadrootsOrderRequest,
    },
};
use radroots_nostr::prelude::RadrootsNostrKeys;
use radroots_sdk::SdkMutationState;
use radroots_sdk::{
    FARM_PUBLISH_OPERATION_KIND, FarmEnqueuePublishRequest, FarmEnqueueReceipt, IntegrityReceipt,
    IntegrityRequest, LISTING_PUBLISH_OPERATION_KIND, ListingEnqueuePublishRequest,
    ListingEnqueueReceipt, NostrProfile, NostrRelayUrlPolicy, PrivacyPreflightConfirmation,
    ProductSensitivityField, PublishMode, RadrootsClient, RadrootsSdkError,
    RadrootsSdkLocalKeySigner, RadrootsSdkSignerProvider, RadrootsSdkStoragePaths, RestoreReceipt,
    RestoreRequest, SatisfactionPolicy, SdkBackupVerification, SdkPublicLocality,
    StorageStatusReceipt, StorageStatusRequest, SyncStatusReceipt, SyncStatusRequest,
    TRADE_CANCELLATION_OPERATION_KIND, TRADE_DECISION_OPERATION_KIND, TRADE_SUBMIT_OPERATION_KIND,
    TargetPolicy, TradeAcceptRequest, TradeCancelRequest, TradeCancellationPlan,
    TradeCancellationReceipt, TradeDecisionPlan, TradeDecisionReceipt, TradeDeclineRequest,
    TradeEvidenceMode, TradeMutationOutcome, TradeProposeRequest, TradeSubmitPlan,
    TradeSubmitReceipt, TransportProfile,
};
use radroots_trade::identity::RadrootsTradeLocator;
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;
use tokio::runtime::Builder as TokioRuntimeBuilder;

use crate::AppDesktopRuntimePaths;

pub const DESKTOP_RUNTIME_STORAGE_DIR_NAME: &str = "sdk";
pub const DESKTOP_RUNTIME_DEFAULT_EFFECT_QUEUE_CAPACITY: usize = 32;
const DESKTOP_RUNTIME_SDK_EFFECT_TIMEOUT_MS: u64 = 2_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeRelayUrlPolicy {
    Public,
    Localhost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeLifecycleState {
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DesktopRuntimeStartupMilestone {
    ShellReady,
    RuntimeStoreReady,
    PrivateStoreReady,
    SignerReady,
    ProjectionsReady,
    NetworkObserved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeSupervisorConfig {
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: DesktopRuntimeRelayUrlPolicy,
    pub effect_queue_capacity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeStoragePaths {
    pub runtime_path: PathBuf,
    pub private_path: PathBuf,
    pub studio_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeIssue {
    pub code: Box<str>,
    pub class: Box<str>,
    pub retryable: bool,
    pub message: Box<str>,
    pub recovery_actions: Box<[String]>,
    pub detail_json: Box<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeSnapshot {
    pub state: DesktopRuntimeLifecycleState,
    pub startup_milestones: Vec<DesktopRuntimeStartupMilestone>,
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: DesktopRuntimeRelayUrlPolicy,
    pub storage_paths: Option<DesktopRuntimeStoragePaths>,
    pub last_issue: Option<DesktopRuntimeIssue>,
    pub last_effect: Option<DesktopRuntimeEffectStatus>,
    pub storage_diagnostics: Option<DesktopRuntimeStorageDiagnostics>,
    pub integrity_diagnostics: Option<DesktopRuntimeIntegrityDiagnostics>,
    pub sync_diagnostics: Option<DesktopRuntimeSyncDiagnostics>,
    pub projection_lifecycle: DesktopRuntimeProjectionLifecycleStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeDiagnostics {
    pub runtime: DesktopRuntimeSnapshot,
    pub storage: DesktopRuntimeStorageDiagnostics,
    pub integrity: DesktopRuntimeIntegrityDiagnostics,
    pub sync: DesktopRuntimeSyncDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeStorageDiagnostics {
    pub storage_kind: String,
    pub paths: Option<DesktopRuntimeStoragePaths>,
    pub event_store: DesktopRuntimeEventStoreDiagnostics,
    pub outbox: DesktopRuntimeOutboxDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeSqliteStoreDiagnostics {
    pub schema_version: i64,
    pub journal_mode: String,
    pub foreign_keys_enabled: bool,
    pub busy_timeout_ms: i64,
    pub integrity_ok: bool,
    pub integrity_result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeEventStoreDiagnostics {
    pub store: DesktopRuntimeSqliteStoreDiagnostics,
    pub total_events: i64,
    pub projection_eligible_events: i64,
    pub transport_observations: i64,
    pub last_event_seq: Option<i64>,
    pub last_event_updated_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeOutboxDiagnostics {
    pub store: DesktopRuntimeSqliteStoreDiagnostics,
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
pub struct DesktopRuntimeIntegrityDiagnostics {
    pub checked_paths: Vec<PathBuf>,
    pub event_store_ok: bool,
    pub outbox_ok: bool,
    pub event_store_result: String,
    pub outbox_result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeSyncDiagnostics {
    pub source: String,
    pub observed_at_ms: i64,
    pub event_store: DesktopRuntimeSyncEventStoreDiagnostics,
    pub outbox: DesktopRuntimeSyncOutboxDiagnostics,
    pub transport_targets: DesktopRuntimeSyncTransportTargetDiagnostics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeSyncEventStoreDiagnostics {
    pub total_events: i64,
    pub projection_eligible_events: i64,
    pub transport_observations: i64,
    pub last_event_seq: Option<i64>,
    pub last_event_updated_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeSyncOutboxDiagnostics {
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
pub struct DesktopRuntimeSyncTransportTargetDiagnostics {
    pub configured_count: usize,
    pub configured_targets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeRestorePreflightRequest {
    pub source: PathBuf,
    pub overwrite_existing_sdk_storage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopRuntimeFarmPublicLocationRequest {
    pub actor_pubkey: String,
    pub farm_d_tag: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopRuntimePublicFarmLocation {
    pub primary: String,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    pub geohash5: String,
}

pub struct DesktopRuntimeLocalSigner {
    keys: RadrootsNostrKeys,
}

impl DesktopRuntimeLocalSigner {
    pub fn from_local_identity_keys(keys: RadrootsNostrKeys) -> Self {
        Self { keys }
    }

    fn into_keys(self) -> RadrootsNostrKeys {
        self.keys
    }
}

pub struct DesktopRuntimeFarmPublishRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer: DesktopRuntimeLocalSigner,
    pub farm: RadrootsFarm,
    pub target_relays: Vec<String>,
    pub relay_url_policy: DesktopRuntimeRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct DesktopRuntimeListingPublishRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer: DesktopRuntimeLocalSigner,
    pub listing: RadrootsListing,
    pub target_relays: Vec<String>,
    pub relay_url_policy: DesktopRuntimeRelayUrlPolicy,
    pub idempotency_key: Option<String>,
}

pub struct DesktopRuntimeTradeProposeRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer: DesktopRuntimeLocalSigner,
    pub listing_event: RadrootsEventPtr,
    pub order_id: RadrootsOrderId,
    pub listing_addr: RadrootsListingAddress,
    pub seller_pubkey: RadrootsPublicKey,
    pub items: Vec<RadrootsOrderItem>,
    pub economics: RadrootsOrderEconomics,
    pub public_note: Option<String>,
    pub confirm_public_note: bool,
    pub idempotency_key: Option<String>,
}

pub enum DesktopRuntimeTradeDecision {
    Accept {
        inventory_commitments: Vec<RadrootsOrderInventoryCommitment>,
    },
    Decline {
        reason: String,
    },
}

pub struct DesktopRuntimeTradeDecisionRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer: DesktopRuntimeLocalSigner,
    pub locator: RadrootsTradeLocator,
    pub decision: DesktopRuntimeTradeDecision,
    pub confirm_public_note: bool,
    pub idempotency_key: Option<String>,
}

pub struct DesktopRuntimeTradeCancellationRequest {
    pub actor_account_id: String,
    pub actor_pubkey: String,
    pub signer: DesktopRuntimeLocalSigner,
    pub locator: RadrootsTradeLocator,
    pub reason: String,
    pub confirm_public_note: bool,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeWorkflowReceipt {
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
pub struct DesktopRuntimeRestorePreflightReceipt {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub state: String,
    pub destination_paths: Option<DesktopRuntimeStoragePaths>,
    pub restored_paths: Option<DesktopRuntimeStoragePaths>,
    pub runtime_path: PathBuf,
    pub private_path: PathBuf,
    pub studio_path: PathBuf,
    pub manifest_path: PathBuf,
    pub verification: DesktopRuntimeBackupVerificationDiagnostics,
    pub source_storage: DesktopRuntimeStorageDiagnostics,
    pub projection_lifecycle: DesktopRuntimeProjectionLifecycleStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeBackupVerificationDiagnostics {
    pub event_store_ok: bool,
    pub outbox_ok: bool,
    pub event_store_events: i64,
    pub outbox_events: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeProjectionLifecycleStatus {
    pub state: DesktopRuntimeProjectionLifecycleState,
    pub reason: Option<String>,
    pub restore_source: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeProjectionLifecycleState {
    Current,
    Stale,
    Rebuilding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeEffectKind {
    RefreshDiagnostics,
    RestorePreflight,
    FarmPublish,
    ListingPublish,
    TradePropose,
    TradeDecision,
    TradeCancellation,
    BeginProjectionRebuild,
    CompleteProjectionRebuild,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopRuntimeEffectState {
    Accepted,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopRuntimeEffectReceipt {
    pub effect_id: u64,
    pub effect_kind: DesktopRuntimeEffectKind,
    pub operation_kind: Option<String>,
    pub actor_pubkey: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopRuntimeEffectStatus {
    pub receipt: DesktopRuntimeEffectReceipt,
    pub state: DesktopRuntimeEffectState,
    pub issue: Option<DesktopRuntimeIssue>,
    pub workflow_receipt: Option<DesktopRuntimeWorkflowReceipt>,
    pub restore_preflight: Option<DesktopRuntimeRestorePreflightReceipt>,
}

#[derive(Debug, Error)]
pub enum DesktopRuntimeSupervisorError {
    #[error("desktop runtime supervisor effect queue capacity must be greater than zero")]
    EffectQueueCapacityZero,
    #[error("failed to start desktop runtime supervisor worker: {0}")]
    WorkerSpawn(#[from] io::Error),
    #[error("desktop runtime supervisor effect queue is full")]
    EffectQueueFull,
    #[error("desktop runtime supervisor effect queue is closed")]
    EffectQueueClosed,
    #[error("desktop runtime supervisor is unavailable: {0}")]
    Unavailable(DesktopRuntimeIssue),
}

#[derive(Debug)]
pub struct DesktopRuntimeSupervisor {
    command_sender: Mutex<Option<SyncSender<DesktopRuntimeEffect>>>,
    shared: Arc<DesktopRuntimeSupervisorShared>,
    next_effect_id: AtomicU64,
}

#[derive(Debug)]
struct DesktopRuntimeSupervisorShared {
    status: Mutex<DesktopRuntimeSnapshot>,
    shutdown_requested: AtomicBool,
}

enum DesktopRuntimeEffect {
    RefreshDiagnostics(DesktopRuntimeEffectReceipt),
    RestorePreflight(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeRestorePreflightRequest>,
    ),
    EnqueueFarmPublish(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeFarmPublishRequest>,
    ),
    EnqueueListingPublish(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeListingPublishRequest>,
    ),
    TradePropose(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeTradeProposeRequest>,
    ),
    TradeDecision(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeTradeDecisionRequest>,
    ),
    TradeCancellation(
        DesktopRuntimeEffectReceipt,
        Box<DesktopRuntimeTradeCancellationRequest>,
    ),
    BeginProjectionRebuild(DesktopRuntimeEffectReceipt),
    CompleteProjectionRebuild(DesktopRuntimeEffectReceipt),
}

impl fmt::Debug for DesktopRuntimeEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RefreshDiagnostics(_) => formatter.write_str("RefreshDiagnostics"),
            Self::RestorePreflight(_, _) => formatter.write_str("RestorePreflight"),
            Self::EnqueueFarmPublish(_, _) => formatter.write_str("EnqueueFarmPublish"),
            Self::EnqueueListingPublish(_, _) => formatter.write_str("EnqueueListingPublish"),
            Self::TradePropose(_, _) => formatter.write_str("TradePropose"),
            Self::TradeDecision(_, _) => formatter.write_str("TradeDecision"),
            Self::TradeCancellation(_, _) => formatter.write_str("TradeCancellation"),
            Self::BeginProjectionRebuild(_) => formatter.write_str("BeginProjectionRebuild"),
            Self::CompleteProjectionRebuild(_) => formatter.write_str("CompleteProjectionRebuild"),
        }
    }
}

impl DesktopRuntimeSupervisorConfig {
    pub fn from_desktop_paths(paths: &AppDesktopRuntimePaths, relay_urls: Vec<String>) -> Self {
        Self::from_app_data_root(paths.app.data.as_path(), relay_urls)
    }

    pub fn from_app_data_root(data_root: &Path, relay_urls: Vec<String>) -> Self {
        Self {
            storage_root: desktop_runtime_storage_root_from_data_root(data_root),
            relay_url_policy: desktop_runtime_relay_url_policy(relay_urls.as_slice()),
            relay_urls,
            effect_queue_capacity: DESKTOP_RUNTIME_DEFAULT_EFFECT_QUEUE_CAPACITY,
        }
    }

    pub fn with_effect_queue_capacity(mut self, capacity: usize) -> Self {
        self.effect_queue_capacity = capacity;
        self
    }
}

impl DesktopRuntimeRestorePreflightRequest {
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

impl DesktopRuntimeProjectionLifecycleStatus {
    pub fn current() -> Self {
        Self {
            state: DesktopRuntimeProjectionLifecycleState::Current,
            reason: None,
            restore_source: None,
        }
    }

    fn stale(reason: impl Into<String>, restore_source: Option<PathBuf>) -> Self {
        Self {
            state: DesktopRuntimeProjectionLifecycleState::Stale,
            reason: Some(reason.into()),
            restore_source,
        }
    }

    fn rebuilding(reason: impl Into<String>, restore_source: Option<PathBuf>) -> Self {
        Self {
            state: DesktopRuntimeProjectionLifecycleState::Rebuilding,
            reason: Some(reason.into()),
            restore_source,
        }
    }
}

impl DesktopRuntimeSupervisor {
    pub fn start(
        config: DesktopRuntimeSupervisorConfig,
    ) -> Result<Self, DesktopRuntimeSupervisorError> {
        if config.effect_queue_capacity == 0 {
            return Err(DesktopRuntimeSupervisorError::EffectQueueCapacityZero);
        }

        let initial_status = DesktopRuntimeSnapshot::from_config(
            &config,
            DesktopRuntimeLifecycleState::Starting,
            Some(vec![DesktopRuntimeStartupMilestone::ShellReady]),
            None,
            None,
        );
        let shared = Arc::new(DesktopRuntimeSupervisorShared {
            status: Mutex::new(initial_status),
            shutdown_requested: AtomicBool::new(false),
        });
        let (command_sender, command_receiver) = mpsc::sync_channel(config.effect_queue_capacity);
        let worker_shared = Arc::clone(&shared);
        let _worker = thread::Builder::new()
            .name("radroots-desktop-runtime-supervisor".to_owned())
            .spawn(move || run_desktop_runtime_worker(config, worker_shared, command_receiver))?;

        Ok(Self {
            command_sender: Mutex::new(Some(command_sender)),
            shared,
            next_effect_id: AtomicU64::new(1),
        })
    }

    pub fn snapshot(&self) -> DesktopRuntimeSnapshot {
        lock_status(&self.shared).clone()
    }

    pub fn request_diagnostics_refresh(
        &self,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        self.submit_effect(
            DesktopRuntimeEffectKind::RefreshDiagnostics,
            None,
            None,
            DesktopRuntimeEffect::RefreshDiagnostics,
        )
    }

    pub fn request_restore_preflight(
        &self,
        request: DesktopRuntimeRestorePreflightRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        self.submit_effect(
            DesktopRuntimeEffectKind::RestorePreflight,
            None,
            None,
            |receipt| DesktopRuntimeEffect::RestorePreflight(receipt, Box::new(request)),
        )
    }

    pub fn enqueue_farm_publish(
        &self,
        request: DesktopRuntimeFarmPublishRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let actor_pubkey = Some(request.actor_pubkey.clone());
        self.submit_effect(
            DesktopRuntimeEffectKind::FarmPublish,
            Some(FARM_PUBLISH_OPERATION_KIND.to_owned()),
            actor_pubkey,
            |receipt| DesktopRuntimeEffect::EnqueueFarmPublish(receipt, Box::new(request)),
        )
    }

    pub fn enqueue_listing_publish(
        &self,
        request: DesktopRuntimeListingPublishRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let actor_pubkey = Some(request.actor_pubkey.clone());
        self.submit_effect(
            DesktopRuntimeEffectKind::ListingPublish,
            Some(LISTING_PUBLISH_OPERATION_KIND.to_owned()),
            actor_pubkey,
            |receipt| DesktopRuntimeEffect::EnqueueListingPublish(receipt, Box::new(request)),
        )
    }

    pub fn trade_propose(
        &self,
        request: DesktopRuntimeTradeProposeRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let actor_pubkey = Some(request.actor_pubkey.clone());
        self.submit_effect(
            DesktopRuntimeEffectKind::TradePropose,
            Some(TRADE_SUBMIT_OPERATION_KIND.to_owned()),
            actor_pubkey,
            |receipt| DesktopRuntimeEffect::TradePropose(receipt, Box::new(request)),
        )
    }

    pub fn trade_decide(
        &self,
        request: DesktopRuntimeTradeDecisionRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let actor_pubkey = Some(request.actor_pubkey.clone());
        self.submit_effect(
            DesktopRuntimeEffectKind::TradeDecision,
            Some(TRADE_DECISION_OPERATION_KIND.to_owned()),
            actor_pubkey,
            |receipt| DesktopRuntimeEffect::TradeDecision(receipt, Box::new(request)),
        )
    }

    pub fn trade_cancel(
        &self,
        request: DesktopRuntimeTradeCancellationRequest,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let actor_pubkey = Some(request.actor_pubkey.clone());
        self.submit_effect(
            DesktopRuntimeEffectKind::TradeCancellation,
            Some(TRADE_CANCELLATION_OPERATION_KIND.to_owned()),
            actor_pubkey,
            |receipt| DesktopRuntimeEffect::TradeCancellation(receipt, Box::new(request)),
        )
    }

    pub fn begin_projection_rebuild(
        &self,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        self.submit_effect(
            DesktopRuntimeEffectKind::BeginProjectionRebuild,
            None,
            None,
            DesktopRuntimeEffect::BeginProjectionRebuild,
        )
    }

    pub fn complete_projection_rebuild(
        &self,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        self.submit_effect(
            DesktopRuntimeEffectKind::CompleteProjectionRebuild,
            None,
            None,
            DesktopRuntimeEffect::CompleteProjectionRebuild,
        )
    }

    pub fn request_shutdown(&self) -> bool {
        if matches!(self.snapshot().state, DesktopRuntimeLifecycleState::Stopped) {
            return false;
        }

        self.shared.shutdown_requested.store(true, Ordering::SeqCst);
        transition_status_state(&self.shared, DesktopRuntimeLifecycleState::ShuttingDown);
        let command_sender = self
            .command_sender
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        drop(command_sender);
        true
    }

    fn submit_effect(
        &self,
        effect_kind: DesktopRuntimeEffectKind,
        operation_kind: Option<String>,
        actor_pubkey: Option<String>,
        effect: impl FnOnce(DesktopRuntimeEffectReceipt) -> DesktopRuntimeEffect,
    ) -> Result<DesktopRuntimeEffectReceipt, DesktopRuntimeSupervisorError> {
        let receipt = DesktopRuntimeEffectReceipt {
            effect_id: self.next_effect_id.fetch_add(1, Ordering::SeqCst),
            effect_kind,
            operation_kind,
            actor_pubkey,
        };
        let command_sender = {
            let command_sender = self
                .command_sender
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if self.shared.shutdown_requested.load(Ordering::SeqCst) {
                return Err(DesktopRuntimeSupervisorError::EffectQueueClosed);
            }
            command_sender
                .as_ref()
                .cloned()
                .ok_or(DesktopRuntimeSupervisorError::EffectQueueClosed)?
        };
        set_last_effect(
            &self.shared,
            DesktopRuntimeEffectStatus::accepted(receipt.clone()),
        );
        match command_sender.try_send(effect(receipt.clone())) {
            Ok(()) => Ok(receipt),
            Err(TrySendError::Full(_)) => {
                clear_last_effect(&self.shared, receipt.effect_id);
                Err(DesktopRuntimeSupervisorError::EffectQueueFull)
            }
            Err(TrySendError::Disconnected(_)) => {
                clear_last_effect(&self.shared, receipt.effect_id);
                Err(DesktopRuntimeSupervisorError::EffectQueueClosed)
            }
        }
    }
}

impl Drop for DesktopRuntimeSupervisor {
    fn drop(&mut self) {
        let _ = self.request_shutdown();
    }
}

impl From<DesktopRuntimeRelayUrlPolicy> for NostrRelayUrlPolicy {
    fn from(policy: DesktopRuntimeRelayUrlPolicy) -> Self {
        match policy {
            DesktopRuntimeRelayUrlPolicy::Public => Self::Public,
            DesktopRuntimeRelayUrlPolicy::Localhost => Self::Localhost,
        }
    }
}

impl From<&RadrootsSdkStoragePaths> for DesktopRuntimeStoragePaths {
    fn from(paths: &RadrootsSdkStoragePaths) -> Self {
        Self {
            runtime_path: paths.runtime_path.clone(),
            private_path: paths.private_path.clone(),
            studio_path: paths.studio_path.clone(),
        }
    }
}

impl DesktopRuntimeIssue {
    fn from_sdk_error(error: &RadrootsSdkError) -> Self {
        Self {
            code: error.code().into(),
            class: sdk_error_class_label(error).into_boxed_str(),
            retryable: error.retryable(),
            message: error.to_string().into_boxed_str(),
            recovery_actions: error
                .recovery_actions()
                .into_iter()
                .filter_map(|action| serde_json::to_value(action).ok())
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            detail_json: Box::new(error.detail_json()),
        }
    }

    fn runtime_error(code: &'static str, message: String) -> Self {
        Self {
            code: code.into(),
            class: "runtime".into(),
            retryable: true,
            message: message.clone().into_boxed_str(),
            recovery_actions: vec!["retry_startup".to_owned()].into_boxed_slice(),
            detail_json: Box::new(json!({
                "code": code,
                "class": "runtime",
                "retryable": true,
                "message": message,
                "recovery_actions": ["retry_startup"],
                "detail": {}
            })),
        }
    }

    fn lifecycle_blocked(state: DesktopRuntimeLifecycleState) -> Self {
        Self {
            code: "sdk_lifecycle_busy".into(),
            class: "runtime".into(),
            retryable: true,
            message: format!("app sdk runtime is {:?}", state).into_boxed_str(),
            recovery_actions: vec!["wait_for_sdk_lifecycle".to_owned()].into_boxed_slice(),
            detail_json: Box::new(json!({
                "code": "sdk_lifecycle_busy",
                "class": "runtime",
                "retryable": true,
                "state": format!("{state:?}"),
                "recovery_actions": ["wait_for_sdk_lifecycle"]
            })),
        }
    }
}

impl From<SdkPublicLocality> for DesktopRuntimePublicFarmLocation {
    fn from(value: SdkPublicLocality) -> Self {
        Self {
            primary: value.primary,
            city: value.city,
            region: value.region,
            country: value.country,
            geohash5: value.geohash5,
        }
    }
}

impl fmt::Display for DesktopRuntimeIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl DesktopRuntimeSnapshot {
    fn from_config(
        config: &DesktopRuntimeSupervisorConfig,
        state: DesktopRuntimeLifecycleState,
        startup_milestones: Option<Vec<DesktopRuntimeStartupMilestone>>,
        storage_paths: Option<DesktopRuntimeStoragePaths>,
        last_issue: Option<DesktopRuntimeIssue>,
    ) -> Self {
        Self {
            state,
            startup_milestones: startup_milestones.unwrap_or_default(),
            storage_root: config.storage_root.clone(),
            relay_urls: config.relay_urls.clone(),
            relay_url_policy: config.relay_url_policy,
            storage_paths,
            last_issue,
            last_effect: None,
            storage_diagnostics: None,
            integrity_diagnostics: None,
            sync_diagnostics: None,
            projection_lifecycle: DesktopRuntimeProjectionLifecycleStatus::current(),
        }
    }
}

impl DesktopRuntimeDiagnostics {
    pub fn from_snapshot(snapshot: DesktopRuntimeSnapshot) -> Option<Self> {
        Some(Self {
            storage: snapshot.storage_diagnostics.clone()?,
            integrity: snapshot.integrity_diagnostics.clone()?,
            sync: snapshot.sync_diagnostics.clone()?,
            runtime: snapshot,
        })
    }
}

impl DesktopRuntimeEffectStatus {
    fn accepted(receipt: DesktopRuntimeEffectReceipt) -> Self {
        Self {
            receipt,
            state: DesktopRuntimeEffectState::Accepted,
            issue: None,
            workflow_receipt: None,
            restore_preflight: None,
        }
    }

    fn completed(
        receipt: DesktopRuntimeEffectReceipt,
        workflow_receipt: Option<DesktopRuntimeWorkflowReceipt>,
        restore_preflight: Option<DesktopRuntimeRestorePreflightReceipt>,
    ) -> Self {
        Self {
            receipt,
            state: DesktopRuntimeEffectState::Completed,
            issue: None,
            workflow_receipt,
            restore_preflight,
        }
    }

    fn failed(receipt: DesktopRuntimeEffectReceipt, issue: DesktopRuntimeIssue) -> Self {
        Self {
            receipt,
            state: DesktopRuntimeEffectState::Failed,
            issue: Some(issue),
            workflow_receipt: None,
            restore_preflight: None,
        }
    }
}

impl From<StorageStatusReceipt> for DesktopRuntimeStorageDiagnostics {
    fn from(receipt: StorageStatusReceipt) -> Self {
        Self {
            storage_kind: serialized_label(&receipt.storage),
            paths: receipt.paths.as_ref().map(DesktopRuntimeStoragePaths::from),
            event_store: DesktopRuntimeEventStoreDiagnostics {
                store: receipt.event_store.store.into(),
                total_events: receipt.event_store.total_events,
                projection_eligible_events: receipt.event_store.projection_eligible_events,
                transport_observations: receipt.event_store.transport_observations,
                last_event_seq: receipt.event_store.last_event_seq,
                last_event_updated_at_ms: receipt.event_store.last_event_updated_at_ms,
            },
            outbox: DesktopRuntimeOutboxDiagnostics {
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

impl From<radroots_sdk::SdkSqliteStoreStatus> for DesktopRuntimeSqliteStoreDiagnostics {
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

impl From<IntegrityReceipt> for DesktopRuntimeIntegrityDiagnostics {
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

impl From<SyncStatusReceipt> for DesktopRuntimeSyncDiagnostics {
    fn from(receipt: SyncStatusReceipt) -> Self {
        Self {
            source: serialized_label(&receipt.source),
            observed_at_ms: receipt.observed_at_ms,
            event_store: DesktopRuntimeSyncEventStoreDiagnostics {
                total_events: receipt.event_store.total_events,
                projection_eligible_events: receipt.event_store.projection_eligible_events,
                transport_observations: receipt.event_store.transport_observations,
                last_event_seq: receipt.event_store.last_event_seq,
                last_event_updated_at_ms: receipt.event_store.last_event_updated_at_ms,
            },
            outbox: DesktopRuntimeSyncOutboxDiagnostics {
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
            transport_targets: DesktopRuntimeSyncTransportTargetDiagnostics {
                configured_count: receipt.transport_profile.configured_transport_target_count,
                configured_targets: receipt
                    .transport_profile
                    .configured_transport_targets
                    .into_iter()
                    .map(|target| target.endpoint_uri)
                    .collect(),
            },
        }
    }
}

impl From<SdkBackupVerification> for DesktopRuntimeBackupVerificationDiagnostics {
    fn from(verification: SdkBackupVerification) -> Self {
        Self {
            event_store_ok: verification.event_store_ok,
            outbox_ok: verification.outbox_ok,
            event_store_events: verification.event_store_events,
            outbox_events: verification.outbox_events,
        }
    }
}

impl DesktopRuntimeRestorePreflightReceipt {
    fn from_restore_receipt(
        receipt: RestoreReceipt,
        destination: PathBuf,
        projection_lifecycle: DesktopRuntimeProjectionLifecycleStatus,
    ) -> Self {
        Self {
            source: receipt.source,
            destination: receipt.destination.unwrap_or(destination),
            state: serialized_label(&receipt.state),
            destination_paths: receipt
                .destination_paths
                .as_ref()
                .map(DesktopRuntimeStoragePaths::from),
            restored_paths: receipt
                .restored_paths
                .as_ref()
                .map(DesktopRuntimeStoragePaths::from),
            runtime_path: receipt.runtime_path,
            private_path: receipt.private_path,
            studio_path: receipt.studio_path,
            manifest_path: receipt.manifest_path,
            verification: receipt.verification.into(),
            source_storage: receipt.manifest.source_status.into(),
            projection_lifecycle,
        }
    }
}

pub fn desktop_runtime_storage_root_from_data_root(data_root: &Path) -> PathBuf {
    data_root.join(DESKTOP_RUNTIME_STORAGE_DIR_NAME)
}

fn desktop_runtime_relay_url_policy(relay_urls: &[String]) -> DesktopRuntimeRelayUrlPolicy {
    if relay_urls
        .iter()
        .any(|relay_url| relay_url.trim().to_ascii_lowercase().starts_with("ws://"))
    {
        DesktopRuntimeRelayUrlPolicy::Localhost
    } else {
        DesktopRuntimeRelayUrlPolicy::Public
    }
}

fn run_desktop_runtime_worker(
    config: DesktopRuntimeSupervisorConfig,
    shared: Arc<DesktopRuntimeSupervisorShared>,
    command_receiver: Receiver<DesktopRuntimeEffect>,
) {
    let runtime = match TokioRuntimeBuilder::new_multi_thread()
        .worker_threads(2)
        .thread_name("radroots-desktop-runtime-supervisor-async")
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            replace_status(
                &shared,
                DesktopRuntimeSnapshot::from_config(
                    &config,
                    DesktopRuntimeLifecycleState::Degraded,
                    Some(vec![DesktopRuntimeStartupMilestone::ShellReady]),
                    None,
                    Some(DesktopRuntimeIssue::runtime_error(
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
            let storage_paths = sdk.storage_paths().map(DesktopRuntimeStoragePaths::from);
            let mut ready_status = DesktopRuntimeSnapshot::from_config(
                &config,
                DesktopRuntimeLifecycleState::Ready,
                Some(ready_startup_milestones(&config, storage_paths.as_ref())),
                storage_paths,
                None,
            );
            match block_on_sdk_result(
                &runtime,
                "desktop_runtime_initial_diagnostics",
                collect_sdk_diagnostics(&sdk, ready_status.clone()),
            ) {
                Ok(diagnostics) => {
                    ready_status.storage_diagnostics = Some(diagnostics.storage);
                    ready_status.integrity_diagnostics = Some(diagnostics.integrity);
                    ready_status.sync_diagnostics = Some(diagnostics.sync);
                }
                Err(error) => {
                    ready_status.state = DesktopRuntimeLifecycleState::Degraded;
                    ready_status.last_issue = Some(error);
                }
            }
            replace_status(&shared, ready_status);
            Some(sdk)
        }
        Err(error) => {
            replace_status(
                &shared,
                DesktopRuntimeSnapshot::from_config(
                    &config,
                    DesktopRuntimeLifecycleState::Degraded,
                    Some(vec![DesktopRuntimeStartupMilestone::ShellReady]),
                    None,
                    Some(DesktopRuntimeIssue::from_sdk_error(&error)),
                ),
            );
            None
        }
    };

    loop {
        if shared.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        let command = match command_receiver.try_recv() {
            Ok(command) => command,
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(TryRecvError::Disconnected) => break,
        };

        match command {
            DesktopRuntimeEffect::RefreshDiagnostics(receipt) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => refresh_supervisor_diagnostics(&runtime, &shared, sdk),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_effect_result(&shared, receipt, result.map(|()| None));
            }
            DesktopRuntimeEffect::RestorePreflight(receipt, request) => {
                let result = match sdk.as_ref() {
                    Some(_) => run_restore_preflight(&runtime, &shared, &config, *request),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                finish_restore_preflight_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::EnqueueFarmPublish(receipt, request) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_farm_publish_with_sdk(&runtime, sdk, *request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_workflow_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::EnqueueListingPublish(receipt, request) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(sdk) => enqueue_listing_publish_with_sdk(&runtime, sdk, *request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_workflow_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::TradePropose(receipt, request) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(_) => trade_propose_with_sdk(&runtime, &config, *request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_workflow_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::TradeDecision(receipt, request) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(_) => trade_decision_with_sdk(&runtime, &config, *request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_workflow_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::TradeCancellation(receipt, request) => {
                let result = if let Some(issue) = lifecycle_busy_issue(&shared) {
                    Err(issue)
                } else {
                    match sdk.as_ref() {
                        Some(_) => trade_cancel_with_sdk(&runtime, &config, *request),
                        None => Err(runtime_unavailable_issue(&shared)),
                    }
                };
                finish_workflow_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::BeginProjectionRebuild(receipt) => {
                let result = match sdk.as_ref() {
                    Some(_) => Ok(begin_projection_rebuild(&shared)),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                finish_projection_result(&shared, receipt, result);
            }
            DesktopRuntimeEffect::CompleteProjectionRebuild(receipt) => {
                let result = match sdk.as_ref() {
                    Some(_) => complete_projection_rebuild(&shared),
                    None => Err(runtime_unavailable_issue(&shared)),
                };
                finish_projection_result(&shared, receipt, result);
            }
        }
    }

    drop(sdk.take());
    transition_status_state(&shared, DesktopRuntimeLifecycleState::Stopped);
}

fn run_degraded_worker(
    config: DesktopRuntimeSupervisorConfig,
    shared: Arc<DesktopRuntimeSupervisorShared>,
    command_receiver: Receiver<DesktopRuntimeEffect>,
) {
    loop {
        if shared.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }

        let command = match command_receiver.try_recv() {
            Ok(command) => command,
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(TryRecvError::Disconnected) => break,
        };
        set_effect_failed(
            &shared,
            effect_receipt(&command),
            runtime_unavailable_issue(&shared),
        );
    }

    let last_issue = lock_status(&shared).last_issue.clone();
    replace_status(
        &shared,
        DesktopRuntimeSnapshot::from_config(
            &config,
            DesktopRuntimeLifecycleState::Stopped,
            Some(vec![DesktopRuntimeStartupMilestone::ShellReady]),
            None,
            last_issue,
        ),
    );
}

async fn build_sdk_runtime(
    config: &DesktopRuntimeSupervisorConfig,
) -> Result<RadrootsClient, RadrootsSdkError> {
    RadrootsClient::builder()
        .directory_storage(config.storage_root.clone())
        .transport_profile(app_transport_profile(config)?)
        .build()
        .await
}

async fn build_sdk_runtime_with_signer(
    config: &DesktopRuntimeSupervisorConfig,
    signer: DesktopRuntimeLocalSigner,
) -> Result<RadrootsClient, DesktopRuntimeIssue> {
    let local_signer = RadrootsLocalEventSigner::new(signer.into_keys()).map_err(|error| {
        DesktopRuntimeIssue::runtime_error("sdk_signer_init_failed", error.to_string())
    })?;
    let signer = RadrootsSdkLocalKeySigner::from_event_signer(local_signer)
        .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    let transport_profile = app_transport_profile(config)
        .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    RadrootsClient::builder()
        .directory_storage(config.storage_root.clone())
        .transport_profile(transport_profile)
        .signer_provider(RadrootsSdkSignerProvider::LocalKey(signer))
        .build()
        .await
        .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))
}

fn app_transport_profile(
    config: &DesktopRuntimeSupervisorConfig,
) -> Result<TransportProfile, RadrootsSdkError> {
    if config.relay_urls.is_empty() {
        return Ok(TransportProfile::local_only());
    }
    Ok(TransportProfile::nostr(NostrProfile::new(
        config.relay_urls.iter().map(String::as_str),
        config.relay_url_policy.into(),
    )?))
}

fn app_trade_publish_mode() -> PublishMode {
    PublishMode::EnqueueOnly
}

fn app_trade_satisfaction_policy() -> SatisfactionPolicy {
    SatisfactionPolicy::NoWait
}

fn app_trade_target_policy() -> TargetPolicy {
    TargetPolicy::default_profile()
}

fn app_trade_privacy_confirmation(confirm_public_note: bool) -> PrivacyPreflightConfirmation {
    if confirm_public_note {
        PrivacyPreflightConfirmation::new()
            .confirm(ProductSensitivityField::PublicButSensitiveNotes)
    } else {
        PrivacyPreflightConfirmation::new()
    }
}

fn block_on_sdk_result<T>(
    runtime: &tokio::runtime::Runtime,
    operation: &'static str,
    future: impl Future<Output = Result<T, RadrootsSdkError>>,
) -> Result<T, DesktopRuntimeIssue> {
    match runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(DESKTOP_RUNTIME_SDK_EFFECT_TIMEOUT_MS),
            future,
        )
        .await
    }) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(DesktopRuntimeIssue::from_sdk_error(&error)),
        Err(_) => Err(sdk_effect_timeout_issue(operation)),
    }
}

fn block_on_desktop_result<T>(
    runtime: &tokio::runtime::Runtime,
    operation: &'static str,
    future: impl Future<Output = Result<T, DesktopRuntimeIssue>>,
) -> Result<T, DesktopRuntimeIssue> {
    match runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(DESKTOP_RUNTIME_SDK_EFFECT_TIMEOUT_MS),
            future,
        )
        .await
    }) {
        Ok(result) => result,
        Err(_) => Err(sdk_effect_timeout_issue(operation)),
    }
}

fn sdk_effect_timeout_issue(operation: &'static str) -> DesktopRuntimeIssue {
    DesktopRuntimeIssue::runtime_error(
        "desktop_runtime_sdk_effect_timeout",
        format!("{operation} did not complete within {DESKTOP_RUNTIME_SDK_EFFECT_TIMEOUT_MS} ms"),
    )
}

fn run_restore_preflight(
    runtime: &tokio::runtime::Runtime,
    shared: &DesktopRuntimeSupervisorShared,
    config: &DesktopRuntimeSupervisorConfig,
    request: DesktopRuntimeRestorePreflightRequest,
) -> Result<DesktopRuntimeRestorePreflightReceipt, DesktopRuntimeIssue> {
    if let Some(issue) = lifecycle_busy_issue(shared) {
        return Err(issue);
    }
    transition_status_state(shared, DesktopRuntimeLifecycleState::Pausing);
    transition_status_state(shared, DesktopRuntimeLifecycleState::Paused);
    transition_status_state(shared, DesktopRuntimeLifecycleState::Restoring);

    let restore_request = RestoreRequest::new(request.source.clone())
        .with_destination(config.storage_root.clone())
        .with_overwrite(request.overwrite_existing_sdk_storage)
        .dry_run();
    let result = block_on_sdk_result(
        runtime,
        "desktop_runtime_restore_preflight",
        RadrootsClient::restore(restore_request),
    )
    .map(|receipt| {
        let projection_lifecycle = mark_projections_stale(
            shared,
            "sdk_restore_preflight",
            Some(request.source.clone()),
        );
        DesktopRuntimeRestorePreflightReceipt::from_restore_receipt(
            receipt,
            config.storage_root.clone(),
            projection_lifecycle,
        )
    });
    if result.is_err() {
        transition_status_state(shared, DesktopRuntimeLifecycleState::Ready);
    }
    result
}

async fn collect_sdk_diagnostics(
    sdk: &RadrootsClient,
    runtime: DesktopRuntimeSnapshot,
) -> Result<DesktopRuntimeDiagnostics, RadrootsSdkError> {
    let storage = sdk.storage_status(StorageStatusRequest::new()).await?;
    let integrity = sdk.integrity(IntegrityRequest::new()).await?;
    let sync = sdk.sync().status(SyncStatusRequest::new()).await?;
    Ok(DesktopRuntimeDiagnostics {
        runtime,
        storage: storage.into(),
        integrity: integrity.into(),
        sync: sync.into(),
    })
}

fn farm_public_location_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsClient,
    request: DesktopRuntimeFarmPublicLocationRequest,
) -> Result<Option<DesktopRuntimePublicFarmLocation>, DesktopRuntimeIssue> {
    let farm_addr = RadrootsAddressableCoordinate::parse(format!(
        "{KIND_FARM}:{}:{}",
        request.actor_pubkey, request.farm_d_tag
    ))
    .map_err(|error| {
        DesktopRuntimeIssue::from_sdk_error(&RadrootsSdkError::InvalidRequest {
            message: format!("farm public location address is invalid: {error}"),
        })
    })?;
    block_on_sdk_result(
        runtime,
        "desktop_runtime_farm_public_location",
        sdk.farms().private_location(&farm_addr),
    )
    .map(|location| location.map(|receipt| receipt.public_locality.into()))
}

fn enqueue_farm_publish_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsClient,
    request: DesktopRuntimeFarmPublishRequest,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Farmer,
    )?;
    let signer = sdk_local_signer(request.signer)?;
    let target_relays = sdk_transport_targets(request.target_relays, request.relay_url_policy)?;
    let mut farm = request.farm;
    if farm.location.is_none() {
        let public_location = farm_public_location_with_sdk(
            runtime,
            sdk,
            DesktopRuntimeFarmPublicLocationRequest {
                actor_pubkey: request.actor_pubkey.clone(),
                farm_d_tag: farm.d_tag.clone(),
            },
        )?;
        farm.location = public_location.map(desktop_runtime_public_farm_location_to_protocol);
    }
    let mut enqueue = FarmEnqueuePublishRequest::new(actor, farm, target_relays);
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = block_on_sdk_result(
        runtime,
        "desktop_runtime_farm_publish",
        sdk.farms()
            .enqueue_publish_with_explicit_signer(enqueue, &signer),
    )?;
    Ok(desktop_runtime_farm_receipt(receipt, request.actor_pubkey))
}

fn enqueue_listing_publish_with_sdk(
    runtime: &tokio::runtime::Runtime,
    sdk: &RadrootsClient,
    request: DesktopRuntimeListingPublishRequest,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let signer = sdk_local_signer(request.signer)?;
    let target_relays = sdk_transport_targets(request.target_relays, request.relay_url_policy)?;
    let mut listing = request.listing;
    if listing.location.is_none() {
        let public_location = farm_public_location_with_sdk(
            runtime,
            sdk,
            DesktopRuntimeFarmPublicLocationRequest {
                actor_pubkey: listing.farm.pubkey.clone(),
                farm_d_tag: listing.farm.d_tag.clone(),
            },
        )?;
        listing.location = public_location.map(desktop_runtime_public_listing_location_to_protocol);
    }
    let mut enqueue = ListingEnqueuePublishRequest::new(actor, listing, target_relays);
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        enqueue = enqueue
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    }
    let receipt = block_on_sdk_result(
        runtime,
        "desktop_runtime_listing_publish",
        sdk.listings()
            .enqueue_publish_with_explicit_signer(enqueue, &signer),
    )?;
    Ok(desktop_runtime_listing_receipt(
        receipt,
        request.actor_pubkey,
    ))
}

fn trade_propose_with_sdk(
    runtime: &tokio::runtime::Runtime,
    config: &DesktopRuntimeSupervisorConfig,
    request: DesktopRuntimeTradeProposeRequest,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let sdk = block_on_desktop_result(
        runtime,
        "desktop_runtime_trade_propose_build",
        build_sdk_runtime_with_signer(config, request.signer),
    )?;
    let buyer_pubkey =
        RadrootsPublicKey::parse(request.actor_pubkey.as_str()).map_err(|error| {
            DesktopRuntimeIssue::from_sdk_error(&RadrootsSdkError::InvalidRequest {
                message: format!("trade proposal buyer public key is invalid: {error}"),
            })
        })?;
    let order = RadrootsOrderRequest {
        order_id: request.order_id,
        listing_addr: request.listing_addr,
        buyer_pubkey,
        seller_pubkey: request.seller_pubkey,
        items: request.items,
        economics: request.economics,
    };
    let mut sdk_request = TradeProposeRequest::new(
        actor,
        request.listing_event,
        order,
        app_trade_target_policy(),
        app_trade_publish_mode(),
        app_trade_satisfaction_policy(),
    )
    .with_optional_public_note(request.public_note)
    .with_privacy_confirmation(app_trade_privacy_confirmation(request.confirm_public_note));
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        sdk_request = sdk_request
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    }
    let outcome = block_on_sdk_result(
        runtime,
        "desktop_runtime_trade_propose",
        sdk.trades().buyer().propose_trade(sdk_request),
    )?;
    desktop_runtime_trade_propose_receipt(outcome, request.actor_pubkey)
}

fn trade_decision_with_sdk(
    runtime: &tokio::runtime::Runtime,
    config: &DesktopRuntimeSupervisorConfig,
    request: DesktopRuntimeTradeDecisionRequest,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Seller,
    )?;
    let sdk = block_on_desktop_result(
        runtime,
        "desktop_runtime_trade_decision_build",
        build_sdk_runtime_with_signer(config, request.signer),
    )?;
    let publish_mode = app_trade_publish_mode();
    let satisfaction_policy = app_trade_satisfaction_policy();
    let outcome = match request.decision {
        DesktopRuntimeTradeDecision::Accept {
            inventory_commitments,
        } => {
            let mut sdk_request = TradeAcceptRequest::new(
                actor,
                request.locator,
                inventory_commitments,
                app_trade_target_policy(),
                publish_mode,
                satisfaction_policy,
                TradeEvidenceMode::ResyncBeforeMutation,
            )
            .with_privacy_confirmation(app_trade_privacy_confirmation(false));
            if let Some(idempotency_key) = request.idempotency_key.as_deref() {
                sdk_request = sdk_request
                    .try_with_idempotency_key(idempotency_key)
                    .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
            }
            block_on_sdk_result(
                runtime,
                "desktop_runtime_trade_accept",
                sdk.trades().seller().accept_trade(sdk_request),
            )?
        }
        DesktopRuntimeTradeDecision::Decline { reason } => {
            let mut sdk_request = TradeDeclineRequest::new(
                actor,
                request.locator,
                reason,
                app_trade_target_policy(),
                publish_mode,
                satisfaction_policy,
                TradeEvidenceMode::ResyncBeforeMutation,
            )
            .with_privacy_confirmation(app_trade_privacy_confirmation(request.confirm_public_note));
            if let Some(idempotency_key) = request.idempotency_key.as_deref() {
                sdk_request = sdk_request
                    .try_with_idempotency_key(idempotency_key)
                    .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
            }
            block_on_sdk_result(
                runtime,
                "desktop_runtime_trade_decline",
                sdk.trades().seller().decline_trade(sdk_request),
            )?
        }
    };
    desktop_runtime_trade_decision_receipt(outcome, request.actor_pubkey)
}

fn trade_cancel_with_sdk(
    runtime: &tokio::runtime::Runtime,
    config: &DesktopRuntimeSupervisorConfig,
    request: DesktopRuntimeTradeCancellationRequest,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    let actor = sdk_actor_context(
        request.actor_pubkey.as_str(),
        request.actor_account_id.as_str(),
        RadrootsActorRole::Buyer,
    )?;
    let sdk = block_on_desktop_result(
        runtime,
        "desktop_runtime_trade_cancel_build",
        build_sdk_runtime_with_signer(config, request.signer),
    )?;
    let mut sdk_request = TradeCancelRequest::new(
        actor,
        request.locator,
        request.reason,
        app_trade_target_policy(),
        app_trade_publish_mode(),
        app_trade_satisfaction_policy(),
        TradeEvidenceMode::ResyncBeforeMutation,
    )
    .with_privacy_confirmation(app_trade_privacy_confirmation(request.confirm_public_note));
    if let Some(idempotency_key) = request.idempotency_key.as_deref() {
        sdk_request = sdk_request
            .try_with_idempotency_key(idempotency_key)
            .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))?;
    }
    let outcome = block_on_sdk_result(
        runtime,
        "desktop_runtime_trade_cancel",
        sdk.trades().buyer().cancel_trade(sdk_request),
    )?;
    desktop_runtime_trade_cancellation_receipt(outcome, request.actor_pubkey)
}

fn sdk_actor_context(
    actor_pubkey: &str,
    actor_account_id: &str,
    role: RadrootsActorRole,
) -> Result<RadrootsActorContext, DesktopRuntimeIssue> {
    RadrootsActorContext::local_account(actor_pubkey, actor_account_id.to_owned(), [role]).map_err(
        |error| DesktopRuntimeIssue::runtime_error("sdk_actor_context_invalid", error.to_string()),
    )
}

fn sdk_local_signer(
    signer: DesktopRuntimeLocalSigner,
) -> Result<RadrootsLocalEventSigner, DesktopRuntimeIssue> {
    RadrootsLocalEventSigner::new(signer.into_keys()).map_err(|error| {
        DesktopRuntimeIssue::runtime_error("sdk_signer_init_failed", error.to_string())
    })
}

fn desktop_runtime_public_farm_location_to_protocol(
    location: DesktopRuntimePublicFarmLocation,
) -> RadrootsFarmPublicLocation {
    RadrootsFarmPublicLocation {
        primary: location.primary,
        city: location.city,
        region: location.region,
        country: location.country,
        geohash: location.geohash5,
    }
}

fn desktop_runtime_public_listing_location_to_protocol(
    location: DesktopRuntimePublicFarmLocation,
) -> RadrootsListingPublicLocation {
    RadrootsListingPublicLocation {
        primary: location.primary,
        city: location.city,
        region: location.region,
        country: location.country,
        geohash: location.geohash5,
    }
}

fn sdk_transport_targets(
    relays: Vec<String>,
    policy: DesktopRuntimeRelayUrlPolicy,
) -> Result<TargetPolicy, DesktopRuntimeIssue> {
    TargetPolicy::try_nostr_relays(relays, policy.into())
        .map_err(|error| DesktopRuntimeIssue::from_sdk_error(&error))
}

fn desktop_runtime_farm_receipt(
    receipt: FarmEnqueueReceipt,
    actor_pubkey: String,
) -> DesktopRuntimeWorkflowReceipt {
    DesktopRuntimeWorkflowReceipt {
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

fn desktop_runtime_listing_receipt(
    receipt: ListingEnqueueReceipt,
    actor_pubkey: String,
) -> DesktopRuntimeWorkflowReceipt {
    DesktopRuntimeWorkflowReceipt {
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

fn desktop_runtime_trade_propose_receipt(
    outcome: TradeMutationOutcome<TradeSubmitPlan, TradeSubmitReceipt>,
    actor_pubkey: String,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    match outcome {
        TradeMutationOutcome::Enqueued { receipt }
        | TradeMutationOutcome::Published { receipt, .. } => Ok(DesktopRuntimeWorkflowReceipt {
            operation_kind: TRADE_SUBMIT_OPERATION_KIND.to_owned(),
            expected_event_id: receipt.expected_event_id.as_str().to_owned(),
            signed_event_id: receipt.signed_event_id.as_str().to_owned(),
            outbox_operation_id: receipt.outbox_operation_id,
            outbox_event_id: receipt.outbox_event_id,
            state: sdk_mutation_state_key(receipt.state).to_owned(),
            idempotency_digest_prefix: receipt.idempotency_digest_prefix,
            actor_pubkey,
        }),
        TradeMutationOutcome::DryRun { .. } => Err(unexpected_trade_dry_run_issue("trade.propose")),
    }
}

fn desktop_runtime_trade_decision_receipt(
    outcome: TradeMutationOutcome<TradeDecisionPlan, TradeDecisionReceipt>,
    actor_pubkey: String,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    match outcome {
        TradeMutationOutcome::Enqueued { receipt }
        | TradeMutationOutcome::Published { receipt, .. } => Ok(DesktopRuntimeWorkflowReceipt {
            operation_kind: TRADE_DECISION_OPERATION_KIND.to_owned(),
            expected_event_id: receipt.expected_event_id.as_str().to_owned(),
            signed_event_id: receipt.signed_event_id.as_str().to_owned(),
            outbox_operation_id: receipt.outbox_operation_id,
            outbox_event_id: receipt.outbox_event_id,
            state: sdk_mutation_state_key(receipt.state).to_owned(),
            idempotency_digest_prefix: receipt.idempotency_digest_prefix,
            actor_pubkey,
        }),
        TradeMutationOutcome::DryRun { .. } => Err(unexpected_trade_dry_run_issue("trade.decide")),
    }
}

fn desktop_runtime_trade_cancellation_receipt(
    outcome: TradeMutationOutcome<TradeCancellationPlan, TradeCancellationReceipt>,
    actor_pubkey: String,
) -> Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue> {
    match outcome {
        TradeMutationOutcome::Enqueued { receipt }
        | TradeMutationOutcome::Published { receipt, .. } => Ok(DesktopRuntimeWorkflowReceipt {
            operation_kind: TRADE_CANCELLATION_OPERATION_KIND.to_owned(),
            expected_event_id: receipt.expected_event_id.as_str().to_owned(),
            signed_event_id: receipt.signed_event_id.as_str().to_owned(),
            outbox_operation_id: receipt.outbox_operation_id,
            outbox_event_id: receipt.outbox_event_id,
            state: sdk_mutation_state_key(receipt.state).to_owned(),
            idempotency_digest_prefix: receipt.idempotency_digest_prefix,
            actor_pubkey,
        }),
        TradeMutationOutcome::DryRun { .. } => Err(unexpected_trade_dry_run_issue("trade.cancel")),
    }
}

fn unexpected_trade_dry_run_issue(operation: &'static str) -> DesktopRuntimeIssue {
    DesktopRuntimeIssue::runtime_error(
        "sdk_trade_unexpected_dry_run",
        format!("{operation} returned a dry-run plan for an enqueue-only Studio command"),
    )
}

fn sdk_mutation_state_key(state: SdkMutationState) -> &'static str {
    match state {
        SdkMutationState::StoredAndQueued => "enqueued",
        SdkMutationState::AlreadyQueued => "already_queued",
        _ => "unknown",
    }
}

fn ready_startup_milestones(
    config: &DesktopRuntimeSupervisorConfig,
    storage_paths: Option<&DesktopRuntimeStoragePaths>,
) -> Vec<DesktopRuntimeStartupMilestone> {
    let mut milestones = vec![DesktopRuntimeStartupMilestone::ShellReady];
    if storage_paths.is_some() {
        milestones.push(DesktopRuntimeStartupMilestone::RuntimeStoreReady);
        milestones.push(DesktopRuntimeStartupMilestone::PrivateStoreReady);
    }
    milestones.push(DesktopRuntimeStartupMilestone::SignerReady);
    milestones.push(DesktopRuntimeStartupMilestone::ProjectionsReady);
    if !config.relay_urls.is_empty() {
        milestones.push(DesktopRuntimeStartupMilestone::NetworkObserved);
    }
    milestones
}

fn refresh_supervisor_diagnostics(
    runtime: &tokio::runtime::Runtime,
    shared: &DesktopRuntimeSupervisorShared,
    sdk: &RadrootsClient,
) -> Result<(), DesktopRuntimeIssue> {
    let mut runtime_status = lock_status(shared).clone();
    runtime_status.last_issue = None;
    let diagnostics = block_on_sdk_result(
        runtime,
        "desktop_runtime_refresh_diagnostics",
        collect_sdk_diagnostics(sdk, runtime_status),
    )?;
    let mut status = lock_status(shared);
    status.storage_diagnostics = Some(diagnostics.storage);
    status.integrity_diagnostics = Some(diagnostics.integrity);
    status.sync_diagnostics = Some(diagnostics.sync);
    status.last_issue = None;
    Ok(())
}

fn finish_effect_result(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    result: Result<Option<DesktopRuntimeWorkflowReceipt>, DesktopRuntimeIssue>,
) {
    match result {
        Ok(workflow_receipt) => set_effect_completed(shared, receipt, workflow_receipt, None),
        Err(issue) => set_effect_failed(shared, receipt, issue),
    }
}

fn finish_workflow_result(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    result: Result<DesktopRuntimeWorkflowReceipt, DesktopRuntimeIssue>,
) {
    finish_effect_result(shared, receipt, result.map(Some));
}

fn finish_restore_preflight_result(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    result: Result<DesktopRuntimeRestorePreflightReceipt, DesktopRuntimeIssue>,
) {
    match result {
        Ok(restore_preflight) => {
            set_effect_completed(shared, receipt, None, Some(restore_preflight));
        }
        Err(issue) => set_effect_failed(shared, receipt, issue),
    }
}

fn finish_projection_result(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    result: Result<DesktopRuntimeProjectionLifecycleStatus, DesktopRuntimeIssue>,
) {
    match result {
        Ok(_) => set_effect_completed(shared, receipt, None, None),
        Err(issue) => set_effect_failed(shared, receipt, issue),
    }
}

fn set_effect_completed(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    workflow_receipt: Option<DesktopRuntimeWorkflowReceipt>,
    restore_preflight: Option<DesktopRuntimeRestorePreflightReceipt>,
) {
    let mut status = lock_status(shared);
    status.last_issue = None;
    status.last_effect = Some(DesktopRuntimeEffectStatus::completed(
        receipt,
        workflow_receipt,
        restore_preflight,
    ));
}

fn set_effect_failed(
    shared: &DesktopRuntimeSupervisorShared,
    receipt: DesktopRuntimeEffectReceipt,
    issue: DesktopRuntimeIssue,
) {
    let mut status = lock_status(shared);
    status.last_issue = Some(issue.clone());
    status.last_effect = Some(DesktopRuntimeEffectStatus::failed(receipt, issue));
}

fn set_last_effect(shared: &DesktopRuntimeSupervisorShared, status: DesktopRuntimeEffectStatus) {
    lock_status(shared).last_effect = Some(status);
}

fn clear_last_effect(shared: &DesktopRuntimeSupervisorShared, effect_id: u64) {
    let mut status = lock_status(shared);
    if status
        .last_effect
        .as_ref()
        .is_some_and(|effect| effect.receipt.effect_id == effect_id)
    {
        status.last_effect = None;
    }
}

fn set_startup_milestone(
    status: &mut DesktopRuntimeSnapshot,
    milestone: DesktopRuntimeStartupMilestone,
    enabled: bool,
) {
    if enabled {
        if !status.startup_milestones.contains(&milestone) {
            status.startup_milestones.push(milestone);
            status.startup_milestones.sort();
        }
    } else {
        status
            .startup_milestones
            .retain(|candidate| *candidate != milestone);
    }
}

fn effect_receipt(effect: &DesktopRuntimeEffect) -> DesktopRuntimeEffectReceipt {
    match effect {
        DesktopRuntimeEffect::RefreshDiagnostics(receipt)
        | DesktopRuntimeEffect::RestorePreflight(receipt, _)
        | DesktopRuntimeEffect::EnqueueFarmPublish(receipt, _)
        | DesktopRuntimeEffect::EnqueueListingPublish(receipt, _)
        | DesktopRuntimeEffect::TradePropose(receipt, _)
        | DesktopRuntimeEffect::TradeDecision(receipt, _)
        | DesktopRuntimeEffect::TradeCancellation(receipt, _)
        | DesktopRuntimeEffect::BeginProjectionRebuild(receipt)
        | DesktopRuntimeEffect::CompleteProjectionRebuild(receipt) => receipt.clone(),
    }
}

fn lifecycle_busy_issue(shared: &DesktopRuntimeSupervisorShared) -> Option<DesktopRuntimeIssue> {
    let state = lock_status(shared).state;
    if matches!(
        state,
        DesktopRuntimeLifecycleState::Pausing
            | DesktopRuntimeLifecycleState::Paused
            | DesktopRuntimeLifecycleState::Restoring
            | DesktopRuntimeLifecycleState::RebuildingProjections
            | DesktopRuntimeLifecycleState::ShuttingDown
    ) {
        Some(DesktopRuntimeIssue::lifecycle_blocked(state))
    } else {
        None
    }
}

fn runtime_unavailable_issue(shared: &DesktopRuntimeSupervisorShared) -> DesktopRuntimeIssue {
    let status = lock_status(shared).clone();
    if let Some(issue) = status.last_issue {
        issue
    } else {
        DesktopRuntimeIssue::runtime_error(
            "sdk_runtime_not_ready",
            format!("app sdk runtime is {:?}", status.state),
        )
    }
}

fn replace_status(shared: &DesktopRuntimeSupervisorShared, status: DesktopRuntimeSnapshot) {
    *lock_status(shared) = status;
}

fn transition_status_state(
    shared: &DesktopRuntimeSupervisorShared,
    state: DesktopRuntimeLifecycleState,
) {
    lock_status(shared).state = state;
}

fn mark_projections_stale(
    shared: &DesktopRuntimeSupervisorShared,
    reason: impl Into<String>,
    restore_source: Option<PathBuf>,
) -> DesktopRuntimeProjectionLifecycleStatus {
    let mut status = lock_status(shared);
    status.projection_lifecycle =
        DesktopRuntimeProjectionLifecycleStatus::stale(reason, restore_source);
    status.state = DesktopRuntimeLifecycleState::Ready;
    set_startup_milestone(
        &mut status,
        DesktopRuntimeStartupMilestone::ProjectionsReady,
        false,
    );
    status.projection_lifecycle.clone()
}

fn begin_projection_rebuild(
    shared: &DesktopRuntimeSupervisorShared,
) -> DesktopRuntimeProjectionLifecycleStatus {
    let restore_source = lock_status(shared)
        .projection_lifecycle
        .restore_source
        .clone();
    let mut status = lock_status(shared);
    status.state = DesktopRuntimeLifecycleState::RebuildingProjections;
    status.projection_lifecycle = DesktopRuntimeProjectionLifecycleStatus::rebuilding(
        "sdk_projection_rebuild",
        restore_source,
    );
    set_startup_milestone(
        &mut status,
        DesktopRuntimeStartupMilestone::ProjectionsReady,
        false,
    );
    status.projection_lifecycle.clone()
}

fn complete_projection_rebuild(
    shared: &DesktopRuntimeSupervisorShared,
) -> Result<DesktopRuntimeProjectionLifecycleStatus, DesktopRuntimeIssue> {
    let mut status = lock_status(shared);
    if !matches!(
        status.state,
        DesktopRuntimeLifecycleState::RebuildingProjections
    ) {
        return Err(DesktopRuntimeIssue::lifecycle_blocked(status.state));
    }
    status.state = DesktopRuntimeLifecycleState::Ready;
    status.projection_lifecycle = DesktopRuntimeProjectionLifecycleStatus::current();
    set_startup_milestone(
        &mut status,
        DesktopRuntimeStartupMilestone::ProjectionsReady,
        true,
    );
    let projection_lifecycle = status.projection_lifecycle.clone();
    Ok(projection_lifecycle)
}

fn lock_status(shared: &DesktopRuntimeSupervisorShared) -> MutexGuard<'_, DesktopRuntimeSnapshot> {
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
        fs, thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use radroots_core::{
        RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreQuantity,
        RadrootsCoreQuantityPrice, RadrootsCoreUnit,
    };
    use radroots_event::{
        farm::RadrootsFarmRef,
        ids::{RadrootsDTag, RadrootsInventoryBinId},
        listing::{
            RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
            RadrootsListingDeliveryMethod, RadrootsListingProduct, RadrootsListingPublicLocation,
            RadrootsListingStatus,
        },
    };
    use radroots_nostr::prelude::{RadrootsNostrKeys, RadrootsNostrSecretKey};
    use radroots_sdk::{
        BackupRequest, LISTING_PUBLISH_OPERATION_KIND, NostrProfile, NostrRelayUrlPolicy,
        RadrootsClient, TransportProfile,
    };

    use crate::{
        APP_RUNTIME_NAMESPACE, AppDesktopRuntimePaths, AppRuntimeHostEnvironment,
        AppRuntimePlatform,
    };

    use super::{
        DESKTOP_RUNTIME_STORAGE_DIR_NAME, DesktopRuntimeEffectKind, DesktopRuntimeEffectState,
        DesktopRuntimeLifecycleState, DesktopRuntimeListingPublishRequest,
        DesktopRuntimeLocalSigner, DesktopRuntimeProjectionLifecycleState,
        DesktopRuntimeRelayUrlPolicy, DesktopRuntimeRestorePreflightRequest,
        DesktopRuntimeSnapshot, DesktopRuntimeStartupMilestone, DesktopRuntimeSupervisor,
        DesktopRuntimeSupervisorConfig, desktop_runtime_storage_root_from_data_root,
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
        let config = DesktopRuntimeSupervisorConfig::from_desktop_paths(
            &paths,
            vec!["wss://relay.example".to_owned()],
        );

        assert_eq!(
            config.storage_root,
            paths.app.data.join(DESKTOP_RUNTIME_STORAGE_DIR_NAME)
        );
        assert_eq!(
            config.storage_root,
            desktop_runtime_storage_root_from_data_root(paths.app.data.as_path())
        );
        assert_eq!(config.storage_root.parent(), Some(paths.app.data.as_path()));
        assert!(paths.app.data.ends_with(APP_RUNTIME_NAMESPACE));
        assert_eq!(
            config.relay_url_policy,
            DesktopRuntimeRelayUrlPolicy::Public
        );
    }

    #[test]
    fn sdk_config_uses_localhost_policy_for_ws_relay_urls() {
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            "/tmp/radroots-app-data".as_ref(),
            vec![
                "wss://relay.example".to_owned(),
                "ws://127.0.0.1:8080".to_owned(),
            ],
        );

        assert_eq!(
            config.relay_url_policy,
            DesktopRuntimeRelayUrlPolicy::Localhost
        );
    }

    #[test]
    fn desktop_runtime_supervisor_reaches_ready_with_snapshot_diagnostics() {
        let storage_root = temp_storage_root("ready");
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = DesktopRuntimeSupervisor::start(config).expect("supervisor should start");

        let status = poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Ready)
                && snapshot.storage_diagnostics.is_some()
                && snapshot.integrity_diagnostics.is_some()
                && snapshot.sync_diagnostics.is_some()
        });

        assert_eq!(status.state, DesktopRuntimeLifecycleState::Ready);
        assert_eq!(status.storage_root, storage_root);
        assert_eq!(
            status.relay_url_policy,
            DesktopRuntimeRelayUrlPolicy::Localhost
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::ShellReady)
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::RuntimeStoreReady)
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::PrivateStoreReady)
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::SignerReady)
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::ProjectionsReady)
        );
        assert!(
            status
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::NetworkObserved)
        );
        let storage_paths = status
            .storage_paths
            .as_ref()
            .expect("storage paths should be present");
        assert_eq!(
            storage_paths.runtime_path,
            storage_root.join("runtime.sqlite")
        );
        assert_eq!(
            storage_paths.private_path,
            storage_root.join("private.sqlite")
        );
        assert_eq!(
            storage_paths.studio_path,
            storage_root.join("studio.sqlite")
        );
        let storage = status
            .storage_diagnostics
            .as_ref()
            .expect("storage diagnostics should load");
        assert_eq!(storage.storage_kind, "directory");
        assert!(storage.event_store.store.integrity_ok);
        assert!(storage.outbox.store.integrity_ok);
        let integrity = status
            .integrity_diagnostics
            .as_ref()
            .expect("integrity diagnostics should load");
        assert!(integrity.event_store_ok);
        assert!(integrity.outbox_ok);
        let sync = status
            .sync_diagnostics
            .as_ref()
            .expect("sync diagnostics should load");
        assert_eq!(sync.source, "sdk_canonical_stores");
        assert_eq!(sync.transport_targets.configured_count, 1);
        assert!(runtime.request_shutdown());
        let stopped = poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Stopped)
        });
        assert_eq!(stopped.state, DesktopRuntimeLifecycleState::Stopped);
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn desktop_runtime_supervisor_enqueues_listing_publish_as_effect() {
        let storage_root = temp_storage_root("listing_enqueue");
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = DesktopRuntimeSupervisor::start(config).expect("supervisor should start");
        poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Ready)
        });
        let secret_key = RadrootsNostrSecretKey::from_hex(SDK_TEST_SELLER_SECRET_KEY_HEX)
            .expect("secret key should parse");
        let local_identity_keys = RadrootsNostrKeys::new(secret_key);
        let seller_pubkey = local_identity_keys.public_key().to_hex();

        let receipt = runtime
            .enqueue_listing_publish(DesktopRuntimeListingPublishRequest {
                actor_account_id: "seller-account".to_owned(),
                actor_pubkey: seller_pubkey.clone(),
                signer: DesktopRuntimeLocalSigner::from_local_identity_keys(local_identity_keys),
                listing: test_listing(seller_pubkey.as_str()),
                target_relays: vec!["wss://relay.radroots.test".to_owned()],
                relay_url_policy: DesktopRuntimeRelayUrlPolicy::Public,
                idempotency_key: Some("01890f0e-6c00-7000-8000-000000000224".to_owned()),
            })
            .expect("listing publish effect should enqueue");

        assert_eq!(
            receipt.effect_kind,
            DesktopRuntimeEffectKind::ListingPublish
        );
        assert_eq!(
            receipt.operation_kind.as_deref(),
            Some(LISTING_PUBLISH_OPERATION_KIND)
        );
        assert_eq!(
            receipt.actor_pubkey.as_deref(),
            Some(seller_pubkey.as_str())
        );
        assert!(receipt.effect_id > 0);

        let completed = poll_snapshot(&runtime, |snapshot| {
            snapshot.last_effect.as_ref().is_some_and(|effect| {
                effect.receipt.effect_id == receipt.effect_id
                    && matches!(effect.state, DesktopRuntimeEffectState::Completed)
                    && effect.workflow_receipt.is_some()
            })
        });
        let completed_effect = completed
            .last_effect
            .as_ref()
            .expect("listing effect should be captured in snapshot");
        assert_eq!(completed_effect.receipt.effect_id, receipt.effect_id);
        assert!(matches!(
            completed_effect.state,
            DesktopRuntimeEffectState::Completed
        ));
        let workflow = completed_effect
            .workflow_receipt
            .as_ref()
            .expect("workflow receipt should be captured in snapshot");
        assert_eq!(workflow.operation_kind, LISTING_PUBLISH_OPERATION_KIND);
        assert_eq!(workflow.actor_pubkey, seller_pubkey);
        assert_eq!(workflow.state, "enqueued");
        assert!(!workflow.expected_event_id.is_empty());
        assert_eq!(workflow.expected_event_id, workflow.signed_event_id);
        assert!(workflow.outbox_operation_id > 0);
        assert!(workflow.outbox_event_id > 0);
        assert!(workflow.idempotency_digest_prefix.is_some());
        assert!(runtime.request_shutdown());
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn desktop_runtime_supervisor_degrades_with_structured_sdk_error() {
        let storage_root = temp_storage_root("invalid_relay");
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://relay.example".to_owned()],
        );
        let runtime = DesktopRuntimeSupervisor::start(config).expect("supervisor should start");

        let status = poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Degraded)
        });

        let issue = status
            .last_issue
            .expect("degraded status should include issue");
        assert_eq!(issue.code, "invalid_relay_url");
        assert_eq!(issue.class, "configuration");
        assert!(!issue.retryable);
        assert!(
            issue
                .recovery_actions
                .contains(&"configure_transport_targets".to_owned())
        );
        assert_eq!(issue.detail_json["code"], "invalid_relay_url");
        let refresh = runtime
            .request_diagnostics_refresh()
            .expect("diagnostics refresh effect should be accepted");
        let failed = poll_snapshot(&runtime, |snapshot| {
            snapshot.last_effect.as_ref().is_some_and(|effect| {
                effect.receipt.effect_id == refresh.effect_id
                    && matches!(effect.state, DesktopRuntimeEffectState::Failed)
            })
        });
        let issue = failed
            .last_effect
            .as_ref()
            .and_then(|effect| effect.issue.as_ref())
            .expect("failed diagnostics effect should include issue");
        assert_eq!(issue.code, "invalid_relay_url");
        assert!(runtime.request_shutdown());
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn desktop_runtime_supervisor_restore_preflight_marks_projections_stale() {
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
                RadrootsClient::builder()
                    .directory_storage(backup_source_root.clone())
                    .transport_profile(TransportProfile::nostr(
                        NostrProfile::new(["ws://127.0.0.1:8080"], NostrRelayUrlPolicy::Localhost)
                            .expect("backup source relay profile"),
                    ))
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
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            app_data_root.as_path(),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = DesktopRuntimeSupervisor::start(config).expect("supervisor should start");
        poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Ready)
        });
        let sentinel = app_storage_root.join("restore-preflight-sentinel");
        fs::write(&sentinel, "existing destination").expect("sentinel should write");

        let effect = runtime
            .request_restore_preflight(
                DesktopRuntimeRestorePreflightRequest::new(backup_archive.clone())
                    .with_overwrite_existing_sdk_storage(true),
            )
            .expect("restore preflight effect should be accepted");

        let completed = poll_snapshot(&runtime, |snapshot| {
            snapshot.last_effect.as_ref().is_some_and(|last_effect| {
                last_effect.receipt.effect_id == effect.effect_id
                    && matches!(last_effect.state, DesktopRuntimeEffectState::Completed)
            })
        });
        let receipt = completed
            .last_effect
            .as_ref()
            .and_then(|effect| effect.restore_preflight.as_ref())
            .expect("restore preflight receipt should be captured");
        assert_eq!(receipt.state, "dry_run");
        assert_eq!(receipt.destination, app_storage_root);
        assert_eq!(receipt.restored_paths, None);
        assert!(sentinel.exists());
        assert_eq!(
            receipt.projection_lifecycle.state,
            DesktopRuntimeProjectionLifecycleState::Stale
        );
        assert_eq!(
            completed.projection_lifecycle.state,
            DesktopRuntimeProjectionLifecycleState::Stale
        );
        assert!(
            !completed
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::ProjectionsReady)
        );
        assert!(runtime.request_shutdown());
        let _ = fs::remove_dir_all(
            backup_source_root
                .parent()
                .expect("backup source should have parent"),
        );
        let _ = fs::remove_dir_all(app_data_root);
    }

    #[test]
    fn desktop_runtime_supervisor_projection_rebuild_uses_effect_snapshots() {
        let storage_root = temp_storage_root("projection_rebuild");
        let config = DesktopRuntimeSupervisorConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = DesktopRuntimeSupervisor::start(config).expect("supervisor should start");
        poll_snapshot(&runtime, |snapshot| {
            matches!(snapshot.state, DesktopRuntimeLifecycleState::Ready)
        });

        let begin = runtime
            .begin_projection_rebuild()
            .expect("projection rebuild begin effect should enqueue");
        let rebuilding = poll_snapshot(&runtime, |snapshot| {
            snapshot
                .last_effect
                .as_ref()
                .is_some_and(|effect| effect.receipt.effect_id == begin.effect_id)
                && matches!(
                    snapshot.state,
                    DesktopRuntimeLifecycleState::RebuildingProjections
                )
        });
        assert_eq!(
            rebuilding.projection_lifecycle.state,
            DesktopRuntimeProjectionLifecycleState::Rebuilding
        );
        assert!(
            !rebuilding
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::ProjectionsReady)
        );

        let refresh = runtime
            .request_diagnostics_refresh()
            .expect("diagnostics refresh should be accepted while rebuilding");
        let blocked = poll_snapshot(&runtime, |snapshot| {
            snapshot.last_effect.as_ref().is_some_and(|effect| {
                effect.receipt.effect_id == refresh.effect_id
                    && matches!(effect.state, DesktopRuntimeEffectState::Failed)
            })
        });
        let issue = blocked
            .last_effect
            .as_ref()
            .and_then(|effect| effect.issue.as_ref())
            .expect("blocked refresh should expose lifecycle issue");
        assert_eq!(issue.code, "sdk_lifecycle_busy");
        assert_eq!(issue.detail_json["state"], "RebuildingProjections");

        let complete = runtime
            .complete_projection_rebuild()
            .expect("projection rebuild complete effect should enqueue");
        let current = poll_snapshot(&runtime, |snapshot| {
            snapshot.last_effect.as_ref().is_some_and(|effect| {
                effect.receipt.effect_id == complete.effect_id
                    && matches!(effect.state, DesktopRuntimeEffectState::Completed)
            }) && matches!(snapshot.state, DesktopRuntimeLifecycleState::Ready)
        });
        assert_eq!(
            current.projection_lifecycle.state,
            DesktopRuntimeProjectionLifecycleState::Current
        );
        assert!(
            current
                .startup_milestones
                .contains(&DesktopRuntimeStartupMilestone::ProjectionsReady)
        );
        assert!(runtime.request_shutdown());
        let _ = fs::remove_dir_all(storage_root);
    }

    fn poll_snapshot(
        runtime: &DesktopRuntimeSupervisor,
        predicate: impl Fn(&DesktopRuntimeSnapshot) -> bool,
    ) -> DesktopRuntimeSnapshot {
        for _ in 0..500 {
            let snapshot = runtime.snapshot();
            if predicate(&snapshot) {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(10));
        }
        runtime.snapshot()
    }

    fn temp_storage_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("radroots_studio_desktop_runtime_{label}_{nanos}"))
            .join(DESKTOP_RUNTIME_STORAGE_DIR_NAME)
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
            location: Some(RadrootsListingPublicLocation {
                primary: "North Farm".to_owned(),
                city: None,
                region: None,
                country: Some("US".to_owned()),
                geohash: "9q8yy".to_owned(),
            }),
            images: None,
        }
    }
}
