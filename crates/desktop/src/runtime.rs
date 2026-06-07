use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Duration, Utc};
use radroots_studio_app_core::{
    AppBuildIdentity, AppDesktopRuntimePaths, AppRuntimeCapture, AppRuntimeMode,
    AppRuntimePathsError, AppRuntimeSnapshot, AppSharedAccountsPaths, PackDayExportWriteError,
    prepare_pack_day_export_bundle_at_data_root,
    shared_local_events_database_path_from_shared_accounts, write_prepared_pack_day_export_bundle,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
};
use radroots_studio_app_sqlite::{
    APP_ACTIVITY_CONTEXT_LIMIT, AppLocalInteropImportReport, AppSqliteError, AppSqliteStore,
    BuyerOrderLocalEventExport, BuyerOrderLocalEventLine, BuyerRepeatDemandApplyOutcome,
    DatabaseTarget, SelectedBuyerOrderScope, SellerOrderDecisionExport, StoredPendingSyncOperation,
    StoredRelayIngestCursor, StoredSyncConflict, derive_farm_rules_readiness,
    projected_order_id_from_trade_request,
};
use radroots_studio_app_state::{
    APP_STATE_FILE_NAME, AppShellProjection, AppStateCommand, AppStatePersistenceRepository,
    AppStateStore, AppStateStoreError, BuyerBrowseScreenProjection, BuyerCartScreenProjection,
    BuyerOrdersScreenProjection, BuyerSearchScreenProjection, BuyerSearchScreenQueryState,
    FarmSetupFlowStage, FarmWorkspaceReadinessProjection, HomeRoute, OrdersScreenProjection,
    PackDayBatchPrintRequest, PackDayExportRequest, PackDayHostHandoffRequest, PackDayPrintRequest,
    PackDayScreenProjection, PersistedAppState, PersonalWorkspaceProjection,
    ProductsScreenProjection, ProductsScreenQueryState, derive_product_publish_blockers,
    derive_sync_projection,
};
use radroots_studio_app_sync::{
    AppFarmProfilePublishPayload, AppListingPublishPayload, AppOrderCancellationPublishPayload,
    AppOrderDecisionInventoryCommitment, AppOrderDecisionPayload, AppOrderDecisionPublishPayload,
    AppOrderFulfillmentPublishPayload, AppOrderReceiptOutcome, AppOrderReceiptPublishPayload,
    AppOrderRequestItemPayload, AppOrderRequestPublishPayload,
    AppOrderRevisionDecisionPublishPayload, AppOrderRevisionProposalPublishPayload,
    AppPublishContext, AppPublishPayload, AppPublishedOperationReceipt,
    AppRelayIngestScopeFreshness, AppSyncProjection, AppSyncRequest, AppSyncResult,
    AppSyncRunStatus, AppSyncTransport, AppSyncTransportError, PendingSyncOperation,
    SyncAggregateRef, SyncCheckpointStatus, SyncConflictSeverity, SyncOperationKind, SyncTrigger,
};
use radroots_studio_app_view::{
    ActiveSurface, AppActivityContext, AppActivityKind, AppIdentityProjection, AppStartupGate,
    BuyerCartLineProjection, BuyerCartProjection, BuyerCartReplaceConfirmationProjection,
    BuyerContext, BuyerOrderDetailProjection, BuyerOrderReviewDraft, BuyerOrderStatus,
    BuyerProductDetailProjection, FarmId, FarmOrderMethod, FarmProfileRecord, FarmReadiness,
    FarmRulesProjection, FarmSetupDraft, FarmSetupProjection, FarmSummary, FarmerSection,
    FulfillmentWindowId, LoggedOutStartupProjection, OrderDetailProjection, OrderFulfillmentAction,
    OrderId, OrderRecoveryProjection, OrderStatus, OrdersFilter, OrdersListProjection,
    OrdersScreenQueryState, PackDayBatchPrintStatus, PackDayExportBundle, PackDayExportInstanceId,
    PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPrintKind,
    PackDayPrintStatus, PackDayProjection, PackDayScreenQueryState, PersonalSection,
    PickupLocationRecord, ProductEditorDraft, ProductId, ProductStatus, ProductsFilter,
    ProductsListProjection, ProductsSort, RecoveryKind, RecoveryQueueProjection, RecoveryRecordId,
    RecoveryState, ReminderDeadlineProjection, ReminderDeliveryState, ReminderFeedProjection,
    ReminderId, ReminderKind, ReminderLogEntryProjection, ReminderLogProjection, ReminderSurface,
    ReminderUrgency, SettingsAccountProjection, SettingsPreference, SettingsSection, ShellSection,
    TodayAgendaProjection,
};
use radroots_core::{
    RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreQuantity,
    RadrootsCoreQuantityPrice, RadrootsCoreUnit,
};
use radroots_events::kinds::{
    KIND_FARM, KIND_LISTING, KIND_LISTING_DRAFT, KIND_PROFILE, KIND_TRADE_CANCEL,
    KIND_TRADE_FULFILLMENT_UPDATE, KIND_TRADE_ORDER_DECISION, KIND_TRADE_ORDER_REQUEST,
    KIND_TRADE_ORDER_REVISION, KIND_TRADE_ORDER_REVISION_RESPONSE, KIND_TRADE_PAYMENT_RECORDED,
    KIND_TRADE_RECEIPT, KIND_TRADE_SETTLEMENT_DECISION,
};
use radroots_events_codec::trade::{
    active_trade_event_context_from_tags, active_trade_payment_recorded_from_event,
    active_trade_settlement_decision_from_event,
};
use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
use radroots_local_events::{
    BUYER_ORDER_REQUEST_ACTOR_SOURCE_RESOLVED_ACCOUNT,
    BUYER_ORDER_REQUEST_ACTOR_SOURCE_UNRESOLVED_APP, BUYER_ORDER_REQUEST_DOCUMENT_KIND,
    BUYER_ORDER_REQUEST_LOCAL_WORK_RECORD_KIND, LocalEventRecord, LocalEventRecordInput,
    LocalEventRecordUpdate, LocalEventsStore, LocalRecordFamily, LocalRecordStatus,
    PublishOutboxStatus, RelayDeliveryEvidence, RelayDeliveryFailure, SourceRuntime,
    buyer_order_request_local_work_record_id, validate_buyer_order_request_local_work_payload,
};
use radroots_nostr::prelude::{
    RadrootsNostrClient, RadrootsNostrEvent, RadrootsNostrFilter, RadrootsNostrOutput,
    RadrootsNostrTimestamp, radroots_nostr_kind, radroots_nostr_parse_pubkey,
};
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
use radroots_sdk::farm::{RadrootsFarm, RadrootsFarmRef};
use radroots_sdk::listing::{
    RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
    RadrootsListingDeliveryMethod, RadrootsListingLocation, RadrootsListingProduct,
    RadrootsListingStatus,
};
use radroots_sdk::trade::{
    RadrootsActiveTradeFulfillmentState, RadrootsTradeBuyerReceipt,
    RadrootsTradeFulfillmentUpdated, RadrootsTradeInventoryCommitment, RadrootsTradeOrderCancelled,
    RadrootsTradeOrderDecision, RadrootsTradeOrderDecisionEvent, RadrootsTradeOrderEconomics,
    RadrootsTradeOrderItem, RadrootsTradeOrderRequested, RadrootsTradeOrderRevisionDecision,
    RadrootsTradeOrderRevisionDecisionEvent, RadrootsTradeOrderRevisionProposed,
};
use radroots_sdk::{
    RadrootsNostrEventPtr, RadrootsSdkClient, RadrootsSdkConfig, RelayConfig, SdkEnvironment,
    SdkPublishReceipt, SdkTransportMode, SdkTransportReceipt, SignerConfig,
};
use radroots_sql_core::SqliteExecutor;
use radroots_trade::order::{
    RadrootsActiveOrderCancellationRecord, RadrootsActiveOrderDecisionRecord,
    RadrootsActiveOrderFulfillmentRecord, RadrootsActiveOrderPaymentRecord,
    RadrootsActiveOrderPaymentState, RadrootsActiveOrderReceiptRecord,
    RadrootsActiveOrderRequestRecord, RadrootsActiveOrderRevisionDecisionRecord,
    RadrootsActiveOrderRevisionProposalRecord, RadrootsActiveOrderSettlementRecord,
    RadrootsActiveOrderStatus, reduce_active_order_events,
};
use serde_json::json;
use thiserror::Error;
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tracing::error;
use uuid::Uuid;

use crate::accounts::{
    DesktopAccountsBootstrapError, DesktopAccountsCommandError, DesktopAccountsProjectionError,
    DesktopLocalIdentityImportRequest, bootstrap_desktop_accounts, generate_local_account,
    identity_projection_from_manager, import_local_account, remove_selected_local_key,
    reset_local_device_state, select_active_surface, select_local_account,
};
use crate::pack_day_host_handoff::{
    PackDayHostHandoffCommandPlan, PackDayHostHandoffError, plan_pack_day_host_handoff,
};
use crate::pack_day_print::{
    PackDayBatchPrintCommandPlan, PackDayBatchPrintError, PackDayPrintCommandPlan,
    PackDayPrintError, cleanup_prepared_customer_label_asset_root,
    cleanup_prepared_customer_label_assets_for_export_instance, plan_pack_day_batch_print,
    plan_pack_day_print,
};
use crate::remote_signer::{
    DesktopRemoteSignerError, DesktopRemoteSignerPaths, activate_pending_session,
    apply_remote_signer_custody, clear_pending_session, load_pending_session, purge_all_state,
    reconcile_startup, store_pending_session,
};

const APP_DATABASE_FILE_NAME: &str = "app.sqlite3";
const SYNC_TRANSPORT_UNAVAILABLE_MESSAGE: &str = "remote sync transport is not configured";
const APP_DIRECT_RELAY_SYNC_TIMEOUT_MS: u64 = 2_000;
const APP_DIRECT_RELAY_CONNECT_TIMEOUT: StdDuration = StdDuration::from_secs(10);
const APP_DIRECT_RELAY_INGEST_LIMIT: usize = 1_000;
const APP_DIRECT_RELAY_INGEST_MAX_PAGES: usize = 5;
const APP_DIRECT_RELAY_INGEST_SCOPE_KEY: &str = "direct_relay_ingest";
const APP_DIRECT_RELAY_INGEST_STALE_AFTER_SECONDS: i64 = 900;
const APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE: u32 = 250;
const APP_DIRECT_RELAY_INGEST_KINDS: &[u16] = &[
    KIND_PROFILE as u16,
    KIND_FARM as u16,
    KIND_LISTING as u16,
    KIND_LISTING_DRAFT as u16,
    KIND_TRADE_ORDER_REQUEST as u16,
    KIND_TRADE_ORDER_DECISION as u16,
    KIND_TRADE_ORDER_REVISION as u16,
    KIND_TRADE_ORDER_REVISION_RESPONSE as u16,
    KIND_TRADE_CANCEL as u16,
    KIND_TRADE_FULFILLMENT_UPDATE as u16,
    KIND_TRADE_RECEIPT as u16,
    KIND_TRADE_PAYMENT_RECORDED as u16,
    KIND_TRADE_SETTLEMENT_DECISION as u16,
];

#[derive(Debug, Default)]
struct UnavailableAppSyncTransport;

impl AppSyncTransport for UnavailableAppSyncTransport {
    fn sync(&mut self, _request: AppSyncRequest) -> Result<AppSyncResult, AppSyncTransportError> {
        Err(AppSyncTransportError::unavailable(
            SYNC_TRANSPORT_UNAVAILABLE_MESSAGE,
        ))
    }

    fn supports_empty_sync_request(&self) -> bool {
        false
    }
}

fn default_sync_transport() -> Box<dyn AppSyncTransport + Send> {
    Box::new(UnavailableAppSyncTransport)
}

#[derive(Debug, Clone)]
struct AppDirectRelayFetchReceipt {
    target_relays: Vec<String>,
    connected_relays: Vec<String>,
    failed_relays: Vec<RelayDeliveryFailure>,
    fetched_relays: Vec<AppDirectRelayFetchedRelay>,
    event_observed_relays: BTreeMap<String, Vec<String>>,
    events: Vec<RadrootsNostrEvent>,
}

#[derive(Debug, Clone)]
struct AppDirectRelayFetchedRelay {
    relay_url: String,
    last_event_created_at_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppSellerOrderDecisionCommand {
    Accept,
    Decline { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppSellerOrderRequest {
    request_event_id: String,
    request_author_pubkey: String,
    listing_event_id: Option<String>,
    payload: RadrootsTradeOrderRequested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppOrderDecisionEvidence {
    event_id: String,
    payload: RadrootsTradeOrderDecisionEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppOrderRevisionProposalEvidence {
    event_id: String,
    payload: RadrootsTradeOrderRevisionProposed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppOrderRevisionDecisionEvidence {
    event_id: String,
    payload: RadrootsTradeOrderRevisionDecisionEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppOrderFulfillmentEvidence {
    event_id: String,
    status: RadrootsActiveTradeFulfillmentState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAppOrderLifecycleEvidence {
    status: RadrootsActiveOrderStatus,
    payment_state: RadrootsActiveOrderPaymentState,
    agreement_event_id: Option<String>,
    last_event_id: Option<String>,
    decision: Option<ResolvedAppOrderDecisionEvidence>,
    revision_proposals: Vec<ResolvedAppOrderRevisionProposalEvidence>,
    revision_decisions: Vec<ResolvedAppOrderRevisionDecisionEvidence>,
    latest_fulfillment: Option<ResolvedAppOrderFulfillmentEvidence>,
    cancellation_event_id: Option<String>,
    receipt_event_id: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AppActiveOrderEvidenceBuckets {
    requests: Vec<RadrootsActiveOrderRequestRecord>,
    decisions: Vec<RadrootsActiveOrderDecisionRecord>,
    revision_proposals: Vec<RadrootsActiveOrderRevisionProposalRecord>,
    revision_decisions: Vec<RadrootsActiveOrderRevisionDecisionRecord>,
    fulfillments: Vec<RadrootsActiveOrderFulfillmentRecord>,
    cancellations: Vec<RadrootsActiveOrderCancellationRecord>,
    receipts: Vec<RadrootsActiveOrderReceiptRecord>,
    payments: Vec<RadrootsActiveOrderPaymentRecord>,
    settlements: Vec<RadrootsActiveOrderSettlementRecord>,
}

#[derive(Debug, Default)]
struct AppDirectRelayIngestReport {
    local_import: AppLocalInteropImportReport,
    freshness_changed: bool,
}

#[derive(Debug, Error)]
enum AppDirectRelayIngestError {
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    Transport(#[from] AppSyncTransportError),
}

#[derive(Clone)]
struct SdkDirectRelayAppSyncTransport {
    accounts_manager: RadrootsNostrAccountsManager,
    relay_urls: Vec<String>,
    timeout_ms: u64,
}

impl SdkDirectRelayAppSyncTransport {
    fn new(accounts_manager: RadrootsNostrAccountsManager, nostr_relay_urls: Vec<String>) -> Self {
        Self {
            accounts_manager,
            relay_urls: nostr_relay_urls,
            timeout_ms: APP_DIRECT_RELAY_SYNC_TIMEOUT_MS,
        }
    }

    #[cfg(test)]
    fn with_relay_urls(
        accounts_manager: RadrootsNostrAccountsManager,
        relay_urls: Vec<String>,
    ) -> Self {
        Self {
            accounts_manager,
            relay_urls,
            timeout_ms: APP_DIRECT_RELAY_SYNC_TIMEOUT_MS,
        }
    }

    fn sync_with_sdk(
        &self,
        request: AppSyncRequest,
    ) -> Result<AppSyncResult, AppSyncTransportError> {
        let run_started_at = current_utc_timestamp();
        let relay_urls = normalized_app_sync_relay_urls(&self.relay_urls)?;
        let client = direct_relay_sdk_client(relay_urls.clone(), self.timeout_ms)?;
        let mut published_receipts = Vec::new();

        for operation in &request.pending_operations {
            match publish_pending_sync_operation(
                &client,
                &self.accounts_manager,
                operation,
                &relay_urls,
            ) {
                Ok(receipt) => published_receipts.push(receipt),
                Err(error) => {
                    if published_receipts.is_empty() {
                        return Err(error);
                    }
                    return Ok(partial_failed_sync_result(
                        &request,
                        published_receipts,
                        run_started_at,
                        error,
                    ));
                }
            }
        }

        Ok(AppSyncResult {
            run_status: radroots_studio_app_sync::AppSyncRunStatus::Succeeded,
            checkpoint: SyncCheckpointStatus::current(
                request.checkpoint.last_sync_started_at.clone(),
                current_utc_timestamp(),
                request.checkpoint.last_remote_cursor.clone(),
            ),
            pushed_operation_count: request.pending_operations.len(),
            pulled_record_count: 0,
            conflicts: request.known_conflicts,
            published_receipts,
        })
    }
}

fn publish_pending_sync_operation(
    client: &RadrootsSdkClient,
    accounts_manager: &RadrootsNostrAccountsManager,
    operation: &PendingSyncOperation,
    relay_urls: &[String],
) -> Result<AppPublishedOperationReceipt, AppSyncTransportError> {
    if operation.operation != SyncOperationKind::Upsert {
        return Err(AppSyncTransportError::failed(
            "direct relay app sync supports upsert publish work only",
        ));
    }
    let publish_payload = operation.publish_payload().map_err(|error| {
        AppSyncTransportError::failed(format!(
            "pending app sync operation is not a typed publish payload: {error}"
        ))
    })?;
    publish_payload.validate().map_err(|error| {
        let reason_codes = error
            .reason_codes
            .into_iter()
            .map(|reason| reason.storage_key())
            .collect::<Vec<_>>()
            .join(",");
        AppSyncTransportError::failed(format!(
            "pending app publish work is blocked: {reason_codes}"
        ))
    })?;
    let identity = signing_identity_for_publish_payload(accounts_manager, &publish_payload)?;
    let receipt = publish_app_payload_sync(client, &identity, &publish_payload, relay_urls)?;
    published_operation_receipt(operation.operation_key.as_str(), &publish_payload, receipt)
}

fn signing_identity_for_publish_payload(
    accounts_manager: &RadrootsNostrAccountsManager,
    publish_payload: &AppPublishPayload,
) -> Result<RadrootsIdentity, AppSyncTransportError> {
    let context = publish_payload_context(publish_payload);
    let account_id = RadrootsIdentityId::parse(context.account_id.trim()).map_err(|error| {
        AppSyncTransportError::failed(format!(
            "pending app publish work has invalid account context: {error}"
        ))
    })?;
    let record = accounts_manager
        .list_accounts()
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?
        .into_iter()
        .find(|record| record.account_id == account_id)
        .ok_or_else(|| {
            AppSyncTransportError::unavailable(format!(
                "publish account is not configured locally: {account_id}"
            ))
        })?;
    let identity = accounts_manager
        .get_signing_identity(&account_id)
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?
        .ok_or_else(|| {
            AppSyncTransportError::unavailable(format!(
                "publish account is not backed by a local signing key: {account_id}"
            ))
        })?;
    if identity.public_key_hex() != record.public_identity.public_key_hex {
        return Err(AppSyncTransportError::failed(
            "publish account signing key does not match account context",
        ));
    }
    Ok(identity)
}

fn publish_payload_context(publish_payload: &AppPublishPayload) -> &AppPublishContext {
    match publish_payload {
        AppPublishPayload::FarmProfile(payload) => &payload.context,
        AppPublishPayload::Listing(payload) => &payload.context,
        AppPublishPayload::OrderRequest(payload) => &payload.context,
        AppPublishPayload::OrderDecision(payload) => &payload.context,
        AppPublishPayload::OrderRevisionProposal(payload) => &payload.context,
        AppPublishPayload::OrderRevisionDecision(payload) => &payload.context,
        AppPublishPayload::OrderCancellation(payload) => &payload.context,
        AppPublishPayload::OrderFulfillment(payload) => &payload.context,
        AppPublishPayload::OrderReceipt(payload) => &payload.context,
    }
}

fn partial_failed_sync_result(
    request: &AppSyncRequest,
    published_receipts: Vec<AppPublishedOperationReceipt>,
    run_started_at: String,
    error: AppSyncTransportError,
) -> AppSyncResult {
    AppSyncResult {
        run_status: radroots_studio_app_sync::AppSyncRunStatus::Failed,
        checkpoint: SyncCheckpointStatus::failed(
            Some(run_started_at),
            Some(current_utc_timestamp()),
            request.checkpoint.last_remote_cursor.clone(),
            error.to_string(),
        ),
        pushed_operation_count: published_receipts.len(),
        pulled_record_count: 0,
        conflicts: request.known_conflicts.clone(),
        published_receipts,
    }
}

impl AppSyncTransport for SdkDirectRelayAppSyncTransport {
    fn sync(&mut self, request: AppSyncRequest) -> Result<AppSyncResult, AppSyncTransportError> {
        self.sync_with_sdk(request)
    }
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntime {
    state: Arc<Mutex<DesktopAppRuntimeState>>,
}

impl DesktopAppRuntime {
    pub fn bootstrap(nostr_relay_urls: Vec<String>, runtime_snapshot: AppRuntimeSnapshot) -> Self {
        let state =
            match DesktopAppRuntimeState::try_bootstrap(nostr_relay_urls, runtime_snapshot.clone())
            {
                Ok(state) => state,
                Err(error) => {
                    DesktopAppRuntimeState::degraded_with_snapshot(error, runtime_snapshot)
                }
            };

        Self::from_state(state)
    }

    pub fn bootstrap_with_paths(
        paths: AppDesktopRuntimePaths,
        nostr_relay_urls: Vec<String>,
    ) -> Self {
        let runtime_snapshot = default_runtime_snapshot();
        let state = match DesktopAppRuntimeState::bootstrap_from_paths(
            paths,
            nostr_relay_urls,
            runtime_snapshot.clone(),
        ) {
            Ok(state) => state,
            Err(error) => DesktopAppRuntimeState::degraded_with_snapshot(error, runtime_snapshot),
        };

        Self::from_state(state)
    }

    pub fn summary(&self) -> DesktopAppRuntimeSummary {
        let state = self.lock_state();
        let sync_status = DesktopAppSyncStatusSummary {
            account_id: state
                .state_store
                .identity_projection()
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.clone()),
            projection: state.state_store.sync_projection().clone(),
            pending_write_count: state.selected_account_pending_sync_write_count,
            conflicts: state.selected_account_sync_conflicts.clone(),
        };

        DesktopAppRuntimeSummary {
            shell_projection: state.state_store.shell_projection().clone(),
            settings_account_projection: state.state_store.settings_account_projection(),
            startup_gate: state.state_store.startup_gate(),
            logged_out_startup: state.state_store.logged_out_startup_projection().clone(),
            home_route: state.state_store.home_route(),
            personal_projection: state.state_store.personal_projection().clone(),
            farm_setup_projection: state.state_store.farm_setup_projection().clone(),
            farm_rules_projection: state.state_store.farm_rules_projection().clone(),
            farm_readiness_projection: state.state_store.farm_readiness_projection().clone(),
            today_projection: state.state_store.today_projection().clone(),
            products_projection: state.state_store.products_projection().clone(),
            orders_projection: state.state_store.orders_projection().clone(),
            pack_day_projection: state.state_store.pack_day_projection().clone(),
            reminder_log: state.state_store.reminder_log_projection().clone(),
            runtime_metadata: state.runtime_metadata.clone(),
            sync_status,
            startup_issue: state.startup_issue.clone(),
        }
    }

    pub fn nostr_relay_urls(&self) -> Vec<String> {
        self.lock_state().nostr_relay_urls.clone()
    }

    pub fn selected_settings_section(&self) -> SettingsSection {
        self.lock_state()
            .state_store
            .shell_projection()
            .settings
            .selected_section
    }

    pub fn sync_settings_section(&self, section: SettingsSection) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::select_settings_section(section))
    }

    pub fn show_startup_identity_choice(&self) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::show_startup_identity_choice())
    }

    pub fn begin_generate_key_startup(&self) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::begin_generate_key_startup())
    }

    pub fn show_startup_signer_entry(&self) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::show_startup_signer_entry())
    }

    pub fn set_startup_signer_source_input(&self, source_input: &str) -> bool {
        self.lock_state_mut().state_store.apply_in_memory(
            AppStateCommand::set_startup_signer_source_input(source_input),
        )
    }

    pub fn load_startup_pending_remote_signer_session(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerPendingSession>, DesktopAppRuntimeCommandError> {
        self.lock_state()
            .load_startup_pending_remote_signer_session()
    }

    pub fn store_startup_pending_remote_signer_session(
        &self,
        pending: &RadrootsAppRemoteSignerPendingSession,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut()
            .store_startup_pending_remote_signer_session(pending)
    }

    pub fn clear_startup_pending_remote_signer_session(
        &self,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut()
            .clear_startup_pending_remote_signer_session()
    }

    pub fn activate_startup_approved_remote_signer_session(
        &self,
        pending: &RadrootsAppRemoteSignerPendingSession,
        approved: &RadrootsAppRemoteSignerApprovedSession,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut()
            .activate_startup_approved_remote_signer_session(pending, approved)
    }

    pub fn select_settings_section(&self, section: SettingsSection) -> bool {
        let changed = self.sync_settings_section(section);

        if changed {
            let _ = self.record_activity(AppActivityKind::SettingsSectionSelected { section });
        }

        changed
    }

    pub fn select_home(&self) -> bool {
        let mut state = self.lock_state_mut();
        let selected_section = match state.state_store.startup_gate() {
            AppStartupGate::Farmer => ShellSection::Farmer(FarmerSection::Today),
            AppStartupGate::Blocked | AppStartupGate::SetupRequired => ShellSection::Home,
            AppStartupGate::Personal => ShellSection::Personal(PersonalSection::Browse),
        };

        let section_changed = state
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(selected_section));
        let editor_changed = state.close_product_editor();

        section_changed || editor_changed
    }

    pub fn select_account(&self) -> bool {
        let mut state = self.lock_state_mut();
        let section_changed = state
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Account));
        let editor_changed = state.close_product_editor();

        section_changed || editor_changed
    }

    pub fn select_personal_section(
        &self,
        section: PersonalSection,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().select_personal_section(section)
    }

    pub fn select_farmer_section(&self, section: FarmerSection) -> bool {
        self.lock_state_mut().select_farmer_section(section)
    }

    pub fn open_personal_product_detail(
        &self,
        section: PersonalSection,
        product_id: ProductId,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .open_personal_product_detail(section, product_id)
    }

    pub fn close_personal_product_detail(&self, section: PersonalSection) -> bool {
        self.lock_state_mut().close_personal_product_detail(section)
    }

    pub fn increase_personal_product_quantity(&self, section: PersonalSection) -> bool {
        self.lock_state_mut()
            .adjust_personal_product_quantity(section, 1)
    }

    pub fn decrease_personal_product_quantity(&self, section: PersonalSection) -> bool {
        self.lock_state_mut()
            .adjust_personal_product_quantity(section, -1)
    }

    pub fn add_personal_product_to_cart(
        &self,
        section: PersonalSection,
        replace_existing: bool,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .add_personal_product_to_cart(section, replace_existing)
    }

    pub fn clear_personal_cart_replace_confirmation(&self) -> bool {
        self.lock_state_mut()
            .clear_personal_cart_replace_confirmation()
    }

    pub fn remove_personal_cart_line(&self, product_id: ProductId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().remove_personal_cart_line(product_id)
    }

    pub fn save_personal_order_review_draft(
        &self,
        draft: BuyerOrderReviewDraft,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .save_personal_order_review_draft(draft)
    }

    pub fn place_personal_order(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().place_personal_order()
    }

    pub fn retry_pending_personal_order_coordination(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .retry_pending_personal_order_coordination()
    }

    pub fn open_personal_order_detail(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_personal_order_detail(order_id)
    }

    pub fn repeat_personal_order(
        &self,
        order_id: OrderId,
        replace_existing: bool,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .repeat_personal_order(order_id, replace_existing)
    }

    pub fn set_personal_search_query(&self, search_query: &str) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .set_personal_search_query(search_query)
    }

    pub fn set_personal_search_fulfillment_method(
        &self,
        method: FarmOrderMethod,
        enabled: bool,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .set_personal_search_fulfillment_method(method, enabled)
    }

    pub fn set_products_search_query(&self, search_query: &str) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .set_products_search_query(search_query)
    }

    pub fn select_products_filter(&self, filter: ProductsFilter) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().select_products_filter(filter)
    }

    pub fn select_products_sort(&self, sort: ProductsSort) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().select_products_sort(sort)
    }

    pub fn open_products_filter(&self, filter: ProductsFilter) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_products_filter(filter)
    }

    pub fn select_orders_filter(&self, filter: OrdersFilter) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().select_orders_filter(filter)
    }

    pub fn open_orders(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_orders()
    }

    pub fn open_orders_fulfillment_window(
        &self,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .open_orders_fulfillment_window(fulfillment_window_id)
    }

    pub fn open_order_detail(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_order_detail(order_id)
    }

    pub fn prepare_order_accept(
        &self,
        order_id: OrderId,
    ) -> Result<AppOrderDecisionPublishPayload, AppSqliteError> {
        self.lock_state_mut()
            .prepare_seller_order_decision(order_id, AppSellerOrderDecisionCommand::Accept)
    }

    pub fn prepare_order_decline(
        &self,
        order_id: OrderId,
        reason: &str,
    ) -> Result<AppOrderDecisionPublishPayload, AppSqliteError> {
        self.lock_state_mut().prepare_seller_order_decision(
            order_id,
            AppSellerOrderDecisionCommand::Decline {
                reason: reason.to_owned(),
            },
        )
    }

    pub fn publish_order_accept(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_seller_order_decision(order_id, AppSellerOrderDecisionCommand::Accept)
    }

    pub fn publish_order_decline(
        &self,
        order_id: OrderId,
        reason: &str,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().publish_seller_order_decision(
            order_id,
            AppSellerOrderDecisionCommand::Decline {
                reason: reason.to_owned(),
            },
        )
    }

    pub fn publish_order_fulfillment_update(
        &self,
        order_id: OrderId,
        action: OrderFulfillmentAction,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_seller_order_fulfillment(order_id, action.fulfillment_status())
    }

    pub fn publish_order_revision_proposal(
        &self,
        order_id: OrderId,
        items: Vec<RadrootsTradeOrderItem>,
        economics: RadrootsTradeOrderEconomics,
        reason: &str,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_seller_order_revision_proposal(order_id, items, economics, reason)
    }

    pub fn publish_buyer_order_cancel(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_buyer_order_cancellation(order_id)
    }

    pub fn publish_buyer_order_revision_accept(
        &self,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_buyer_order_revision_accept(order_id)
    }

    pub fn publish_buyer_order_revision_decline(
        &self,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_buyer_order_revision_decline(order_id)
    }

    pub fn publish_buyer_order_receipt(
        &self,
        order_id: OrderId,
        outcome: AppOrderReceiptOutcome,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .publish_buyer_order_receipt(order_id, outcome)
    }

    pub fn start_order_recovery(
        &self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().start_order_recovery(order_id, kind)
    }

    pub fn review_order_recovery(
        &self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().review_order_recovery(order_id, kind)
    }

    pub fn reopen_order_recovery(
        &self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().reopen_order_recovery(order_id, kind)
    }

    pub fn resolve_order_recovery(
        &self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().resolve_order_recovery(order_id, kind)
    }

    pub fn open_pack_day(
        &self,
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_pack_day(fulfillment_window_id)
    }

    pub fn export_pack_day(&self) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().export_pack_day()
    }

    pub fn prepare_pack_day_host_handoff(
        &self,
        kind: PackDayHostHandoffKind,
    ) -> Result<
        Option<(PackDayHostHandoffRequest, PackDayHostHandoffCommandPlan)>,
        DesktopAppRuntimeCommandError,
    > {
        self.lock_state_mut().prepare_pack_day_host_handoff(kind)
    }

    pub fn finish_pack_day_host_handoff(
        &self,
        request: PackDayHostHandoffRequest,
        result: Result<(), PackDayHostHandoffError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut()
            .finish_pack_day_host_handoff(request, result)
    }

    pub fn prepare_pack_day_print(
        &self,
        kind: PackDayPrintKind,
    ) -> Result<Option<(PackDayPrintRequest, PackDayPrintCommandPlan)>, DesktopAppRuntimeCommandError>
    {
        self.lock_state_mut().prepare_pack_day_print(kind)
    }

    pub fn finish_pack_day_print(
        &self,
        request: PackDayPrintRequest,
        result: Result<(), PackDayPrintError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().finish_pack_day_print(request, result)
    }

    pub fn prepare_pack_day_batch_print(
        &self,
    ) -> Result<
        Option<(PackDayBatchPrintRequest, PackDayBatchPrintCommandPlan)>,
        DesktopAppRuntimeCommandError,
    > {
        self.lock_state_mut().prepare_pack_day_batch_print()
    }

    pub fn finish_pack_day_batch_print(
        &self,
        request: PackDayBatchPrintRequest,
        result: Result<(), PackDayBatchPrintError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut()
            .finish_pack_day_batch_print(request, result)
    }

    pub fn update_product_stock(
        &self,
        product_id: ProductId,
        stock_quantity: u32,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .update_product_stock(product_id, stock_quantity)
    }

    pub fn open_new_product_editor(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_new_product_editor()
    }

    pub fn open_existing_product_editor(
        &self,
        product_id: ProductId,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .open_existing_product_editor(product_id)
    }

    pub fn save_product_editor_draft(
        &self,
        draft: ProductEditorDraft,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().save_product_editor_draft(draft)
    }

    pub fn close_product_editor(&self) -> bool {
        self.lock_state_mut().close_product_editor()
    }

    pub fn set_settings_preference(&self, preference: SettingsPreference, enabled: bool) -> bool {
        let changed = self.lock_state_mut().state_store.apply_in_memory(
            AppStateCommand::SetSettingsPreference {
                preference,
                enabled,
            },
        );

        if changed {
            let _ = self.record_activity(AppActivityKind::SettingsPreferenceUpdated {
                preference,
                enabled,
            });
        }

        changed
    }

    pub fn generate_local_account(
        &self,
        label: Option<String>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().generate_local_account(label)
    }

    pub fn import_local_account(
        &self,
        request: DesktopLocalIdentityImportRequest,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().import_local_account(&request)
    }

    pub fn select_local_account(
        &self,
        account_id: &str,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().select_local_account(account_id)
    }

    pub fn select_active_surface(
        &self,
        active_surface: ActiveSurface,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().select_active_surface(active_surface)
    }

    pub fn remove_selected_local_key(&self) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().remove_selected_local_key()
    }

    pub fn reset_local_device_state(&self) -> Result<bool, DesktopAppRuntimeCommandError> {
        self.lock_state_mut().reset_local_device_state()
    }

    #[allow(dead_code)]
    pub fn replace_today_agenda(&self, projection: TodayAgendaProjection) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::replace_today_agenda(projection))
    }

    pub fn select_farm_setup_flow_stage(&self, stage: FarmSetupFlowStage) -> bool {
        self.lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::select_farm_setup_flow_stage(stage))
    }

    pub fn save_farm_setup_draft(
        &self,
        draft: FarmSetupDraft,
    ) -> Result<FarmSetupProjection, DesktopAppRuntimeFarmSetupError> {
        self.lock_state_mut().save_farm_setup_draft(draft)
    }

    pub fn finish_farm_setup(
        &self,
    ) -> Result<FarmSetupProjection, DesktopAppRuntimeFarmSetupError> {
        self.lock_state_mut().finish_farm_setup()
    }

    pub fn load_farm_rules_projection(
        &self,
    ) -> Result<FarmRulesProjection, DesktopAppRuntimeFarmRulesError> {
        self.lock_state().load_farm_rules_projection()
    }

    pub fn save_farm_rules_projection(
        &self,
        projection: FarmRulesProjection,
    ) -> Result<FarmRulesProjection, DesktopAppRuntimeFarmRulesError> {
        self.lock_state_mut().save_farm_rules_projection(projection)
    }

    pub fn sync_on_app_launch(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().attempt_sync(SyncTrigger::AppLaunch)
    }

    pub fn sync_on_foreground_resume(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().sync_on_foreground_resume()
    }

    pub fn sync_on_manual_refresh(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .attempt_sync(SyncTrigger::ManualRefresh)
    }

    pub fn refresh_shared_local_events(
        &self,
    ) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        let mut state = self.lock_state_mut();
        let report = state.import_shared_local_events()?;
        let _ = state.refresh_selected_account_context_after_local_events()?;
        Ok(report)
    }

    pub fn resolve_sync_conflict(
        &self,
        conflict_id: &str,
        resolution: radroots_studio_app_sync::SyncConflictResolutionStatus,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut()
            .resolve_sync_conflict(conflict_id, resolution)
    }

    pub fn acknowledge_reminder(&self, reminder_id: ReminderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().acknowledge_reminder(reminder_id)
    }

    pub fn record_home_opened(&self) -> bool {
        self.record_activity(AppActivityKind::HomeOpened)
    }

    pub fn record_settings_opened(&self, section: SettingsSection) -> bool {
        self.record_activity(AppActivityKind::SettingsOpened { section })
    }

    pub fn activity_context(
        &self,
        limit: Option<usize>,
    ) -> Result<AppActivityContext, DesktopAppRuntimeActivityContextError> {
        let state = self.lock_state();
        let store = state
            .sqlite_store
            .as_ref()
            .ok_or(DesktopAppRuntimeActivityContextError::RuntimeUnavailable)?;

        store
            .load_activity_context(limit.unwrap_or(APP_ACTIVITY_CONTEXT_LIMIT))
            .map_err(DesktopAppRuntimeActivityContextError::from)
    }

    fn from_state(state: DesktopAppRuntimeState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, DesktopAppRuntimeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_state_mut(&self) -> MutexGuard<'_, DesktopAppRuntimeState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn record_activity(&self, kind: AppActivityKind) -> bool {
        let result = self.lock_state().record_activity(kind.clone());
        if let Err(error) = result {
            error!(
                target: "activity",
                event = "activity.record_failed",
                activity_kind = kind.storage_key(),
                error = %error,
                "failed to record activity event"
            );
            return false;
        }

        true
    }
}

fn default_runtime_snapshot() -> AppRuntimeSnapshot {
    AppRuntimeSnapshot::from_capture(
        AppBuildIdentity {
            package_name: env!("CARGO_PKG_NAME").to_owned(),
            package_version: env!("CARGO_PKG_VERSION").to_owned(),
            build_profile: option_env!("PROFILE").unwrap_or("debug").to_owned(),
            target_triple: option_env!("TARGET").unwrap_or("unknown-target").to_owned(),
            projection_source: "rust".to_owned(),
            git_commit: None,
        },
        AppRuntimeMode::Development,
        AppRuntimeCapture {
            host_locale: "en_US.UTF-8".to_owned(),
            operating_system: "macos".to_owned(),
            run_id: "runtime-summary-test-run".to_owned(),
        },
    )
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesktopAppSyncStatusSummary {
    pub account_id: Option<String>,
    pub projection: AppSyncProjection,
    pub pending_write_count: usize,
    pub conflicts: Vec<DesktopAppSyncConflictSummary>,
}

impl DesktopAppSyncStatusSummary {
    pub const fn is_enabled(&self) -> bool {
        self.account_id.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAppSyncConflictSummary {
    pub conflict_id: String,
    pub conflict: radroots_studio_app_sync::SyncConflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopAppRuntimeMetadataSummary {
    pub snapshot: AppRuntimeSnapshot,
    pub data_root: Option<PathBuf>,
    pub logs_root: Option<PathBuf>,
    pub database_path: Option<PathBuf>,
    pub database_schema_version: Option<u32>,
}

impl DesktopAppRuntimeMetadataSummary {
    fn available(
        snapshot: AppRuntimeSnapshot,
        paths: &AppDesktopRuntimePaths,
        database_path: PathBuf,
        database_schema_version: u32,
    ) -> Self {
        Self {
            snapshot,
            data_root: Some(paths.app.data.clone()),
            logs_root: Some(paths.app.logs.clone()),
            database_path: Some(database_path),
            database_schema_version: Some(database_schema_version),
        }
    }

    fn unavailable(snapshot: AppRuntimeSnapshot) -> Self {
        Self {
            snapshot,
            data_root: None,
            logs_root: None,
            database_path: None,
            database_schema_version: None,
        }
    }
}

impl Default for DesktopAppRuntimeMetadataSummary {
    fn default() -> Self {
        Self::unavailable(default_runtime_snapshot())
    }
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntimeSummary {
    pub shell_projection: AppShellProjection,
    pub settings_account_projection: SettingsAccountProjection,
    pub startup_gate: AppStartupGate,
    pub logged_out_startup: LoggedOutStartupProjection,
    pub home_route: HomeRoute,
    pub personal_projection: PersonalWorkspaceProjection,
    pub farm_setup_projection: FarmSetupProjection,
    pub farm_rules_projection: FarmRulesProjection,
    pub farm_readiness_projection: FarmWorkspaceReadinessProjection,
    pub today_projection: TodayAgendaProjection,
    pub products_projection: ProductsScreenProjection,
    pub orders_projection: OrdersScreenProjection,
    pub pack_day_projection: PackDayScreenProjection,
    pub reminder_log: ReminderLogProjection,
    pub runtime_metadata: DesktopAppRuntimeMetadataSummary,
    pub sync_status: DesktopAppSyncStatusSummary,
    pub startup_issue: Option<String>,
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeActivityContextError {
    #[error("desktop runtime activity context is unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
}

#[derive(Clone, Debug, Default)]
struct DesktopSelectedAccountContext {
    personal_projection: PersonalWorkspaceProjection,
    farm_setup_projection: FarmSetupProjection,
    farm_rules_projection: FarmRulesProjection,
    today_projection: TodayAgendaProjection,
    products_query: ProductsScreenQueryState,
    products_list: ProductsListProjection,
    orders_query: OrdersScreenQueryState,
    orders_list: OrdersListProjection,
    orders_reminders: ReminderFeedProjection,
    recovery_queue: RecoveryQueueProjection,
    order_detail: Option<OrderDetailProjection>,
    pack_day_query: PackDayScreenQueryState,
    pack_day_projection: PackDayProjection,
    product_editor_draft: Option<(ProductId, ProductEditorDraft)>,
    reminder_log: ReminderLogProjection,
}

#[derive(Clone, Debug, Default)]
struct DesktopSellerReminderContext {
    today_feed: ReminderFeedProjection,
    orders_feed: ReminderFeedProjection,
    pack_day_feed: ReminderFeedProjection,
    recovery_queue: RecoveryQueueProjection,
    selected_order_recoveries: Vec<OrderRecoveryProjection>,
    due_soon_count: u32,
    recovery_actions_open: u32,
    reminder_log: ReminderLogProjection,
}

#[derive(Clone, Debug, Default)]
struct DesktopReminderSyncTruth {
    checkpoint: SyncCheckpointStatus,
    pending_write_count: usize,
    unresolved_conflict_count: usize,
    blocking_conflict_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DesktopSelectedAccountSyncContext {
    projection: AppSyncProjection,
    relay_ingest: AppRelayIngestScopeFreshness,
    pending_write_count: usize,
    conflicts: Vec<DesktopAppSyncConflictSummary>,
}

#[derive(Clone, Debug)]
struct DesktopPreparedSyncRequest {
    account_id: String,
    checkpoint: SyncCheckpointStatus,
    conflicts: Vec<StoredSyncConflict>,
    pending_operations: Vec<StoredPendingSyncOperation>,
}

struct DesktopAppRuntimeState {
    state_store: AppStateStore<AppStatePersistenceRepository>,
    nostr_relay_urls: Vec<String>,
    shared_accounts_paths: Option<AppSharedAccountsPaths>,
    remote_signer_paths: Option<DesktopRemoteSignerPaths>,
    accounts_manager: Option<RadrootsNostrAccountsManager>,
    sqlite_store: Option<AppSqliteStore>,
    sync_transport: Box<dyn AppSyncTransport + Send>,
    runtime_metadata: DesktopAppRuntimeMetadataSummary,
    selected_account_pending_sync_write_count: usize,
    selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness,
    selected_account_sync_conflicts: Vec<DesktopAppSyncConflictSummary>,
    startup_issue: Option<String>,
}

impl fmt::Debug for DesktopAppRuntimeState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DesktopAppRuntimeState")
            .field("state_store", &self.state_store)
            .field(
                "shared_accounts_paths",
                &self.shared_accounts_paths.as_ref().map(|_| "available"),
            )
            .field(
                "remote_signer_paths",
                &self.remote_signer_paths.as_ref().map(|_| "available"),
            )
            .field(
                "accounts_manager",
                &self.accounts_manager.as_ref().map(|_| "available"),
            )
            .field(
                "sqlite_store",
                &self.sqlite_store.as_ref().map(|_| "available"),
            )
            .field("sync_transport", &"configured")
            .field("runtime_metadata", &self.runtime_metadata)
            .field(
                "selected_account_pending_sync_write_count",
                &self.selected_account_pending_sync_write_count,
            )
            .field(
                "selected_account_relay_ingest_freshness",
                &self.selected_account_relay_ingest_freshness,
            )
            .field(
                "selected_account_sync_conflicts",
                &self.selected_account_sync_conflicts,
            )
            .field("startup_issue", &self.startup_issue)
            .finish()
    }
}

impl DesktopAppRuntimeState {
    fn try_bootstrap(
        nostr_relay_urls: Vec<String>,
        runtime_snapshot: AppRuntimeSnapshot,
    ) -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let paths = AppDesktopRuntimePaths::current_desktop()?;
        Self::bootstrap_from_paths(paths, nostr_relay_urls, runtime_snapshot)
    }

    fn bootstrap_from_paths(
        paths: AppDesktopRuntimePaths,
        nostr_relay_urls: Vec<String>,
        runtime_snapshot: AppRuntimeSnapshot,
    ) -> Result<Self, DesktopAppRuntimeBootstrapError> {
        if let Err(error) = cleanup_prepared_customer_label_asset_root() {
            error!(
                target: "pack_day",
                event = "pack_day.print_prepared_asset_bootstrap_sweep_failed",
                error = %error,
                "failed to sweep prepared pack day print assets during bootstrap"
            );
        }
        let database_path = paths.app.data.join(APP_DATABASE_FILE_NAME);
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::Path(database_path.clone()))?;
        let shared_local_events_database_path = paths.shared_local_events_database_path()?;
        let _ = sqlite_store
            .import_shared_local_events_from_path(shared_local_events_database_path.as_path())?;
        let database_schema_version = sqlite_store.schema_version()?;
        let mut state_store = AppStateStore::load(AppStatePersistenceRepository::file_backed(
            paths.app.data.join(APP_STATE_FILE_NAME),
        ))?;
        let continuity_state = state_store.persisted_state().clone();
        let remote_signer_paths = DesktopRemoteSignerPaths::from_runtime_paths(&paths);
        let accounts_bootstrap = bootstrap_desktop_accounts(&paths.shared_accounts, &sqlite_store)?;
        if let Some(accounts_manager) = accounts_bootstrap.accounts_manager.as_ref() {
            reconcile_startup(accounts_manager, &remote_signer_paths)?;
        }
        let identity_projection = apply_remote_signer_custody(
            identity_projection_from_manager(
                accounts_bootstrap
                    .accounts_manager
                    .as_ref()
                    .expect("desktop bootstrap always returns an accounts manager"),
                &sqlite_store,
            )?,
            &remote_signer_paths,
        )?;
        let selected_account_context =
            load_selected_account_context(&sqlite_store, &identity_projection, &continuity_state)?;
        let selected_account_sync_context = load_selected_account_sync_context(
            &sqlite_store,
            &identity_projection,
            &nostr_relay_urls,
        )?;
        let _ = state_store.apply_in_memory(AppStateCommand::replace_identity_projection(
            identity_projection.clone(),
        ));
        if identity_projection.startup_gate() == AppStartupGate::SetupRequired
            && load_pending_session(&remote_signer_paths)?.is_some()
        {
            let _ = state_store.apply_in_memory(AppStateCommand::show_startup_signer_entry());
        }
        let pending_sync_write_count = selected_account_sync_context.pending_write_count;
        let selected_account_relay_ingest_freshness =
            selected_account_sync_context.relay_ingest.clone();
        let selected_account_sync_conflicts = selected_account_sync_context.conflicts;
        let _ = state_store.apply_in_memory(AppStateCommand::replace_sync_projection(
            selected_account_sync_context.projection,
        ));
        let sync_transport: Box<dyn AppSyncTransport + Send> =
            match accounts_bootstrap.accounts_manager.as_ref() {
                Some(accounts_manager) => Box::new(SdkDirectRelayAppSyncTransport::new(
                    accounts_manager.clone(),
                    nostr_relay_urls.clone(),
                )),
                None => default_sync_transport(),
            };
        let mut state = Self {
            state_store,
            nostr_relay_urls,
            shared_accounts_paths: Some(paths.shared_accounts.clone()),
            remote_signer_paths: Some(remote_signer_paths),
            accounts_manager: accounts_bootstrap.accounts_manager,
            sqlite_store: Some(sqlite_store),
            sync_transport,
            runtime_metadata: DesktopAppRuntimeMetadataSummary::available(
                runtime_snapshot,
                &paths,
                database_path,
                database_schema_version,
            ),
            selected_account_pending_sync_write_count: pending_sync_write_count,
            selected_account_relay_ingest_freshness,
            selected_account_sync_conflicts,
            startup_issue: None,
        };
        let _ = state.apply_selected_account_context(&selected_account_context);
        if let Err(error) = state.retry_pending_personal_order_coordination() {
            error!(
                target: "buyer_order",
                event = "buyer_order.coordination_bootstrap_retry_failed",
                error = %error,
                "failed to retry pending buyer order coordination during bootstrap"
            );
        }

        Ok(state)
    }

    #[cfg(test)]
    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self::degraded_with_snapshot(error, default_runtime_snapshot())
    }

    fn degraded_with_snapshot(
        error: DesktopAppRuntimeBootstrapError,
        runtime_snapshot: AppRuntimeSnapshot,
    ) -> Self {
        Self {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: Vec::new(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: None,
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::unavailable(runtime_snapshot),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: Some(error.to_string()),
        }
    }

    fn generate_local_account(
        &mut self,
        label: Option<String>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            generate_local_account(accounts_manager, sqlite_store, label)?
        };

        self.replace_identity_projection(projection)
    }

    fn import_local_account(
        &mut self,
        request: &DesktopLocalIdentityImportRequest,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            import_local_account(accounts_manager, sqlite_store, request)?
        };

        self.replace_identity_projection(projection)
    }

    fn select_local_account(
        &mut self,
        account_id: &str,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            select_local_account(accounts_manager, sqlite_store, account_id)?
        };

        self.replace_identity_projection(projection)
    }

    fn select_active_surface(
        &mut self,
        active_surface: ActiveSurface,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            select_active_surface(accounts_manager, sqlite_store, active_surface)?
        };

        self.replace_identity_projection(projection)
    }

    fn remove_selected_local_key(&mut self) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            remove_selected_local_key(accounts_manager, sqlite_store)?
        };

        self.replace_identity_projection(projection)
    }

    fn reset_local_device_state(&mut self) -> Result<bool, DesktopAppRuntimeCommandError> {
        if let Some(paths) = self.remote_signer_paths.as_ref() {
            purge_all_state(paths)?;
        }
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            let shared_accounts_paths = self.shared_accounts_paths()?;
            reset_local_device_state(accounts_manager, sqlite_store, shared_accounts_paths)?
        };

        self.replace_identity_projection(projection)
    }

    fn record_activity(&self, kind: AppActivityKind) -> Result<(), AppSqliteError> {
        match self.sqlite_store.as_ref() {
            Some(store) => store.record_activity_event(&kind),
            None => Ok(()),
        }
    }

    fn select_farmer_section(&mut self, section: FarmerSection) -> bool {
        match section {
            FarmerSection::Today => {
                let section_changed =
                    self.state_store
                        .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                            FarmerSection::Today,
                        )));
                let editor_changed = self.close_product_editor();

                section_changed || editor_changed
            }
            FarmerSection::Products if self.has_saved_farm() => {
                self.state_store
                    .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                        FarmerSection::Products,
                    )))
            }
            FarmerSection::Orders if self.has_saved_farm() => {
                let section_changed =
                    self.state_store
                        .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                            FarmerSection::Orders,
                        )));
                let detail_changed = self
                    .state_store
                    .apply_in_memory(AppStateCommand::replace_order_detail(None));
                let editor_changed = self.close_product_editor();

                section_changed || detail_changed || editor_changed
            }
            FarmerSection::PackDay if self.has_saved_farm() && self.has_pack_day_context() => {
                let section_changed =
                    self.state_store
                        .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                            FarmerSection::PackDay,
                        )));
                let editor_changed = self.close_product_editor();

                section_changed || editor_changed
            }
            FarmerSection::Products
            | FarmerSection::Orders
            | FarmerSection::PackDay
            | FarmerSection::Farm => false,
        }
    }

    fn select_personal_section(
        &mut self,
        section: PersonalSection,
    ) -> Result<bool, AppSqliteError> {
        let freshness_changed = if section == PersonalSection::Browse {
            self.refresh_personal_browse_navigation()?
        } else {
            false
        };
        let section_changed = self.apply_personal_section_selection(section);

        Ok(freshness_changed || section_changed)
    }

    fn apply_personal_section_selection(&mut self, section: PersonalSection) -> bool {
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Personal(
                section,
            )));
        let editor_changed = self.close_product_editor();

        section_changed || editor_changed
    }

    fn refresh_personal_browse_navigation(&mut self) -> Result<bool, AppSqliteError> {
        let report = self.import_shared_local_events()?;
        let local_changed = report.imported_records > 0 || report.skipped_records > 0;
        let context_changed = self.refresh_selected_account_context_after_local_events()?;

        Ok(local_changed || context_changed)
    }

    fn open_personal_product_detail(
        &mut self,
        section: PersonalSection,
        product_id: ProductId,
    ) -> Result<bool, AppSqliteError> {
        let should_refresh_before_lookup =
            matches!(section, PersonalSection::Browse | PersonalSection::Search);
        let freshness_changed = if should_refresh_before_lookup {
            self.refresh_personal_browse_navigation()?
        } else {
            false
        };
        let section_changed = if should_refresh_before_lookup {
            self.apply_personal_section_selection(section)
        } else {
            false
        };
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(freshness_changed || section_changed);
        };
        let Some(detail) = sqlite_store.load_buyer_product_detail(product_id)? else {
            return Ok(freshness_changed || section_changed);
        };

        let detail_changed = self.set_personal_product_detail(section, Some(detail));

        Ok(freshness_changed || section_changed || detail_changed)
    }

    fn close_personal_product_detail(&mut self, section: PersonalSection) -> bool {
        self.set_personal_product_detail(section, None)
    }

    fn adjust_personal_product_quantity(&mut self, section: PersonalSection, delta: i32) -> bool {
        self.mutate_personal_projection(|projection| {
            let Some(detail) = personal_detail_mut(projection, section) else {
                return false;
            };
            let next_quantity = if delta.is_negative() {
                detail
                    .selected_quantity
                    .saturating_sub(delta.unsigned_abs())
            } else {
                detail.selected_quantity.saturating_add(delta as u32)
            };

            if next_quantity == 0 || next_quantity == detail.selected_quantity {
                return false;
            }

            detail.selected_quantity = next_quantity;
            true
        })
    }

    fn add_personal_product_to_cart(
        &mut self,
        section: PersonalSection,
        replace_existing: bool,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(detail) =
            personal_detail(self.state_store.personal_projection(), section).cloned()
        else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let current_cart = sqlite_store.load_buyer_cart(&buyer_context)?;

        if !replace_existing
            && !current_cart.is_empty()
            && current_cart.farm_id != Some(detail.listing.farm_id)
        {
            let current_farm_display_name = current_cart
                .farm_display_name
                .clone()
                .or_else(|| {
                    current_cart
                        .lines
                        .first()
                        .map(|line| line.farm_display_name.clone())
                })
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer cart farm display name is missing",
                })?;
            let replace_confirmation = BuyerCartReplaceConfirmationProjection {
                current_farm_display_name,
                incoming_farm_display_name: detail.listing.farm_display_name.clone(),
            };

            return Ok(self.mutate_personal_projection(|projection| {
                let cart = &mut projection.cart.cart;
                if cart.replace_confirmation.as_ref() == Some(&replace_confirmation) {
                    return false;
                }

                cart.replace_confirmation = Some(replace_confirmation);
                true
            }));
        }

        let next_cart = next_buyer_cart_for_detail(current_cart, &detail, replace_existing)?;
        sqlite_store.replace_buyer_cart(&buyer_context, &next_cart)?;
        let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
        let refreshed_order_review = sqlite_store.load_buyer_order_review(&buyer_context)?;
        let cart_changed = self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.order_review != refreshed_order_review {
                projection.cart.order_review = refreshed_order_review.clone();
                changed = true;
            }
            changed
        });
        let section_changed = self.select_personal_section(PersonalSection::Cart)?;

        Ok(cart_changed || section_changed)
    }

    fn clear_personal_cart_replace_confirmation(&mut self) -> bool {
        self.mutate_personal_projection(|projection| {
            if projection.cart.cart.replace_confirmation.is_none() {
                return false;
            }

            projection.cart.cart.replace_confirmation = None;
            true
        })
    }

    fn remove_personal_cart_line(&mut self, product_id: ProductId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let current_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
        let Some(next_cart) = next_buyer_cart_after_removing_line(current_cart, product_id)? else {
            return Ok(false);
        };

        if next_cart.lines.is_empty() {
            sqlite_store.clear_buyer_cart(&buyer_context)?;
        } else {
            sqlite_store.replace_buyer_cart(&buyer_context, &next_cart)?;
        }

        let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
        let refreshed_order_review = sqlite_store.load_buyer_order_review(&buyer_context)?;

        Ok(self.refresh_personal_cart_and_order_review(refreshed_cart, refreshed_order_review))
    }

    fn save_personal_order_review_draft(
        &mut self,
        draft: BuyerOrderReviewDraft,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        sqlite_store.save_buyer_order_review_draft(&buyer_context, &draft)?;
        let refreshed_order_review = sqlite_store.load_buyer_order_review(&buyer_context)?;

        Ok(self.mutate_personal_projection(|projection| {
            if projection.cart.order_review == refreshed_order_review {
                return false;
            }

            projection.cart.order_review = refreshed_order_review;
            true
        }))
    }

    fn place_personal_order(&mut self) -> Result<bool, AppSqliteError> {
        let buyer_context = self.state_store.identity_projection().buyer_context();
        if matches!(buyer_context, BuyerContext::Guest) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order review requires a selected account",
            });
        }
        let (refreshed_cart, refreshed_order_review, refreshed_orders, order_detail, order_export) = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            let order_id = sqlite_store.place_buyer_order(&buyer_context)?;
            let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
            let refreshed_order_review = sqlite_store.load_buyer_order_review(&buyer_context)?;
            let refreshed_orders = sqlite_store.load_buyer_orders(&buyer_context)?;
            if !refreshed_orders
                .rows
                .iter()
                .any(|row| row.order_id == order_id)
            {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "buyer order write did not surface in buyer order history",
                });
            }
            let Some(order_detail) =
                sqlite_store.load_buyer_order_detail(&buyer_context, order_id)?
            else {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "buyer order write did not surface in buyer order detail",
                });
            };
            let Some(order_export) =
                sqlite_store.load_buyer_order_local_event_export(&buyer_context, order_id)?
            else {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "buyer order write did not surface in buyer order local event export",
                });
            };
            (
                refreshed_cart,
                refreshed_order_review,
                refreshed_orders,
                order_detail,
                order_export,
            )
        };
        let personal_changed = self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.order_review != refreshed_order_review {
                projection.cart.order_review = refreshed_order_review.clone();
                changed = true;
            }
            if projection.orders.list != refreshed_orders {
                projection.orders.list = refreshed_orders.clone();
                changed = true;
            }
            if projection.orders.detail.as_ref() != Some(&order_detail) {
                projection.orders.detail = Some(order_detail.clone());
                changed = true;
            }
            if !projection.orders.has_recoverable_coordination {
                projection.orders.has_recoverable_coordination = true;
                changed = true;
            }

            changed
        });
        let section_changed = self.select_personal_section(PersonalSection::Orders)?;
        let order_local_work = {
            let sqlite_store =
                self.sqlite_store
                    .as_ref()
                    .ok_or_else(|| AppSqliteError::InvalidProjection {
                        reason: "sqlite store became unavailable during buyer order placement",
                    })?;
            self.append_app_buyer_order_request_local_work_record(
                sqlite_store,
                &buyer_context,
                &order_export,
            )?
        };
        let pending_changed = if matches!(buyer_context, BuyerContext::Account(_)) {
            self.enqueue_selected_account_order_sync_operation(
                &buyer_context,
                &order_export,
                order_local_work.as_ref(),
            )?
        } else {
            false
        };
        let coordination_changed =
            self.refresh_personal_orders_coordination_retry_state(&buyer_context)?;

        Ok(personal_changed || section_changed || pending_changed || coordination_changed)
    }

    fn retry_pending_personal_order_coordination(&mut self) -> Result<bool, AppSqliteError> {
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let records = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store.load_recoverable_buyer_order_coordination_records(&buyer_context)?
        };
        if records.is_empty() {
            return self.refresh_personal_orders_coordination_retry_state(&buyer_context);
        }
        let mut changed = false;
        let mut refreshed_order_id = None;

        for record in records {
            let order_export = {
                let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                    return Ok(changed);
                };
                let Some(order_export) = sqlite_store
                    .load_buyer_order_local_event_export(&buyer_context, record.order_id)?
                else {
                    changed |= sqlite_store.mark_buyer_order_coordination_failed(
                        &buyer_context,
                        record.order_id,
                        "buyer order local event export is unavailable",
                    )?;
                    continue;
                };
                order_export
            };

            let order_local_work = {
                let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                    return Ok(changed);
                };
                self.append_app_buyer_order_request_local_work_record(
                    sqlite_store,
                    &buyer_context,
                    &order_export,
                )?
            };
            if let Some(order_local_work) = order_local_work.as_ref()
                && matches!(buyer_context, BuyerContext::Account(_))
            {
                let _ = self.enqueue_selected_account_order_sync_operation(
                    &buyer_context,
                    &order_export,
                    Some(order_local_work),
                )?;
            }
            if order_local_work.is_some() {
                refreshed_order_id.get_or_insert(record.order_id);
                changed = true;
            }
        }

        if changed {
            changed |=
                self.refresh_personal_orders_projection(&buyer_context, refreshed_order_id)?;
        }

        Ok(changed)
    }

    fn refresh_personal_orders_coordination_retry_state(
        &mut self,
        buyer_context: &BuyerContext,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let has_recoverable_coordination = !sqlite_store
            .load_recoverable_buyer_order_coordination_records(buyer_context)?
            .is_empty();
        Ok(self.mutate_personal_projection(|projection| {
            if projection.orders.has_recoverable_coordination == has_recoverable_coordination {
                false
            } else {
                projection.orders.has_recoverable_coordination = has_recoverable_coordination;
                true
            }
        }))
    }

    fn refresh_personal_orders_projection(
        &mut self,
        buyer_context: &BuyerContext,
        preferred_order_id: Option<OrderId>,
    ) -> Result<bool, AppSqliteError> {
        let current_detail_order_id = self
            .state_store
            .personal_projection()
            .orders
            .detail
            .as_ref()
            .map(|detail| detail.order_id);
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());
        let (
            refreshed_cart,
            refreshed_order_review,
            refreshed_orders,
            refreshed_order_detail,
            has_recoverable_coordination,
        ) = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            let refreshed_cart = sqlite_store.load_buyer_cart(buyer_context)?;
            let refreshed_order_review = sqlite_store.load_buyer_order_review(buyer_context)?;
            let refreshed_orders = sqlite_store.load_buyer_orders_for_scope(&buyer_order_scope)?;
            let has_recoverable_coordination = !sqlite_store
                .load_recoverable_buyer_order_coordination_records(buyer_context)?
                .is_empty();
            let detail_order_id = current_detail_order_id
                .filter(|order_id| {
                    refreshed_orders
                        .rows
                        .iter()
                        .any(|row| row.order_id == *order_id)
                })
                .or_else(|| {
                    preferred_order_id.filter(|order_id| {
                        refreshed_orders
                            .rows
                            .iter()
                            .any(|row| row.order_id == *order_id)
                    })
                });
            let refreshed_order_detail = match detail_order_id {
                Some(order_id) => {
                    sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
                }
                None => None,
            };
            (
                refreshed_cart,
                refreshed_order_review,
                refreshed_orders,
                refreshed_order_detail,
                has_recoverable_coordination,
            )
        };

        Ok(self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.order_review != refreshed_order_review {
                projection.cart.order_review = refreshed_order_review.clone();
                changed = true;
            }
            if projection.orders.list != refreshed_orders {
                projection.orders.list = refreshed_orders.clone();
                changed = true;
            }
            if projection.orders.detail != refreshed_order_detail {
                projection.orders.detail = refreshed_order_detail.clone();
                changed = true;
            }
            if projection.orders.has_recoverable_coordination != has_recoverable_coordination {
                projection.orders.has_recoverable_coordination = has_recoverable_coordination;
                changed = true;
            }

            changed
        }))
    }

    fn open_personal_order_detail(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());
        let Some(order_detail) =
            sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
        else {
            return Ok(false);
        };

        let detail_changed = self.set_personal_order_detail(Some(order_detail));
        let section_changed = self.select_personal_section(PersonalSection::Orders)?;

        Ok(detail_changed || section_changed)
    }

    fn repeat_personal_order(
        &mut self,
        order_id: OrderId,
        replace_existing: bool,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());

        match sqlite_store.apply_buyer_repeat_demand_from_scope_to_cart(
            &buyer_order_scope,
            &buyer_context,
            order_id,
            replace_existing,
        )? {
            BuyerRepeatDemandApplyOutcome::Applied => {
                let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
                let refreshed_order_review =
                    sqlite_store.load_buyer_order_review(&buyer_context)?;
                let refreshed_orders =
                    sqlite_store.load_buyer_orders_for_scope(&buyer_order_scope)?;
                let refreshed_detail =
                    sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?;
                let personal_changed = self.mutate_personal_projection(|projection| {
                    let mut changed = false;
                    if projection.cart.cart != refreshed_cart {
                        projection.cart.cart = refreshed_cart.clone();
                        changed = true;
                    }
                    if projection.cart.order_review != refreshed_order_review {
                        projection.cart.order_review = refreshed_order_review.clone();
                        changed = true;
                    }
                    if projection.orders.list != refreshed_orders {
                        projection.orders.list = refreshed_orders.clone();
                        changed = true;
                    }
                    if projection.orders.detail != refreshed_detail {
                        projection.orders.detail = refreshed_detail.clone();
                        changed = true;
                    }

                    changed
                });
                let section_changed = self.select_personal_section(PersonalSection::Cart)?;

                Ok(personal_changed || section_changed)
            }
            BuyerRepeatDemandApplyOutcome::ConfirmationRequired(replace_confirmation) => Ok(self
                .mutate_personal_projection(|projection| {
                    let cart = &mut projection.cart.cart;
                    if cart.replace_confirmation.as_ref() == Some(&replace_confirmation) {
                        return false;
                    }

                    cart.replace_confirmation = Some(replace_confirmation);
                    true
                })),
            BuyerRepeatDemandApplyOutcome::Unavailable => Ok(false),
        }
    }

    fn set_personal_search_query(&mut self, search_query: &str) -> Result<bool, AppSqliteError> {
        let query = self.state_store.personal_projection().search.query.clone();
        if query.search_query == search_query {
            return self.replace_personal_search_query(query);
        }

        self.replace_personal_search_query(BuyerSearchScreenQueryState::new(
            search_query,
            query.fulfillment_methods,
        ))
    }

    fn set_personal_search_fulfillment_method(
        &mut self,
        method: FarmOrderMethod,
        enabled: bool,
    ) -> Result<bool, AppSqliteError> {
        let mut query = self.state_store.personal_projection().search.query.clone();
        let changed = if enabled {
            query.fulfillment_methods.insert(method)
        } else {
            query.fulfillment_methods.remove(&method)
        };

        if !changed {
            return Ok(false);
        }

        self.replace_personal_search_query(query)
    }

    fn set_products_search_query(&mut self, search_query: &str) -> Result<bool, AppSqliteError> {
        let query = self.state_store.products_projection().query.clone();
        if query.search_query == search_query {
            return Ok(false);
        }

        self.replace_products_query(ProductsScreenQueryState::new(
            search_query,
            query.filter,
            query.sort,
        ))
    }

    fn select_products_filter(&mut self, filter: ProductsFilter) -> Result<bool, AppSqliteError> {
        let query = self.state_store.products_projection().query.clone();
        if query.filter == filter {
            return Ok(false);
        }

        self.replace_products_query(ProductsScreenQueryState::new(
            query.search_query,
            filter,
            query.sort,
        ))
    }

    fn select_products_sort(&mut self, sort: ProductsSort) -> Result<bool, AppSqliteError> {
        let query = self.state_store.products_projection().query.clone();
        if query.sort == sort {
            return Ok(false);
        }

        self.replace_products_query(ProductsScreenQueryState::new(
            query.search_query,
            query.filter,
            sort,
        ))
    }

    fn open_products_filter(&mut self, filter: ProductsFilter) -> Result<bool, AppSqliteError> {
        if !self.state_store.farm_setup_projection().has_saved_farm() {
            return Ok(false);
        }

        let filter_changed = self.select_products_filter(filter)?;
        let section_changed = self.select_farmer_section(FarmerSection::Products);

        Ok(filter_changed || section_changed)
    }

    fn select_orders_filter(&mut self, filter: OrdersFilter) -> Result<bool, AppSqliteError> {
        if !self.has_saved_farm() {
            return Ok(false);
        }

        let query = self.state_store.orders_projection().query.clone();
        if query.filter == filter {
            return Ok(false);
        }

        self.replace_orders_query(OrdersScreenQueryState {
            filter,
            fulfillment_window_id: query.fulfillment_window_id,
        })
    }

    fn open_orders(&mut self) -> Result<bool, AppSqliteError> {
        if !self.has_saved_farm() {
            return Ok(false);
        }

        self.open_orders_query(OrdersScreenQueryState::default())
    }

    fn open_orders_fulfillment_window(
        &mut self,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<bool, AppSqliteError> {
        if !self.has_saved_farm() {
            return Ok(false);
        }

        self.open_orders_query(OrdersScreenQueryState {
            filter: OrdersFilter::All,
            fulfillment_window_id: Some(fulfillment_window_id),
        })
    }

    fn open_orders_query(&mut self, query: OrdersScreenQueryState) -> Result<bool, AppSqliteError> {
        let query_changed = self.replace_orders_query(query)?;
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                FarmerSection::Orders,
            )));
        let editor_changed = self.close_product_editor();

        Ok(query_changed || section_changed || editor_changed)
    }

    fn open_order_detail(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };
        let Some(_) = sqlite_store.load_order_detail(farm_id, order_id)? else {
            return Ok(false);
        };
        let continuity_state = self.continuity_state_with_order_detail(Some(order_id));
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let detail_changed = self.apply_selected_account_context(&selected_account_context);
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                FarmerSection::Orders,
            )));
        let editor_changed = self.close_product_editor();

        Ok(detail_changed || section_changed || editor_changed)
    }

    fn prepare_seller_order_decision(
        &mut self,
        order_id: OrderId,
        command: AppSellerOrderDecisionCommand,
    ) -> Result<AppOrderDecisionPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "seller order decision requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires local state",
            });
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires a selected farm",
            });
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires a selected seller account",
            });
        };
        let account_id = selected_account.account.account_id.clone();
        let seller_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "seller order decision requires a selected seller public key",
            },
        )?;
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.seller_pubkey.trim() != seller_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision seller account does not match order seller",
            });
        }
        let listing_address =
            radroots_sdk::trade::parse_listing_address(request.payload.listing_addr.as_str())
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "seller order decision listing address is invalid",
                })?;
        if listing_address.seller_pubkey != seller_pubkey {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision listing address is outside seller authority",
            });
        }
        let Some(order_export) =
            sqlite_store.load_seller_order_decision_export(farm_id, order_id)?
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires a visible seller order",
            });
        };
        if order_export.status != OrderStatus::NeedsAction {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires an undecided order",
            });
        }

        let decision = match command {
            AppSellerOrderDecisionCommand::Accept => AppOrderDecisionPayload::Accepted {
                inventory_commitments: seller_order_inventory_commitments(&order_export)?,
            },
            AppSellerOrderDecisionCommand::Decline { reason } => {
                let reason = reason.trim();
                if reason.is_empty() {
                    return Err(AppSqliteError::InvalidProjection {
                        reason: "seller order decline requires a non-empty reason",
                    });
                }
                AppOrderDecisionPayload::Declined {
                    reason: reason.to_owned(),
                }
            }
        };
        let payload = AppOrderDecisionPublishPayload {
            context: AppPublishContext::new(account_id, "seller_order_decision"),
            app_order_id: order_id,
            farm_id,
            trade_order_id: request.payload.order_id.clone(),
            request_event_id: request.request_event_id,
            listing_event_id: request.listing_event_id,
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            decision,
        };
        AppPublishPayload::OrderDecision(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "seller order decision publish payload is invalid",
            })?;

        Ok(payload)
    }

    fn refresh_configured_relay_state_before_order_lifecycle(
        &mut self,
    ) -> Result<(), AppSqliteError> {
        match self.ingest_configured_relay_events() {
            Ok(report) => {
                if report.freshness_changed
                    || report.local_import.imported_records > 0
                    || report.local_import.skipped_records > 0
                {
                    let _ = self.refresh_selected_account_context_after_local_events()?;
                }
                Ok(())
            }
            Err(AppDirectRelayIngestError::Sqlite(error)) => Err(error),
            Err(AppDirectRelayIngestError::Transport(_)) => {
                Err(AppSqliteError::InvalidProjection {
                    reason: "order lifecycle publish requires fresh configured relay state",
                })
            }
        }
    }

    fn publish_seller_order_decision(
        &mut self,
        order_id: OrderId,
        command: AppSellerOrderDecisionCommand,
    ) -> Result<bool, AppSqliteError> {
        let payload = self.prepare_seller_order_decision(order_id, command)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderDecision(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "seller order decision publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn prepare_seller_order_fulfillment(
        &mut self,
        order_id: OrderId,
        status: RadrootsActiveTradeFulfillmentState,
    ) -> Result<AppOrderFulfillmentPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires local state",
            });
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires a selected farm",
            });
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires a selected seller account",
            });
        };
        let account_id = selected_account.account.account_id.clone();
        let seller_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires a selected seller public key",
            },
        )?;
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.seller_pubkey.trim() != seller_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment seller account does not match order seller",
            });
        }
        let lifecycle = self.resolve_order_lifecycle_evidence(&request)?;
        let Some(decision) = lifecycle.decision.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires accepted order decision evidence",
            });
        };
        if !matches!(
            decision.payload.decision,
            RadrootsTradeOrderDecision::Accepted { .. }
        ) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires accepted order decision evidence",
            });
        }
        if lifecycle.cancellation_event_id.is_some() || lifecycle.receipt_event_id.is_some() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires an active order",
            });
        }
        if lifecycle
            .latest_fulfillment
            .as_ref()
            .is_some_and(|fulfillment| {
                matches!(
                    fulfillment.status,
                    RadrootsActiveTradeFulfillmentState::Delivered
                        | RadrootsActiveTradeFulfillmentState::SellerCancelled
                )
            })
        {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment is already terminal",
            });
        }
        if sqlite_store.load_order_detail(farm_id, order_id)?.is_none() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment requires a visible seller order",
            });
        };
        if !status.is_publishable_update() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment status must be publishable",
            });
        }
        let prev_event_id = match lifecycle.latest_fulfillment.as_ref() {
            Some(fulfillment) => fulfillment.event_id.clone(),
            None => active_order_current_parent_event_id(
                &lifecycle,
                "seller order fulfillment requires current lifecycle parent evidence",
            )?,
        };
        let payload = AppOrderFulfillmentPublishPayload {
            context: AppPublishContext::new(account_id, "seller_order_fulfillment"),
            app_order_id: order_id,
            farm_id,
            trade_order_id: request.payload.order_id,
            request_event_id: request.request_event_id,
            prev_event_id,
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            status,
        };
        AppPublishPayload::OrderFulfillment(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "seller order fulfillment publish payload is invalid",
            })?;
        Ok(payload)
    }

    fn publish_seller_order_fulfillment(
        &mut self,
        order_id: OrderId,
        status: RadrootsActiveTradeFulfillmentState,
    ) -> Result<bool, AppSqliteError> {
        let payload = self.prepare_seller_order_fulfillment(order_id, status)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderFulfillment(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "seller order fulfillment publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn prepare_seller_order_revision_proposal(
        &mut self,
        order_id: OrderId,
        items: Vec<RadrootsTradeOrderItem>,
        economics: RadrootsTradeOrderEconomics,
        reason: &str,
    ) -> Result<AppOrderRevisionProposalPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "seller order revision requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires local state",
            });
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a selected farm",
            });
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a selected seller account",
            });
        };
        let account_id = selected_account.account.account_id.clone();
        let seller_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a selected seller public key",
            },
        )?;
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.seller_pubkey.trim() != seller_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision seller account does not match order seller",
            });
        }
        let listing_address =
            radroots_sdk::trade::parse_listing_address(request.payload.listing_addr.as_str())
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "seller order revision listing address is invalid",
                })?;
        if listing_address.seller_pubkey != seller_pubkey {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision listing address is outside seller authority",
            });
        }
        let lifecycle = self.resolve_order_lifecycle_evidence(&request)?;
        let Some(decision) = lifecycle.decision.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires accepted order decision evidence",
            });
        };
        if !matches!(
            decision.payload.decision,
            RadrootsTradeOrderDecision::Accepted { .. }
        ) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires accepted order decision evidence",
            });
        }
        if active_order_payment_blocks_lifecycle_write(&lifecycle) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires no recorded or settled payment",
            });
        }
        if lifecycle.cancellation_event_id.is_some()
            || lifecycle.receipt_event_id.is_some()
            || lifecycle.latest_fulfillment.is_some()
        {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires an unfulfilled active order",
            });
        }
        let Some(order_detail) = sqlite_store.load_order_detail(farm_id, order_id)? else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a visible seller order",
            });
        };
        if order_detail.status != OrderStatus::Scheduled {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a scheduled order",
            });
        }
        let Some(prev_event_id) = active_order_revision_parent_event_id(&lifecycle) else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires no pending revision proposal",
            });
        };
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order revision requires a non-empty reason",
            });
        }
        let payload = AppOrderRevisionProposalPublishPayload {
            context: AppPublishContext::new(account_id, "seller_order_revision_proposal"),
            app_order_id: order_id,
            farm_id,
            trade_order_id: request.payload.order_id,
            request_event_id: request.request_event_id,
            prev_event_id,
            revision_id: format!("app-revision-{}", d_tag_from_uuid(Uuid::now_v7())),
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            items,
            economics,
            reason: reason.to_owned(),
        };
        AppPublishPayload::OrderRevisionProposal(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "seller order revision publish payload is invalid",
            })?;
        Ok(payload)
    }

    fn publish_seller_order_revision_proposal(
        &mut self,
        order_id: OrderId,
        items: Vec<RadrootsTradeOrderItem>,
        economics: RadrootsTradeOrderEconomics,
        reason: &str,
    ) -> Result<bool, AppSqliteError> {
        let payload =
            self.prepare_seller_order_revision_proposal(order_id, items, economics, reason)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderRevisionProposal(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "seller order revision publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn prepare_buyer_order_revision_decision(
        &mut self,
        order_id: OrderId,
        decision: RadrootsTradeOrderRevisionDecision,
    ) -> Result<AppOrderRevisionDecisionPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let BuyerContext::Account(account_id) = &buyer_context else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a selected buyer account",
            });
        };
        let Some(selected_account) = self.selected_buyer_account(&buyer_context) else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a selected buyer account",
            });
        };
        let buyer_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a selected buyer public key",
            },
        )?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires local state",
            });
        };
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());
        let Some(detail) =
            sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a visible buyer order",
            });
        };
        if detail.status != BuyerOrderStatus::Scheduled {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a scheduled order",
            });
        }
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.buyer_pubkey.trim() != buyer_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision buyer account does not match order buyer",
            });
        }
        let lifecycle = self.resolve_order_lifecycle_evidence(&request)?;
        let Some(order_decision) = lifecycle.decision.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires accepted order decision evidence",
            });
        };
        if !matches!(
            order_decision.payload.decision,
            RadrootsTradeOrderDecision::Accepted { .. }
        ) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires accepted order decision evidence",
            });
        }
        if lifecycle.cancellation_event_id.is_some()
            || lifecycle.receipt_event_id.is_some()
            || lifecycle.latest_fulfillment.is_some()
        {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires an unfulfilled active order",
            });
        }
        let Some(proposal) = active_order_pending_revision_proposal(&lifecycle) else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order revision requires a pending seller proposal",
            });
        };
        let payload = AppOrderRevisionDecisionPublishPayload {
            context: AppPublishContext::new(account_id.clone(), "buyer_order_revision_decision"),
            app_order_id: order_id,
            farm_id: detail.farm_id,
            trade_order_id: request.payload.order_id,
            request_event_id: request.request_event_id,
            prev_event_id: proposal.event_id.clone(),
            revision_id: proposal.payload.revision_id.clone(),
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            decision,
        };
        AppPublishPayload::OrderRevisionDecision(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "buyer order revision publish payload is invalid",
            })?;
        Ok(payload)
    }

    fn publish_buyer_order_revision_accept(
        &mut self,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.publish_buyer_order_revision_decision(
            order_id,
            RadrootsTradeOrderRevisionDecision::Accepted,
        )
    }

    fn publish_buyer_order_revision_decline(
        &mut self,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.publish_buyer_order_revision_decision(
            order_id,
            RadrootsTradeOrderRevisionDecision::Declined {
                reason: "buyer kept order as placed".to_owned(),
            },
        )
    }

    fn publish_buyer_order_revision_decision(
        &mut self,
        order_id: OrderId,
        decision: RadrootsTradeOrderRevisionDecision,
    ) -> Result<bool, AppSqliteError> {
        let payload = self.prepare_buyer_order_revision_decision(order_id, decision)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderRevisionDecision(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "buyer order revision publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn prepare_buyer_order_cancellation(
        &mut self,
        order_id: OrderId,
    ) -> Result<AppOrderCancellationPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let BuyerContext::Account(account_id) = &buyer_context else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires a selected buyer account",
            });
        };
        let Some(selected_account) = self.selected_buyer_account(&buyer_context) else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires a selected buyer account",
            });
        };
        let buyer_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires a selected buyer public key",
            },
        )?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires local state",
            });
        };
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());
        let Some(detail) =
            sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires a visible buyer order",
            });
        };
        if !matches!(
            detail.status,
            BuyerOrderStatus::Placed | BuyerOrderStatus::Scheduled
        ) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires an open order",
            });
        }
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.buyer_pubkey.trim() != buyer_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation buyer account does not match order buyer",
            });
        }
        let lifecycle = self.resolve_order_lifecycle_evidence(&request)?;
        if lifecycle.cancellation_event_id.is_some()
            || lifecycle.receipt_event_id.is_some()
            || lifecycle.latest_fulfillment.is_some()
        {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires an unfulfilled order",
            });
        }
        if active_order_payment_blocks_lifecycle_write(&lifecycle) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires no recorded or settled payment",
            });
        }
        let prev_event_id = match lifecycle.status {
            RadrootsActiveOrderStatus::Requested => request.request_event_id.clone(),
            RadrootsActiveOrderStatus::Accepted => active_order_current_parent_event_id(
                &lifecycle,
                "buyer order cancellation requires order decision evidence",
            )?,
            RadrootsActiveOrderStatus::Missing
            | RadrootsActiveOrderStatus::Declined
            | RadrootsActiveOrderStatus::Cancelled
            | RadrootsActiveOrderStatus::Completed
            | RadrootsActiveOrderStatus::Disputed
            | RadrootsActiveOrderStatus::Invalid => {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "buyer order cancellation requires an open order",
                });
            }
        };
        let payload = AppOrderCancellationPublishPayload {
            context: AppPublishContext::new(account_id.clone(), "buyer_order_cancellation"),
            app_order_id: order_id,
            farm_id: detail.farm_id,
            trade_order_id: request.payload.order_id,
            request_event_id: request.request_event_id,
            prev_event_id,
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            reason: "buyer cancelled order".to_owned(),
        };
        AppPublishPayload::OrderCancellation(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation publish payload is invalid",
            })?;
        Ok(payload)
    }

    fn publish_buyer_order_cancellation(
        &mut self,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        let payload = self.prepare_buyer_order_cancellation(order_id)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderCancellation(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "buyer order cancellation publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn prepare_buyer_order_receipt(
        &mut self,
        order_id: OrderId,
        outcome: AppOrderReceiptOutcome,
    ) -> Result<AppOrderReceiptPublishPayload, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let relay_urls = normalized_app_sync_relay_urls(&self.nostr_relay_urls).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires valid configured relays",
            }
        })?;
        if relay_urls.is_empty() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires configured relays",
            });
        }
        self.refresh_configured_relay_state_before_order_lifecycle()?;
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let BuyerContext::Account(account_id) = &buyer_context else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires a selected buyer account",
            });
        };
        let Some(selected_account) = self.selected_buyer_account(&buyer_context) else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires a selected buyer account",
            });
        };
        let buyer_pubkey = self.local_events_owner_pubkey(selected_account).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires a selected buyer public key",
            },
        )?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires local state",
            });
        };
        let buyer_order_scope = selected_buyer_order_scope(self.state_store.identity_projection());
        let Some(detail) =
            sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires a visible buyer order",
            });
        };
        let request = self.resolve_seller_order_request_evidence(order_id)?;
        if request.payload.buyer_pubkey.trim() != buyer_pubkey.as_str() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt buyer account does not match order buyer",
            });
        }
        let lifecycle = self.resolve_order_lifecycle_evidence(&request)?;
        if lifecycle.cancellation_event_id.is_some() || lifecycle.receipt_event_id.is_some() {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires an active ready order",
            });
        }
        let fulfillment =
            lifecycle
                .latest_fulfillment
                .as_ref()
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer order receipt requires fulfillment evidence",
                })?;
        if !matches!(
            fulfillment.status,
            RadrootsActiveTradeFulfillmentState::ReadyForPickup
                | RadrootsActiveTradeFulfillmentState::Delivered
        ) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order receipt requires ready fulfillment evidence",
            });
        }
        let received_at = u64::try_from(current_runtime_time_seconds()?).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "buyer order receipt timestamp must be non-negative",
            }
        })?;
        let received = outcome.received();
        let payload = AppOrderReceiptPublishPayload {
            context: AppPublishContext::new(account_id.clone(), "buyer_order_receipt"),
            app_order_id: order_id,
            farm_id: detail.farm_id,
            trade_order_id: request.payload.order_id,
            request_event_id: request.request_event_id,
            prev_event_id: fulfillment.event_id.clone(),
            listing_addr: request.payload.listing_addr,
            buyer_pubkey: request.payload.buyer_pubkey,
            seller_pubkey: request.payload.seller_pubkey,
            received,
            issue: outcome.issue_text(),
            received_at,
        };
        AppPublishPayload::OrderReceipt(payload.clone())
            .validate()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "buyer order receipt publish payload is invalid",
            })?;
        Ok(payload)
    }

    fn publish_buyer_order_receipt(
        &mut self,
        order_id: OrderId,
        outcome: AppOrderReceiptOutcome,
    ) -> Result<bool, AppSqliteError> {
        let payload = self.prepare_buyer_order_receipt(order_id, outcome)?;
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderReceipt(payload),
            current_utc_timestamp(),
        )
        .map_err(|_| AppSqliteError::InvalidProjection {
            reason: "buyer order receipt publish payload must serialize",
        })?;
        let _ = self.enqueue_selected_account_sync_operation_once(operation)?;
        self.attempt_sync(SyncTrigger::ManualRefresh)
    }

    fn start_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.upsert_order_recovery(order_id, kind, RecoveryState::Open, "start_order_recovery")
    }

    fn review_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.upsert_order_recovery(
            order_id,
            kind,
            RecoveryState::InReview,
            "review_order_recovery",
        )
    }

    fn reopen_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.upsert_order_recovery(order_id, kind, RecoveryState::Open, "reopen_order_recovery")
    }

    fn resolve_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<bool, AppSqliteError> {
        self.upsert_order_recovery(
            order_id,
            kind,
            RecoveryState::Resolved,
            "resolve_order_recovery",
        )
    }

    fn upsert_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        state: RecoveryState,
        source: &str,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };
        let Some(_) = sqlite_store.load_order_detail(farm_id, order_id)? else {
            return Ok(false);
        };

        let account_id = selected_account.account.account_id.as_str();
        let last_updated_at = current_utc_timestamp();
        let summary = order_recovery_summary(kind, state).to_owned();
        let note = Some(order_recovery_note(kind, state).to_owned());
        let mut record = sqlite_store
            .load_recovery_record(account_id, order_id, kind)?
            .unwrap_or(OrderRecoveryProjection {
                recovery_record_id: RecoveryRecordId::new(),
                order_id,
                kind,
                state,
                summary: summary.clone(),
                note: note.clone(),
                last_updated_at: last_updated_at.clone(),
            });

        if record.state == state && record.summary == summary && record.note == note {
            return Ok(false);
        }

        record.state = state;
        record.summary = summary;
        record.note = note;
        record.last_updated_at = last_updated_at;
        sqlite_store.save_recovery_record(account_id, farm_id, &record)?;

        let continuity_state = self
            .continuity_state_with_order_detail(self.selected_order_detail_id().or(Some(order_id)));
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Order(order_id),
                order_recovery_sync_payload(order_id, farm_id, kind, state, source),
            )])?;

        Ok(context_changed || pending_changed)
    }

    fn open_pack_day(
        &mut self,
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Result<bool, AppSqliteError> {
        if !self.has_saved_farm() {
            return Ok(false);
        }

        let query = PackDayScreenQueryState {
            fulfillment_window_id,
        };
        let query_changed = self.replace_pack_day_query(query)?;
        if !self.has_pack_day_context() {
            return Ok(false);
        }
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                FarmerSection::PackDay,
            )));
        let editor_changed = self.close_product_editor();

        Ok(query_changed || section_changed || editor_changed)
    }

    fn export_pack_day(&mut self) -> Result<bool, DesktopAppRuntimeCommandError> {
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };
        let previous_export_instance_id = self.current_pack_day_export_instance_id();
        let Some(fulfillment_window_id) = self
            .state_store
            .pack_day_projection()
            .projection
            .fulfillment_window
            .as_ref()
            .map(|window| window.fulfillment_window_id)
        else {
            return Ok(false);
        };
        let Some(data_root) = self.runtime_metadata.data_root.clone() else {
            return Err(self.command_unavailable_error());
        };

        let source = {
            let sqlite_store = self.sqlite_store()?;
            sqlite_store.load_pack_day_output_source(farm_id, fulfillment_window_id)?
        };
        let Some(source) = source else {
            return Ok(false);
        };
        if source.is_empty() {
            return Ok(false);
        }

        let request = PackDayExportRequest::for_fulfillment_window(
            source.fulfillment_window.fulfillment_window_id,
        );
        let _ = self
            .state_store
            .apply_in_memory(AppStateCommand::begin_pack_day_export(request.clone()));
        self.cleanup_prepared_pack_day_print_assets_if_export_changed(
            previous_export_instance_id,
            "export_reset",
        );
        let prepared =
            prepare_pack_day_export_bundle_at_data_root(data_root.as_path(), &source, Utc::now());

        match write_prepared_pack_day_export_bundle(&prepared) {
            Ok(()) => {
                let _ = self
                    .state_store
                    .apply_in_memory(AppStateCommand::succeed_pack_day_export(
                        request,
                        prepared.bundle,
                    ));
                Ok(true)
            }
            Err(error) => {
                let _ = self
                    .state_store
                    .apply_in_memory(AppStateCommand::fail_pack_day_export(
                        request,
                        error.to_string(),
                    ));
                Err(error.into())
            }
        }
    }

    fn prepare_pack_day_host_handoff(
        &mut self,
        kind: PackDayHostHandoffKind,
    ) -> Result<
        Option<(PackDayHostHandoffRequest, PackDayHostHandoffCommandPlan)>,
        DesktopAppRuntimeCommandError,
    > {
        if self.state_store.pack_day_projection().host_handoff.status
            == PackDayHostHandoffStatus::Running
            || self.state_store.pack_day_projection().print.status == PackDayPrintStatus::Running
            || self.state_store.pack_day_projection().batch_print.status
                == PackDayBatchPrintStatus::Running
        {
            return Ok(None);
        }

        let Some(bundle) = self.current_pack_day_export_bundle() else {
            return Ok(None);
        };
        let request = PackDayHostHandoffRequest::for_bundle(kind, &bundle);
        let _ = self
            .state_store
            .apply_in_memory(AppStateCommand::begin_pack_day_host_handoff(
                request.clone(),
            ));

        match plan_pack_day_host_handoff(&bundle, kind) {
            Ok(plan) => Ok(Some((request, plan))),
            Err(error) => {
                let _ =
                    self.state_store
                        .apply_in_memory(AppStateCommand::fail_pack_day_host_handoff(
                            request,
                            error.to_string(),
                        ));
                Err(error.into())
            }
        }
    }

    fn prepare_pack_day_print(
        &mut self,
        kind: PackDayPrintKind,
    ) -> Result<Option<(PackDayPrintRequest, PackDayPrintCommandPlan)>, DesktopAppRuntimeCommandError>
    {
        if self.state_store.pack_day_projection().print.status == PackDayPrintStatus::Running
            || self.state_store.pack_day_projection().host_handoff.status
                == PackDayHostHandoffStatus::Running
            || self.state_store.pack_day_projection().batch_print.status
                == PackDayBatchPrintStatus::Running
        {
            return Ok(None);
        }

        let Some(bundle) = self.current_pack_day_export_bundle() else {
            return Ok(None);
        };
        let request = PackDayPrintRequest::for_bundle(kind, &bundle);
        let _ = self
            .state_store
            .apply_in_memory(AppStateCommand::begin_pack_day_print(request.clone()));

        match plan_pack_day_print(&bundle, kind) {
            Ok(plan) => Ok(Some((request, plan))),
            Err(error) => {
                let failure_command = match error.failure_kind() {
                    Some(failure) => {
                        AppStateCommand::fail_pack_day_print_with_kind(request, failure)
                    }
                    None => AppStateCommand::fail_pack_day_print(request),
                };
                let _ = self.state_store.apply_in_memory(failure_command);
                Err(error.into())
            }
        }
    }

    fn prepare_pack_day_batch_print(
        &mut self,
    ) -> Result<
        Option<(PackDayBatchPrintRequest, PackDayBatchPrintCommandPlan)>,
        DesktopAppRuntimeCommandError,
    > {
        if self.state_store.pack_day_projection().batch_print.status
            == PackDayBatchPrintStatus::Running
            || self.state_store.pack_day_projection().print.status == PackDayPrintStatus::Running
            || self.state_store.pack_day_projection().host_handoff.status
                == PackDayHostHandoffStatus::Running
        {
            return Ok(None);
        }

        let Some(bundle) = self.current_pack_day_export_bundle() else {
            return Ok(None);
        };
        let request = PackDayBatchPrintRequest::for_bundle(&bundle);
        let _ = self
            .state_store
            .apply_in_memory(AppStateCommand::begin_pack_day_batch_print(request.clone()));

        match plan_pack_day_batch_print(&bundle, &request) {
            Ok(plan) => Ok(Some((request, plan))),
            Err(error) => {
                let _ =
                    self.state_store
                        .apply_in_memory(AppStateCommand::fail_pack_day_batch_print(
                            request,
                            error.failed_artifact(),
                            error.failure_kind(),
                        ));
                Err(error.into())
            }
        }
    }

    fn finish_pack_day_batch_print(
        &mut self,
        request: PackDayBatchPrintRequest,
        result: Result<(), PackDayBatchPrintError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        if !self.current_pack_day_batch_print_request_matches(&request) {
            return Ok(false);
        }

        let cleanup_export_instance_id = request.export_instance_id;

        match result {
            Ok(()) => {
                let changed = self
                    .state_store
                    .apply_in_memory(AppStateCommand::succeed_pack_day_batch_print(request));
                self.cleanup_prepared_pack_day_print_assets_for_export_instance(
                    cleanup_export_instance_id,
                    "batch_print_completion",
                );
                Ok(changed)
            }
            Err(error) => {
                let _ =
                    self.state_store
                        .apply_in_memory(AppStateCommand::fail_pack_day_batch_print(
                            request,
                            error.failed_artifact(),
                            error.failure_kind(),
                        ));
                self.cleanup_prepared_pack_day_print_assets_for_export_instance(
                    cleanup_export_instance_id,
                    "batch_print_completion",
                );
                Err(error.into())
            }
        }
    }

    fn finish_pack_day_print(
        &mut self,
        request: PackDayPrintRequest,
        result: Result<(), PackDayPrintError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        if !self.current_pack_day_print_request_matches(&request) {
            return Ok(false);
        }

        let cleanup_export_instance_id = (request.kind == PackDayPrintKind::PrintCustomerLabels)
            .then_some(request.export_instance_id);

        match result {
            Ok(()) => {
                let changed = self
                    .state_store
                    .apply_in_memory(AppStateCommand::succeed_pack_day_print(request));
                if let Some(export_instance_id) = cleanup_export_instance_id {
                    self.cleanup_prepared_pack_day_print_assets_for_export_instance(
                        export_instance_id,
                        "print_completion",
                    );
                }
                Ok(changed)
            }
            Err(error) => {
                let failure_command = match error.failure_kind() {
                    Some(failure) => {
                        AppStateCommand::fail_pack_day_print_with_kind(request, failure)
                    }
                    None => AppStateCommand::fail_pack_day_print(request),
                };
                let _ = self.state_store.apply_in_memory(failure_command);
                if let Some(export_instance_id) = cleanup_export_instance_id {
                    self.cleanup_prepared_pack_day_print_assets_for_export_instance(
                        export_instance_id,
                        "print_completion",
                    );
                }
                Err(error.into())
            }
        }
    }

    fn finish_pack_day_host_handoff(
        &mut self,
        request: PackDayHostHandoffRequest,
        result: Result<(), PackDayHostHandoffError>,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        if !self.current_pack_day_host_handoff_request_matches(&request) {
            return Ok(false);
        }

        match result {
            Ok(()) => Ok(self
                .state_store
                .apply_in_memory(AppStateCommand::succeed_pack_day_host_handoff(request))),
            Err(error) => {
                let _ =
                    self.state_store
                        .apply_in_memory(AppStateCommand::fail_pack_day_host_handoff(
                            request,
                            error.to_string(),
                        ));
                Err(error.into())
            }
        }
    }

    fn update_product_stock(
        &mut self,
        product_id: ProductId,
        stock_quantity: u32,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(_) = self.selected_farm_id() else {
            return Ok(false);
        };
        if self
            .state_store
            .identity_projection()
            .selected_account
            .is_none()
        {
            return Ok(false);
        }

        let updated = sqlite_store.update_product_stock(product_id, stock_quantity)?;
        if !updated {
            return Ok(false);
        }

        let continuity_state =
            self.continuity_state_with_order_detail(self.selected_order_detail_id());
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let publish_changed = self.enqueue_selected_account_product_publish_operation(
            product_id,
            "update_product_stock",
            None,
        )?;

        Ok(updated || context_changed || publish_changed)
    }

    fn open_new_product_editor(&mut self) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };

        let product_id = sqlite_store.create_product_draft(farm_id)?;
        let Some(draft) = sqlite_store.load_product_editor_draft(product_id)? else {
            return Ok(false);
        };
        let continuity_state = self.continuity_state();
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let section_changed = self.select_farmer_section(FarmerSection::Products);
        let editor_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::open_existing_product_editor(
                    product_id, draft,
                ));

        Ok(context_changed || section_changed || editor_changed)
    }

    fn open_existing_product_editor(
        &mut self,
        product_id: ProductId,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(draft) = sqlite_store.load_product_editor_draft(product_id)? else {
            return Ok(false);
        };
        let section_changed = self.select_farmer_section(FarmerSection::Products);
        let editor_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::open_existing_product_editor(
                    product_id, draft,
                ));

        Ok(section_changed || editor_changed)
    }

    fn save_product_editor_draft(
        &mut self,
        draft: ProductEditorDraft,
    ) -> Result<bool, AppSqliteError> {
        let Some(product_id) = self.selected_product_editor_id() else {
            return Ok(false);
        };

        let saved = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store.save_product_editor_draft(product_id, &draft)?
        };
        if !saved {
            return Ok(false);
        }

        let selected_account_context = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            let continuity_state = self.continuity_state();
            load_selected_account_context(
                sqlite_store,
                self.state_store.identity_projection(),
                &continuity_state,
            )?
        };
        let reloaded_draft = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store
                .load_product_editor_draft(product_id)?
                .unwrap_or(draft)
        };
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let draft_payload = reloaded_draft.clone();
        let editor_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_product_editor_draft(
                    reloaded_draft,
                ));
        let source_local_event_id =
            self.append_app_listing_local_work_record(product_id, &draft_payload)?;
        let pending_changed = self.enqueue_selected_account_product_publish_operation(
            product_id,
            "save_product_editor_draft",
            source_local_event_id.as_deref(),
        )?;

        Ok(saved
            || context_changed
            || editor_changed
            || source_local_event_id.is_some()
            || pending_changed)
    }

    fn close_product_editor(&mut self) -> bool {
        self.state_store
            .apply_in_memory(AppStateCommand::close_product_editor())
    }

    fn save_farm_setup_draft(
        &mut self,
        draft: FarmSetupDraft,
    ) -> Result<FarmSetupProjection, DesktopAppRuntimeFarmSetupError> {
        let account_id = self.selected_account_id()?;
        let sqlite_store = self.sqlite_store_for_farm_setup()?;
        let projection = FarmSetupProjection::from_draft(draft);
        sqlite_store.save_farm_setup(account_id.as_str(), &projection)?;

        let selected_account_context = self.refresh_selected_account_context()?;
        self.apply_selected_account_context(&selected_account_context);

        Ok(selected_account_context.farm_setup_projection)
    }

    fn finish_farm_setup(
        &mut self,
    ) -> Result<FarmSetupProjection, DesktopAppRuntimeFarmSetupError> {
        let account = self.selected_account_for_farm_setup()?.clone();
        let sqlite_store = self.sqlite_store_for_farm_setup()?;
        let draft = self.state_store.farm_setup_projection().draft.clone();

        if !draft.blockers().is_empty() {
            return Err(DesktopAppRuntimeFarmSetupError::IncompleteDraft);
        }

        let saved_farm = FarmSummary {
            farm_id: account
                .farmer_activation
                .farm_id
                .unwrap_or_else(FarmId::new),
            display_name: draft.farm_name.trim().to_owned(),
            readiness: FarmReadiness::Incomplete,
        };
        let projection = FarmSetupProjection::new(draft, Some(saved_farm.clone()));

        sqlite_store.save_farm_summary(&saved_farm)?;
        sqlite_store.save_farm_setup(account.account.account_id.as_str(), &projection)?;
        let source_local_event_id =
            self.append_app_farm_local_work_record(&account, &projection, &saved_farm)?;

        let selected_account_context = self.refresh_selected_account_context()?;
        self.apply_selected_account_context(&selected_account_context);
        let _ = self.enqueue_selected_account_farm_publish_operation(
            saved_farm.farm_id,
            saved_farm.display_name.as_str(),
            Some(saved_farm.readiness),
            "finish_farm_setup",
            source_local_event_id.as_deref(),
        )?;

        Ok(selected_account_context.farm_setup_projection)
    }

    fn load_farm_rules_projection(
        &self,
    ) -> Result<FarmRulesProjection, DesktopAppRuntimeFarmRulesError> {
        let farm_id = self
            .selected_farm_id()
            .ok_or(DesktopAppRuntimeFarmRulesError::FarmRequired)?;
        let fallback_profile = self.fallback_farm_profile(farm_id);
        let projection = self
            .sqlite_store_for_farm_rules()?
            .load_farm_rules(farm_id)
            .map(|projection| {
                prepare_loaded_farm_rules_projection(projection, &fallback_profile)
            })?;

        Ok(projection)
    }

    fn save_farm_rules_projection(
        &mut self,
        projection: FarmRulesProjection,
    ) -> Result<FarmRulesProjection, DesktopAppRuntimeFarmRulesError> {
        let account_id = self.selected_account_id_for_farm_rules()?;
        let farm_id = self
            .selected_farm_id()
            .ok_or(DesktopAppRuntimeFarmRulesError::FarmRequired)?;
        let fallback_profile = self.fallback_farm_profile(farm_id);
        let normalized = normalize_farm_rules_projection(projection, &fallback_profile);
        let saved_projection = {
            let sqlite_store = self.sqlite_store_for_farm_rules()?;
            sqlite_store.save_farm_rules(&normalized)?;

            let mut refreshed = sqlite_store.load_farm_rules(farm_id)?;
            refreshed = prepare_loaded_farm_rules_projection(refreshed, &fallback_profile);

            let saved_farm = FarmSummary {
                farm_id,
                display_name: refreshed
                    .farm_profile
                    .as_ref()
                    .map(|profile| profile.display_name.clone())
                    .unwrap_or_default(),
                readiness: if refreshed.is_ready() {
                    FarmReadiness::Ready
                } else {
                    FarmReadiness::Incomplete
                },
            };
            let mut farm_setup_projection = self.state_store.farm_setup_projection().clone();
            farm_setup_projection.draft.farm_name = saved_farm.display_name.clone();
            farm_setup_projection.saved_farm = Some(saved_farm.clone());

            sqlite_store.save_farm_summary(&saved_farm)?;
            sqlite_store.save_farm_setup(account_id.as_str(), &farm_setup_projection)?;

            refreshed
        };

        let selected_account_context = {
            let sqlite_store = self.sqlite_store_for_farm_rules()?;
            let continuity_state = self.continuity_state();
            load_selected_account_context(
                sqlite_store,
                self.state_store.identity_projection(),
                &continuity_state,
            )?
        };
        self.apply_selected_account_context(&selected_account_context);
        let display_name = saved_projection
            .farm_profile
            .as_ref()
            .map(|profile| profile.display_name.as_str())
            .unwrap_or_default();
        let readiness = if saved_projection.is_ready() {
            FarmReadiness::Ready
        } else {
            FarmReadiness::Incomplete
        };
        let _ = self.enqueue_selected_account_farm_publish_operation(
            farm_id,
            display_name,
            Some(readiness),
            "save_farm_rules_projection",
            None,
        )?;

        Ok(saved_projection)
    }

    fn replace_identity_projection(
        &mut self,
        projection: AppIdentityProjection,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = self.decorate_identity_projection(projection)?;
        let _ = self.import_shared_local_events()?;
        let continuity_state = self.continuity_state();
        let selected_account_context =
            load_selected_account_context(self.sqlite_store()?, &projection, &continuity_state)?;
        let selected_account_sync_context = load_selected_account_sync_context(
            self.sqlite_store()?,
            &projection,
            &self.nostr_relay_urls,
        )?;
        let identity_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_identity_projection(projection));
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let sync_changed = self.apply_selected_account_sync_context(&selected_account_sync_context);
        let editor_changed = self.close_product_editor();

        Ok(identity_changed || context_changed || sync_changed || editor_changed)
    }

    fn refresh_selected_account_context(
        &self,
    ) -> Result<DesktopSelectedAccountContext, DesktopAppRuntimeFarmSetupError> {
        let _ = self.import_shared_local_events()?;
        let continuity_state = self.continuity_state();
        Ok(load_selected_account_context(
            self.sqlite_store_for_farm_setup()?,
            self.state_store.identity_projection(),
            &continuity_state,
        )?)
    }

    fn apply_selected_account_context(&mut self, context: &DesktopSelectedAccountContext) -> bool {
        self.apply_selected_account_context_with_options(context, true)
    }

    fn apply_selected_account_seller_context(
        &mut self,
        context: &DesktopSelectedAccountContext,
    ) -> bool {
        self.apply_selected_account_context_with_options(context, false)
    }

    fn apply_selected_account_context_with_options(
        &mut self,
        context: &DesktopSelectedAccountContext,
        include_personal: bool,
    ) -> bool {
        let previous_export_instance_id = self.current_pack_day_export_instance_id();
        let personal_changed = if include_personal {
            self.state_store
                .apply_in_memory(AppStateCommand::replace_personal_projection(
                    context.personal_projection.clone(),
                ))
        } else {
            false
        };
        let farm_setup_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_farm_setup_projection(
                    context.farm_setup_projection.clone(),
                ));
        let farm_rules_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_farm_rules_projection(
                    context.farm_rules_projection.clone(),
                ));
        let today_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_today_agenda(
                    context.today_projection.clone(),
                ));
        let products_query_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::set_products_search_query(
                    context.products_query.search_query.clone(),
                ))
                || self
                    .state_store
                    .apply_in_memory(AppStateCommand::select_products_filter(
                        context.products_query.filter,
                    ))
                || self
                    .state_store
                    .apply_in_memory(AppStateCommand::select_products_sort(
                        context.products_query.sort,
                    ));
        let products_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_products_list(
                    context.products_list.clone(),
                ));
        let orders_query_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::select_orders_filter(
                    context.orders_query.filter,
                ))
                || self.state_store.apply_in_memory(
                    AppStateCommand::select_orders_fulfillment_window(
                        context.orders_query.fulfillment_window_id,
                    ),
                );
        let orders_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_orders_list(
                    context.orders_list.clone(),
                ));
        let orders_reminders_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_orders_reminders(
                    context.orders_reminders.clone(),
                ));
        let recovery_queue_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_orders_recovery_queue(
                    context.recovery_queue.clone(),
                ));
        let reminder_log_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_reminder_log(
                    context.reminder_log.clone(),
                ));
        let order_detail_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_order_detail(
                    context.order_detail.clone(),
                ));
        let pack_day_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_pack_day_projection(
                    context.pack_day_projection.clone(),
                ));
        let pack_day_query_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::set_pack_day_fulfillment_window(
                    context.pack_day_query.fulfillment_window_id,
                ));
        let editor_changed =
            if let Some((product_id, draft)) = context.product_editor_draft.as_ref() {
                self.state_store
                    .apply_in_memory(AppStateCommand::open_existing_product_editor(
                        *product_id,
                        draft.clone(),
                    ))
            } else {
                self.close_product_editor()
            };
        let shell_changed = self.sync_truthful_farmer_section();
        self.cleanup_prepared_pack_day_print_assets_if_export_changed(
            previous_export_instance_id,
            "context_refresh",
        );

        personal_changed
            || farm_setup_changed
            || farm_rules_changed
            || today_changed
            || products_query_changed
            || products_changed
            || orders_query_changed
            || orders_changed
            || orders_reminders_changed
            || recovery_queue_changed
            || reminder_log_changed
            || order_detail_changed
            || pack_day_query_changed
            || pack_day_changed
            || editor_changed
            || shell_changed
    }

    fn refresh_selected_account_sync_context(
        &self,
    ) -> Result<DesktopSelectedAccountSyncContext, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(DesktopSelectedAccountSyncContext::default());
        };

        load_selected_account_sync_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &self.nostr_relay_urls,
        )
    }

    fn apply_selected_account_sync_context(
        &mut self,
        context: &DesktopSelectedAccountSyncContext,
    ) -> bool {
        let projection_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_sync_projection(
                    context.projection.clone(),
                ));
        let pending_changed =
            self.selected_account_pending_sync_write_count != context.pending_write_count;
        let relay_ingest_changed =
            self.selected_account_relay_ingest_freshness != context.relay_ingest;
        let conflicts_changed = self.selected_account_sync_conflicts != context.conflicts;

        self.selected_account_pending_sync_write_count = context.pending_write_count;
        self.selected_account_relay_ingest_freshness = context.relay_ingest.clone();
        self.selected_account_sync_conflicts = context.conflicts.clone();

        projection_changed || pending_changed || relay_ingest_changed || conflicts_changed
    }

    fn refresh_selected_account_sync(&mut self) -> Result<bool, AppSqliteError> {
        let context = self.refresh_selected_account_sync_context()?;
        let sync_changed = self.apply_selected_account_sync_context(&context);
        let selected_account_changed = match self.sqlite_store.as_ref() {
            Some(sqlite_store) => {
                let continuity_state = self.continuity_state();
                let selected_account_context = load_selected_account_context(
                    sqlite_store,
                    self.state_store.identity_projection(),
                    &continuity_state,
                )?;
                self.apply_selected_account_seller_context(&selected_account_context)
            }
            None => false,
        };

        Ok(sync_changed || selected_account_changed)
    }

    fn resolve_sync_conflict(
        &mut self,
        conflict_id: &str,
        resolution: radroots_studio_app_sync::SyncConflictResolutionStatus,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(false);
        };
        let account_id = selected_account.account.account_id.as_str();
        let stored_conflicts = sqlite_store.load_sync_conflicts(account_id)?;
        let Some(stored_conflict) = stored_conflicts
            .iter()
            .find(|stored| stored.conflict_id == conflict_id)
        else {
            return Ok(false);
        };
        if !stored_conflict.conflict.is_unresolved() {
            return Ok(false);
        }
        if matches!(
            (stored_conflict.conflict.severity, resolution,),
            (
                SyncConflictSeverity::Blocking,
                radroots_studio_app_sync::SyncConflictResolutionStatus::Dismissed,
            )
        ) {
            return Ok(false);
        }

        if !sqlite_store.resolve_sync_conflict(
            account_id,
            conflict_id,
            resolution,
            current_utc_timestamp().as_str(),
        )? {
            return Ok(false);
        }

        self.refresh_selected_account_sync()
    }

    fn acknowledge_reminder(&mut self, reminder_id: ReminderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };
        let account_id = selected_account.account.account_id.clone();
        let mut schedule = sqlite_store.load_reminder_schedule(account_id.as_str(), farm_id)?;
        let Some(reminder) = schedule
            .items
            .iter_mut()
            .find(|item| item.reminder_id == reminder_id)
        else {
            return Ok(false);
        };
        if matches!(
            reminder.delivery_state,
            ReminderDeliveryState::Acknowledged | ReminderDeliveryState::Resolved
        ) {
            return Ok(false);
        }

        reminder.delivery_state = ReminderDeliveryState::Acknowledged;
        let reminder_log_entry =
            build_reminder_log_entry(reminder, ReminderDeliveryState::Acknowledged);
        sqlite_store.apply_reminder_schedule_update(
            account_id.as_str(),
            farm_id,
            &schedule,
            &[reminder_log_entry],
        )?;

        let continuity_state = self.continuity_state();
        let selected_account_context = load_selected_account_context_with_options(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
            false,
        )?;

        let _ = self.apply_selected_account_context(&selected_account_context);

        Ok(true)
    }

    fn attempt_sync(&mut self, trigger: SyncTrigger) -> Result<bool, AppSqliteError> {
        let Some(prepared) = self.prepare_sync_request(trigger)? else {
            return Ok(false);
        };

        let started_at = current_utc_timestamp();
        let syncing_checkpoint = SyncCheckpointStatus::syncing(
            started_at.clone(),
            prepared.checkpoint.last_remote_cursor.clone(),
        );
        {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store.save_sync_checkpoint(prepared.account_id.as_str(), &syncing_checkpoint)?;
        }

        let mut changed = self.refresh_selected_account_sync()?;
        let request = AppSyncRequest {
            trigger,
            checkpoint: prepared.checkpoint.clone(),
            pending_operations: prepared
                .pending_operations
                .iter()
                .map(|stored| stored.operation.clone())
                .collect(),
            known_conflicts: prepared
                .conflicts
                .iter()
                .map(|stored| stored.conflict.clone())
                .collect(),
        };

        match self.run_sync_transport_or_relay_only(request, started_at.as_str()) {
            Ok(mut result) => {
                let relay_context_changed =
                    self.ingest_configured_relay_events_for_sync(Some(&mut result), &started_at)?;
                changed |= self.apply_sync_result(
                    prepared.account_id.as_str(),
                    &prepared.pending_operations,
                    &result,
                )?;
                if relay_context_changed {
                    changed |= self.refresh_selected_account_context_after_local_events()?;
                }
            }
            Err(error) => {
                changed |= self.apply_sync_transport_error(
                    prepared.account_id.as_str(),
                    &prepared.checkpoint,
                    &prepared.pending_operations,
                    started_at.as_str(),
                    error,
                )?;
                let relay_context_changed =
                    self.ingest_configured_relay_events_for_sync(None, &started_at)?;
                if relay_context_changed {
                    changed |= self.refresh_selected_account_sync()?;
                    changed |= self.refresh_selected_account_context_after_local_events()?;
                }
            }
        }

        Ok(changed)
    }

    fn run_sync_transport_or_relay_only(
        &mut self,
        request: AppSyncRequest,
        started_at: &str,
    ) -> Result<AppSyncResult, AppSyncTransportError> {
        if request.pending_operations.is_empty()
            && self.has_configured_relay_ingest()
            && !self.sync_transport.supports_empty_sync_request()
        {
            return Ok(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    Some(started_at.to_owned()),
                    current_utc_timestamp(),
                    request.checkpoint.last_remote_cursor.clone(),
                ),
                pushed_operation_count: 0,
                pulled_record_count: 0,
                conflicts: request.known_conflicts,
                published_receipts: Vec::new(),
            });
        }

        self.sync_transport.sync(request)
    }

    fn has_configured_relay_ingest(&self) -> bool {
        self.nostr_relay_urls
            .iter()
            .any(|relay_url| !relay_url.trim().is_empty())
    }

    fn ingest_configured_relay_events(
        &self,
    ) -> Result<AppDirectRelayIngestReport, AppDirectRelayIngestError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(AppDirectRelayIngestReport::default());
        };
        let relay_urls = normalized_app_relay_ingest_urls(&self.nostr_relay_urls)?;
        if relay_urls.is_empty() {
            return Ok(AppDirectRelayIngestReport::default());
        }
        let started_at = current_utc_timestamp();
        let started_unix_seconds = current_runtime_time_seconds()?;
        let cursors = sqlite_store
            .load_relay_ingest_cursors(APP_DIRECT_RELAY_INGEST_SCOPE_KEY, &relay_urls)?;
        let receipt = fetch_app_events_from_relays_windowed(cursors.as_slice())?;
        let completed_at = current_utc_timestamp();
        let completed_unix_seconds = current_runtime_time_seconds()?;
        self.record_relay_ingest_freshness(
            &receipt,
            started_at.as_str(),
            started_unix_seconds,
            completed_at.as_str(),
            completed_unix_seconds,
        )?;
        if receipt.connected_relays.is_empty() {
            return Err(AppSyncTransportError::unavailable(format!(
                "direct relay app ingest connection failed: {}",
                summarize_app_relay_failures(&receipt.failed_relays)
            ))
            .into());
        }
        if receipt.events.is_empty() {
            return Ok(AppDirectRelayIngestReport {
                local_import: AppLocalInteropImportReport::default(),
                freshness_changed: true,
            });
        }
        let records = direct_relay_event_records(&receipt, current_runtime_time_ms()?)?;
        let local_import = sqlite_store
            .import_local_event_records(records.as_slice())
            .map_err(AppDirectRelayIngestError::from)?;
        Ok(AppDirectRelayIngestReport {
            local_import,
            freshness_changed: true,
        })
    }

    fn record_relay_ingest_freshness(
        &self,
        receipt: &AppDirectRelayFetchReceipt,
        started_at: &str,
        started_unix_seconds: i64,
        completed_at: &str,
        completed_unix_seconds: i64,
    ) -> Result<(), AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(());
        };
        for relay in &receipt.fetched_relays {
            let cursor_since_unix_seconds = relay
                .last_event_created_at_unix_seconds
                .map_or(started_unix_seconds, |last_event_created_at| {
                    started_unix_seconds.max(last_event_created_at)
                });
            sqlite_store.record_relay_ingest_success(
                APP_DIRECT_RELAY_INGEST_SCOPE_KEY,
                relay.relay_url.as_str(),
                cursor_since_unix_seconds,
                relay.last_event_created_at_unix_seconds,
                started_at,
                started_unix_seconds,
                completed_at,
                completed_unix_seconds,
            )?;
        }
        for failure in &receipt.failed_relays {
            sqlite_store.record_relay_ingest_failure(
                APP_DIRECT_RELAY_INGEST_SCOPE_KEY,
                failure.relay_url.as_str(),
                started_at,
                started_unix_seconds,
                completed_at,
                completed_unix_seconds,
                failure.error.as_str(),
            )?;
        }

        Ok(())
    }

    fn ingest_configured_relay_events_for_sync(
        &mut self,
        mut result: Option<&mut AppSyncResult>,
        started_at: &str,
    ) -> Result<bool, AppSqliteError> {
        if !self.has_configured_relay_ingest() {
            return Ok(false);
        }
        match self.ingest_configured_relay_events() {
            Ok(report) => {
                if let Some(result) = result.as_mut() {
                    result.pulled_record_count = result
                        .pulled_record_count
                        .saturating_add(report.local_import.scanned_records as usize);
                }
                Ok(report.freshness_changed
                    || report.local_import.imported_records > 0
                    || report.local_import.skipped_records > 0)
            }
            Err(AppDirectRelayIngestError::Sqlite(error)) => Err(error),
            Err(AppDirectRelayIngestError::Transport(error)) => {
                if let Some(result) = result.as_mut() {
                    result.run_status = AppSyncRunStatus::Failed;
                    result.checkpoint = SyncCheckpointStatus::failed(
                        Some(started_at.to_owned()),
                        Some(current_utc_timestamp()),
                        result.checkpoint.last_remote_cursor.clone(),
                        error.to_string(),
                    );
                }
                Ok(true)
            }
        }
    }

    fn prepare_sync_request(
        &self,
        trigger: SyncTrigger,
    ) -> Result<Option<DesktopPreparedSyncRequest>, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(None);
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(None);
        };

        let account_id = selected_account.account.account_id.clone();
        let checkpoint = sqlite_store.load_sync_checkpoint(account_id.as_str())?;
        let conflicts = sqlite_store.load_sync_conflicts(account_id.as_str())?;
        let pending_operations = sqlite_store.load_pending_sync_operations(account_id.as_str())?;

        if conflicts.iter().any(|stored| {
            stored.conflict.is_unresolved()
                && matches!(stored.conflict.severity, SyncConflictSeverity::Blocking)
        }) {
            return Ok(None);
        }

        if !matches!(trigger, SyncTrigger::ManualRefresh)
            && !self.has_configured_relay_ingest()
            && !self.has_sync_eligible_runtime_state(&checkpoint, &conflicts, &pending_operations)
        {
            return Ok(None);
        }

        Ok(Some(DesktopPreparedSyncRequest {
            account_id,
            checkpoint,
            conflicts,
            pending_operations,
        }))
    }

    fn has_sync_eligible_runtime_state(
        &self,
        checkpoint: &SyncCheckpointStatus,
        conflicts: &[StoredSyncConflict],
        pending_operations: &[StoredPendingSyncOperation],
    ) -> bool {
        !pending_operations.is_empty()
            || !conflicts.is_empty()
            || *checkpoint != SyncCheckpointStatus::never_synced()
            || self.selected_farm_id().is_some()
            || !self
                .state_store
                .personal_projection()
                .orders
                .list
                .rows
                .is_empty()
            || !self.state_store.orders_projection().list.rows.is_empty()
            || !self.state_store.products_projection().list.rows.is_empty()
    }

    fn apply_sync_result(
        &mut self,
        account_id: &str,
        pending_operations: &[StoredPendingSyncOperation],
        result: &AppSyncResult,
    ) -> Result<bool, AppSqliteError> {
        self.record_published_sync_receipts(result.published_receipts.as_slice())?;
        let receipt_import_changed = if result.published_receipts.is_empty() {
            false
        } else {
            let report = self.import_shared_local_events()?;
            report.imported_records > 0 || report.skipped_records > 0
        };
        {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store.save_sync_checkpoint(account_id, &result.checkpoint)?;
            sqlite_store.replace_sync_conflicts(account_id, &result.conflicts)?;

            for pending in pending_operations
                .iter()
                .take(result.pushed_operation_count)
            {
                let _ = sqlite_store
                    .dequeue_pending_sync_operation(account_id, pending.operation_id.as_str())?;
            }
            if result.run_status == AppSyncRunStatus::Failed {
                let retry_available_at = result
                    .checkpoint
                    .last_sync_completed_at
                    .clone()
                    .unwrap_or_else(current_utc_timestamp);
                let last_error_message = result.checkpoint.last_error_message.as_deref();
                for pending in pending_operations
                    .iter()
                    .skip(result.pushed_operation_count)
                {
                    let _ = sqlite_store.update_pending_sync_operation_retry(
                        account_id,
                        pending.operation_id.as_str(),
                        retry_available_at.as_str(),
                        pending.operation.attempt_count.saturating_add(1),
                        last_error_message,
                    )?;
                }
            }
        }

        let sync_changed = self.refresh_selected_account_sync()?;
        Ok(receipt_import_changed || sync_changed)
    }

    fn apply_sync_transport_error(
        &mut self,
        account_id: &str,
        previous_checkpoint: &SyncCheckpointStatus,
        pending_operations: &[StoredPendingSyncOperation],
        started_at: &str,
        error: AppSyncTransportError,
    ) -> Result<bool, AppSqliteError> {
        let error_message = error.to_string();
        let failed_checkpoint = SyncCheckpointStatus::failed(
            Some(started_at.to_owned()),
            previous_checkpoint.last_sync_completed_at.clone(),
            previous_checkpoint.last_remote_cursor.clone(),
            error_message.clone(),
        );
        {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            sqlite_store.save_sync_checkpoint(account_id, &failed_checkpoint)?;

            for pending in pending_operations {
                let _ = sqlite_store.update_pending_sync_operation_retry(
                    account_id,
                    pending.operation_id.as_str(),
                    started_at,
                    pending.operation.attempt_count.saturating_add(1),
                    Some(error_message.as_str()),
                )?;
            }
        }

        self.refresh_selected_account_sync()
    }

    fn enqueue_selected_account_sync_operations(
        &mut self,
        operations: Vec<PendingSyncOperation>,
    ) -> Result<bool, AppSqliteError> {
        if operations.is_empty() {
            return Ok(false);
        }

        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(false);
        };
        {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            for operation in &operations {
                let _ = sqlite_store.enqueue_pending_sync_operation(
                    selected_account.account.account_id.as_str(),
                    operation,
                )?;
            }
        }

        self.refresh_selected_account_sync()
    }

    fn enqueue_selected_account_order_sync_operation(
        &mut self,
        buyer_context: &BuyerContext,
        order: &BuyerOrderLocalEventExport,
        local_work: Option<&AppOrderLocalWorkPublishSource>,
    ) -> Result<bool, AppSqliteError> {
        let Some(operation) =
            self.order_request_publish_operation(buyer_context, order, local_work)?
        else {
            return self.refresh_selected_account_sync();
        };

        self.enqueue_selected_account_sync_operation_once(operation)
    }

    fn enqueue_selected_account_farm_publish_operation(
        &mut self,
        farm_id: FarmId,
        display_name: &str,
        readiness: Option<FarmReadiness>,
        source: &str,
        source_local_event_id: Option<&str>,
    ) -> Result<bool, AppSqliteError> {
        let existing_source_local_event_id = if source_local_event_id.is_none() {
            self.selected_account_pending_farm_source_local_event_id(farm_id)?
        } else {
            None
        };
        let source_local_event_id =
            source_local_event_id.or(existing_source_local_event_id.as_deref());

        let Some(operation) = self.farm_profile_publish_operation(
            farm_id,
            display_name,
            readiness,
            source,
            source_local_event_id,
        )?
        else {
            return self.refresh_selected_account_sync();
        };

        self.enqueue_selected_account_sync_operations(vec![operation])
    }

    fn selected_account_pending_farm_source_local_event_id(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<String>, AppSqliteError> {
        let Some(account_id) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            return Ok(None);
        };
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(None);
        };
        let existing = sqlite_store.load_pending_sync_operations(account_id.as_str())?;
        existing
            .iter()
            .find(|pending| {
                pending.operation.aggregate == SyncAggregateRef::Farm(farm_id)
                    && pending.operation.operation == SyncOperationKind::Upsert
            })
            .map(|pending| {
                pending
                    .operation
                    .publish_payload()
                    .map_err(|_| AppSqliteError::InvalidProjection {
                        reason: "farm profile publish payload must parse",
                    })
            })
            .transpose()
            .map(|payload| {
                payload.and_then(|payload| match payload {
                    AppPublishPayload::FarmProfile(payload) => {
                        payload.context.source_local_event_id
                    }
                    _ => None,
                })
            })
    }

    fn enqueue_selected_account_sync_operation_once(
        &mut self,
        operation: PendingSyncOperation,
    ) -> Result<bool, AppSqliteError> {
        let Some(account_id) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            return Ok(false);
        };
        let already_enqueued = {
            let Some(sqlite_store) = self.sqlite_store.as_ref() else {
                return Ok(false);
            };
            let existing = sqlite_store.load_pending_sync_operations(account_id.as_str())?;
            existing.iter().any(|pending| {
                pending.operation.aggregate == operation.aggregate
                    && pending.operation.operation == operation.operation
                    && pending.operation.payload_json == operation.payload_json
            })
        };
        if already_enqueued {
            return self.refresh_selected_account_sync();
        }

        self.enqueue_selected_account_sync_operations(vec![operation])
    }

    fn enqueue_selected_account_product_publish_operation(
        &mut self,
        product_id: ProductId,
        source: &str,
        source_local_event_id: Option<&str>,
    ) -> Result<bool, AppSqliteError> {
        let Some(operation) =
            self.product_publish_operation(product_id, source, source_local_event_id)?
        else {
            return self.refresh_selected_account_sync();
        };

        self.enqueue_selected_account_sync_operations(vec![operation])
    }

    fn farm_profile_publish_operation(
        &self,
        farm_id: FarmId,
        display_name: &str,
        readiness: Option<FarmReadiness>,
        source: &str,
        source_local_event_id: Option<&str>,
    ) -> Result<Option<PendingSyncOperation>, AppSqliteError> {
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(None);
        };
        let mut context = AppPublishContext::new(
            selected_account.account.account_id.clone(),
            source.to_owned(),
        );
        if let Some(source_local_event_id) = source_local_event_id {
            context = context.with_source_local_event_id(source_local_event_id.to_owned());
        }
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context,
            farm_id,
            display_name: display_name.trim().to_owned(),
            readiness,
        });
        if payload.validate().is_err() {
            return Ok(None);
        }

        PendingSyncOperation::from_publish_payload(payload, current_utc_timestamp())
            .map(Some)
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "farm profile publish payload must serialize",
            })
    }

    fn product_publish_operation(
        &self,
        product_id: ProductId,
        source: &str,
        source_local_event_id: Option<&str>,
    ) -> Result<Option<PendingSyncOperation>, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(None);
        };
        let Some(selected_account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(None);
        };
        let Some(draft) = sqlite_store.load_product_editor_draft(product_id)? else {
            return Ok(None);
        };
        if !product_status_needs_relay_publish(draft.status) {
            return Ok(None);
        }
        let farm_rules = self.state_store.farm_rules_projection();
        if !derive_product_publish_blockers(
            &draft,
            self.state_store.farm_readiness_projection(),
            farm_rules,
        )
        .is_empty()
        {
            return Ok(None);
        }
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(None);
        };
        let Some(farm_pubkey) = self.local_events_owner_pubkey(selected_account) else {
            return Ok(None);
        };
        let farm_setup = self.state_store.farm_setup_projection();
        let (availability_starts_at, availability_ends_at) =
            listing_availability_window_times(&draft, farm_rules);
        let listing_d_tag = d_tag_from_uuid(product_id.as_uuid());
        let mut context = AppPublishContext::new(
            selected_account.account.account_id.clone(),
            source.to_owned(),
        );
        if let Some(source_local_event_id) = source_local_event_id {
            context = context.with_source_local_event_id(source_local_event_id.to_owned());
        }
        let payload = AppPublishPayload::Listing(AppListingPublishPayload {
            context,
            product_id,
            listing_d_tag: Some(listing_d_tag),
            farm_id: Some(farm_id),
            farm_pubkey: Some(farm_pubkey),
            farm_d_tag: Some(d_tag_from_uuid(farm_id.as_uuid())),
            title: draft.title.trim().to_owned(),
            subtitle: non_empty_string(draft.subtitle.as_str()),
            category: non_empty_string(draft.category.as_str()),
            unit_label: draft.unit_label.trim().to_owned(),
            price_minor_units: draft.price_minor_units,
            price_currency: draft.price_currency.trim().to_uppercase(),
            stock_quantity: draft.stock_quantity,
            availability_window_id: draft.availability_window_id,
            availability_starts_at,
            availability_ends_at,
            fulfillment_method: listing_fulfillment_method(&draft, farm_setup, farm_rules),
            fulfillment_location: listing_fulfillment_location(&draft, farm_setup, farm_rules),
            status: draft.status,
        });
        if payload.validate().is_err() {
            return Ok(None);
        }

        PendingSyncOperation::from_publish_payload(payload, current_utc_timestamp())
            .map(Some)
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "product publish payload must serialize",
            })
    }

    fn order_request_publish_operation(
        &self,
        buyer_context: &BuyerContext,
        order: &BuyerOrderLocalEventExport,
        local_work: Option<&AppOrderLocalWorkPublishSource>,
    ) -> Result<Option<PendingSyncOperation>, AppSqliteError> {
        let Some(local_work) = local_work else {
            return Ok(None);
        };
        let Some(buyer_account) = self.selected_buyer_account(buyer_context) else {
            return Ok(None);
        };
        let buyer_pubkey = self.local_events_owner_pubkey(buyer_account);
        let export = AppBuyerOrderRequestExport::from_order(order, buyer_pubkey.as_deref())?;
        if !export.is_supported() {
            return Ok(None);
        }
        let Some((currency_code, total_minor_units)) = order_currency_and_total(order)? else {
            return Ok(None);
        };
        let context = AppPublishContext::new(
            buyer_account.account.account_id.clone(),
            "place_personal_order",
        )
        .with_source_local_event_id(local_work.record_id.clone());
        let payload = AppPublishPayload::OrderRequest(AppOrderRequestPublishPayload {
            context,
            order_id: order.order_id,
            farm_id: order.farm_id,
            status: Some(order.status.clone()),
            order_document_json: Some(local_work.payload.clone()),
            listing_addr: export.listing_addr,
            listing_event_id: export.listing_event_id,
            listing_relays: export.listing_relays,
            buyer_pubkey: export.buyer_pubkey,
            seller_pubkey: export.seller_pubkey,
            items: order
                .lines
                .iter()
                .map(|line| AppOrderRequestItemPayload {
                    product_id: line.product_id,
                    quantity: line.quantity,
                })
                .collect(),
            currency_code: Some(currency_code),
            total_minor_units: Some(total_minor_units),
            note: non_empty_string(order.buyer_order_note.as_str()),
        });
        if payload.validate().is_err() {
            return Ok(None);
        }

        PendingSyncOperation::from_publish_payload(payload, current_utc_timestamp())
            .map(Some)
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "order request publish payload must serialize",
            })
    }

    fn selected_account_id(&self) -> Result<String, DesktopAppRuntimeFarmSetupError> {
        self.selected_account_for_farm_setup()
            .map(|account| account.account.account_id.clone())
    }

    fn selected_account_id_for_farm_rules(
        &self,
    ) -> Result<String, DesktopAppRuntimeFarmRulesError> {
        self.selected_account_for_farm_rules()
            .map(|account| account.account.account_id.clone())
    }

    fn selected_account_for_farm_setup(
        &self,
    ) -> Result<&radroots_studio_app_view::SelectedAccountProjection, DesktopAppRuntimeFarmSetupError>
    {
        self.state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .ok_or(DesktopAppRuntimeFarmSetupError::AccountRequired)
    }

    fn selected_account_for_farm_rules(
        &self,
    ) -> Result<&radroots_studio_app_view::SelectedAccountProjection, DesktopAppRuntimeFarmRulesError>
    {
        self.state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .ok_or(DesktopAppRuntimeFarmRulesError::AccountRequired)
    }

    fn accounts_manager(
        &self,
    ) -> Result<&RadrootsNostrAccountsManager, DesktopAppRuntimeCommandError> {
        self.accounts_manager
            .as_ref()
            .ok_or_else(|| self.command_unavailable_error())
    }

    fn sqlite_store(&self) -> Result<&AppSqliteStore, DesktopAppRuntimeCommandError> {
        self.sqlite_store
            .as_ref()
            .ok_or(DesktopAppRuntimeCommandError::RuntimeUnavailable)
    }

    fn sqlite_store_for_farm_setup(
        &self,
    ) -> Result<&AppSqliteStore, DesktopAppRuntimeFarmSetupError> {
        self.sqlite_store
            .as_ref()
            .ok_or(DesktopAppRuntimeFarmSetupError::RuntimeUnavailable)
    }

    fn sqlite_store_for_farm_rules(
        &self,
    ) -> Result<&AppSqliteStore, DesktopAppRuntimeFarmRulesError> {
        self.sqlite_store
            .as_ref()
            .ok_or(DesktopAppRuntimeFarmRulesError::RuntimeUnavailable)
    }

    fn mutate_personal_projection(
        &mut self,
        mutator: impl FnOnce(&mut PersonalWorkspaceProjection) -> bool,
    ) -> bool {
        let mut projection = self.state_store.personal_projection().clone();
        if !mutator(&mut projection) {
            return false;
        }

        self.state_store
            .apply_in_memory(AppStateCommand::replace_personal_projection(projection))
    }

    fn set_personal_product_detail(
        &mut self,
        section: PersonalSection,
        detail: Option<BuyerProductDetailProjection>,
    ) -> bool {
        self.mutate_personal_projection(|projection| {
            let current_detail = match section {
                PersonalSection::Browse => &mut projection.browse.detail,
                PersonalSection::Search => &mut projection.search.detail,
                PersonalSection::Cart | PersonalSection::Orders => return false,
            };
            if *current_detail == detail {
                return false;
            }

            *current_detail = detail;
            true
        })
    }

    fn set_personal_order_detail(&mut self, detail: Option<BuyerOrderDetailProjection>) -> bool {
        self.mutate_personal_projection(|projection| {
            if projection.orders.detail == detail {
                return false;
            }

            projection.orders.detail = detail;
            true
        })
    }

    fn replace_personal_search_query(
        &mut self,
        query: BuyerSearchScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let search_listings = self.load_personal_listings_for_query(&query)?;
        let mut personal_projection = self.state_store.personal_projection().clone();

        if personal_projection.search.query == query
            && personal_projection.search.listings == search_listings
        {
            return Ok(false);
        }

        personal_projection.search.query = query;
        personal_projection.search.listings = search_listings;

        Ok(self
            .state_store
            .apply_in_memory(AppStateCommand::replace_personal_projection(
                personal_projection,
            )))
    }

    fn load_personal_listings_for_query(
        &self,
        query: &BuyerSearchScreenQueryState,
    ) -> Result<radroots_studio_app_view::BuyerListingsProjection, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(Default::default());
        };

        sqlite_store.load_buyer_listings(&query.search_query, &query.fulfillment_methods)
    }

    fn refresh_personal_cart_and_order_review(
        &mut self,
        refreshed_cart: BuyerCartProjection,
        refreshed_order_review: radroots_studio_app_view::BuyerOrderReviewProjection,
    ) -> bool {
        self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.order_review != refreshed_order_review {
                projection.cart.order_review = refreshed_order_review.clone();
                changed = true;
            }

            changed
        })
    }

    fn replace_products_query(
        &mut self,
        query: ProductsScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let products_list = self.load_products_list_for_query(&query)?;
        let query_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::set_products_search_query(
                    query.search_query.clone(),
                ));
        let filter_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::select_products_filter(query.filter));
        let sort_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::select_products_sort(query.sort));
        let list_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_products_list(products_list));

        Ok(query_changed || filter_changed || sort_changed || list_changed)
    }

    fn selected_farm_id(&self) -> Option<FarmId> {
        selected_farm_id_from_context(
            self.state_store.identity_projection(),
            self.state_store.farm_setup_projection(),
        )
    }

    fn has_saved_farm(&self) -> bool {
        self.state_store.farm_setup_projection().has_saved_farm()
    }

    fn has_pack_day_context(&self) -> bool {
        self.state_store
            .pack_day_projection()
            .projection
            .fulfillment_window
            .is_some()
    }

    fn selected_product_editor_id(&self) -> Option<ProductId> {
        match &self.state_store.products_projection().editor {
            radroots_studio_app_state::ProductEditorState::Open(session) => session.selected_product_id,
            radroots_studio_app_state::ProductEditorState::Closed => None,
        }
    }

    fn selected_order_detail_id(&self) -> Option<OrderId> {
        self.state_store
            .orders_projection()
            .detail
            .as_ref()
            .map(|detail| detail.order_id)
    }

    fn continuity_state(&self) -> PersistedAppState {
        self.state_store.persisted_state().clone()
    }

    fn continuity_state_with_order_detail(&self, order_id: Option<OrderId>) -> PersistedAppState {
        let mut state = self.continuity_state();
        state.seller.order_detail_order_id = order_id;
        state
    }

    fn continuity_state_with_orders_query(
        &self,
        query: OrdersScreenQueryState,
        order_id: Option<OrderId>,
    ) -> PersistedAppState {
        let mut state = self.continuity_state();
        state.seller.orders_query = query;
        state.seller.order_detail_order_id = order_id;
        state
    }

    fn continuity_state_with_pack_day_query(
        &self,
        query: PackDayScreenQueryState,
    ) -> PersistedAppState {
        let mut state = self.continuity_state();
        state.seller.pack_day_query = query;
        state
    }

    fn fallback_farm_profile(&self, farm_id: FarmId) -> FarmProfileRecord {
        fallback_farm_profile_for_projection(farm_id, self.state_store.farm_setup_projection())
    }

    fn load_products_list_for_query(
        &self,
        query: &ProductsScreenQueryState,
    ) -> Result<ProductsListProjection, AppSqliteError> {
        let _ = self.import_shared_local_events()?;
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(ProductsListProjection::default());
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(ProductsListProjection::default());
        };

        sqlite_store.load_products(farm_id, &query.search_query, query.filter, query.sort)
    }

    fn import_shared_local_events(&self) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(AppLocalInteropImportReport::default());
        };
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(AppLocalInteropImportReport::default());
        };
        let Some(database_path) =
            shared_local_events_database_path_from_shared_accounts(shared_accounts_paths)
        else {
            return Ok(AppLocalInteropImportReport::default());
        };
        sqlite_store.import_shared_local_events_from_path(database_path.as_path())
    }

    fn resolve_seller_order_request_evidence(
        &self,
        order_id: OrderId,
    ) -> Result<ResolvedAppSellerOrderRequest, AppSqliteError> {
        let mut matched_requests = BTreeMap::new();
        self.collect_seller_order_request_evidence_from_shared_events(
            &order_id,
            &mut matched_requests,
        )?;
        self.collect_seller_order_request_evidence_from_local_interop(
            &order_id,
            &mut matched_requests,
        )?;

        if matched_requests.len() > 1 {
            return Err(AppSqliteError::InvalidProjection {
                reason: "seller order decision found multiple signed order requests",
            });
        }

        matched_requests
            .into_values()
            .next()
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires signed order request evidence",
            })
    }

    fn collect_seller_order_request_evidence_from_shared_events(
        &self,
        order_id: &OrderId,
        matched_requests: &mut BTreeMap<String, ResolvedAppSellerOrderRequest>,
    ) -> Result<(), AppSqliteError> {
        let store = self.open_shared_local_events_store()?;
        let Some(store) = store else {
            return Ok(());
        };
        let mut before = None;

        loop {
            let records = match before {
                Some((before_change_seq, before_seq)) => store
                    .list_records_changed_before(
                        before_change_seq,
                        before_seq,
                        APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE,
                    )
                    .map_err(|source| AppSqliteError::LocalEvents {
                        operation: "load shared order request evidence",
                        source,
                    })?,
                None => store
                    .list_records_changed_latest(APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE)
                    .map_err(|source| AppSqliteError::LocalEvents {
                        operation: "load shared order request evidence",
                        source,
                    })?,
            };
            if records.is_empty() {
                break;
            }
            let is_last_page =
                records.len() < APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE as usize;
            before = records.last().map(|record| (record.change_seq, record.seq));

            for record in records {
                if record.family != LocalRecordFamily::SignedEvent
                    || record.event_kind
                        != Some(i64::from(
                            radroots_sdk::trade::RadrootsActiveTradeMessageType::TradeOrderRequested
                                .kind(),
                        ))
                    || !signed_order_request_evidence_record_is_usable(&record)
                {
                    continue;
                }
                let Some(event) = signed_event_from_local_record(&record)? else {
                    continue;
                };
                let Ok(envelope) = radroots_sdk::trade::parse_order_request(&event) else {
                    continue;
                };
                insert_seller_order_request_evidence(
                    order_id,
                    &event,
                    envelope.payload,
                    matched_requests,
                );
            }

            if before.is_none() || is_last_page {
                break;
            }
        }

        Ok(())
    }

    fn collect_seller_order_request_evidence_from_local_interop(
        &self,
        order_id: &OrderId,
        matched_requests: &mut BTreeMap<String, ResolvedAppSellerOrderRequest>,
    ) -> Result<(), AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(());
        };
        let events = sqlite_store.load_local_interop_signed_events_by_kind(i64::from(
            radroots_sdk::trade::RadrootsActiveTradeMessageType::TradeOrderRequested.kind(),
        ))?;

        for event in events {
            let Ok(envelope) = radroots_sdk::trade::parse_order_request(&event) else {
                continue;
            };
            insert_seller_order_request_evidence(
                order_id,
                &event,
                envelope.payload,
                matched_requests,
            );
        }

        Ok(())
    }

    fn resolve_order_lifecycle_evidence(
        &self,
        request: &ResolvedAppSellerOrderRequest,
    ) -> Result<ResolvedAppOrderLifecycleEvidence, AppSqliteError> {
        let mut events = self.collect_order_lifecycle_signed_events()?;
        events.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut buckets = AppActiveOrderEvidenceBuckets::default();
        buckets.requests.push(RadrootsActiveOrderRequestRecord {
            event_id: request.request_event_id.clone(),
            author_pubkey: request.request_author_pubkey.clone(),
            payload: request.payload.clone(),
        });
        for event in events {
            if trade_chain_tag_value(&event, "e_root").as_deref()
                != Some(request.request_event_id.as_str())
            {
                continue;
            }
            match event.kind {
                KIND_TRADE_ORDER_DECISION => {
                    let envelope =
                        radroots_sdk::trade::parse_order_decision(&event).map_err(|_| {
                            AppSqliteError::InvalidProjection {
                                reason: "order lifecycle evidence is invalid",
                            }
                        })?;
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets.decisions.push(RadrootsActiveOrderDecisionRecord {
                        event_id: event.id,
                        author_pubkey: event.author,
                        counterparty_pubkey: context.0,
                        root_event_id: context.1,
                        prev_event_id: context.2,
                        payload: envelope.payload,
                    });
                }
                KIND_TRADE_ORDER_REVISION => {
                    let Ok(envelope) = radroots_sdk::trade::parse_order_revision_proposal(&event)
                    else {
                        return Err(AppSqliteError::InvalidProjection {
                            reason: "order lifecycle evidence is invalid",
                        });
                    };
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets
                        .revision_proposals
                        .push(RadrootsActiveOrderRevisionProposalRecord {
                            event_id: event.id,
                            author_pubkey: event.author,
                            counterparty_pubkey: context.0,
                            root_event_id: context.1,
                            prev_event_id: context.2,
                            payload: envelope.payload,
                        });
                }
                KIND_TRADE_ORDER_REVISION_RESPONSE => {
                    let Ok(envelope) = radroots_sdk::trade::parse_order_revision_decision(&event)
                    else {
                        return Err(AppSqliteError::InvalidProjection {
                            reason: "order lifecycle evidence is invalid",
                        });
                    };
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets
                        .revision_decisions
                        .push(RadrootsActiveOrderRevisionDecisionRecord {
                            event_id: event.id,
                            author_pubkey: event.author,
                            counterparty_pubkey: context.0,
                            root_event_id: context.1,
                            prev_event_id: context.2,
                            payload: envelope.payload,
                        });
                }
                KIND_TRADE_CANCEL => {
                    let Ok(envelope) = radroots_sdk::trade::parse_order_cancellation(&event) else {
                        return Err(AppSqliteError::InvalidProjection {
                            reason: "order lifecycle evidence is invalid",
                        });
                    };
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets
                        .cancellations
                        .push(RadrootsActiveOrderCancellationRecord {
                            event_id: event.id,
                            author_pubkey: event.author,
                            counterparty_pubkey: context.0,
                            root_event_id: context.1,
                            prev_event_id: context.2,
                            payload: envelope.payload,
                        });
                }
                KIND_TRADE_FULFILLMENT_UPDATE => {
                    let Ok(envelope) = radroots_sdk::trade::parse_fulfillment_update(&event) else {
                        return Err(AppSqliteError::InvalidProjection {
                            reason: "order lifecycle evidence is invalid",
                        });
                    };
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets
                        .fulfillments
                        .push(RadrootsActiveOrderFulfillmentRecord {
                            event_id: event.id,
                            author_pubkey: event.author,
                            counterparty_pubkey: context.0,
                            root_event_id: context.1,
                            prev_event_id: context.2,
                            payload: envelope.payload,
                        });
                }
                KIND_TRADE_RECEIPT => {
                    let Ok(envelope) = radroots_sdk::trade::parse_buyer_receipt(&event) else {
                        return Err(AppSqliteError::InvalidProjection {
                            reason: "order lifecycle evidence is invalid",
                        });
                    };
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets.receipts.push(RadrootsActiveOrderReceiptRecord {
                        event_id: event.id,
                        author_pubkey: event.author,
                        counterparty_pubkey: context.0,
                        root_event_id: context.1,
                        prev_event_id: context.2,
                        payload: envelope.payload,
                    });
                }
                KIND_TRADE_PAYMENT_RECORDED => {
                    let envelope =
                        active_trade_payment_recorded_from_event(&event).map_err(|_| {
                            AppSqliteError::InvalidProjection {
                                reason: "order lifecycle evidence is invalid",
                            }
                        })?;
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets.payments.push(RadrootsActiveOrderPaymentRecord {
                        event_id: event.id,
                        author_pubkey: event.author,
                        counterparty_pubkey: context.0,
                        root_event_id: context.1,
                        prev_event_id: context.2,
                        payload: envelope.payload,
                    });
                }
                KIND_TRADE_SETTLEMENT_DECISION => {
                    let envelope =
                        active_trade_settlement_decision_from_event(&event).map_err(|_| {
                            AppSqliteError::InvalidProjection {
                                reason: "order lifecycle evidence is invalid",
                            }
                        })?;
                    let context = active_order_event_record_context(&event, envelope.message_type)?;
                    buckets
                        .settlements
                        .push(RadrootsActiveOrderSettlementRecord {
                            event_id: event.id,
                            author_pubkey: event.author,
                            counterparty_pubkey: context.0,
                            root_event_id: context.1,
                            prev_event_id: context.2,
                            payload: envelope.payload,
                        });
                }
                _ => {}
            }
        }

        let projection = reduce_active_order_events(
            request.payload.order_id.as_str(),
            buckets.requests.clone(),
            buckets.decisions.clone(),
            buckets.revision_proposals.clone(),
            buckets.revision_decisions.clone(),
            buckets.fulfillments.clone(),
            buckets.cancellations.clone(),
            buckets.receipts.clone(),
            buckets.payments.clone(),
            buckets.settlements.clone(),
        );
        if !projection.issues.is_empty() || projection.status == RadrootsActiveOrderStatus::Invalid
        {
            return Err(AppSqliteError::InvalidProjection {
                reason: "order lifecycle evidence is invalid",
            });
        }
        if projection.request_event_id.as_deref() != Some(request.request_event_id.as_str()) {
            return Err(AppSqliteError::InvalidProjection {
                reason: "order lifecycle evidence is invalid",
            });
        }

        let decision = projection
            .decision_event_id
            .as_deref()
            .map(|event_id| {
                buckets
                    .decisions
                    .iter()
                    .find(|decision| decision.event_id == event_id)
                    .map(|decision| ResolvedAppOrderDecisionEvidence {
                        event_id: decision.event_id.clone(),
                        payload: decision.payload.clone(),
                    })
                    .ok_or(AppSqliteError::InvalidProjection {
                        reason: "order lifecycle evidence is invalid",
                    })
            })
            .transpose()?;
        let latest_fulfillment = projection
            .fulfillment_event_id
            .as_deref()
            .map(|event_id| {
                buckets
                    .fulfillments
                    .iter()
                    .find(|fulfillment| fulfillment.event_id == event_id)
                    .map(|fulfillment| ResolvedAppOrderFulfillmentEvidence {
                        event_id: fulfillment.event_id.clone(),
                        status: fulfillment.payload.status,
                    })
                    .ok_or(AppSqliteError::InvalidProjection {
                        reason: "order lifecycle evidence is invalid",
                    })
            })
            .transpose()?;

        Ok(ResolvedAppOrderLifecycleEvidence {
            status: projection.status,
            payment_state: projection.payment.state,
            agreement_event_id: projection.agreement_event_id,
            last_event_id: projection.last_event_id,
            decision,
            revision_proposals: buckets
                .revision_proposals
                .into_iter()
                .map(|proposal| ResolvedAppOrderRevisionProposalEvidence {
                    event_id: proposal.event_id,
                    payload: proposal.payload,
                })
                .collect(),
            revision_decisions: buckets
                .revision_decisions
                .into_iter()
                .map(|decision| ResolvedAppOrderRevisionDecisionEvidence {
                    event_id: decision.event_id,
                    payload: decision.payload,
                })
                .collect(),
            latest_fulfillment,
            cancellation_event_id: projection.cancellation_event_id,
            receipt_event_id: projection.receipt_event_id,
        })
    }

    fn collect_order_lifecycle_signed_events(
        &self,
    ) -> Result<Vec<radroots_sdk::RadrootsNostrEvent>, AppSqliteError> {
        let mut events = Vec::new();
        let mut seen_event_ids = BTreeSet::new();
        let kinds = [
            KIND_TRADE_ORDER_DECISION,
            KIND_TRADE_ORDER_REVISION,
            KIND_TRADE_ORDER_REVISION_RESPONSE,
            KIND_TRADE_CANCEL,
            KIND_TRADE_FULFILLMENT_UPDATE,
            KIND_TRADE_RECEIPT,
            KIND_TRADE_PAYMENT_RECORDED,
            KIND_TRADE_SETTLEMENT_DECISION,
        ];

        if let Some(sqlite_store) = self.sqlite_store.as_ref() {
            for kind in kinds {
                for event in
                    sqlite_store.load_local_interop_signed_events_by_kind(i64::from(kind))?
                {
                    if seen_event_ids.insert(event.id.clone()) {
                        events.push(event);
                    }
                }
            }
        }

        let Some(store) = self.open_shared_local_events_store()? else {
            return Ok(events);
        };
        let mut before = None;
        loop {
            let records = match before {
                Some((before_change_seq, before_seq)) => store
                    .list_records_changed_before(
                        before_change_seq,
                        before_seq,
                        APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE,
                    )
                    .map_err(|source| AppSqliteError::LocalEvents {
                        operation: "load shared order lifecycle evidence",
                        source,
                    })?,
                None => store
                    .list_records_changed_latest(APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE)
                    .map_err(|source| AppSqliteError::LocalEvents {
                        operation: "load shared order lifecycle evidence",
                        source,
                    })?,
            };
            if records.is_empty() {
                break;
            }
            let is_last_page =
                records.len() < APP_SELLER_ORDER_DECISION_EVIDENCE_PAGE_SIZE as usize;
            before = records.last().map(|record| (record.change_seq, record.seq));

            for record in records {
                let Some(kind) = record.event_kind else {
                    continue;
                };
                if record.family != LocalRecordFamily::SignedEvent
                    || !kinds.contains(&u32::try_from(kind).unwrap_or_default())
                    || !signed_order_request_evidence_record_is_usable(&record)
                {
                    continue;
                }
                let Some(event) = signed_event_from_local_record(&record)? else {
                    continue;
                };
                if seen_event_ids.insert(event.id.clone()) {
                    events.push(event);
                }
            }

            if before.is_none() || is_last_page {
                break;
            }
        }

        Ok(events)
    }

    fn open_shared_local_events_store(
        &self,
    ) -> Result<Option<LocalEventsStore<SqliteExecutor>>, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(None);
        };
        let Some(database_path) =
            shared_local_events_database_path_from_shared_accounts(shared_accounts_paths)
        else {
            return Ok(None);
        };
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AppSqliteError::CreateParentDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let executor = SqliteExecutor::open(database_path.as_path()).map_err(|source| {
            AppSqliteError::LocalEventsSql {
                operation: "open shared local events database",
                source,
            }
        })?;
        let store = LocalEventsStore::new(executor);
        store
            .migrate_up()
            .map_err(|source| AppSqliteError::LocalEventsSql {
                operation: "migrate shared local events database",
                source,
            })?;

        Ok(Some(store))
    }

    fn append_app_farm_local_work_record(
        &self,
        account: &radroots_studio_app_view::SelectedAccountProjection,
        projection: &FarmSetupProjection,
        saved_farm: &FarmSummary,
    ) -> Result<Option<String>, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(None);
        };
        let timestamp = current_runtime_time_ms()?;
        let farm_d_tag = d_tag_from_uuid(saved_farm.farm_id.as_uuid());
        let owner_pubkey = self.local_events_owner_pubkey(account);
        let exportability = local_work_exportability(owner_pubkey.as_deref());
        let delivery_method = projection
            .draft
            .order_methods
            .iter()
            .next()
            .map(|method| method.storage_key());
        let payload = json!({
            "record_kind": "farm_config_v1",
            "scope": "app",
            "exportability": exportability,
            "document": {
                "version": 1,
                "selection": {
                    "scope": "app",
                    "account": account.account.account_id,
                    "farm_d_tag": farm_d_tag,
                },
                "profile": {
                    "name": saved_farm.display_name,
                    "display_name": saved_farm.display_name,
                },
                "farm": {
                    "d_tag": farm_d_tag,
                    "name": saved_farm.display_name,
                    "location": {
                        "primary": projection.draft.location_or_service_area,
                    },
                },
                "listing_defaults": {
                    "delivery_method": delivery_method,
                    "location": {
                        "primary": projection.draft.location_or_service_area,
                    },
                },
            },
        });
        let record_id = format!("app:local_work:farm:{farm_d_tag}:{}", Uuid::now_v7());
        let input = LocalEventRecordInput {
            record_id: record_id.clone(),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::App,
            created_at_ms: timestamp,
            inserted_at_ms: timestamp,
            owner_account_id: Some(account.account.account_id.clone()),
            owner_pubkey,
            farm_id: Some(farm_d_tag),
            listing_addr: None,
            local_work_json: Some(payload.clone()),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status: PublishOutboxStatus::None,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        };

        self.append_app_local_work_record(shared_accounts_paths, &input)?;
        Ok(Some(record_id))
    }

    fn append_app_listing_local_work_record(
        &self,
        product_id: ProductId,
        draft: &ProductEditorDraft,
    ) -> Result<Option<String>, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(None);
        };
        let Some(account) = self
            .state_store
            .identity_projection()
            .selected_account
            .as_ref()
        else {
            return Ok(None);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(None);
        };
        let timestamp = current_runtime_time_ms()?;
        let farm_d_tag = d_tag_from_uuid(farm_id.as_uuid());
        let listing_d_tag = d_tag_from_uuid(product_id.as_uuid());
        let primary_bin_id = listing_primary_bin_id(listing_d_tag.as_str());
        let owner_pubkey = self.local_events_owner_pubkey(account);
        let listing_addr = owner_pubkey
            .as_ref()
            .map(|pubkey| format!("{KIND_LISTING}:{pubkey}:{listing_d_tag}"));
        let exportability = local_work_exportability(owner_pubkey.as_deref());
        let farm_setup = self.state_store.farm_setup_projection();
        let farm_rules = self.state_store.farm_rules_projection();
        let delivery_method = listing_fulfillment_method(draft, farm_setup, farm_rules);
        let location_primary = listing_fulfillment_location(draft, farm_setup, farm_rules);
        let category = non_empty_string(draft.category.as_str());
        let unit_label = non_empty_string(draft.unit_label.as_str());
        let price_amount = draft.price_minor_units.map(decimal_from_minor_units);
        let available = draft.stock_quantity.map(|value| value.to_string());
        let publish_blockers = derive_product_publish_blockers(
            draft,
            self.state_store.farm_readiness_projection(),
            farm_rules,
        )
        .into_iter()
        .map(|blocker| blocker.storage_key())
        .collect::<Vec<_>>();
        let payload = json!({
            "record_kind": "listing_draft_v1",
            "exportability": exportability,
            "publishability": {
                "state": if publish_blockers.is_empty() { "publishable" } else { "blocked" },
                "blockers": publish_blockers,
            },
            "document": {
                "version": 1,
                "kind": "listing_draft_v1",
                "listing": {
                    "d_tag": listing_d_tag,
                    "farm_d_tag": farm_d_tag,
                },
                "seller_actor": {
                    "account_id": account.account.account_id,
                    "pubkey": owner_pubkey.as_deref(),
                    "source": "farm_config",
                },
                "product": {
                    "key": listing_d_tag,
                    "title": draft.title,
                    "category": category,
                    "summary": draft.subtitle,
                },
                "primary_bin": {
                    "bin_id": primary_bin_id,
                    "quantity_amount": "1",
                    "quantity_unit": unit_label,
                    "price_amount": price_amount,
                    "price_currency": draft.price_currency,
                    "price_per_amount": "1",
                    "price_per_unit": unit_label,
                },
                "inventory": {
                    "available": available,
                },
                "availability": {
                    "kind": "local",
                    "status": draft.status.storage_key(),
                },
                "delivery": {
                    "method": delivery_method,
                },
                "location": {
                    "primary": location_primary,
                },
            },
        });
        let record_id = format!("app:local_work:listing:{listing_d_tag}:{}", Uuid::now_v7());
        let input = LocalEventRecordInput {
            record_id: record_id.clone(),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::App,
            created_at_ms: timestamp,
            inserted_at_ms: timestamp,
            owner_account_id: Some(account.account.account_id.clone()),
            owner_pubkey,
            farm_id: Some(farm_d_tag),
            listing_addr,
            local_work_json: Some(payload.clone()),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status: PublishOutboxStatus::None,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        };

        self.append_app_local_work_record(shared_accounts_paths, &input)?;
        Ok(Some(record_id))
    }

    fn append_app_buyer_order_request_local_work_record(
        &self,
        sqlite_store: &AppSqliteStore,
        buyer_context: &BuyerContext,
        order: &BuyerOrderLocalEventExport,
    ) -> Result<Option<AppOrderLocalWorkPublishSource>, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(None);
        };
        let timestamp = current_runtime_time_ms()?;
        let record_id = buyer_order_request_local_work_record_id(
            order.order_id.to_string().as_str(),
        )
        .map_err(|source| AppSqliteError::LocalEvents {
            operation: "build app buyer order request record id",
            source,
        })?;
        let buyer_account = self.selected_buyer_account(buyer_context);
        let owner_account_id = buyer_account.map(|account| account.account.account_id.clone());
        let buyer_pubkey =
            buyer_account.and_then(|account| self.local_events_owner_pubkey(account));
        let export = AppBuyerOrderRequestExport::from_order(order, buyer_pubkey.as_deref())?;
        let payload = buyer_order_request_local_work_payload(
            order,
            buyer_context,
            &record_id,
            &export,
            timestamp,
        );
        if sqlite_store.buyer_order_coordination_is_synced(buyer_context, order.order_id)? {
            return Ok(Some(AppOrderLocalWorkPublishSource { record_id, payload }));
        }
        validate_buyer_order_request_local_work_payload(&payload).map_err(|source| {
            AppSqliteError::LocalEvents {
                operation: "validate app buyer order request local work payload",
                source,
            }
        })?;
        let payload_json =
            serde_json::to_string(&payload).map_err(|_| AppSqliteError::InvalidProjection {
                reason: "buyer order request local work payload must encode",
            })?;
        sqlite_store.prepare_buyer_order_coordination_attempt(
            buyer_context,
            order.order_id,
            record_id.as_str(),
            payload_json.as_str(),
        )?;
        let input = LocalEventRecordInput {
            record_id: record_id.clone(),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::App,
            created_at_ms: timestamp,
            inserted_at_ms: timestamp,
            owner_account_id,
            owner_pubkey: buyer_pubkey,
            farm_id: export.farm_key.clone(),
            listing_addr: export.listing_addr.clone(),
            local_work_json: Some(payload.clone()),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status: PublishOutboxStatus::None,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        };

        if let Err(error) = self.append_app_local_work_record(shared_accounts_paths, &input) {
            let failure_message = error.to_string();
            let _ = sqlite_store.mark_buyer_order_coordination_failed(
                buyer_context,
                order.order_id,
                failure_message.as_str(),
            );
            return Err(error);
        }
        sqlite_store.mark_buyer_order_coordination_synced(buyer_context, order.order_id)?;
        Ok(Some(AppOrderLocalWorkPublishSource { record_id, payload }))
    }

    fn append_app_local_work_record(
        &self,
        shared_accounts_paths: &AppSharedAccountsPaths,
        input: &LocalEventRecordInput,
    ) -> Result<(), AppSqliteError> {
        let Some(database_path) =
            shared_local_events_database_path_from_shared_accounts(shared_accounts_paths)
        else {
            return Ok(());
        };
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AppSqliteError::CreateParentDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let executor = SqliteExecutor::open(database_path.as_path()).map_err(|source| {
            AppSqliteError::LocalEventsSql {
                operation: "open shared local events database",
                source,
            }
        })?;
        let store = LocalEventsStore::new(executor);
        store
            .migrate_up()
            .map_err(|source| AppSqliteError::LocalEventsSql {
                operation: "migrate shared local events database",
                source,
            })?;
        store
            .append_record(input)
            .map_err(|source| AppSqliteError::LocalEvents {
                operation: "append app local work record",
                source,
            })?;
        Ok(())
    }

    fn record_published_sync_receipts(
        &self,
        receipts: &[AppPublishedOperationReceipt],
    ) -> Result<(), AppSqliteError> {
        if receipts.is_empty() {
            return Ok(());
        }
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(());
        };
        let Some(database_path) =
            shared_local_events_database_path_from_shared_accounts(shared_accounts_paths)
        else {
            return Ok(());
        };
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AppSqliteError::CreateParentDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let executor = SqliteExecutor::open(database_path.as_path()).map_err(|source| {
            AppSqliteError::LocalEventsSql {
                operation: "open shared local events database",
                source,
            }
        })?;
        let store = LocalEventsStore::new(executor);
        store
            .migrate_up()
            .map_err(|source| AppSqliteError::LocalEventsSql {
                operation: "migrate shared local events database",
                source,
            })?;
        let timestamp = current_runtime_time_ms()?;

        for receipt in receipts {
            let source_record = receipt
                .source_local_event_id
                .as_deref()
                .map(|source_record_id| {
                    store.get_record(source_record_id).map_err(|source| {
                        AppSqliteError::LocalEvents {
                            operation: "load app publish source record",
                            source,
                        }
                    })
                })
                .transpose()?
                .flatten();
            if source_record
                .as_ref()
                .and_then(|record| record.owner_account_id.as_deref())
                .is_some_and(|owner_account_id| owner_account_id != receipt.source_account_id)
            {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "published operation source account does not match local event owner",
                });
            }
            let farm_id = source_record
                .as_ref()
                .and_then(|record| record.farm_id.clone())
                .or_else(|| signed_event_farm_id(receipt));
            let listing_addr = source_record
                .as_ref()
                .and_then(|record| record.listing_addr.clone())
                .or_else(|| receipt.listing_addr.clone())
                .or_else(|| signed_event_listing_addr(receipt));
            let event_record = LocalEventRecordInput {
                record_id: format!("app:signed_event:{}", receipt.event_id),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::App,
                created_at_ms: i64::from(receipt.event_created_at) * 1_000,
                inserted_at_ms: timestamp,
                owner_account_id: Some(receipt.source_account_id.clone()),
                owner_pubkey: Some(receipt.event_pubkey.clone()),
                farm_id,
                listing_addr,
                local_work_json: None,
                event_id: Some(receipt.event_id.clone()),
                event_kind: Some(i64::from(receipt.event_kind)),
                event_pubkey: Some(receipt.event_pubkey.clone()),
                event_created_at: Some(i64::from(receipt.event_created_at)),
                event_tags_json: Some(receipt.event_tags_json.clone()),
                event_content: Some(receipt.event_content.clone()),
                event_sig: Some(receipt.event_sig.clone()),
                raw_event_json: Some(receipt.raw_event_json.clone()),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some(receipt.relay_set_fingerprint.clone()),
                relay_delivery_json: Some(receipt.relay_delivery_json.clone()),
            };
            store
                .append_record(&event_record)
                .map_err(|source| AppSqliteError::LocalEvents {
                    operation: "append app published event record",
                    source,
                })?;

            if let Some(source_record_id) = receipt.source_local_event_id.as_deref() {
                let Some(source_record) = source_record.as_ref() else {
                    continue;
                };
                if source_record.family == LocalRecordFamily::LocalWork {
                    continue;
                }
                store
                    .update_outbox(&LocalEventRecordUpdate {
                        record_id: source_record_id.to_owned(),
                        status: LocalRecordStatus::Published,
                        outbox_status: PublishOutboxStatus::Acknowledged,
                        relay_set_fingerprint: Some(receipt.relay_set_fingerprint.clone()),
                        relay_delivery_json: Some(receipt.relay_delivery_json.clone()),
                        updated_at_ms: timestamp,
                    })
                    .map_err(|source| AppSqliteError::LocalEvents {
                        operation: "update app publish source evidence",
                        source,
                    })?;
            }
        }

        Ok(())
    }

    fn local_events_owner_pubkey(
        &self,
        account: &radroots_studio_app_view::SelectedAccountProjection,
    ) -> Option<String> {
        if is_hex_64(account.account.account_id.as_str()) {
            return Some(account.account.account_id.clone());
        }
        self.accounts_manager
            .as_ref()
            .and_then(|manager| {
                manager
                    .resolve_account_selector(account.account.account_id.as_str())
                    .ok()
            })
            .map(|record| record.public_identity.public_key_hex)
            .filter(|pubkey| is_hex_64(pubkey))
    }

    fn selected_buyer_account(
        &self,
        buyer_context: &BuyerContext,
    ) -> Option<&radroots_studio_app_view::SelectedAccountProjection> {
        let BuyerContext::Account(account_id) = buyer_context else {
            return None;
        };
        self.state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .filter(|account| account.account.account_id == *account_id)
    }

    fn refresh_selected_account_context_after_local_events(
        &mut self,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let continuity_state = self.continuity_state();
        let identity_projection = self.state_store.identity_projection().clone();
        let selected_account_context =
            load_selected_account_context(sqlite_store, &identity_projection, &continuity_state)?;

        Ok(self.apply_selected_account_context(&selected_account_context))
    }

    fn sync_on_foreground_resume(&mut self) -> Result<bool, AppSqliteError> {
        let report = self.import_shared_local_events()?;
        let local_changed = report.imported_records > 0 || report.skipped_records > 0;
        let context_changed = self.refresh_selected_account_context_after_local_events()?;
        let coordination_changed = self.retry_pending_personal_order_coordination()?;
        let sync_changed = self.attempt_sync(SyncTrigger::ForegroundResume)?;

        Ok(local_changed || context_changed || coordination_changed || sync_changed)
    }

    fn replace_orders_query(
        &mut self,
        query: OrdersScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let filter_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::select_orders_filter(query.filter));
        let fulfillment_window_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::select_orders_fulfillment_window(
                    query.fulfillment_window_id,
                ));
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(filter_changed || fulfillment_window_changed);
        };
        let continuity_state = self.continuity_state_with_orders_query(query, None);
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);

        Ok(filter_changed || fulfillment_window_changed || context_changed)
    }

    fn replace_pack_day_query(
        &mut self,
        query: PackDayScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let previous_export_instance_id = self.current_pack_day_export_instance_id();
        let fulfillment_window_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::set_pack_day_fulfillment_window(
                    query.fulfillment_window_id,
                ));
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(fulfillment_window_changed);
        };
        let continuity_state = self.continuity_state_with_pack_day_query(query);
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        self.cleanup_prepared_pack_day_print_assets_if_export_changed(
            previous_export_instance_id,
            "query_reset",
        );

        Ok(fulfillment_window_changed || context_changed)
    }
    fn sync_truthful_farmer_section(&mut self) -> bool {
        let selected_section = self.state_store.shell_projection().selected_section;
        let should_reset_to_today = match selected_section {
            ShellSection::Farmer(FarmerSection::Today) => false,
            ShellSection::Farmer(FarmerSection::Products | FarmerSection::Orders) => {
                !self.has_saved_farm()
            }
            ShellSection::Farmer(FarmerSection::PackDay) => {
                !self.has_saved_farm() || !self.has_pack_day_context()
            }
            ShellSection::Farmer(FarmerSection::Farm) => true,
            ShellSection::Home
            | ShellSection::Account
            | ShellSection::Personal(_)
            | ShellSection::Settings(_) => false,
        };

        should_reset_to_today
            && self
                .state_store
                .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                    FarmerSection::Today,
                )))
    }

    fn shared_accounts_paths(
        &self,
    ) -> Result<&AppSharedAccountsPaths, DesktopAppRuntimeCommandError> {
        self.shared_accounts_paths
            .as_ref()
            .ok_or(DesktopAppRuntimeCommandError::RuntimeUnavailable)
    }

    fn remote_signer_paths(&self) -> Option<&DesktopRemoteSignerPaths> {
        self.remote_signer_paths.as_ref()
    }

    fn load_startup_pending_remote_signer_session(
        &self,
    ) -> Result<Option<RadrootsAppRemoteSignerPendingSession>, DesktopAppRuntimeCommandError> {
        let Some(paths) = self.remote_signer_paths() else {
            return Ok(None);
        };
        Ok(load_pending_session(paths)?)
    }

    fn store_startup_pending_remote_signer_session(
        &mut self,
        pending: &RadrootsAppRemoteSignerPendingSession,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let Some(paths) = self.remote_signer_paths() else {
            return Err(DesktopAppRuntimeCommandError::RuntimeUnavailable);
        };
        store_pending_session(paths, pending)?;
        Ok(true)
    }

    fn clear_startup_pending_remote_signer_session(
        &mut self,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let Some(paths) = self.remote_signer_paths() else {
            return Ok(false);
        };
        clear_pending_session(paths)?;
        Ok(true)
    }

    fn activate_startup_approved_remote_signer_session(
        &mut self,
        pending: &RadrootsAppRemoteSignerPendingSession,
        approved: &RadrootsAppRemoteSignerApprovedSession,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let Some(paths) = self.remote_signer_paths() else {
            return Err(DesktopAppRuntimeCommandError::RuntimeUnavailable);
        };
        {
            let accounts_manager = self.accounts_manager()?;
            activate_pending_session(
                accounts_manager,
                paths,
                pending.record.client_account_id(),
                approved,
            )?;
        }
        let projection = {
            let accounts_manager = self.accounts_manager()?;
            let sqlite_store = self.sqlite_store()?;
            identity_projection_from_manager(accounts_manager, sqlite_store)?
        };
        self.replace_identity_projection(projection)
    }

    fn decorate_identity_projection(
        &self,
        projection: AppIdentityProjection,
    ) -> Result<AppIdentityProjection, DesktopAppRuntimeCommandError> {
        let Some(paths) = self.remote_signer_paths() else {
            return Ok(projection);
        };
        Ok(apply_remote_signer_custody(projection, paths)?)
    }

    fn command_unavailable_error(&self) -> DesktopAppRuntimeCommandError {
        let _ = self;
        DesktopAppRuntimeCommandError::RuntimeUnavailable
    }

    fn current_pack_day_export_bundle(&self) -> Option<PackDayExportBundle> {
        let pack_day = self.state_store.pack_day_projection();
        if pack_day.export.status != PackDayExportStatus::Succeeded {
            return None;
        }

        let bundle = pack_day.export.bundle.clone()?;
        let fulfillment_window = pack_day.projection.fulfillment_window.as_ref()?;
        (fulfillment_window.fulfillment_window_id == bundle.fulfillment_window_id).then_some(bundle)
    }

    fn current_pack_day_export_instance_id(&self) -> Option<PackDayExportInstanceId> {
        self.current_pack_day_export_bundle()
            .map(|bundle| bundle.export_instance_id)
    }

    fn cleanup_prepared_pack_day_print_assets_for_export_instance(
        &self,
        export_instance_id: PackDayExportInstanceId,
        trigger: &'static str,
    ) {
        if let Err(error) =
            cleanup_prepared_customer_label_assets_for_export_instance(export_instance_id)
        {
            error!(
                target: "pack_day",
                event = "pack_day.print_prepared_asset_cleanup_failed",
                trigger,
                export_instance_id = %export_instance_id,
                error = %error,
                "failed to clean prepared pack day print assets"
            );
        }
    }

    fn cleanup_prepared_pack_day_print_assets_if_export_changed(
        &self,
        previous_export_instance_id: Option<PackDayExportInstanceId>,
        trigger: &'static str,
    ) {
        let current_export_instance_id = self.current_pack_day_export_instance_id();
        if let Some(export_instance_id) = previous_export_instance_id
            .filter(|export_instance_id| Some(*export_instance_id) != current_export_instance_id)
        {
            self.cleanup_prepared_pack_day_print_assets_for_export_instance(
                export_instance_id,
                trigger,
            );
        }
    }

    fn current_pack_day_host_handoff_request_matches(
        &self,
        request: &PackDayHostHandoffRequest,
    ) -> bool {
        let pack_day = self.state_store.pack_day_projection();
        pack_day.host_handoff.status == PackDayHostHandoffStatus::Running
            && pack_day.host_handoff.request.as_ref() == Some(request)
    }

    fn current_pack_day_print_request_matches(&self, request: &PackDayPrintRequest) -> bool {
        let pack_day = self.state_store.pack_day_projection();
        pack_day.print.status == PackDayPrintStatus::Running
            && pack_day.print.request.as_ref() == Some(request)
    }

    fn current_pack_day_batch_print_request_matches(
        &self,
        request: &PackDayBatchPrintRequest,
    ) -> bool {
        let pack_day = self.state_store.pack_day_projection();
        pack_day.batch_print.status == PackDayBatchPrintStatus::Running
            && pack_day.batch_print.request.as_ref() == Some(request)
    }
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeCommandError {
    #[error("desktop runtime commands are unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error(transparent)]
    Accounts(#[from] DesktopAccountsCommandError),
    #[error(transparent)]
    Projection(#[from] DesktopAccountsProjectionError),
    #[error("remote signer command failed: {0}")]
    RemoteSigner(String),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    PackDayExportWrite(#[from] PackDayExportWriteError),
    #[error(transparent)]
    PackDayHostHandoff(#[from] PackDayHostHandoffError),
    #[error(transparent)]
    PackDayPrint(#[from] PackDayPrintError),
    #[error(transparent)]
    PackDayBatchPrint(#[from] PackDayBatchPrintError),
}

impl From<DesktopRemoteSignerError> for DesktopAppRuntimeCommandError {
    fn from(error: DesktopRemoteSignerError) -> Self {
        Self::RemoteSigner(error.to_string())
    }
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeFarmSetupError {
    #[error("desktop runtime commands are unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error("farm setup requires a selected account")]
    AccountRequired,
    #[error("farm setup is incomplete")]
    IncompleteDraft,
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeFarmRulesError {
    #[error("desktop runtime commands are unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error("farm settings require a selected account")]
    AccountRequired,
    #[error("farm settings require a configured farm")]
    FarmRequired,
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
}

#[derive(Debug, Error)]
enum DesktopAppRuntimeBootstrapError {
    #[error(transparent)]
    RuntimePaths(#[from] AppRuntimePathsError),
    #[error(transparent)]
    Accounts(#[from] DesktopAccountsBootstrapError),
    #[error(transparent)]
    Projection(#[from] DesktopAccountsProjectionError),
    #[error(transparent)]
    RemoteSigner(#[from] DesktopRemoteSignerError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
    #[error(transparent)]
    State(#[from] AppStateStoreError),
}

fn current_runtime_time_ms() -> Result<i64, AppSqliteError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        AppSqliteError::InvalidProjection {
            reason: "current runtime timestamp must be after unix epoch",
        }
    })?;
    i64::try_from(duration.as_millis()).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "current runtime timestamp must fit i64 milliseconds",
    })
}

fn current_runtime_time_seconds() -> Result<i64, AppSqliteError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        AppSqliteError::InvalidProjection {
            reason: "current runtime timestamp must be after unix epoch",
        }
    })?;
    i64::try_from(duration.as_secs()).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "current runtime timestamp must fit i64 seconds",
    })
}

fn normalized_app_sync_relay_urls(
    relay_urls: &[String],
) -> Result<Vec<String>, AppSyncTransportError> {
    let normalized = radroots_local_events::normalize_relay_urls(relay_urls).map_err(|error| {
        AppSyncTransportError::failed(format!("invalid direct relay app sync relay url: {error}"))
    })?;
    if normalized.is_empty() {
        return Err(AppSyncTransportError::unavailable(
            "direct relay app sync requires at least one configured relay",
        ));
    }
    Ok(normalized)
}

fn normalized_app_relay_ingest_urls(relay_urls: &[String]) -> Result<Vec<String>, AppSqliteError> {
    let normalized = radroots_local_events::normalize_relay_urls(relay_urls).map_err(|_| {
        AppSqliteError::InvalidProjection {
            reason: "app relay ingest requires valid relay urls",
        }
    })?;
    Ok(normalized)
}

fn fetch_app_events_from_relays_windowed(
    cursors: &[StoredRelayIngestCursor],
) -> Result<AppDirectRelayFetchReceipt, AppSyncTransportError> {
    let target_relays = cursors
        .iter()
        .map(|cursor| cursor.relay_url.clone())
        .collect::<Vec<_>>();
    let mut merged: Option<AppDirectRelayFetchReceipt> = None;

    for cursor in cursors {
        match fetch_app_events_from_single_relay_windowed(cursor) {
            Ok(receipt) => merge_app_direct_relay_fetch_receipt(&mut merged, receipt),
            Err(error) => merge_app_direct_relay_fetch_receipt(
                &mut merged,
                AppDirectRelayFetchReceipt {
                    target_relays: vec![cursor.relay_url.clone()],
                    connected_relays: Vec::new(),
                    failed_relays: vec![
                        RelayDeliveryFailure::new(cursor.relay_url.clone(), error.to_string())
                            .map_err(|source| AppSyncTransportError::failed(source.to_string()))?,
                    ],
                    fetched_relays: Vec::new(),
                    event_observed_relays: BTreeMap::new(),
                    events: Vec::new(),
                },
            ),
        }
    }

    Ok(merged.unwrap_or_else(|| AppDirectRelayFetchReceipt {
        target_relays,
        connected_relays: Vec::new(),
        failed_relays: Vec::new(),
        fetched_relays: Vec::new(),
        event_observed_relays: BTreeMap::new(),
        events: Vec::new(),
    }))
}

fn fetch_app_events_from_single_relay_windowed(
    cursor: &StoredRelayIngestCursor,
) -> Result<AppDirectRelayFetchReceipt, AppSyncTransportError> {
    let base_filter = direct_relay_ingest_filter_since(cursor.cursor_since_unix_seconds)?;
    let mut next_filter = base_filter.clone();
    let mut merged: Option<AppDirectRelayFetchReceipt> = None;

    for _ in 0..APP_DIRECT_RELAY_INGEST_MAX_PAGES {
        let receipt = fetch_app_events_from_single_relay(cursor.relay_url.as_str(), next_filter)?;
        let page_len = receipt.events.len();
        let oldest_created_at = receipt
            .events
            .iter()
            .map(|event| event.created_at.as_secs())
            .min();
        merge_app_direct_relay_fetch_receipt(&mut merged, receipt);
        if page_len < APP_DIRECT_RELAY_INGEST_LIMIT {
            break;
        }
        let Some(oldest_created_at) = oldest_created_at else {
            break;
        };
        if cursor.cursor_since_unix_seconds.is_some_and(|since| {
            i64::try_from(oldest_created_at).is_ok_and(|oldest| oldest <= since)
        }) || oldest_created_at == 0
        {
            break;
        }
        next_filter = base_filter
            .clone()
            .until(RadrootsNostrTimestamp::from(oldest_created_at - 1))
            .limit(APP_DIRECT_RELAY_INGEST_LIMIT);
    }

    Ok(merged.unwrap_or_else(|| AppDirectRelayFetchReceipt {
        target_relays: vec![cursor.relay_url.clone()],
        connected_relays: Vec::new(),
        failed_relays: Vec::new(),
        fetched_relays: Vec::new(),
        event_observed_relays: BTreeMap::new(),
        events: Vec::new(),
    }))
}

fn fetch_app_events_from_single_relay(
    relay_url: &str,
    filter: RadrootsNostrFilter,
) -> Result<AppDirectRelayFetchReceipt, AppSyncTransportError> {
    let runtime = TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?;
    runtime.block_on(fetch_app_events_from_single_relay_async(relay_url, filter))
}

async fn fetch_app_events_from_single_relay_async(
    relay_url: &str,
    filter: RadrootsNostrFilter,
) -> Result<AppDirectRelayFetchReceipt, AppSyncTransportError> {
    let client = RadrootsNostrClient::new_signerless();

    client
        .add_read_relay(relay_url)
        .await
        .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;

    let connection_output = client.try_connect(APP_DIRECT_RELAY_CONNECT_TIMEOUT).await;
    let failed_relays = direct_relay_failures_from_output(&connection_output)?;
    if connection_output.success.is_empty() {
        return Err(AppSyncTransportError::unavailable(format!(
            "direct relay app ingest connection failed: {}",
            summarize_app_relay_failures(&failed_relays)
        )));
    }

    let events = client
        .fetch_events(
            filter,
            StdDuration::from_millis(APP_DIRECT_RELAY_SYNC_TIMEOUT_MS),
        )
        .await
        .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;
    let last_event_created_at_unix_seconds = events
        .iter()
        .map(|event| relay_event_created_at_unix_seconds_for_fetch(event))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max();
    let mut event_observed_relays = BTreeMap::new();
    for event in &events {
        event_observed_relays.insert(event.id.to_hex(), vec![relay_url.to_owned()]);
    }

    Ok(AppDirectRelayFetchReceipt {
        target_relays: vec![relay_url.to_owned()],
        connected_relays: connection_output
            .success
            .iter()
            .map(ToString::to_string)
            .collect(),
        failed_relays,
        fetched_relays: vec![AppDirectRelayFetchedRelay {
            relay_url: relay_url.to_owned(),
            last_event_created_at_unix_seconds,
        }],
        event_observed_relays,
        events,
    })
}

fn direct_relay_ingest_filter() -> RadrootsNostrFilter {
    RadrootsNostrFilter::new()
        .kinds(
            APP_DIRECT_RELAY_INGEST_KINDS
                .iter()
                .copied()
                .map(radroots_nostr_kind),
        )
        .limit(APP_DIRECT_RELAY_INGEST_LIMIT)
}

fn direct_relay_ingest_filter_since(
    since_unix_seconds: Option<i64>,
) -> Result<RadrootsNostrFilter, AppSyncTransportError> {
    let mut filter = direct_relay_ingest_filter();
    if let Some(since_unix_seconds) = since_unix_seconds {
        let since = u64::try_from(since_unix_seconds).map_err(|_| {
            AppSyncTransportError::failed("relay ingest cursor must be non-negative")
        })?;
        filter = filter.since(RadrootsNostrTimestamp::from(since));
    }
    Ok(filter)
}

fn direct_relay_failures_from_output<T: fmt::Debug>(
    output: &RadrootsNostrOutput<T>,
) -> Result<Vec<RelayDeliveryFailure>, AppSyncTransportError> {
    output
        .failed
        .iter()
        .map(|(relay, reason)| {
            RelayDeliveryFailure::new(relay.to_string(), reason.to_string())
                .map_err(|source| AppSyncTransportError::failed(source.to_string()))
        })
        .collect()
}

fn summarize_app_relay_failures(failed_relays: &[RelayDeliveryFailure]) -> String {
    if failed_relays.is_empty() {
        return "no relay acknowledged the operation".to_owned();
    }

    failed_relays
        .iter()
        .map(|failure| format!("{}: {}", failure.relay_url, failure.error))
        .collect::<Vec<_>>()
        .join("; ")
}

fn merge_app_direct_relay_fetch_receipt(
    merged: &mut Option<AppDirectRelayFetchReceipt>,
    receipt: AppDirectRelayFetchReceipt,
) {
    let Some(existing) = merged.as_mut() else {
        *merged = Some(receipt);
        return;
    };

    append_unique_relays(&mut existing.target_relays, receipt.target_relays);
    append_unique_relays(&mut existing.connected_relays, receipt.connected_relays);
    for fetched_relay in receipt.fetched_relays {
        if !existing
            .fetched_relays
            .iter()
            .any(|known| known.relay_url == fetched_relay.relay_url)
        {
            existing.fetched_relays.push(fetched_relay);
        }
    }
    for failure in receipt.failed_relays {
        if !existing
            .failed_relays
            .iter()
            .any(|known| known.relay_url == failure.relay_url && known.error == failure.error)
        {
            existing.failed_relays.push(failure);
        }
    }
    for (event_id, relays) in receipt.event_observed_relays {
        let observed = existing
            .event_observed_relays
            .entry(event_id)
            .or_insert_with(Vec::new);
        append_unique_relays(observed, relays);
    }
    let mut seen_event_ids = existing
        .events
        .iter()
        .map(|event| event.id.to_hex())
        .collect::<BTreeSet<_>>();
    for event in receipt.events {
        if seen_event_ids.insert(event.id.to_hex()) {
            existing.events.push(event);
        }
    }
}

fn append_unique_relays(target: &mut Vec<String>, relays: Vec<String>) {
    for relay in relays {
        if !target.iter().any(|known| known == &relay) {
            target.push(relay);
        }
    }
}

fn direct_relay_event_records(
    receipt: &AppDirectRelayFetchReceipt,
    inserted_at_ms: i64,
) -> Result<Vec<LocalEventRecord>, AppDirectRelayIngestError> {
    let mut records = Vec::with_capacity(receipt.events.len());

    for (index, event) in receipt.events.iter().enumerate() {
        let event_id = event.id.to_hex();
        let observed_relays = receipt
            .event_observed_relays
            .get(event_id.as_str())
            .cloned()
            .unwrap_or_default();
        let delivery_evidence = RelayDeliveryEvidence::observed(
            &receipt.target_relays,
            &receipt.connected_relays,
            observed_relays,
            receipt.failed_relays.clone(),
        )
        .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;
        let relay_set_fingerprint = delivery_evidence.relay_set_fingerprint().ok_or_else(|| {
            AppSyncTransportError::failed("app relay ingest requires a non-empty relay set")
        })?;
        let relay_delivery_json = delivery_evidence
            .to_json_value()
            .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;
        let tags = relay_event_tags(event);
        let kind = relay_event_kind(event);
        let event_pubkey = event.pubkey.to_string();
        let listing_d_tag = relay_event_tag_value(&tags, "d", 1);
        let farm_id = direct_relay_event_farm_id(kind, &tags);
        let listing_addr =
            direct_relay_event_listing_addr(kind, &event_pubkey, listing_d_tag.as_deref());
        let created_at_ms = relay_event_created_at_ms(event)?;
        let local_seq = created_at_ms.saturating_add(i64::try_from(index).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "app relay ingest sequence must fit i64",
            }
        })?);
        records.push(LocalEventRecord {
            seq: local_seq,
            change_seq: local_seq,
            record_id: format!("app:relay_event:{event_id}"),
            family: LocalRecordFamily::SignedEvent,
            status: LocalRecordStatus::Published,
            source_runtime: direct_relay_event_source_runtime(kind, listing_d_tag.as_deref()),
            created_at_ms,
            inserted_at_ms,
            updated_at_ms: inserted_at_ms,
            owner_account_id: None,
            owner_pubkey: Some(event_pubkey.clone()),
            farm_id,
            listing_addr,
            local_work_json: None,
            event_id: Some(event_id),
            event_kind: Some(i64::from(kind)),
            event_pubkey: Some(event_pubkey),
            event_created_at: Some(relay_event_created_at_i64(event)?),
            event_tags_json: Some(json!(tags)),
            event_content: Some(event.content.clone()),
            event_sig: Some(event.sig.to_string()),
            raw_event_json: Some(relay_raw_event_json(event)?),
            outbox_status: PublishOutboxStatus::None,
            relay_set_fingerprint: Some(relay_set_fingerprint.clone()),
            relay_delivery_json: Some(relay_delivery_json.clone()),
        });
    }

    Ok(records)
}

fn direct_relay_event_farm_id(kind: u16, tags: &[Vec<String>]) -> Option<String> {
    match kind {
        kind if kind == KIND_FARM as u16 => relay_event_tag_value(tags, "d", 1),
        kind if kind == KIND_LISTING as u16 || kind == KIND_LISTING_DRAFT as u16 => {
            relay_event_tag_value(tags, "a", 1).and_then(|address| relay_address_d_tag(&address))
        }
        _ => None,
    }
}

fn direct_relay_event_listing_addr(
    kind: u16,
    event_pubkey: &str,
    listing_d_tag: Option<&str>,
) -> Option<String> {
    match kind {
        kind if kind == KIND_LISTING as u16 || kind == KIND_LISTING_DRAFT as u16 => {
            listing_d_tag.map(|d_tag| format!("{kind}:{event_pubkey}:{d_tag}"))
        }
        _ => None,
    }
}

fn direct_relay_event_source_runtime(_kind: u16, _d_tag: Option<&str>) -> SourceRuntime {
    SourceRuntime::Network
}

fn relay_event_kind(event: &RadrootsNostrEvent) -> u16 {
    event.kind.as_u16()
}

fn relay_event_created_at_i64(event: &RadrootsNostrEvent) -> Result<i64, AppSqliteError> {
    i64::try_from(event.created_at.as_secs()).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "app relay ingest event timestamp must fit i64",
    })
}

fn relay_event_created_at_unix_seconds_for_fetch(
    event: &RadrootsNostrEvent,
) -> Result<i64, AppSyncTransportError> {
    i64::try_from(event.created_at.as_secs())
        .map_err(|_| AppSyncTransportError::failed("app relay ingest event timestamp must fit i64"))
}

fn relay_event_created_at_ms(event: &RadrootsNostrEvent) -> Result<i64, AppSqliteError> {
    relay_event_created_at_i64(event)?
        .checked_mul(1_000)
        .ok_or(AppSqliteError::InvalidProjection {
            reason: "app relay ingest event timestamp milliseconds must fit i64",
        })
}

fn relay_event_tags(event: &RadrootsNostrEvent) -> Vec<Vec<String>> {
    event
        .tags
        .iter()
        .map(|tag| tag.as_slice().to_vec())
        .collect()
}

fn relay_event_tag_value(tags: &[Vec<String>], tag_name: &str, index: usize) -> Option<String> {
    tags.iter().find_map(|tag| {
        (tag.first().map(String::as_str) == Some(tag_name))
            .then(|| tag.get(index))
            .flatten()
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn relay_raw_event_json(event: &RadrootsNostrEvent) -> Result<serde_json::Value, AppSqliteError> {
    Ok(json!({
        "id": event.id.to_hex(),
        "pubkey": event.pubkey.to_string(),
        "created_at": relay_event_created_at_i64(event)?,
        "kind": u32::from(event.kind.as_u16()),
        "tags": relay_event_tags(event),
        "content": event.content.clone(),
        "sig": event.sig.to_string(),
    }))
}

fn relay_address_d_tag(address: &str) -> Option<String> {
    address
        .rsplit(':')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn product_status_needs_relay_publish(status: ProductStatus) -> bool {
    !matches!(status, ProductStatus::Draft)
}

fn listing_primary_bin_id(listing_d_tag: &str) -> String {
    format!("{listing_d_tag}:primary")
}

fn listing_availability_window_times(
    draft: &ProductEditorDraft,
    farm_rules: &FarmRulesProjection,
) -> (Option<String>, Option<String>) {
    draft
        .availability_window_id
        .and_then(|window_id| {
            farm_rules
                .fulfillment_windows
                .iter()
                .find(|window| window.fulfillment_window_id == window_id)
        })
        .map(|window| (Some(window.starts_at.clone()), Some(window.ends_at.clone())))
        .unwrap_or((None, None))
}

fn listing_fulfillment_method(
    draft: &ProductEditorDraft,
    farm_setup: &FarmSetupProjection,
    farm_rules: &FarmRulesProjection,
) -> Option<String> {
    if draft.availability_window_id.is_some_and(|window_id| {
        farm_rules
            .fulfillment_windows
            .iter()
            .any(|window| window.fulfillment_window_id == window_id)
    }) {
        return Some(FarmOrderMethod::Pickup.storage_key().to_owned());
    }

    farm_setup
        .draft
        .order_methods
        .iter()
        .next()
        .map(|method| method.storage_key().to_owned())
}

fn listing_fulfillment_location(
    draft: &ProductEditorDraft,
    farm_setup: &FarmSetupProjection,
    farm_rules: &FarmRulesProjection,
) -> Option<String> {
    draft
        .availability_window_id
        .and_then(|window_id| {
            farm_rules
                .fulfillment_windows
                .iter()
                .find(|window| window.fulfillment_window_id == window_id)
        })
        .and_then(|window| {
            farm_rules
                .pickup_locations
                .iter()
                .find(|location| location.pickup_location_id == window.pickup_location_id)
        })
        .and_then(|location| {
            non_empty_string(location.address_line.as_str())
                .or_else(|| non_empty_string(location.label.as_str()))
        })
        .or_else(|| non_empty_string(farm_setup.draft.location_or_service_area.as_str()))
}

fn direct_relay_sdk_client(
    relay_urls: Vec<String>,
    timeout_ms: u64,
) -> Result<RadrootsSdkClient, AppSyncTransportError> {
    let mut config = RadrootsSdkConfig::for_environment(SdkEnvironment::Custom);
    config.transport = SdkTransportMode::RelayDirect;
    config.signer = SignerConfig::LocalIdentity;
    config.relay = RelayConfig { urls: relay_urls };
    config.network.timeout_ms = timeout_ms;
    RadrootsSdkClient::from_config(config)
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))
}

fn publish_app_payload_sync(
    client: &RadrootsSdkClient,
    identity: &RadrootsIdentity,
    payload: &AppPublishPayload,
    configured_relay_urls: &[String],
) -> Result<SdkPublishReceipt, AppSyncTransportError> {
    let runtime = TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?;
    runtime.block_on(async {
        publish_app_payload(client, identity, payload, configured_relay_urls).await
    })
}

async fn publish_app_payload(
    client: &RadrootsSdkClient,
    identity: &RadrootsIdentity,
    payload: &AppPublishPayload,
    configured_relay_urls: &[String],
) -> Result<SdkPublishReceipt, AppSyncTransportError> {
    match payload {
        AppPublishPayload::FarmProfile(payload) => {
            let farm = RadrootsFarm {
                d_tag: d_tag_from_uuid(payload.farm_id.as_uuid()),
                name: payload.display_name.trim().to_owned(),
                about: None,
                website: None,
                picture: None,
                banner: None,
                location: None,
                tags: payload.readiness.map(|readiness| match readiness {
                    FarmReadiness::Incomplete => {
                        vec!["radroots:readiness:incomplete".to_owned()]
                    }
                    FarmReadiness::Ready => vec!["radroots:readiness:ready".to_owned()],
                }),
            };
            client
                .farm()
                .publish_with_identity(identity, &farm)
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::Listing(payload) => {
            let listing = listing_publish_payload_to_sdk_listing(payload)?;
            client
                .listing()
                .publish_with_identity(identity, &listing)
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderRequest(payload) => {
            let listing_event = order_request_listing_event_ptr(payload, configured_relay_urls)?;
            let order = order_request_publish_payload_to_sdk_order(payload)?;
            client
                .trade()
                .publish_order_request_with_identity(identity, &listing_event, &order)
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderDecision(payload) => {
            let decision = order_decision_publish_payload_to_sdk_decision(payload);
            client
                .trade()
                .publish_order_decision_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.request_event_id.as_str(),
                    &decision,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderRevisionProposal(payload) => {
            let proposal = order_revision_proposal_publish_payload_to_sdk_revision(payload);
            client
                .trade()
                .publish_order_revision_proposal_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.prev_event_id.as_str(),
                    &proposal,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderRevisionDecision(payload) => {
            let decision =
                order_revision_decision_publish_payload_to_sdk_revision_decision(payload);
            client
                .trade()
                .publish_order_revision_decision_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.prev_event_id.as_str(),
                    &decision,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderCancellation(payload) => {
            let cancellation = order_cancellation_publish_payload_to_sdk_cancellation(payload);
            client
                .trade()
                .publish_order_cancellation_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.prev_event_id.as_str(),
                    &cancellation,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderFulfillment(payload) => {
            let fulfillment = order_fulfillment_publish_payload_to_sdk_fulfillment(payload);
            client
                .trade()
                .publish_fulfillment_update_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.prev_event_id.as_str(),
                    &fulfillment,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
        AppPublishPayload::OrderReceipt(payload) => {
            let receipt = order_receipt_publish_payload_to_sdk_receipt(payload);
            client
                .trade()
                .publish_buyer_receipt_with_identity(
                    identity,
                    payload.request_event_id.as_str(),
                    payload.prev_event_id.as_str(),
                    &receipt,
                )
                .await
                .map_err(|error| AppSyncTransportError::failed(error.to_string()))
        }
    }
}

fn listing_publish_payload_to_sdk_listing(
    payload: &AppListingPublishPayload,
) -> Result<RadrootsListing, AppSyncTransportError> {
    let currency = payload
        .price_currency
        .parse::<RadrootsCoreCurrency>()
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?;
    let unit = parse_app_listing_unit(payload.unit_label.as_str())?;
    let price_minor_units = payload.price_minor_units.ok_or_else(|| {
        AppSyncTransportError::failed("publishable listing requires price minor units")
    })?;
    let farm_id = payload
        .farm_id
        .ok_or_else(|| AppSyncTransportError::failed("publishable listing requires farm id"))?;
    let farm_pubkey = payload
        .farm_pubkey
        .as_deref()
        .ok_or_else(|| AppSyncTransportError::failed("publishable listing requires farm pubkey"))?
        .trim()
        .to_owned();
    let d_tag = payload
        .listing_d_tag
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| d_tag_from_uuid(payload.product_id.as_uuid()));
    let bin_id = listing_primary_bin_id(d_tag.as_str());

    Ok(RadrootsListing {
        d_tag,
        farm: RadrootsFarmRef {
            pubkey: farm_pubkey,
            d_tag: payload
                .farm_d_tag
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| d_tag_from_uuid(farm_id.as_uuid())),
        },
        product: RadrootsListingProduct {
            key: payload.product_id.to_string(),
            title: payload.title.trim().to_owned(),
            category: payload
                .category
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_owned(),
            summary: payload
                .subtitle
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned),
            process: None,
            lot: None,
            location: None,
            profile: None,
            year: None,
        },
        primary_bin_id: bin_id.clone(),
        bins: vec![RadrootsListingBin {
            bin_id,
            quantity: RadrootsCoreQuantity::new(RadrootsCoreDecimal::from(1u32), unit),
            price_per_canonical_unit: RadrootsCoreQuantityPrice::new(
                RadrootsCoreMoney::from_minor_units_u32(price_minor_units, currency),
                RadrootsCoreQuantity::new(RadrootsCoreDecimal::from(1u32), unit),
            ),
            display_amount: Some(RadrootsCoreDecimal::from(1u32)),
            display_unit: Some(unit),
            display_label: Some(payload.unit_label.trim().to_owned()),
            display_price: Some(RadrootsCoreMoney::from_minor_units_u32(
                price_minor_units,
                currency,
            )),
            display_price_unit: Some(unit),
        }],
        resource_area: None,
        plot: None,
        discounts: None,
        inventory_available: payload.stock_quantity.map(RadrootsCoreDecimal::from),
        availability: listing_publish_payload_availability(payload)?,
        delivery_method: Some(parse_app_listing_delivery_method(
            payload.fulfillment_method.as_deref().unwrap_or_default(),
        )?),
        location: payload
            .fulfillment_location
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|primary| RadrootsListingLocation {
                primary: primary.trim().to_owned(),
                city: None,
                region: None,
                country: None,
                lat: None,
                lng: None,
                geohash: None,
            }),
        images: None,
    })
}

fn listing_publish_payload_availability(
    payload: &AppListingPublishPayload,
) -> Result<Option<RadrootsListingAvailability>, AppSyncTransportError> {
    if payload.status == ProductStatus::Published {
        let start = parse_listing_availability_timestamp(
            payload.availability_starts_at.as_deref(),
            "publishable listing requires availability start",
        )?;
        let end = parse_listing_availability_timestamp(
            payload.availability_ends_at.as_deref(),
            "publishable listing requires availability end",
        )?;
        if end <= start {
            return Err(AppSyncTransportError::failed(
                "publishable listing availability end must be after start",
            ));
        }
        return Ok(Some(RadrootsListingAvailability::Window {
            start: Some(start),
            end: Some(end),
        }));
    }

    Ok(Some(RadrootsListingAvailability::Status {
        status: match payload.status {
            ProductStatus::Archived => RadrootsListingStatus::Sold,
            other => RadrootsListingStatus::Other {
                value: other.storage_key().to_owned(),
            },
        },
    }))
}

fn parse_listing_availability_timestamp(
    value: Option<&str>,
    missing_message: &'static str,
) -> Result<u64, AppSyncTransportError> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppSyncTransportError::failed(missing_message))?;
    let timestamp = DateTime::parse_from_rfc3339(value)
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))?
        .timestamp();
    u64::try_from(timestamp)
        .map_err(|_| AppSyncTransportError::failed("listing availability timestamp is negative"))
}

fn parse_app_listing_unit(value: &str) -> Result<RadrootsCoreUnit, AppSyncTransportError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "each" | "ea" | "unit" | "units" => Ok(RadrootsCoreUnit::Each),
        "kg" | "kilogram" | "kilograms" => Ok(RadrootsCoreUnit::MassKg),
        "g" | "gram" | "grams" => Ok(RadrootsCoreUnit::MassG),
        "oz" | "ounce" | "ounces" => Ok(RadrootsCoreUnit::MassOz),
        "lb" | "pound" | "pounds" => Ok(RadrootsCoreUnit::MassLb),
        "l" | "liter" | "liters" => Ok(RadrootsCoreUnit::VolumeL),
        "ml" | "milliliter" | "milliliters" => Ok(RadrootsCoreUnit::VolumeMl),
        other => Err(AppSyncTransportError::failed(format!(
            "unsupported listing unit `{other}`"
        ))),
    }
}

fn parse_app_listing_delivery_method(
    value: &str,
) -> Result<RadrootsListingDeliveryMethod, AppSyncTransportError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pickup" | "local_pickup" => Ok(RadrootsListingDeliveryMethod::Pickup),
        "delivery" | "local_delivery" => Ok(RadrootsListingDeliveryMethod::LocalDelivery),
        "shipping" | "ship" => Ok(RadrootsListingDeliveryMethod::Shipping),
        "" => Err(AppSyncTransportError::failed(
            "publishable listing requires fulfillment method",
        )),
        other => Ok(RadrootsListingDeliveryMethod::Other {
            method: other.to_owned(),
        }),
    }
}

fn order_request_listing_event_ptr(
    payload: &AppOrderRequestPublishPayload,
    configured_relay_urls: &[String],
) -> Result<RadrootsNostrEventPtr, AppSyncTransportError> {
    let listing_event_id = payload
        .listing_event_id
        .as_deref()
        .ok_or_else(|| {
            AppSyncTransportError::failed("order request publish requires listing event id")
        })?
        .trim()
        .to_owned();
    let listing_relay = selected_listing_relay(&payload.listing_relays, configured_relay_urls)?;

    Ok(RadrootsNostrEventPtr {
        id: listing_event_id,
        relays: Some(listing_relay),
    })
}

fn selected_listing_relay(
    listing_relays: &[String],
    configured_relay_urls: &[String],
) -> Result<String, AppSyncTransportError> {
    let mut seen = BTreeSet::new();
    let mut known_relays = Vec::new();
    for relay in listing_relays {
        let relay = relay.trim();
        if !relay.is_empty() && seen.insert(relay.to_owned()) {
            known_relays.push(relay.to_owned());
        }
    }
    if known_relays.is_empty() {
        return Err(AppSyncTransportError::failed(
            "order request publish requires listing relay",
        ));
    }
    for configured_relay in configured_relay_urls {
        let configured_relay = configured_relay.trim();
        if !configured_relay.is_empty()
            && known_relays.iter().any(|relay| relay == configured_relay)
        {
            return Ok(configured_relay.to_owned());
        }
    }
    Err(missing_listing_provenance_relay_error(&known_relays))
}

fn missing_listing_provenance_relay_error(known_relays: &[String]) -> AppSyncTransportError {
    AppSyncTransportError::failed(
        json!({
            "code": "missing_listing_provenance_relay",
            "missing_provenance_relays": known_relays,
        })
        .to_string(),
    )
}

fn order_request_publish_payload_to_sdk_order(
    payload: &AppOrderRequestPublishPayload,
) -> Result<RadrootsTradeOrderRequested, AppSyncTransportError> {
    let Some(document_json) = payload.order_document_json.as_ref() else {
        return Err(AppSyncTransportError::failed(
            "order request publish requires order document",
        ));
    };
    let order_json = document_json
        .pointer("/document/order")
        .or_else(|| document_json.get("order"))
        .unwrap_or(document_json);
    serde_json::from_value::<RadrootsTradeOrderRequested>(order_json.clone())
        .map_err(|error| AppSyncTransportError::failed(error.to_string()))
}

fn published_operation_receipt(
    operation_key: &str,
    payload: &AppPublishPayload,
    receipt: SdkPublishReceipt,
) -> Result<AppPublishedOperationReceipt, AppSyncTransportError> {
    let SdkTransportReceipt::RelayDirect(relay_receipt) = receipt.transport_receipt else {
        return Err(AppSyncTransportError::failed(
            "direct relay app sync received non-relay receipt",
        ));
    };
    let (source_account_id, source_local_event_id, listing_addr) = match payload {
        AppPublishPayload::FarmProfile(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            None,
        ),
        AppPublishPayload::Listing(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            None,
        ),
        AppPublishPayload::OrderRequest(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            payload.listing_addr.clone(),
        ),
        AppPublishPayload::OrderDecision(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
        AppPublishPayload::OrderRevisionProposal(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
        AppPublishPayload::OrderRevisionDecision(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
        AppPublishPayload::OrderCancellation(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
        AppPublishPayload::OrderFulfillment(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
        AppPublishPayload::OrderReceipt(payload) => (
            payload.context.account_id.clone(),
            payload.context.source_local_event_id.clone(),
            Some(payload.listing_addr.clone()),
        ),
    };
    let failed_relays = relay_receipt
        .failed_relays
        .iter()
        .map(|failure| {
            RelayDeliveryFailure::new(failure.relay_url.as_str(), failure.error.as_str())
                .map_err(|source| AppSyncTransportError::failed(source.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let delivery_evidence = RelayDeliveryEvidence::acknowledged(
        &relay_receipt.target_relays,
        &relay_receipt.connected_relays,
        &relay_receipt.acknowledged_relays,
        failed_relays,
    )
    .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;
    let relay_set_fingerprint = delivery_evidence.relay_set_fingerprint().ok_or_else(|| {
        AppSyncTransportError::failed("direct relay publish requires a non-empty relay set")
    })?;
    let relay_delivery_json = delivery_evidence
        .to_json_value()
        .map_err(|source| AppSyncTransportError::failed(source.to_string()))?;
    let raw_event_json = json!({
        "id": relay_receipt.event.id.clone(),
        "pubkey": relay_receipt.event.author.clone(),
        "created_at": relay_receipt.event.created_at,
        "kind": relay_receipt.event.kind,
        "tags": relay_receipt.event.tags.clone(),
        "content": relay_receipt.event.content.clone(),
        "sig": relay_receipt.event.sig.clone(),
    });

    Ok(AppPublishedOperationReceipt {
        operation_key: operation_key.to_owned(),
        source_account_id,
        source_local_event_id,
        listing_addr,
        event_id: relay_receipt.event_id,
        event_kind: relay_receipt.event_kind,
        event_pubkey: relay_receipt.event.author.clone(),
        event_created_at: relay_receipt.event.created_at,
        event_tags_json: json!(relay_receipt.event.tags),
        event_content: relay_receipt.event.content.clone(),
        event_sig: relay_receipt.signature,
        raw_event_json,
        relay_set_fingerprint,
        relay_delivery_json,
    })
}

fn d_tag_from_uuid(uuid: Uuid) -> String {
    base64_url_no_pad(uuid.as_bytes())
}

fn signed_event_farm_id(receipt: &AppPublishedOperationReceipt) -> Option<String> {
    match receipt.event_kind {
        KIND_FARM => signed_event_tag_value(&receipt.event_tags_json, "d", 1),
        KIND_LISTING => signed_event_tag_value(&receipt.event_tags_json, "a", 1)
            .and_then(|address| signed_event_address_d_tag(address.as_str())),
        _ => None,
    }
}

fn signed_event_listing_addr(receipt: &AppPublishedOperationReceipt) -> Option<String> {
    if receipt.event_kind != KIND_LISTING {
        return None;
    }
    let pubkey = receipt.event_pubkey.trim();
    if pubkey.is_empty() {
        return None;
    }
    signed_event_tag_value(&receipt.event_tags_json, "d", 1)
        .map(|d_tag| format!("{KIND_LISTING}:{pubkey}:{d_tag}"))
}

fn signed_event_tag_value(
    tags: &serde_json::Value,
    tag_name: &str,
    index: usize,
) -> Option<String> {
    tags.as_array()?.iter().find_map(|tag| {
        let values = tag.as_array()?;
        (values.first()?.as_str()? == tag_name)
            .then(|| values.get(index).and_then(serde_json::Value::as_str))
            .flatten()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn signed_event_address_d_tag(address: &str) -> Option<String> {
    address
        .rsplit(':')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut chunks = bytes.chunks_exact(3);
    for chunk in &mut chunks {
        output.push(ALPHABET[(chunk[0] >> 2) as usize] as char);
        output.push(ALPHABET[(((chunk[0] & 0b0000_0011) << 4) | (chunk[1] >> 4)) as usize] as char);
        output.push(ALPHABET[(((chunk[1] & 0b0000_1111) << 2) | (chunk[2] >> 6)) as usize] as char);
        output.push(ALPHABET[(chunk[2] & 0b0011_1111) as usize] as char);
    }
    match chunks.remainder() {
        [one] => {
            output.push(ALPHABET[(one >> 2) as usize] as char);
            output.push(ALPHABET[((one & 0b0000_0011) << 4) as usize] as char);
        }
        [one, two] => {
            output.push(ALPHABET[(one >> 2) as usize] as char);
            output.push(ALPHABET[(((one & 0b0000_0011) << 4) | (two >> 4)) as usize] as char);
            output.push(ALPHABET[((two & 0b0000_1111) << 2) as usize] as char);
        }
        [] => {}
        _ => {}
    }
    output
}

fn decimal_from_minor_units(value: u32) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

fn normalize_currency_code(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "USD".to_owned()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

fn is_hex_64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn local_work_exportability(owner_pubkey: Option<&str>) -> serde_json::Value {
    match owner_pubkey {
        Some(_) => json!({
            "state": "exportable"
        }),
        None => json!({
            "state": "identity_unresolved",
            "reason": "canonical_hex_pubkey_required"
        }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AppBuyerOrderRequestExport {
    buyer_pubkey: Option<String>,
    seller_pubkey: Option<String>,
    listing_addr: Option<String>,
    listing_event_id: Option<String>,
    listing_relays: Vec<String>,
    farm_key: Option<String>,
    order_items: Vec<serde_json::Value>,
    line_refs: Vec<serde_json::Value>,
    economics: Option<serde_json::Value>,
    support_issues: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct AppOrderLocalWorkPublishSource {
    record_id: String,
    payload: serde_json::Value,
}

impl AppBuyerOrderRequestExport {
    fn from_order(
        order: &BuyerOrderLocalEventExport,
        buyer_pubkey: Option<&str>,
    ) -> Result<Self, AppSqliteError> {
        let mut support_issues = Vec::new();
        if buyer_pubkey.is_none() {
            support_issues.push("buyer_pubkey_required");
        }
        if order.lines.is_empty() {
            support_issues.push("order_lines_required");
        }
        let listing_addr =
            shared_optional_line_value(&order.lines, |line| line.listing_addr.as_deref());
        let listing_event_id =
            shared_optional_line_value(&order.lines, |line| line.listing_event_id.as_deref());
        let listing_relays = shared_listing_relays(&order.lines);
        let seller_pubkey =
            shared_optional_line_value(&order.lines, |line| line.seller_pubkey.as_deref());
        let farm_key = shared_optional_line_value(&order.lines, |line| line.farm_key.as_deref())
            .or_else(|| Some(d_tag_from_uuid(order.farm_id.as_uuid())));

        if listing_addr.is_none() {
            support_issues.push("single_listing_addr_required");
        }
        if listing_event_id.is_none() {
            support_issues.push("listing_event_id_required");
        }
        if listing_relays.is_empty() {
            support_issues.push("listing_relays_required");
        }
        if seller_pubkey.is_none() {
            support_issues.push("seller_pubkey_required");
        }

        let mut order_items = Vec::with_capacity(order.lines.len());
        let mut line_refs = Vec::with_capacity(order.lines.len());
        for line in &order.lines {
            let line_bin_id = line
                .listing_bin_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty());
            if line_bin_id.is_none() && !support_issues.contains(&"listing_bin_id_required") {
                support_issues.push("listing_bin_id_required");
            }
            order_items.push(json!({
                "bin_id": line_bin_id.unwrap_or_default(),
                "bin_count": line.quantity,
            }));
            line_refs.push(json!({
                "product_id": line.product_id.to_string(),
                "title": line.title,
                "quantity": {
                    "count": line.quantity,
                    "display": line.quantity_display,
                    "unit_label": line.quantity_unit_label,
                },
                "listing_addr": line.listing_addr,
                "listing_event_id": line.listing_event_id,
                "listing_relays": line.listing_relays,
                "listing_bin_id": line.listing_bin_id,
                "seller_pubkey": line.seller_pubkey,
                "farm_key": line.farm_key,
            }));
        }

        let economics = order_economics_json(order, &mut support_issues)?;

        Ok(Self {
            buyer_pubkey: buyer_pubkey.map(str::to_owned),
            seller_pubkey,
            listing_addr,
            listing_event_id,
            listing_relays,
            farm_key,
            order_items,
            line_refs,
            economics,
            support_issues,
        })
    }

    fn is_supported(&self) -> bool {
        self.support_issues.is_empty()
    }
}

fn buyer_order_request_local_work_payload(
    order: &BuyerOrderLocalEventExport,
    buyer_context: &BuyerContext,
    record_id: &str,
    export: &AppBuyerOrderRequestExport,
    timestamp: i64,
) -> serde_json::Value {
    let buyer_account_id = match buyer_context {
        BuyerContext::Account(account_id) => account_id.as_str(),
        BuyerContext::Guest => "",
    };
    let buyer_actor_source = if export.buyer_pubkey.is_some() {
        BUYER_ORDER_REQUEST_ACTOR_SOURCE_RESOLVED_ACCOUNT
    } else {
        BUYER_ORDER_REQUEST_ACTOR_SOURCE_UNRESOLVED_APP
    };

    json!({
        "record_kind": BUYER_ORDER_REQUEST_LOCAL_WORK_RECORD_KIND,
        "scope": "app",
        "exportability": local_work_exportability(export.buyer_pubkey.as_deref()),
        "support_status": {
            "state": if export.is_supported() { "supported" } else { "unsupported" },
            "issues": export.support_issues.clone(),
        },
        "currentness": {
            "current": true,
            "source": "app_sqlite_order",
            "record_id": record_id,
            "order_id": order.order_id.to_string(),
            "order_updated_at": order.updated_at,
            "created_at_ms": timestamp,
        },
        "payment_display": {
            "state": "not_recorded",
            "allows_payment_action": false,
        },
        "document": {
            "version": 1,
            "kind": BUYER_ORDER_REQUEST_DOCUMENT_KIND,
            "order": {
                "order_id": order.order_id.to_string(),
                "listing_addr": export.listing_addr.as_deref().unwrap_or_default(),
                "listing_event_id": export.listing_event_id.as_deref().unwrap_or_default(),
                "listing_relays": export.listing_relays.clone(),
                "buyer_pubkey": export.buyer_pubkey.as_deref().unwrap_or_default(),
                "seller_pubkey": export.seller_pubkey.as_deref().unwrap_or_default(),
                "items": export.order_items.clone(),
                "economics": export.economics.clone(),
            },
            "buyer_actor": {
                "account_id": buyer_account_id,
                "pubkey": export.buyer_pubkey.as_deref().unwrap_or_default(),
                "source": buyer_actor_source,
            },
            "listing_lookup": export.listing_addr.clone(),
        },
        "app_order": {
            "order_id": order.order_id.to_string(),
            "order_number": order.order_number,
            "farm_id": order.farm_id.to_string(),
            "farm_display_name": order.farm_display_name,
            "farm_key": export.farm_key.clone(),
            "status": order.status,
            "buyer_context_key": order.buyer_context_key,
            "buyer_name": order.buyer_name,
            "buyer_email": order.buyer_email,
            "buyer_phone": order.buyer_phone,
            "buyer_order_note": order.buyer_order_note,
            "fulfillment": {
                "window_id": order.fulfillment_window_id.map(|id| id.to_string()),
                "label": order.fulfillment_window_label,
                "starts_at": order.fulfillment_starts_at,
                "ends_at": order.fulfillment_ends_at,
            },
            "lines": export.line_refs.clone(),
        },
    })
}

fn order_economics_json(
    order: &BuyerOrderLocalEventExport,
    support_issues: &mut Vec<&'static str>,
) -> Result<Option<serde_json::Value>, AppSqliteError> {
    let mut economics_items = Vec::with_capacity(order.lines.len());
    let mut subtotal_minor_units = 0_u32;
    let mut currency = None::<String>;

    for line in &order.lines {
        let line_bin_id = line
            .listing_bin_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if line_bin_id.is_none() && !support_issues.contains(&"listing_bin_id_required") {
            support_issues.push("listing_bin_id_required");
            continue;
        }
        let Some(quantity_unit) = canonical_quantity_unit(line.quantity_unit_label.as_str()) else {
            support_issues.push("canonical_quantity_unit_required");
            continue;
        };
        let Some(unit_price_minor_units) = line.unit_price_minor_units else {
            support_issues.push("unit_price_required");
            continue;
        };
        if unit_price_minor_units == 0 {
            support_issues.push("positive_unit_price_required");
            continue;
        }
        let line_currency = normalize_currency_code(line.price_currency.as_str());
        if line_currency.len() != 3 || !line_currency.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            support_issues.push("canonical_currency_required");
            continue;
        }
        if let Some(existing_currency) = currency.as_deref() {
            if existing_currency != line_currency {
                support_issues.push("single_currency_required");
                continue;
            }
        } else {
            currency = Some(line_currency.clone());
        }
        let line_subtotal_minor_units = unit_price_minor_units.checked_mul(line.quantity).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "buyer order local event line subtotal overflowed",
            },
        )?;
        subtotal_minor_units = subtotal_minor_units
            .checked_add(line_subtotal_minor_units)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer order local event subtotal overflowed",
            })?;
        economics_items.push(json!({
            "bin_id": line_bin_id.unwrap_or_default(),
            "bin_count": line.quantity,
            "quantity_amount": "1",
            "quantity_unit": quantity_unit,
            "unit_price_amount": decimal_from_minor_units(unit_price_minor_units),
            "unit_price_currency": line_currency,
            "line_subtotal": {
                "amount": decimal_from_minor_units(line_subtotal_minor_units),
                "currency": line_currency,
            },
        }));
    }

    if economics_items.len() != order.lines.len() || economics_items.is_empty() {
        return Ok(None);
    }

    let currency = currency.unwrap_or_else(|| "USD".to_owned());
    let subtotal = json!({
        "amount": decimal_from_minor_units(subtotal_minor_units),
        "currency": currency,
    });
    Ok(Some(json!({
        "quote_id": format!("app-order:{}", order.order_id),
        "quote_version": 1,
        "pricing_basis": "listing_event",
        "currency": currency,
        "items": economics_items,
        "discounts": [],
        "adjustments": [],
        "subtotal": subtotal,
        "discount_total": {
            "amount": "0",
            "currency": currency,
        },
        "adjustment_total": {
            "amount": "0",
            "currency": currency,
        },
        "total": subtotal,
    })))
}

fn order_currency_and_total(
    order: &BuyerOrderLocalEventExport,
) -> Result<Option<(String, u32)>, AppSqliteError> {
    let mut currency = None::<String>;
    let mut total_minor_units = 0_u32;

    for line in &order.lines {
        let Some(unit_price_minor_units) = line.unit_price_minor_units else {
            return Ok(None);
        };
        let line_currency = normalize_currency_code(line.price_currency.as_str());
        if line_currency.len() != 3 || !line_currency.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Ok(None);
        }
        if let Some(existing_currency) = currency.as_deref() {
            if existing_currency != line_currency {
                return Ok(None);
            }
        } else {
            currency = Some(line_currency.clone());
        }
        let line_total = unit_price_minor_units.checked_mul(line.quantity).ok_or(
            AppSqliteError::InvalidProjection {
                reason: "buyer order publish line total overflowed",
            },
        )?;
        total_minor_units =
            total_minor_units
                .checked_add(line_total)
                .ok_or(AppSqliteError::InvalidProjection {
                    reason: "buyer order publish total overflowed",
                })?;
    }

    Ok(currency.map(|currency| (currency, total_minor_units)))
}

fn shared_optional_line_value(
    lines: &[BuyerOrderLocalEventLine],
    value: impl Fn(&BuyerOrderLocalEventLine) -> Option<&str>,
) -> Option<String> {
    let mut resolved = None::<String>;
    for line in lines {
        let Some(next) = value(line).map(str::trim).filter(|next| !next.is_empty()) else {
            return None;
        };
        if let Some(existing) = resolved.as_deref() {
            if existing != next {
                return None;
            }
        } else {
            resolved = Some(next.to_owned());
        }
    }
    resolved
}

fn shared_listing_relays(lines: &[BuyerOrderLocalEventLine]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut relays = Vec::new();
    for line in lines {
        for relay in &line.listing_relays {
            let relay = relay.trim();
            if !relay.is_empty() && seen.insert(relay.to_owned()) {
                relays.push(relay.to_owned());
            }
        }
    }
    relays
}

fn canonical_quantity_unit(unit_label: &str) -> Option<&'static str> {
    match unit_label.trim().to_ascii_lowercase().as_str() {
        "each" | "ea" | "count" => Some("each"),
        "kg" | "kilogram" | "kilograms" => Some("kg"),
        "g" | "gram" | "grams" => Some("g"),
        "oz" | "ounce" | "ounces" => Some("oz"),
        "lb" | "pound" | "pounds" => Some("lb"),
        "l" | "liter" | "litre" | "liters" | "litres" => Some("l"),
        "ml" | "milliliter" | "millilitre" | "milliliters" | "millilitres" => Some("ml"),
        _ => None,
    }
}

fn load_selected_account_context(
    sqlite_store: &AppSqliteStore,
    identity_projection: &AppIdentityProjection,
    continuity_state: &PersistedAppState,
) -> Result<DesktopSelectedAccountContext, AppSqliteError> {
    load_selected_account_context_with_options(
        sqlite_store,
        identity_projection,
        continuity_state,
        true,
    )
}

fn selected_buyer_order_scope(
    identity_projection: &AppIdentityProjection,
) -> SelectedBuyerOrderScope {
    let buyer_context = identity_projection.buyer_context();
    match &buyer_context {
        BuyerContext::Account(account_id) => SelectedBuyerOrderScope::for_selected_account(
            account_id,
            identity_projection
                .selected_account
                .as_ref()
                .and_then(selected_account_public_key_hex)
                .as_deref(),
        ),
        BuyerContext::Guest => SelectedBuyerOrderScope::from_buyer_context(&buyer_context),
    }
}

fn selected_account_public_key_hex(
    selected_account: &radroots_studio_app_view::SelectedAccountProjection,
) -> Option<String> {
    let npub = selected_account.account.npub.trim();
    radroots_nostr_parse_pubkey(npub)
        .ok()
        .map(|public_key| public_key.to_hex())
        .filter(|public_key| is_hex_64(public_key))
        .or_else(|| {
            let account_id = selected_account.account.account_id.trim();
            is_hex_64(account_id).then(|| account_id.to_owned())
        })
}

fn load_selected_account_context_with_options(
    sqlite_store: &AppSqliteStore,
    identity_projection: &AppIdentityProjection,
    continuity_state: &PersistedAppState,
    allow_auto_present: bool,
) -> Result<DesktopSelectedAccountContext, AppSqliteError> {
    let buyer_context = identity_projection.buyer_context();
    let buyer_order_scope = selected_buyer_order_scope(identity_projection);
    let browse_fulfillment_methods = BTreeSet::new();
    let browse_listings = sqlite_store.load_buyer_listings("", &browse_fulfillment_methods)?;
    let search_query = continuity_state.buyer.search_query.clone();
    let search_listings = sqlite_store.load_buyer_listings(
        &search_query.search_query,
        &search_query.fulfillment_methods,
    )?;
    let browse_detail = match continuity_state.buyer.browse_detail_product_id {
        Some(product_id) => sqlite_store.load_buyer_product_detail(product_id)?,
        None => None,
    };
    let search_detail = match continuity_state.buyer.search_detail_product_id {
        Some(product_id) => sqlite_store.load_buyer_product_detail(product_id)?,
        None => None,
    };
    let buyer_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
    let buyer_order_review = sqlite_store.load_buyer_order_review(&buyer_context)?;
    let buyer_orders = sqlite_store.load_buyer_orders_for_scope(&buyer_order_scope)?;
    let has_recoverable_coordination = !sqlite_store
        .load_recoverable_buyer_order_coordination_records(&buyer_context)?
        .is_empty();
    let buyer_order_detail = match continuity_state.buyer.orders_detail_order_id {
        Some(order_id) => {
            sqlite_store.load_buyer_order_detail_for_scope(&buyer_order_scope, order_id)?
        }
        None => None,
    };
    let personal_projection = PersonalWorkspaceProjection {
        browse: BuyerBrowseScreenProjection {
            listings: browse_listings,
            detail: browse_detail,
        },
        search: BuyerSearchScreenProjection {
            query: search_query,
            listings: search_listings,
            detail: search_detail,
        },
        cart: BuyerCartScreenProjection {
            cart: buyer_cart,
            order_review: buyer_order_review,
        },
        orders: BuyerOrdersScreenProjection {
            list: buyer_orders,
            detail: buyer_order_detail,
            has_recoverable_coordination,
        },
        ..PersonalWorkspaceProjection::default()
    };
    let Some(selected_account) = identity_projection.selected_account.as_ref() else {
        return Ok(DesktopSelectedAccountContext {
            personal_projection,
            products_query: ProductsScreenQueryState::default(),
            orders_list: OrdersListProjection::default(),
            orders_query: OrdersScreenQueryState::default(),
            orders_reminders: ReminderFeedProjection::default(),
            recovery_queue: RecoveryQueueProjection::default(),
            pack_day_query: PackDayScreenQueryState::default(),
            product_editor_draft: None,
            reminder_log: ReminderLogProjection::default(),
            ..DesktopSelectedAccountContext::default()
        });
    };
    let farm_setup_projection =
        sqlite_store.load_farm_setup(&selected_account.account.account_id)?;
    let today_farm_id = selected_farm_id_from_context(identity_projection, &farm_setup_projection);
    let (
        farm_rules_projection,
        mut today_projection,
        products_query,
        products_list,
        orders_query,
        orders_list,
        canonical_orders_list,
        mut order_detail,
        pack_day_query,
        mut pack_day_projection,
        product_editor_draft,
    ) = match today_farm_id {
        Some(farm_id) => {
            let fallback_profile =
                fallback_farm_profile_for_projection(farm_id, &farm_setup_projection);
            let farm_rules_projection =
                sqlite_store.load_farm_rules(farm_id).map(|projection| {
                    prepare_loaded_farm_rules_projection(projection, &fallback_profile)
                })?;
            let today_projection = sqlite_store.load_today_agenda(Some(farm_id))?;
            let products_query = continuity_state.seller.products_query.clone();
            let products_list = sqlite_store.load_products(
                farm_id,
                &products_query.search_query,
                products_query.filter,
                products_query.sort,
            )?;
            let orders_query = sanitize_orders_query(
                sqlite_store,
                farm_id,
                continuity_state.seller.orders_query.clone(),
            )?;
            let orders_list = sqlite_store.load_orders_list(farm_id, &orders_query)?;
            let canonical_orders_list =
                sqlite_store.load_orders_list(farm_id, &OrdersScreenQueryState::default())?;
            let order_detail = match continuity_state.seller.order_detail_order_id {
                Some(order_id) => sqlite_store.load_order_detail(farm_id, order_id)?,
                None => None,
            };
            let (pack_day_query, pack_day_projection) = sanitize_pack_day_query(
                sqlite_store,
                farm_id,
                continuity_state.seller.pack_day_query.clone(),
            )?;
            let product_editor_draft = if matches!(
                continuity_state.shell.selected_section,
                ShellSection::Farmer(FarmerSection::Products)
            ) {
                match continuity_state.seller.product_editor_product_id {
                    Some(product_id) => sqlite_store
                        .load_product_editor_draft(product_id)?
                        .map(|draft| (product_id, draft)),
                    None => None,
                }
            } else {
                None
            };

            (
                farm_rules_projection,
                today_projection,
                products_query,
                products_list,
                orders_query,
                orders_list,
                canonical_orders_list,
                order_detail,
                pack_day_query,
                pack_day_projection,
                product_editor_draft,
            )
        }
        None => (
            FarmRulesProjection::default(),
            TodayAgendaProjection::default(),
            ProductsScreenQueryState::default(),
            ProductsListProjection::default(),
            OrdersScreenQueryState::default(),
            OrdersListProjection::default(),
            OrdersListProjection::default(),
            None,
            PackDayScreenQueryState::default(),
            PackDayProjection::default(),
            None,
        ),
    };
    let (orders_reminders, recovery_queue, reminder_log) = match today_farm_id {
        Some(farm_id) => {
            let reminder_context = load_selected_account_reminder_context_with_options(
                sqlite_store,
                selected_account.account.account_id.as_str(),
                farm_id,
                &today_projection,
                &canonical_orders_list,
                &pack_day_projection,
                order_detail.as_ref(),
                allow_auto_present,
            )?;
            today_projection.reminders = reminder_context.today_feed;
            if let Some(summary) = today_projection.summary.as_mut() {
                summary.reminders_due_soon = reminder_context.due_soon_count;
                summary.recovery_actions_open = reminder_context.recovery_actions_open;
            }
            if let Some(detail) = order_detail.as_mut() {
                detail.recoveries = reminder_context.selected_order_recoveries;
            }
            pack_day_projection.reminders = reminder_context.pack_day_feed;

            (
                reminder_context.orders_feed,
                reminder_context.recovery_queue,
                reminder_context.reminder_log,
            )
        }
        None => (
            ReminderFeedProjection::default(),
            RecoveryQueueProjection::default(),
            ReminderLogProjection::default(),
        ),
    };

    Ok(DesktopSelectedAccountContext {
        personal_projection,
        farm_setup_projection,
        farm_rules_projection,
        today_projection,
        products_query,
        products_list,
        orders_query,
        orders_list,
        orders_reminders,
        recovery_queue,
        reminder_log,
        order_detail,
        pack_day_query,
        pack_day_projection,
        product_editor_draft,
    })
}

fn sanitize_orders_query(
    sqlite_store: &AppSqliteStore,
    farm_id: FarmId,
    query: OrdersScreenQueryState,
) -> Result<OrdersScreenQueryState, AppSqliteError> {
    let Some(fulfillment_window_id) = query.fulfillment_window_id else {
        return Ok(query);
    };
    let pack_day = sqlite_store.load_pack_day(
        farm_id,
        &PackDayScreenQueryState {
            fulfillment_window_id: Some(fulfillment_window_id),
        },
    )?;
    if pack_day
        .fulfillment_window
        .as_ref()
        .map(|window| window.fulfillment_window_id)
        == Some(fulfillment_window_id)
    {
        Ok(query)
    } else {
        Ok(OrdersScreenQueryState {
            filter: query.filter,
            fulfillment_window_id: None,
        })
    }
}

fn sanitize_pack_day_query(
    sqlite_store: &AppSqliteStore,
    farm_id: FarmId,
    query: PackDayScreenQueryState,
) -> Result<(PackDayScreenQueryState, PackDayProjection), AppSqliteError> {
    let projection = sqlite_store.load_pack_day(farm_id, &query)?;
    if query.fulfillment_window_id.is_none()
        || projection
            .fulfillment_window
            .as_ref()
            .map(|window| window.fulfillment_window_id)
            == query.fulfillment_window_id
    {
        return Ok((query, projection));
    }

    let default_query = PackDayScreenQueryState::default();
    let default_projection = sqlite_store.load_pack_day(farm_id, &default_query)?;

    Ok((default_query, default_projection))
}

fn load_selected_account_reminder_context_with_options(
    sqlite_store: &AppSqliteStore,
    account_id: &str,
    farm_id: FarmId,
    today_projection: &TodayAgendaProjection,
    canonical_orders_list: &OrdersListProjection,
    pack_day_projection: &PackDayProjection,
    selected_order_detail: Option<&OrderDetailProjection>,
    allow_auto_present: bool,
) -> Result<DesktopSellerReminderContext, AppSqliteError> {
    let existing_schedule = sqlite_store.load_reminder_schedule(account_id, farm_id)?;
    let recovery_queue = sqlite_store.load_recovery_queue(account_id, farm_id)?;
    let sync_truth = load_selected_account_reminder_sync_truth(sqlite_store, account_id)?;
    let mut schedule = derive_selected_account_reminder_schedule(
        farm_id,
        today_projection,
        canonical_orders_list,
        pack_day_projection,
        &recovery_queue,
        &sync_truth,
        &existing_schedule,
    );
    let mut reminder_log_entries =
        reconcile_resolved_reminder_log_entries(&existing_schedule, &schedule);
    promote_desktop_reminder_presentation(
        &mut schedule,
        &mut reminder_log_entries,
        allow_auto_present,
    );
    if schedule != existing_schedule || !reminder_log_entries.is_empty() {
        sqlite_store.apply_reminder_schedule_update(
            account_id,
            farm_id,
            &schedule,
            &reminder_log_entries,
        )?;
    }
    let reminder_log = sqlite_store.load_reminder_log(account_id, farm_id, 8)?;

    let selected_order_recoveries = selected_order_detail
        .map(|detail| ordered_order_recoveries_for_detail(&recovery_queue, detail.order_id))
        .unwrap_or_default();
    let due_soon_count = schedule
        .items
        .iter()
        .filter(|item| {
            !matches!(
                item.kind,
                ReminderKind::MissedPickupRecovery | ReminderKind::RefundRecovery
            ) && matches!(
                item.urgency,
                ReminderUrgency::DueSoon | ReminderUrgency::Overdue | ReminderUrgency::Blocking
            )
        })
        .count() as u32;
    let recovery_actions_open = recovery_queue
        .items
        .iter()
        .filter(|record| record.state != RecoveryState::Resolved)
        .count() as u32;

    Ok(DesktopSellerReminderContext {
        today_feed: filter_reminder_surface(&schedule, ReminderSurface::Today),
        orders_feed: filter_reminder_surface(&schedule, ReminderSurface::Orders),
        pack_day_feed: filter_reminder_surface(&schedule, ReminderSurface::PackDay),
        recovery_queue,
        selected_order_recoveries,
        due_soon_count,
        recovery_actions_open,
        reminder_log,
    })
}

fn load_selected_account_reminder_sync_truth(
    sqlite_store: &AppSqliteStore,
    account_id: &str,
) -> Result<DesktopReminderSyncTruth, AppSqliteError> {
    let checkpoint = sqlite_store.load_sync_checkpoint(account_id)?;
    let conflicts = sqlite_store.load_sync_conflicts(account_id)?;
    let pending_write_count = sqlite_store.load_pending_sync_operations(account_id)?.len();
    let unresolved_conflict_count = conflicts
        .iter()
        .filter(|stored| stored.conflict.is_unresolved())
        .count();
    let blocking_conflict_count = conflicts
        .iter()
        .filter(|stored| {
            stored.conflict.is_unresolved()
                && matches!(stored.conflict.severity, SyncConflictSeverity::Blocking)
        })
        .count();

    Ok(DesktopReminderSyncTruth {
        checkpoint,
        pending_write_count,
        unresolved_conflict_count,
        blocking_conflict_count,
    })
}

fn derive_selected_account_reminder_schedule(
    farm_id: FarmId,
    today_projection: &TodayAgendaProjection,
    canonical_orders_list: &OrdersListProjection,
    pack_day_projection: &PackDayProjection,
    recovery_queue: &RecoveryQueueProjection,
    sync_truth: &DesktopReminderSyncTruth,
    existing_schedule: &ReminderFeedProjection,
) -> ReminderFeedProjection {
    let mut items = Vec::new();

    if let Some(window) = today_projection.next_fulfillment_window.as_ref() {
        items.push(build_reminder_projection(
            farm_id,
            format!(
                "reminder:today:fulfillment_window:{}",
                window.fulfillment_window_id
            ),
            None,
            Some(window.fulfillment_window_id),
            ReminderKind::FulfillmentWindow,
            ReminderSurface::Today,
            "Prepare the next fulfillment window".to_owned(),
            format!(
                "The next fulfillment window starts at {}.",
                window.starts_at
            ),
            window.starts_at.clone(),
            Some("Open pack day".to_owned()),
            None,
            existing_schedule,
        ));
    }

    if canonical_orders_list.summary.needs_action_orders > 0 {
        let deadline_at = today_projection
            .next_fulfillment_window
            .as_ref()
            .map(|window| window.starts_at.clone())
            .unwrap_or_else(current_utc_timestamp);
        let detail = canonical_orders_list
            .rows
            .first()
            .and_then(|row| row.fulfillment_window_label.as_ref())
            .map(|label| {
                format!(
                    "{} order(s) still need review before {}.",
                    canonical_orders_list.summary.needs_action_orders, label
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "{} order(s) still need review.",
                    canonical_orders_list.summary.needs_action_orders
                )
            });
        items.push(build_reminder_projection(
            farm_id,
            "reminder:orders:needs_action".to_owned(),
            canonical_orders_list.rows.first().map(|row| row.order_id),
            canonical_orders_list
                .rows
                .first()
                .and_then(|row| row.fulfillment_window_id),
            ReminderKind::OrderAction,
            ReminderSurface::Orders,
            "Review open orders".to_owned(),
            detail,
            deadline_at,
            Some("Review".to_owned()),
            None,
            existing_schedule,
        ));
    }

    if let Some(window) = pack_day_projection.fulfillment_window.as_ref() {
        items.push(build_reminder_projection(
            farm_id,
            format!(
                "reminder:pack_day:fulfillment_window:{}",
                window.fulfillment_window_id
            ),
            None,
            Some(window.fulfillment_window_id),
            ReminderKind::FulfillmentWindow,
            ReminderSurface::PackDay,
            "Pack for this fulfillment window".to_owned(),
            format!("Packing needs to be ready before {}.", window.starts_at),
            window.starts_at.clone(),
            Some("Review pack day".to_owned()),
            None,
            existing_schedule,
        ));
    }

    if let Some(sync_reminder) =
        build_sync_reminder_projection(farm_id, sync_truth, existing_schedule)
    {
        items.push(sync_reminder);
    }

    for record in recovery_queue
        .items
        .iter()
        .filter(|record| record.state != RecoveryState::Resolved)
    {
        let kind = match record.kind {
            RecoveryKind::MissedPickup => ReminderKind::MissedPickupRecovery,
            RecoveryKind::RefundFollowUp => ReminderKind::RefundRecovery,
        };
        items.push(build_reminder_projection(
            farm_id,
            format!(
                "reminder:orders:recovery:{}:{}",
                record.kind.storage_key(),
                record.order_id
            ),
            Some(record.order_id),
            None,
            kind,
            ReminderSurface::Orders,
            record.summary.clone(),
            record
                .note
                .clone()
                .unwrap_or_else(|| "Recovery follow-up is still open.".to_owned()),
            record.last_updated_at.clone(),
            Some("Review".to_owned()),
            None,
            existing_schedule,
        ));
    }

    items.sort_by(|left, right| {
        left.deadline_at.cmp(&right.deadline_at).then_with(|| {
            left.reminder_id
                .to_string()
                .cmp(&right.reminder_id.to_string())
        })
    });

    ReminderFeedProjection { items }
}

fn build_sync_reminder_projection(
    farm_id: FarmId,
    sync_truth: &DesktopReminderSyncTruth,
    existing_schedule: &ReminderFeedProjection,
) -> Option<ReminderDeadlineProjection> {
    if sync_truth.blocking_conflict_count > 0 {
        return Some(build_reminder_projection(
            farm_id,
            "reminder:orders:sync:blocking_conflicts".to_owned(),
            None,
            None,
            ReminderKind::SyncImpact,
            ReminderSurface::Orders,
            "Resolve blocking sync conflicts".to_owned(),
            format!(
                "{} blocking sync conflict(s) need review before the next sync run.",
                sync_truth.blocking_conflict_count
            ),
            current_utc_timestamp(),
            Some("Review".to_owned()),
            Some(ReminderUrgency::Blocking),
            existing_schedule,
        ));
    }

    if sync_truth.unresolved_conflict_count > 0 {
        return Some(build_reminder_projection(
            farm_id,
            "reminder:orders:sync:conflicts".to_owned(),
            None,
            None,
            ReminderKind::SyncImpact,
            ReminderSurface::Orders,
            "Review sync conflicts".to_owned(),
            format!(
                "{} sync conflict(s) are still unresolved.",
                sync_truth.unresolved_conflict_count
            ),
            current_utc_timestamp(),
            Some("Review".to_owned()),
            Some(ReminderUrgency::DueSoon),
            existing_schedule,
        ));
    }

    if sync_truth.checkpoint.is_failed() {
        return Some(build_reminder_projection(
            farm_id,
            "reminder:orders:sync:failed".to_owned(),
            None,
            None,
            ReminderKind::SyncImpact,
            ReminderSurface::Orders,
            "Retry sync".to_owned(),
            sync_truth
                .checkpoint
                .last_error_message
                .clone()
                .unwrap_or_else(|| "The last sync attempt failed.".to_owned()),
            current_utc_timestamp(),
            Some("Review".to_owned()),
            Some(ReminderUrgency::Blocking),
            existing_schedule,
        ));
    }

    if sync_truth.pending_write_count > 0 {
        return Some(build_reminder_projection(
            farm_id,
            "reminder:orders:sync:pending".to_owned(),
            None,
            None,
            ReminderKind::SyncImpact,
            ReminderSurface::Orders,
            "Pending local changes".to_owned(),
            format!(
                "{} local change(s) are waiting to sync.",
                sync_truth.pending_write_count
            ),
            current_utc_timestamp(),
            Some("Review".to_owned()),
            Some(ReminderUrgency::Upcoming),
            existing_schedule,
        ));
    }

    None
}

fn build_reminder_projection(
    farm_id: FarmId,
    identity_key: String,
    order_id: Option<OrderId>,
    fulfillment_window_id: Option<FulfillmentWindowId>,
    kind: ReminderKind,
    surface: ReminderSurface,
    title: String,
    detail: String,
    deadline_at: String,
    action_label: Option<String>,
    urgency_override: Option<ReminderUrgency>,
    existing_schedule: &ReminderFeedProjection,
) -> ReminderDeadlineProjection {
    let reminder_id = stable_reminder_id(identity_key.as_str());
    let urgency = urgency_override.unwrap_or_else(|| reminder_urgency(deadline_at.as_str()));
    let delivery_state = existing_schedule
        .items
        .iter()
        .find(|item| item.reminder_id == reminder_id)
        .map(|item| item.delivery_state)
        .unwrap_or(ReminderDeliveryState::Scheduled);

    ReminderDeadlineProjection {
        reminder_id,
        farm_id,
        order_id,
        fulfillment_window_id,
        kind,
        surface,
        urgency,
        title,
        detail,
        deadline_at,
        action_label,
        delivery_state,
    }
}

fn stable_reminder_id(identity_key: &str) -> ReminderId {
    ReminderId::from(Uuid::new_v5(&Uuid::NAMESPACE_URL, identity_key.as_bytes()))
}

fn reminder_urgency(deadline_at: &str) -> ReminderUrgency {
    let Ok(deadline) = chrono::DateTime::parse_from_rfc3339(deadline_at) else {
        return ReminderUrgency::Upcoming;
    };
    let deadline = deadline.with_timezone(&Utc);
    let now = Utc::now();

    if deadline <= now {
        ReminderUrgency::Overdue
    } else if deadline <= now + Duration::hours(48) {
        ReminderUrgency::DueSoon
    } else {
        ReminderUrgency::Upcoming
    }
}

fn filter_reminder_surface(
    schedule: &ReminderFeedProjection,
    surface: ReminderSurface,
) -> ReminderFeedProjection {
    ReminderFeedProjection {
        items: schedule
            .items
            .iter()
            .filter(|item| item.surface == surface)
            .cloned()
            .collect(),
    }
}

fn reconcile_resolved_reminder_log_entries(
    existing_schedule: &ReminderFeedProjection,
    schedule: &ReminderFeedProjection,
) -> Vec<ReminderLogEntryProjection> {
    existing_schedule
        .items
        .iter()
        .filter(|existing| {
            existing.delivery_state != ReminderDeliveryState::Scheduled
                && existing.delivery_state != ReminderDeliveryState::Resolved
                && !schedule
                    .items
                    .iter()
                    .any(|current| current.reminder_id == existing.reminder_id)
        })
        .map(|reminder| build_reminder_log_entry(reminder, ReminderDeliveryState::Resolved))
        .collect()
}

fn promote_desktop_reminder_presentation(
    schedule: &mut ReminderFeedProjection,
    reminder_log_entries: &mut Vec<ReminderLogEntryProjection>,
    allow_auto_present: bool,
) {
    if !allow_auto_present || schedule.items.iter().any(is_desktop_presented_reminder) {
        return;
    }

    let Some(index) = schedule
        .items
        .iter()
        .enumerate()
        .filter(|(_, reminder)| {
            reminder.delivery_state == ReminderDeliveryState::Scheduled
                && is_desktop_presentation_candidate(reminder)
        })
        .min_by(|(_, left), (_, right)| desktop_reminder_sort(left, right))
        .map(|(index, _)| index)
    else {
        return;
    };

    schedule.items[index].delivery_state = ReminderDeliveryState::Presented;
    reminder_log_entries.push(build_reminder_log_entry(
        &schedule.items[index],
        ReminderDeliveryState::Presented,
    ));
}

fn is_desktop_presented_reminder(reminder: &ReminderDeadlineProjection) -> bool {
    reminder.delivery_state == ReminderDeliveryState::Presented
        && is_desktop_presentation_candidate(reminder)
}

fn is_desktop_presentation_candidate(reminder: &ReminderDeadlineProjection) -> bool {
    matches!(
        reminder.urgency,
        ReminderUrgency::DueSoon | ReminderUrgency::Overdue | ReminderUrgency::Blocking
    )
}

fn desktop_reminder_sort(
    left: &ReminderDeadlineProjection,
    right: &ReminderDeadlineProjection,
) -> std::cmp::Ordering {
    desktop_reminder_priority(left.urgency)
        .cmp(&desktop_reminder_priority(right.urgency))
        .then_with(|| left.deadline_at.cmp(&right.deadline_at))
        .then_with(|| left.reminder_id.cmp(&right.reminder_id))
}

fn desktop_reminder_priority(urgency: ReminderUrgency) -> u8 {
    match urgency {
        ReminderUrgency::Blocking => 0,
        ReminderUrgency::Overdue => 1,
        ReminderUrgency::DueSoon => 2,
        ReminderUrgency::Upcoming => 3,
    }
}

fn build_reminder_log_entry(
    reminder: &ReminderDeadlineProjection,
    delivery_state: ReminderDeliveryState,
) -> ReminderLogEntryProjection {
    ReminderLogEntryProjection {
        reminder_id: reminder.reminder_id,
        kind: reminder.kind,
        title: reminder.title.clone(),
        recorded_at: current_utc_timestamp(),
        delivery_state,
        detail: (!reminder.detail.trim().is_empty()).then_some(reminder.detail.clone()),
    }
}

fn ordered_order_recoveries_for_detail(
    recovery_queue: &RecoveryQueueProjection,
    order_id: OrderId,
) -> Vec<OrderRecoveryProjection> {
    let mut items = recovery_queue
        .items
        .iter()
        .filter(|record| record.order_id == order_id)
        .cloned()
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        order_recovery_kind_rank(left.kind)
            .cmp(&order_recovery_kind_rank(right.kind))
            .then_with(|| right.last_updated_at.cmp(&left.last_updated_at))
            .then_with(|| left.recovery_record_id.cmp(&right.recovery_record_id))
    });
    items
}

fn order_recovery_kind_rank(kind: RecoveryKind) -> u8 {
    match kind {
        RecoveryKind::MissedPickup => 0,
        RecoveryKind::RefundFollowUp => 1,
    }
}

fn order_recovery_summary(kind: RecoveryKind, state: RecoveryState) -> &'static str {
    match (kind, state) {
        (RecoveryKind::MissedPickup, RecoveryState::Open) => "Missed pickup follow-up is open",
        (RecoveryKind::MissedPickup, RecoveryState::InReview) => {
            "Missed pickup follow-up is in review"
        }
        (RecoveryKind::MissedPickup, RecoveryState::Resolved) => {
            "Missed pickup follow-up is resolved"
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::Open) => "Payment status follow-up is open",
        (RecoveryKind::RefundFollowUp, RecoveryState::InReview) => {
            "Payment status follow-up is in review"
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::Resolved) => {
            "Payment status follow-up is resolved"
        }
    }
}

fn order_recovery_note(kind: RecoveryKind, state: RecoveryState) -> &'static str {
    match (kind, state) {
        (RecoveryKind::MissedPickup, RecoveryState::Open) => {
            "Check in with the buyer and agree on the next step."
        }
        (RecoveryKind::MissedPickup, RecoveryState::InReview) => {
            "Use notes outside the app to confirm a new pickup or another resolution."
        }
        (RecoveryKind::MissedPickup, RecoveryState::Resolved) => {
            "The seller and buyer have agreed on the next step."
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::Open) => {
            "Review the order record and agree on the next step."
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::InReview) => {
            "Confirm the outcome with the order parties."
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::Resolved) => {
            "The payment status follow-up is resolved."
        }
    }
}

fn order_recovery_sync_payload(
    order_id: OrderId,
    farm_id: FarmId,
    kind: RecoveryKind,
    state: RecoveryState,
    source: &str,
) -> String {
    json!({
        "aggregate_kind": "order_recovery",
        "order_id": order_id.to_string(),
        "farm_id": farm_id.to_string(),
        "recovery_kind": kind.storage_key(),
        "recovery_state": state.storage_key(),
        "source": source,
    })
    .to_string()
}

fn load_selected_account_sync_context(
    sqlite_store: &AppSqliteStore,
    identity_projection: &AppIdentityProjection,
    relay_urls: &[String],
) -> Result<DesktopSelectedAccountSyncContext, AppSqliteError> {
    let Some(selected_account) = identity_projection.selected_account.as_ref() else {
        return Ok(DesktopSelectedAccountSyncContext::default());
    };
    let account_id = selected_account.account.account_id.as_str();
    let checkpoint = sqlite_store.load_sync_checkpoint(account_id)?;
    let stored_conflicts = sqlite_store.load_sync_conflicts(account_id)?;
    let conflicts = stored_conflicts
        .iter()
        .map(|stored| stored.conflict.clone())
        .collect::<Vec<_>>();
    let pending_write_count = sqlite_store.load_pending_sync_operations(account_id)?.len();
    let relay_urls = normalized_app_relay_ingest_urls(relay_urls)?;
    let relay_ingest = sqlite_store.load_relay_ingest_freshness(
        APP_DIRECT_RELAY_INGEST_SCOPE_KEY,
        &relay_urls,
        current_runtime_time_seconds()?,
        APP_DIRECT_RELAY_INGEST_STALE_AFTER_SECONDS,
    )?;

    Ok(DesktopSelectedAccountSyncContext {
        projection: derive_sync_projection(&checkpoint, &conflicts),
        relay_ingest,
        pending_write_count,
        conflicts: stored_conflicts
            .into_iter()
            .map(|stored| DesktopAppSyncConflictSummary {
                conflict_id: stored.conflict_id,
                conflict: stored.conflict,
            })
            .collect(),
    })
}

fn personal_detail(
    projection: &PersonalWorkspaceProjection,
    section: PersonalSection,
) -> Option<&BuyerProductDetailProjection> {
    match section {
        PersonalSection::Browse => projection.browse.detail.as_ref(),
        PersonalSection::Search => projection.search.detail.as_ref(),
        PersonalSection::Cart | PersonalSection::Orders => None,
    }
}

fn personal_detail_mut(
    projection: &mut PersonalWorkspaceProjection,
    section: PersonalSection,
) -> Option<&mut BuyerProductDetailProjection> {
    match section {
        PersonalSection::Browse => projection.browse.detail.as_mut(),
        PersonalSection::Search => projection.search.detail.as_mut(),
        PersonalSection::Cart | PersonalSection::Orders => None,
    }
}

fn next_buyer_cart_for_detail(
    mut current_cart: BuyerCartProjection,
    detail: &BuyerProductDetailProjection,
    replace_existing: bool,
) -> Result<BuyerCartProjection, AppSqliteError> {
    let incoming_line = buyer_cart_line_from_detail(detail)?;
    let current_farm_id = current_cart.farm_id;
    let should_replace_lines = replace_existing
        || current_cart.is_empty()
        || current_farm_id != Some(detail.listing.farm_id);

    if should_replace_lines {
        current_cart.lines.clear();
    }

    current_cart.farm_id = Some(detail.listing.farm_id);
    current_cart.farm_display_name = Some(detail.listing.farm_display_name.clone());
    current_cart.replace_confirmation = None;

    if let Some(existing_line) = current_cart
        .lines
        .iter_mut()
        .find(|line| line.product_id == incoming_line.product_id)
    {
        existing_line.quantity = existing_line
            .quantity
            .checked_add(incoming_line.quantity)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart quantity overflow",
            })?;
        existing_line.line_total_minor_units = existing_line
            .unit_price
            .amount_minor_units
            .checked_mul(existing_line.quantity)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart line total overflow",
            })?;
    } else {
        current_cart.lines.push(incoming_line);
    }

    refresh_buyer_cart_totals(&mut current_cart)?;

    Ok(current_cart)
}

fn next_buyer_cart_after_removing_line(
    mut current_cart: BuyerCartProjection,
    product_id: ProductId,
) -> Result<Option<BuyerCartProjection>, AppSqliteError> {
    let previous_line_count = current_cart.lines.len();
    current_cart
        .lines
        .retain(|line| line.product_id != product_id);
    if current_cart.lines.len() == previous_line_count {
        return Ok(None);
    }

    if current_cart.lines.is_empty() {
        current_cart.farm_id = None;
        current_cart.farm_display_name = None;
        current_cart.replace_confirmation = None;
        refresh_buyer_cart_totals(&mut current_cart)?;
        return Ok(Some(current_cart));
    }

    let farm_id = current_cart.lines[0].farm_id;
    let farm_display_name = current_cart.lines[0].farm_display_name.clone();
    current_cart.farm_id = Some(farm_id);
    current_cart.farm_display_name = Some(farm_display_name);
    current_cart.replace_confirmation = None;
    refresh_buyer_cart_totals(&mut current_cart)?;

    Ok(Some(current_cart))
}

fn buyer_cart_line_from_detail(
    detail: &BuyerProductDetailProjection,
) -> Result<BuyerCartLineProjection, AppSqliteError> {
    Ok(BuyerCartLineProjection {
        product_id: detail.listing.product_id,
        farm_id: detail.listing.farm_id,
        farm_display_name: detail.listing.farm_display_name.clone(),
        title: detail.listing.title.clone(),
        quantity: detail.selected_quantity,
        unit_price: detail.listing.price.clone(),
        line_total_minor_units: detail
            .listing
            .price
            .amount_minor_units
            .checked_mul(detail.selected_quantity)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart line total overflow",
            })?,
        fulfillment_summary: detail
            .listing
            .next_fulfillment_window_label
            .clone()
            .unwrap_or_else(|| detail.listing.availability.label.clone()),
    })
}

fn refresh_buyer_cart_totals(cart: &mut BuyerCartProjection) -> Result<(), AppSqliteError> {
    if cart.lines.is_empty() {
        cart.subtotal_minor_units = None;
        cart.currency_code = None;
        cart.replace_confirmation = None;
        return Ok(());
    }

    let currency_code = cart.lines[0].unit_price.currency_code.clone();
    let subtotal_minor_units = cart.lines.iter().try_fold(0u32, |subtotal, line| {
        if line.unit_price.currency_code != currency_code {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer cart currency mismatch",
            });
        }

        subtotal
            .checked_add(line.line_total_minor_units)
            .ok_or(AppSqliteError::InvalidProjection {
                reason: "buyer cart subtotal overflow",
            })
    })?;

    cart.subtotal_minor_units = Some(subtotal_minor_units);
    cart.currency_code = Some(currency_code);

    Ok(())
}

fn selected_farm_id_from_context(
    identity_projection: &AppIdentityProjection,
    farm_setup_projection: &FarmSetupProjection,
) -> Option<FarmId> {
    farm_setup_projection
        .saved_farm
        .as_ref()
        .map(|farm| farm.farm_id)
        .or_else(|| {
            identity_projection
                .selected_account
                .as_ref()
                .and_then(|account| account.farmer_activation.farm_id)
        })
}

fn fallback_farm_profile_for_projection(
    farm_id: FarmId,
    farm_setup_projection: &FarmSetupProjection,
) -> FarmProfileRecord {
    let saved_farm_name = farm_setup_projection
        .saved_farm
        .as_ref()
        .filter(|farm| farm.farm_id == farm_id)
        .map(|farm| farm.display_name.clone());
    let drafted_farm_name = farm_setup_projection.draft.farm_name.trim().to_owned();
    let display_name = saved_farm_name
        .or_else(|| (!drafted_farm_name.is_empty()).then_some(drafted_farm_name))
        .unwrap_or_default();

    FarmProfileRecord {
        farm_id,
        display_name,
        timezone: "UTC".to_owned(),
        currency_code: "USD".to_owned(),
    }
}

fn prepare_loaded_farm_rules_projection(
    mut projection: FarmRulesProjection,
    fallback_profile: &FarmProfileRecord,
) -> FarmRulesProjection {
    if projection.farm_profile.is_none() {
        projection.farm_profile = Some(fallback_profile.clone());
    }

    normalize_pickup_location_defaults(&mut projection.pickup_locations);
    projection.readiness = derive_farm_rules_readiness(&projection);
    projection
}

fn normalize_farm_rules_projection(
    mut projection: FarmRulesProjection,
    fallback_profile: &FarmProfileRecord,
) -> FarmRulesProjection {
    let mut farm_profile = projection
        .farm_profile
        .take()
        .unwrap_or_else(|| fallback_profile.clone());
    farm_profile.farm_id = fallback_profile.farm_id;
    farm_profile.display_name = farm_profile.display_name.trim().to_owned();
    farm_profile.timezone = farm_profile.timezone.trim().to_owned();
    farm_profile.currency_code = farm_profile.currency_code.trim().to_uppercase();
    projection.farm_profile = Some(farm_profile);

    for pickup_location in &mut projection.pickup_locations {
        pickup_location.farm_id = fallback_profile.farm_id;
        pickup_location.label = pickup_location.label.trim().to_owned();
        pickup_location.address_line = pickup_location.address_line.trim().to_owned();
        pickup_location.directions = pickup_location
            .directions
            .take()
            .map(|directions| directions.trim().to_owned())
            .filter(|directions| !directions.is_empty());
    }

    if let Some(operating_rules) = projection.operating_rules.as_mut() {
        operating_rules.farm_id = fallback_profile.farm_id;
        operating_rules.substitution_policy = operating_rules.substitution_policy.trim().to_owned();
        operating_rules.missed_pickup_policy =
            operating_rules.missed_pickup_policy.trim().to_owned();
    }

    for fulfillment_window in &mut projection.fulfillment_windows {
        fulfillment_window.farm_id = fallback_profile.farm_id;
        fulfillment_window.label = fulfillment_window.label.trim().to_owned();
        fulfillment_window.starts_at = fulfillment_window.starts_at.trim().to_owned();
        fulfillment_window.ends_at = fulfillment_window.ends_at.trim().to_owned();
        fulfillment_window.order_cutoff_at = fulfillment_window.order_cutoff_at.trim().to_owned();
    }

    for blackout_period in &mut projection.blackout_periods {
        blackout_period.farm_id = fallback_profile.farm_id;
        blackout_period.label = blackout_period.label.trim().to_owned();
        blackout_period.starts_at = blackout_period.starts_at.trim().to_owned();
        blackout_period.ends_at = blackout_period.ends_at.trim().to_owned();
    }

    normalize_pickup_location_defaults(&mut projection.pickup_locations);
    projection.readiness = derive_farm_rules_readiness(&projection);
    projection
}

fn normalize_pickup_location_defaults(pickup_locations: &mut [PickupLocationRecord]) {
    let default_index = pickup_locations
        .iter()
        .position(|pickup_location| pickup_location.is_default)
        .or_else(|| (!pickup_locations.is_empty()).then_some(0));

    for (index, pickup_location) in pickup_locations.iter_mut().enumerate() {
        pickup_location.is_default = Some(index) == default_index;
    }
}

fn current_utc_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn signed_event_from_local_record(
    record: &LocalEventRecord,
) -> Result<Option<radroots_sdk::RadrootsNostrEvent>, AppSqliteError> {
    let Some(id) = record.event_id.as_deref().map(str::trim) else {
        return Ok(None);
    };
    let Some(author) = record.event_pubkey.as_deref().map(str::trim) else {
        return Ok(None);
    };
    let Some(kind) = record.event_kind else {
        return Ok(None);
    };
    let Some(content) = record.event_content.as_ref() else {
        return Ok(None);
    };
    let Some(sig) = record.event_sig.as_deref().map(str::trim) else {
        return Ok(None);
    };
    let created_at = record.event_created_at.unwrap_or_default();
    let created_at = u32::try_from(created_at).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "signed local event created_at must fit u32",
    })?;
    let kind = u32::try_from(kind).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "signed local event kind must fit u32",
    })?;

    Ok(Some(radroots_sdk::RadrootsNostrEvent {
        id: id.to_owned(),
        author: author.to_owned(),
        created_at,
        kind,
        tags: event_tags_from_value(record.event_tags_json.as_ref())?,
        content: content.clone(),
        sig: sig.to_owned(),
    }))
}

fn event_tags_from_value(
    value: Option<&serde_json::Value>,
) -> Result<Vec<Vec<String>>, AppSqliteError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(tags) = value.as_array() else {
        return Err(AppSqliteError::InvalidProjection {
            reason: "signed local event tags must be an array",
        });
    };

    tags.iter()
        .map(|tag| {
            let Some(values) = tag.as_array() else {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "signed local event tag must be an array",
                });
            };
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or(AppSqliteError::InvalidProjection {
                            reason: "signed local event tag values must be strings",
                        })
                })
                .collect()
        })
        .collect()
}

fn trade_chain_tag_value(event: &radroots_sdk::RadrootsNostrEvent, key: &str) -> Option<String> {
    event.tags.iter().find_map(|tag| {
        if tag.first().map(String::as_str) == Some(key) {
            tag.get(1)
                .map(String::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        } else {
            None
        }
    })
}

fn active_order_event_record_context(
    event: &radroots_sdk::RadrootsNostrEvent,
    message_type: radroots_sdk::trade::RadrootsActiveTradeMessageType,
) -> Result<(String, String, String), AppSqliteError> {
    let context =
        active_trade_event_context_from_tags(message_type, &event.tags).map_err(|_| {
            AppSqliteError::InvalidProjection {
                reason: "order lifecycle evidence is invalid",
            }
        })?;
    let root_event_id = context
        .root_event_id
        .ok_or(AppSqliteError::InvalidProjection {
            reason: "order lifecycle evidence is invalid",
        })?;
    let prev_event_id = context
        .prev_event_id
        .ok_or(AppSqliteError::InvalidProjection {
            reason: "order lifecycle evidence is invalid",
        })?;
    Ok((context.counterparty_pubkey, root_event_id, prev_event_id))
}

fn active_order_current_parent_event_id(
    lifecycle: &ResolvedAppOrderLifecycleEvidence,
    reason: &'static str,
) -> Result<String, AppSqliteError> {
    lifecycle
        .last_event_id
        .clone()
        .ok_or(AppSqliteError::InvalidProjection { reason })
}

fn active_order_payment_blocks_lifecycle_write(
    lifecycle: &ResolvedAppOrderLifecycleEvidence,
) -> bool {
    matches!(
        lifecycle.payment_state,
        RadrootsActiveOrderPaymentState::Recorded | RadrootsActiveOrderPaymentState::Settled
    )
}

fn active_order_revision_parent_event_id(
    lifecycle: &ResolvedAppOrderLifecycleEvidence,
) -> Option<String> {
    if active_order_pending_revision_proposal(lifecycle).is_some() {
        None
    } else {
        lifecycle.last_event_id.clone()
    }
}

fn active_order_pending_revision_proposal(
    lifecycle: &ResolvedAppOrderLifecycleEvidence,
) -> Option<&ResolvedAppOrderRevisionProposalEvidence> {
    let decision = lifecycle.decision.as_ref()?;
    let mut parent_event_id = decision.event_id.as_str();
    loop {
        let proposals = lifecycle
            .revision_proposals
            .iter()
            .filter(|proposal| proposal.payload.prev_event_id == parent_event_id)
            .collect::<Vec<_>>();
        let proposal = match proposals.as_slice() {
            [] => return None,
            [proposal] => *proposal,
            _ => return None,
        };
        let decisions = lifecycle
            .revision_decisions
            .iter()
            .filter(|decision| {
                decision.payload.prev_event_id == proposal.event_id
                    && decision.payload.revision_id == proposal.payload.revision_id
            })
            .collect::<Vec<_>>();
        let decision = match decisions.as_slice() {
            [] => return Some(proposal),
            [decision] => *decision,
            _ => return None,
        };
        parent_event_id = decision.event_id.as_str();
    }
}

fn insert_seller_order_request_evidence(
    order_id: &OrderId,
    event: &radroots_sdk::RadrootsNostrEvent,
    payload: RadrootsTradeOrderRequested,
    matched_requests: &mut BTreeMap<String, ResolvedAppSellerOrderRequest>,
) {
    let app_order_id = projected_order_id_from_trade_request(
        payload.order_id.as_str(),
        payload.buyer_pubkey.as_str(),
    );
    if app_order_id != *order_id {
        return;
    }
    matched_requests
        .entry(event.id.clone())
        .or_insert_with(|| ResolvedAppSellerOrderRequest {
            request_event_id: event.id.clone(),
            request_author_pubkey: event.author.clone(),
            listing_event_id: listing_event_id_from_tags(&event.tags),
            payload,
        });
}

fn listing_event_id_from_tags(tags: &[Vec<String>]) -> Option<String> {
    tags.iter().find_map(|tag| {
        if tag.first().map(String::as_str) == Some("listing_event") {
            tag.get(1)
                .map(String::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        } else {
            None
        }
    })
}

fn seller_order_inventory_commitments(
    order: &SellerOrderDecisionExport,
) -> Result<Vec<AppOrderDecisionInventoryCommitment>, AppSqliteError> {
    if order.lines.is_empty() {
        return Err(AppSqliteError::InvalidProjection {
            reason: "seller order decision requires order lines",
        });
    }

    order
        .lines
        .iter()
        .map(|line| {
            let bin_id =
                line.listing_bin_id
                    .as_deref()
                    .ok_or(AppSqliteError::InvalidProjection {
                        reason: "seller order decision requires listing bin evidence",
                    })?;
            let stock_count = line.stock_count.ok_or(AppSqliteError::InvalidProjection {
                reason: "seller order decision requires current product stock",
            })?;
            let available_quantity = stock_count.checked_sub(line.reserved_quantity).ok_or(
                AppSqliteError::InvalidProjection {
                    reason: "seller order decision inventory is over-reserved",
                },
            )?;
            if line.quantity > available_quantity {
                return Err(AppSqliteError::InvalidProjection {
                    reason: "seller order decision would over-reserve inventory",
                });
            }

            Ok(AppOrderDecisionInventoryCommitment {
                bin_id: bin_id.to_owned(),
                bin_count: line.quantity,
            })
        })
        .collect()
}

fn order_decision_publish_payload_to_sdk_decision(
    payload: &AppOrderDecisionPublishPayload,
) -> RadrootsTradeOrderDecisionEvent {
    RadrootsTradeOrderDecisionEvent {
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        decision: match &payload.decision {
            AppOrderDecisionPayload::Accepted {
                inventory_commitments,
            } => RadrootsTradeOrderDecision::Accepted {
                inventory_commitments: inventory_commitments
                    .iter()
                    .map(|commitment| RadrootsTradeInventoryCommitment {
                        bin_id: commitment.bin_id.clone(),
                        bin_count: commitment.bin_count,
                    })
                    .collect(),
            },
            AppOrderDecisionPayload::Declined { reason } => RadrootsTradeOrderDecision::Declined {
                reason: reason.clone(),
            },
        },
    }
}

fn order_revision_proposal_publish_payload_to_sdk_revision(
    payload: &AppOrderRevisionProposalPublishPayload,
) -> RadrootsTradeOrderRevisionProposed {
    RadrootsTradeOrderRevisionProposed {
        revision_id: payload.revision_id.clone(),
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        root_event_id: payload.request_event_id.clone(),
        prev_event_id: payload.prev_event_id.clone(),
        items: payload.items.clone(),
        economics: payload.economics.clone(),
        reason: payload.reason.clone(),
    }
}

fn order_revision_decision_publish_payload_to_sdk_revision_decision(
    payload: &AppOrderRevisionDecisionPublishPayload,
) -> RadrootsTradeOrderRevisionDecisionEvent {
    RadrootsTradeOrderRevisionDecisionEvent {
        revision_id: payload.revision_id.clone(),
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        root_event_id: payload.request_event_id.clone(),
        prev_event_id: payload.prev_event_id.clone(),
        decision: payload.decision.clone(),
    }
}

fn order_fulfillment_publish_payload_to_sdk_fulfillment(
    payload: &AppOrderFulfillmentPublishPayload,
) -> RadrootsTradeFulfillmentUpdated {
    RadrootsTradeFulfillmentUpdated {
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        status: payload.status,
    }
}

fn order_cancellation_publish_payload_to_sdk_cancellation(
    payload: &AppOrderCancellationPublishPayload,
) -> RadrootsTradeOrderCancelled {
    RadrootsTradeOrderCancelled {
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        reason: payload.reason.clone(),
    }
}

fn order_receipt_publish_payload_to_sdk_receipt(
    payload: &AppOrderReceiptPublishPayload,
) -> RadrootsTradeBuyerReceipt {
    RadrootsTradeBuyerReceipt {
        order_id: payload.trade_order_id.clone(),
        listing_addr: payload.listing_addr.clone(),
        buyer_pubkey: payload.buyer_pubkey.clone(),
        seller_pubkey: payload.seller_pubkey.clone(),
        received: payload.received,
        issue: payload.issue.clone(),
        received_at: payload.received_at,
    }
}

fn pending_sync_upsert(aggregate: SyncAggregateRef, payload_json: String) -> PendingSyncOperation {
    let created_at = current_utc_timestamp();

    PendingSyncOperation::new(
        aggregate,
        SyncOperationKind::Upsert,
        payload_json,
        created_at,
    )
}

#[cfg(test)]
fn farm_sync_payload(
    farm_id: FarmId,
    display_name: &str,
    readiness: Option<FarmReadiness>,
    source: &str,
) -> String {
    json!({
        "aggregate_kind": "farm",
        "farm_id": farm_id.to_string(),
        "display_name": display_name,
        "readiness": readiness.map(|value| match value {
            FarmReadiness::Incomplete => "incomplete",
            FarmReadiness::Ready => "ready",
        }),
        "source": source,
    })
    .to_string()
}

fn signed_order_request_evidence_record_is_usable(record: &LocalEventRecord) -> bool {
    if record.status != LocalRecordStatus::Published
        || matches!(
            record.outbox_status,
            PublishOutboxStatus::Pending | PublishOutboxStatus::Failed
        )
    {
        return false;
    }
    let Some(relay_delivery_json) = record.relay_delivery_json.as_ref() else {
        return false;
    };
    let Ok(relay_delivery) = RelayDeliveryEvidence::from_json_value(relay_delivery_json) else {
        return false;
    };
    matches!(relay_delivery.state.as_str(), "acknowledged" | "observed")
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        path::PathBuf,
        sync::mpsc,
        sync::{Arc, Mutex},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{Duration, Utc};
    use futures_util::{SinkExt, StreamExt};
    use radroots_studio_app_core::{
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePlatform,
        AppSharedAccountsPaths, SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITY_FILE_NAME,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_sqlite::{
        AppSqliteError, AppSqliteStore, BuyerOrderCoordinationState, DatabaseTarget,
        latest_schema_version, projected_order_id_from_trade_request,
    };
    use radroots_studio_app_state::{
        APP_STATE_FILE_NAME, AppStateCommand, AppStatePersistenceRepository, AppStateRepository,
        AppStateRepositoryError, AppStateStore, AppStateStoreError, FileBackedAppStateRepository,
        HomeRoute,
    };
    use radroots_studio_app_sync::{
        AppFarmProfilePublishPayload, AppListingPublishPayload, AppOrderCancellationPublishPayload,
        AppOrderDecisionInventoryCommitment, AppOrderDecisionPayload,
        AppOrderDecisionPublishPayload, AppOrderFulfillmentPublishPayload, AppOrderReceiptOutcome,
        AppOrderReceiptPublishPayload, AppOrderRequestItemPayload, AppOrderRequestPublishPayload,
        AppOrderRevisionDecisionPublishPayload, AppOrderRevisionProposalPublishPayload,
        AppPublishContext, AppPublishPayload, AppPublishedOperationReceipt,
        AppRelayIngestScopeFreshness, AppRelayIngestScopeStatus, AppSyncRequest, AppSyncResult,
        AppSyncRunStatus, AppSyncTransport, AppSyncTransportError, PendingSyncOperation,
        PendingSyncOperationState, RecordedAppSyncTransport, SyncAggregateRef, SyncCheckpointState,
        SyncCheckpointStatus, SyncConflict, SyncConflictKind, SyncConflictResolutionStatus,
        SyncConflictSeverity, SyncOperationKind, SyncTrigger,
    };
    use radroots_studio_app_view::{
        AccountCustody, AccountSummary, AccountSurfaceActivationProjection, ActiveSurface,
        AppActivityKind, AppIdentityProjection, AppStartupGate, BlackoutPeriodId,
        BlackoutPeriodRecord, BuyerOrderReviewDisabledReason, BuyerOrderReviewDraft,
        BuyerOrderStatus, FarmId, FarmOperatingRulesRecord, FarmOrderMethod, FarmProfileRecord,
        FarmReadiness, FarmReadinessBlocker, FarmRulesProjection, FarmSetupDraft,
        FarmSetupProjection, FarmSummary, FarmerActivationProjection, FarmerSection,
        FulfillmentWindowId, FulfillmentWindowRecord, LoggedOutStartupProjection,
        OrderFulfillmentAction, OrderId, OrderStatus, OrdersFilter, PackDayBatchPrintArtifact,
        PackDayBatchPrintFailureKind, PackDayBatchPrintStatus, PackDayExportInstanceId,
        PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPackListRow,
        PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintStatus, PackDayProductTotalRow,
        PackDayProjection, PackDayRosterRow, PersonalSection, PickupLocationId,
        PickupLocationRecord, ProductEditorDraft, ProductId, ProductPublishBlocker, ProductStatus,
        ProductsFilter, ProductsSort, RecoveryKind, RecoveryRecordId, ReminderDeliveryState,
        ReminderFeedProjection, ReminderKind, SelectedAccountProjection, SelectedSurfaceProjection,
        SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TodaySummary,
    };
    use radroots_core::{
        RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreUnit,
    };
    use radroots_events_codec::trade::{
        active_trade_payment_recorded_event_build, active_trade_settlement_decision_event_build,
    };
    use radroots_identity::{RadrootsIdentity, RadrootsIdentityId};
    use radroots_local_events::{
        BUYER_ORDER_REQUEST_LOCAL_WORK_RECORD_KIND, CANONICAL_RELAY_SET_FINGERPRINT_VERSION,
        LocalEventRecord, LocalEventRecordInput, LocalEventsStore, LocalRecordFamily,
        LocalRecordStatus, PublishOutboxStatus, RelayDeliveryEvidence, SourceRuntime,
        canonical_relay_set_fingerprint,
    };
    use radroots_nostr::prelude::radroots_nostr_build_event;
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory, RadrootsSecretVault,
        account_secret_slot,
    };
    use radroots_sdk::RadrootsNostrEventPtr;
    use radroots_sdk::trade::{
        RadrootsActiveTradeFulfillmentState, RadrootsTradeBuyerReceipt,
        RadrootsTradeFulfillmentUpdated, RadrootsTradeInventoryCommitment,
        RadrootsTradeOrderCancelled, RadrootsTradeOrderDecision, RadrootsTradeOrderDecisionEvent,
        RadrootsTradeOrderEconomicItem, RadrootsTradeOrderEconomics, RadrootsTradeOrderItem,
        RadrootsTradeOrderRequested, RadrootsTradeOrderRevisionDecision,
        RadrootsTradeOrderRevisionDecisionEvent, RadrootsTradeOrderRevisionProposed,
        RadrootsTradePaymentMethod, RadrootsTradePaymentRecorded, RadrootsTradePricingBasis,
        RadrootsTradeSettlementDecision, RadrootsTradeSettlementDecisionEvent,
    };
    use radroots_sql_core::{SqlExecutor, SqliteExecutor};
    use radroots_trade::order::radroots_trade_order_economics_digest;
    use serde_json::json;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio_tungstenite::tungstenite::Message;
    use uuid::Uuid;

    use crate::accounts::DesktopLocalIdentityImportRequest;

    use super::{
        APP_DATABASE_FILE_NAME, DesktopAppRuntime, DesktopAppRuntimeActivityContextError,
        DesktopAppRuntimeCommandError, DesktopAppRuntimeMetadataSummary, DesktopAppRuntimeState,
        DesktopAppSyncStatusSummary, DesktopRemoteSignerPaths, SYNC_TRANSPORT_UNAVAILABLE_MESSAGE,
        SdkDirectRelayAppSyncTransport, TokioRuntimeBuilder, default_sync_transport,
        direct_relay_event_source_runtime, farm_sync_payload, is_hex_64,
        order_decision_publish_payload_to_sdk_decision, pending_sync_upsert,
        signed_event_from_local_record,
    };
    use crate::pack_day_host_handoff::PackDayHostHandoffError;
    use crate::pack_day_print::{
        PackDayBatchPrintError, PackDayPrintCommandResult, PackDayPrintError,
        execute_pack_day_batch_print_plan_with, prepared_customer_label_asset_root,
    };

    #[derive(Clone)]
    struct SharedRecordedSyncTransport(Arc<Mutex<RecordedAppSyncTransport>>);

    impl AppSyncTransport for SharedRecordedSyncTransport {
        fn sync(
            &mut self,
            request: AppSyncRequest,
        ) -> Result<AppSyncResult, AppSyncTransportError> {
            self.0
                .lock()
                .expect("recorded sync transport lock")
                .sync(request)
        }
    }

    #[test]
    fn direct_relay_trade_events_use_network_source_runtime_for_app_shaped_d_tags() {
        let app_shaped_d_tag =
            super::d_tag_from_uuid(Uuid::from_u128(0x12345678123446789123456781234567));

        assert_eq!(
            direct_relay_event_source_runtime(30340, Some(app_shaped_d_tag.as_str())),
            SourceRuntime::Network
        );
        assert_eq!(
            direct_relay_event_source_runtime(30402, Some(app_shaped_d_tag.as_str())),
            SourceRuntime::Network
        );
        assert_eq!(
            direct_relay_event_source_runtime(30403, Some(app_shaped_d_tag.as_str())),
            SourceRuntime::Network
        );
    }

    struct ThreadedAckRelay {
        url: String,
        events: Arc<Mutex<Vec<serde_json::Value>>>,
        shutdown_tx: Option<oneshot::Sender<()>>,
        join_handle: Option<thread::JoinHandle<()>>,
    }

    impl ThreadedAckRelay {
        fn spawn() -> Self {
            let (url_tx, url_rx) = mpsc::channel();
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let events: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
            let thread_events = events.clone();
            let join_handle = thread::spawn(move || {
                let runtime = TokioRuntimeBuilder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("relay runtime should build");
                runtime.block_on(async move {
                    let listener = TcpListener::bind("127.0.0.1:0")
                        .await
                        .expect("test relay should bind");
                    let url = format!(
                        "ws://{}",
                        listener.local_addr().expect("test relay local addr")
                    );
                    url_tx.send(url).expect("relay url should send");
                    let mut shutdown_rx = shutdown_rx;
                    loop {
                        tokio::select! {
                            _ = &mut shutdown_rx => break,
                            accepted = listener.accept() => {
                                let Ok((stream, _)) = accepted else {
                                    break;
                                };
                                let events = thread_events.clone();
                                tokio::spawn(async move {
                                    let Ok(websocket) = tokio_tungstenite::accept_async(stream).await else {
                                        return;
                                    };
                                    let (mut writer, mut reader) = websocket.split();
                                    while let Some(message) = reader.next().await {
                                        let Ok(Message::Text(text)) = message else {
                                            continue;
                                        };
                                        let Ok(value) = serde_json::from_str::<serde_json::Value>(text.as_str()) else {
                                            continue;
                                        };
                                        let Some(items) = value.as_array() else {
                                            continue;
                                        };
                                        match items.as_slice() {
                                            [kind, event, ..] if kind.as_str() == Some("EVENT") => {
                                                let Some(event_id) = event.get("id").and_then(|id| id.as_str()) else {
                                                    continue;
                                                };
                                                events.lock().expect("relay events lock").push(event.clone());
                                                let response = json!(["OK", event_id, true, ""]).to_string();
                                                if writer.send(Message::Text(response.into())).await.is_err() {
                                                    break;
                                                }
                                            }
                                            [kind, subscription_id, filters @ ..] if kind.as_str() == Some("REQ") => {
                                                let Some(subscription_id) = subscription_id.as_str() else {
                                                    continue;
                                                };
                                                let snapshot = events.lock().expect("relay events lock").clone();
                                                for event in snapshot.iter().filter(|event| relay_event_matches_filters(event, filters)) {
                                                    let response = json!(["EVENT", subscription_id, event]).to_string();
                                                    if writer.send(Message::Text(response.into())).await.is_err() {
                                                        break;
                                                    }
                                                }
                                                let response = json!(["EOSE", subscription_id]).to_string();
                                                if writer.send(Message::Text(response.into())).await.is_err() {
                                                    break;
                                                }
                                            }
                                            [kind, ..] if kind.as_str() == Some("CLOSE") => break,
                                            _ => {}
                                        }
                                    }
                                });
                            }
                        }
                    }
                });
            });
            let url = url_rx.recv().expect("relay url should be received");

            Self {
                url,
                events,
                shutdown_tx: Some(shutdown_tx),
                join_handle: Some(join_handle),
            }
        }

        fn url(&self) -> &str {
            self.url.as_str()
        }

        fn event_count(&self) -> usize {
            self.events.lock().expect("relay events lock").len()
        }

        fn push_event(&self, event: &radroots_nostr::prelude::RadrootsNostrEvent) {
            self.events
                .lock()
                .expect("relay events lock")
                .push(serde_json::to_value(event).expect("relay event json"));
        }
    }

    fn relay_event_matches_filters(
        event: &serde_json::Value,
        filters: &[serde_json::Value],
    ) -> bool {
        filters.is_empty()
            || filters
                .iter()
                .any(|filter| relay_event_matches_filter(event, filter))
    }

    fn relay_event_matches_filter(event: &serde_json::Value, filter: &serde_json::Value) -> bool {
        let event_kind = event.get("kind").and_then(serde_json::Value::as_u64);
        if let Some(kinds) = filter.get("kinds").and_then(serde_json::Value::as_array)
            && !kinds
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .any(|kind| Some(kind) == event_kind)
        {
            return false;
        }

        let event_created_at = event.get("created_at").and_then(serde_json::Value::as_u64);
        if let Some(until) = filter.get("until").and_then(serde_json::Value::as_u64)
            && event_created_at.is_some_and(|created_at| created_at > until)
        {
            return false;
        }

        true
    }

    fn assert_missing_listing_provenance_relay_error(
        error: &AppSyncTransportError,
        relay_url: &str,
    ) {
        let AppSyncTransportError::Failed { message } = error else {
            panic!("unexpected error: {error}");
        };
        let value =
            serde_json::from_str::<serde_json::Value>(message).expect("structured relay error");
        assert_eq!(value["code"], "missing_listing_provenance_relay");
        assert_eq!(value["missing_provenance_relays"], json!([relay_url]));
    }

    impl Drop for ThreadedAckRelay {
        fn drop(&mut self) {
            if let Some(shutdown_tx) = self.shutdown_tx.take() {
                let _ = shutdown_tx.send(());
            }
            if let Some(join_handle) = self.join_handle.take() {
                let _ = join_handle.join();
            }
        }
    }

    fn install_recorded_sync_transport(
        runtime: &DesktopAppRuntime,
        transport: RecordedAppSyncTransport,
    ) -> Arc<Mutex<RecordedAppSyncTransport>> {
        let shared = Arc::new(Mutex::new(transport));
        runtime.lock_state_mut().sync_transport =
            Box::new(SharedRecordedSyncTransport(shared.clone()));
        shared
    }

    fn install_direct_relay_sync_transport(runtime: &DesktopAppRuntime, relay: &ThreadedAckRelay) {
        let accounts_manager = runtime
            .lock_state()
            .accounts_manager
            .as_ref()
            .expect("accounts manager")
            .clone();
        runtime.lock_state_mut().nostr_relay_urls = vec![relay.url().to_owned()];
        runtime.lock_state_mut().sync_transport =
            Box::new(SdkDirectRelayAppSyncTransport::with_relay_urls(
                accounts_manager,
                vec![relay.url().to_owned()],
            ));
    }

    fn configure_runtime_relay_ingest(runtime: &DesktopAppRuntime, relay: &ThreadedAckRelay) {
        runtime.lock_state_mut().nostr_relay_urls = vec![relay.url().to_owned()];
    }

    #[test]
    fn runtime_direct_relay_transport_publishes_typed_farm_work() {
        let relay_a = ThreadedAckRelay::spawn();
        let relay_b = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Farmer".to_owned()), true)
            .expect("local signing account should generate");
        let farm_id = FarmId::new();
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "farm_setup")
                .with_source_local_event_id("app:local_work:farm:direct"),
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed farm publish work should serialize");
        let mut transport = SdkDirectRelayAppSyncTransport::with_relay_urls(
            manager,
            vec![relay_a.url().to_owned(), relay_b.url().to_owned()],
        );

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("direct relay farm publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 1);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(result.published_receipts[0].event_kind, 30340);
        assert_eq!(
            result.published_receipts[0].source_account_id,
            account_id.to_string()
        );
        assert_eq!(
            result.published_receipts[0].relay_set_fingerprint,
            canonical_relay_set_fingerprint([relay_a.url(), relay_b.url()]).expect("fingerprint")
        );
        assert!(
            result.published_receipts[0]
                .relay_set_fingerprint
                .starts_with(CANONICAL_RELAY_SET_FINGERPRINT_VERSION)
        );
        assert_eq!(
            result.published_receipts[0]
                .source_local_event_id
                .as_deref(),
            Some("app:local_work:farm:direct")
        );
        assert_eq!(
            result.published_receipts[0].relay_delivery_json["acknowledged_relays"],
            json!([relay_a.url(), relay_b.url()])
        );
        assert_eq!(
            result.published_receipts[0].relay_delivery_json["state"],
            json!("acknowledged")
        );
    }

    #[test]
    fn runtime_direct_relay_transport_publishes_typed_listing_work() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Farmer".to_owned()), true)
            .expect("local signing account should generate");
        let identity = manager
            .get_signing_identity(&account_id)
            .expect("seller signer lookup should succeed")
            .expect("seller account should have local signer");
        let farm_id = FarmId::new();
        let product_id = ProductId::new();
        let payload = AppPublishPayload::Listing(AppListingPublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "listing_publish")
                .with_source_local_event_id("app:local_work:listing:direct"),
            product_id,
            listing_d_tag: Some(super::d_tag_from_uuid(product_id.as_uuid())),
            farm_id: Some(farm_id),
            farm_pubkey: Some(identity.public_key_hex()),
            farm_d_tag: Some(super::d_tag_from_uuid(farm_id.as_uuid())),
            title: "North field eggs".to_owned(),
            subtitle: Some("Pasture raised".to_owned()),
            category: Some("eggs".to_owned()),
            unit_label: "each".to_owned(),
            price_minor_units: Some(750),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(12),
            availability_window_id: Some(FulfillmentWindowId::new()),
            availability_starts_at: Some("2099-05-25T14:00:00Z".to_owned()),
            availability_ends_at: Some("2099-05-25T18:00:00Z".to_owned()),
            fulfillment_method: Some("pickup".to_owned()),
            fulfillment_location: Some("farmstand".to_owned()),
            status: ProductStatus::Published,
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed listing publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("direct relay listing publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 1);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(result.published_receipts[0].event_kind, 30402);
        assert_eq!(
            result.published_receipts[0].event_pubkey,
            identity.public_key_hex()
        );
        assert_eq!(
            result.published_receipts[0]
                .source_local_event_id
                .as_deref(),
            Some("app:local_work:listing:direct")
        );
        assert_eq!(relay.event_count(), 1);
    }

    #[test]
    fn runtime_direct_relay_transport_publishes_typed_order_request_work() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Buyer".to_owned()), true)
            .expect("local signing account should generate");
        let buyer_identity = manager
            .get_signing_identity(&account_id)
            .expect("buyer signer lookup should succeed")
            .expect("buyer account should have local signer");
        let seller_identity = RadrootsIdentity::generate();
        let product_id = ProductId::new();
        let order_id = OrderId::new();
        let listing_event_id = "1".repeat(64);
        let listing_addr = format!(
            "30402:{}:{}",
            seller_identity.public_key_hex(),
            super::d_tag_from_uuid(ProductId::new().as_uuid())
        );
        let order_document = RadrootsTradeOrderRequested {
            order_id: order_id.to_string(),
            listing_addr: listing_addr.clone(),
            buyer_pubkey: buyer_identity.public_key_hex(),
            seller_pubkey: seller_identity.public_key_hex(),
            items: vec![RadrootsTradeOrderItem {
                bin_id: "bin-1".to_owned(),
                bin_count: 1,
            }],
            economics: RadrootsTradeOrderEconomics {
                quote_id: format!("quote-{order_id}"),
                quote_version: 1,
                pricing_basis: RadrootsTradePricingBasis::ListingEvent,
                currency: RadrootsCoreCurrency::USD,
                items: vec![RadrootsTradeOrderEconomicItem {
                    bin_id: "bin-1".to_owned(),
                    bin_count: 1,
                    quantity_amount: RadrootsCoreDecimal::from(1u32),
                    quantity_unit: RadrootsCoreUnit::Each,
                    unit_price_amount: RadrootsCoreDecimal::from(5u32),
                    unit_price_currency: RadrootsCoreCurrency::USD,
                    line_subtotal: RadrootsCoreMoney::from_minor_units_u32(
                        500,
                        RadrootsCoreCurrency::USD,
                    ),
                }],
                discounts: Vec::new(),
                adjustments: Vec::new(),
                subtotal: RadrootsCoreMoney::from_minor_units_u32(500, RadrootsCoreCurrency::USD),
                discount_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
                adjustment_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
                total: RadrootsCoreMoney::from_minor_units_u32(500, RadrootsCoreCurrency::USD),
            },
        };
        let payload = AppPublishPayload::OrderRequest(AppOrderRequestPublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "place_personal_order")
                .with_source_local_event_id("app:local_work:order_request:direct"),
            order_id,
            farm_id: FarmId::new(),
            status: Some("needs_action".to_owned()),
            order_document_json: Some(json!({"document": {"order": order_document}})),
            listing_addr: Some(listing_addr),
            listing_event_id: Some(listing_event_id),
            listing_relays: vec![relay.url().to_owned()],
            buyer_pubkey: Some(buyer_identity.public_key_hex()),
            seller_pubkey: Some(seller_identity.public_key_hex()),
            items: vec![AppOrderRequestItemPayload {
                product_id,
                quantity: 1,
            }],
            currency_code: Some("USD".to_owned()),
            total_minor_units: Some(500),
            note: Some("coordinate pickup".to_owned()),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed order request publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("direct relay order request publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 1);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(result.published_receipts[0].event_kind, 3422);
        assert_eq!(
            result.published_receipts[0].event_pubkey,
            buyer_identity.public_key_hex()
        );
        assert_eq!(
            result.published_receipts[0]
                .source_local_event_id
                .as_deref(),
            Some("app:local_work:order_request:direct")
        );
        assert_eq!(relay.event_count(), 1);
    }

    #[test]
    fn runtime_direct_relay_transport_publishes_typed_order_decision_work() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Seller".to_owned()), true)
            .expect("local signing account should generate");
        let identity = manager
            .get_signing_identity(&account_id)
            .expect("seller signer lookup should succeed")
            .expect("seller account should have local signer");
        let buyer_pubkey = "1111111111111111111111111111111111111111111111111111111111111111";
        let payload = AppPublishPayload::OrderDecision(AppOrderDecisionPublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "seller_order_decision"),
            app_order_id: OrderId::new(),
            farm_id: FarmId::new(),
            trade_order_id: "order-1".to_owned(),
            request_event_id: "order-request-event-1".to_owned(),
            listing_event_id: Some("listing-event-1".to_owned()),
            listing_addr: format!("30402:{}:listing-key", identity.public_key_hex()),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: identity.public_key_hex(),
            decision: AppOrderDecisionPayload::Accepted {
                inventory_commitments: vec![AppOrderDecisionInventoryCommitment {
                    bin_id: "bin-1".to_owned(),
                    bin_count: 2,
                }],
            },
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed order decision publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("direct relay order decision publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 1);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(result.published_receipts[0].event_kind, 3423);
        assert_eq!(
            result.published_receipts[0].event_pubkey,
            identity.public_key_hex()
        );
        assert_eq!(relay.event_count(), 1);
    }

    #[test]
    fn runtime_direct_relay_transport_publishes_typed_order_lifecycle_work() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let buyer_account_id = manager
            .generate_identity(Some("Buyer".to_owned()), true)
            .expect("buyer account should generate");
        let seller_account_id = manager
            .generate_identity(Some("Seller".to_owned()), true)
            .expect("seller account should generate");
        let buyer_identity = manager
            .get_signing_identity(&buyer_account_id)
            .expect("buyer signer lookup should succeed")
            .expect("buyer account should have local signer");
        let seller_identity = manager
            .get_signing_identity(&seller_account_id)
            .expect("seller signer lookup should succeed")
            .expect("seller account should have local signer");
        let app_order_id = OrderId::new();
        let farm_id = FarmId::new();
        let listing_addr = format!(
            "30402:{}:AAAAAAAAAAAAAAAAAAAAAg",
            seller_identity.public_key_hex()
        );
        let common = (
            app_order_id,
            farm_id,
            "order-1".to_owned(),
            "order-request-event-1".to_owned(),
            listing_addr,
            buyer_identity.public_key_hex(),
            seller_identity.public_key_hex(),
        );
        let revision_economics = RadrootsTradeOrderEconomics {
            quote_id: "quote-revision-1".to_owned(),
            quote_version: 2,
            pricing_basis: RadrootsTradePricingBasis::ListingEvent,
            currency: RadrootsCoreCurrency::USD,
            items: vec![RadrootsTradeOrderEconomicItem {
                bin_id: "bin-1".to_owned(),
                bin_count: 3,
                quantity_amount: RadrootsCoreDecimal::from(1u32),
                quantity_unit: RadrootsCoreUnit::Each,
                unit_price_amount: RadrootsCoreDecimal::from(8u32),
                unit_price_currency: RadrootsCoreCurrency::USD,
                line_subtotal: RadrootsCoreMoney::from_minor_units_u32(
                    2400,
                    RadrootsCoreCurrency::USD,
                ),
            }],
            discounts: Vec::new(),
            adjustments: Vec::new(),
            subtotal: RadrootsCoreMoney::from_minor_units_u32(2400, RadrootsCoreCurrency::USD),
            discount_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
            adjustment_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
            total: RadrootsCoreMoney::from_minor_units_u32(2400, RadrootsCoreCurrency::USD),
        };
        let revision_proposal =
            AppPublishPayload::OrderRevisionProposal(AppOrderRevisionProposalPublishPayload {
                context: AppPublishContext::new(
                    seller_account_id.to_string(),
                    "seller_order_revision_proposal",
                ),
                app_order_id: common.0,
                farm_id: common.1,
                trade_order_id: common.2.clone(),
                request_event_id: common.3.clone(),
                prev_event_id: "order-decision-event-1".to_owned(),
                revision_id: "revision-1".to_owned(),
                listing_addr: common.4.clone(),
                buyer_pubkey: common.5.clone(),
                seller_pubkey: common.6.clone(),
                items: vec![RadrootsTradeOrderItem {
                    bin_id: "bin-1".to_owned(),
                    bin_count: 3,
                }],
                economics: revision_economics,
                reason: "harvest count updated".to_owned(),
            });
        let revision_decision =
            AppPublishPayload::OrderRevisionDecision(AppOrderRevisionDecisionPublishPayload {
                context: AppPublishContext::new(
                    buyer_account_id.to_string(),
                    "buyer_order_revision_decision",
                ),
                app_order_id: common.0,
                farm_id: common.1,
                trade_order_id: common.2.clone(),
                request_event_id: common.3.clone(),
                prev_event_id: "order-revision-proposal-event-1".to_owned(),
                revision_id: "revision-1".to_owned(),
                listing_addr: common.4.clone(),
                buyer_pubkey: common.5.clone(),
                seller_pubkey: common.6.clone(),
                decision: RadrootsTradeOrderRevisionDecision::Accepted,
            });
        let cancellation =
            AppPublishPayload::OrderCancellation(AppOrderCancellationPublishPayload {
                context: AppPublishContext::new(
                    buyer_account_id.to_string(),
                    "buyer_order_cancellation",
                ),
                app_order_id: common.0,
                farm_id: common.1,
                trade_order_id: common.2.clone(),
                request_event_id: common.3.clone(),
                prev_event_id: common.3.clone(),
                listing_addr: common.4.clone(),
                buyer_pubkey: common.5.clone(),
                seller_pubkey: common.6.clone(),
                reason: "buyer cancelled order".to_owned(),
            });
        let fulfillment = AppPublishPayload::OrderFulfillment(AppOrderFulfillmentPublishPayload {
            context: AppPublishContext::new(
                seller_account_id.to_string(),
                "seller_order_fulfillment",
            ),
            app_order_id: common.0,
            farm_id: common.1,
            trade_order_id: common.2.clone(),
            request_event_id: common.3.clone(),
            prev_event_id: "order-decision-event-1".to_owned(),
            listing_addr: common.4.clone(),
            buyer_pubkey: common.5.clone(),
            seller_pubkey: common.6.clone(),
            status: RadrootsActiveTradeFulfillmentState::ReadyForPickup,
        });
        let receipt = AppPublishPayload::OrderReceipt(AppOrderReceiptPublishPayload {
            context: AppPublishContext::new(buyer_account_id.to_string(), "buyer_order_receipt"),
            app_order_id: common.0,
            farm_id: common.1,
            trade_order_id: common.2.clone(),
            request_event_id: common.3.clone(),
            prev_event_id: "fulfillment-event-1".to_owned(),
            listing_addr: common.4.clone(),
            buyer_pubkey: common.5.clone(),
            seller_pubkey: common.6.clone(),
            received: true,
            issue: None,
            received_at: 1_785_000_000,
        });
        let operations = [
            revision_proposal,
            revision_decision,
            cancellation,
            fulfillment,
            receipt,
        ]
        .into_iter()
        .map(|payload| {
            PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
                .expect("typed lifecycle publish work should serialize")
        })
        .collect::<Vec<_>>();
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: operations,
                known_conflicts: Vec::new(),
            })
            .expect("direct relay lifecycle publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 5);
        assert_eq!(relay.event_count(), 5);
        let kinds = result
            .published_receipts
            .iter()
            .map(|receipt| receipt.event_kind)
            .collect::<Vec<_>>();
        assert_eq!(kinds, vec![3424, 3425, 3432, 3433, 3434]);
        for receipt in &result.published_receipts {
            let event = published_receipt_event(receipt);
            match receipt.event_kind {
                3424 => {
                    let envelope = radroots_sdk::trade::parse_order_revision_proposal(&event)
                        .expect("order revision proposal should parse");
                    assert_eq!(envelope.payload.reason, "harvest count updated");
                    assert_eq!(envelope.payload.items[0].bin_count, 3);
                }
                3425 => {
                    let envelope = radroots_sdk::trade::parse_order_revision_decision(&event)
                        .expect("order revision decision should parse");
                    assert_eq!(envelope.payload.revision_id, "revision-1");
                    assert_eq!(
                        envelope.payload.decision,
                        RadrootsTradeOrderRevisionDecision::Accepted
                    );
                }
                3432 => {
                    let envelope = radroots_sdk::trade::parse_order_cancellation(&event)
                        .expect("order cancellation should parse");
                    assert_eq!(envelope.payload.reason, "buyer cancelled order");
                }
                3433 => {
                    let envelope = radroots_sdk::trade::parse_fulfillment_update(&event)
                        .expect("fulfillment update should parse");
                    assert_eq!(
                        envelope.payload.status,
                        RadrootsActiveTradeFulfillmentState::ReadyForPickup
                    );
                }
                3434 => {
                    let envelope = radroots_sdk::trade::parse_buyer_receipt(&event)
                        .expect("buyer receipt should parse");
                    assert!(envelope.payload.received);
                }
                _ => panic!("unexpected lifecycle event kind"),
            }
        }
    }

    #[test]
    fn runtime_configured_relay_sync_triggers_ingest_listing_into_fresh_buyer_projection() {
        let relay = ThreadedAckRelay::spawn();
        let projected_product_id = publish_relay_ingest_listing_fixture(&relay);

        assert_fresh_buyer_relay_ingest(
            relay.url(),
            "relay_ingest_manual_refresh",
            SyncTrigger::ManualRefresh,
            projected_product_id,
        );
        assert_fresh_buyer_relay_ingest(
            relay.url(),
            "relay_ingest_app_launch",
            SyncTrigger::AppLaunch,
            projected_product_id,
        );
        assert_fresh_buyer_relay_ingest(
            relay.url(),
            "relay_ingest_foreground_resume",
            SyncTrigger::ForegroundResume,
            projected_product_id,
        );
    }

    #[test]
    fn runtime_relay_ingest_does_not_use_connected_relays_as_listing_provenance() {
        let listing_relay = ThreadedAckRelay::spawn();
        let empty_relay = ThreadedAckRelay::spawn();
        let projected_product_id = publish_relay_ingest_listing_fixture(&listing_relay);
        let (runtime, paths) = bootstrapped_runtime("relay_ingest_connected_not_provenance");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("buyer account should generate")
        );
        runtime.lock_state_mut().nostr_relay_urls =
            vec![listing_relay.url().to_owned(), empty_relay.url().to_owned()];

        assert!(
            runtime
                .sync_on_manual_refresh()
                .expect("manual relay ingest should complete")
        );

        let summary = runtime.summary();
        let listing = summary
            .personal_projection
            .browse
            .listings
            .rows
            .iter()
            .find(|listing| listing.product_id == projected_product_id)
            .expect("fresh buyer app should project relay listing");
        assert_eq!(listing.title, "Relay ingest lettuce");
        assert_eq!(listing.listing_relays, vec![listing_relay.url().to_owned()]);

        let product_id_string = projected_product_id.to_string();
        let imports = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_local_interop_records()
            .expect("local interop records should load");
        let listing_import = imports
            .iter()
            .find(|record| {
                record.projected_kind == "listing"
                    && record.projected_id.as_deref() == Some(product_id_string.as_str())
            })
            .expect("listing import");
        let delivery = serde_json::from_str::<serde_json::Value>(
            listing_import
                .relay_delivery_json
                .as_deref()
                .expect("listing delivery evidence"),
        )
        .expect("delivery json");

        assert_eq!(
            listing_import.source_runtime,
            SourceRuntime::Network.as_str()
        );
        assert_eq!(listing_import.outbox_status, "none");
        assert_eq!(delivery["state"], json!("observed"));
        assert_eq!(delivery["acknowledged_relays"], json!([]));
        assert_eq!(delivery["observed_relays"], json!([listing_relay.url()]));
        assert_eq!(
            delivery["target_relays"],
            json!([listing_relay.url(), empty_relay.url()])
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    fn publish_relay_ingest_listing_fixture(relay: &ThreadedAckRelay) -> ProductId {
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Farmer".to_owned()), true)
            .expect("local signing account should generate");
        let identity = manager
            .get_signing_identity(&account_id)
            .expect("seller signing lookup should succeed")
            .expect("seller account should have local signer");
        let seller_pubkey = identity.public_key_hex();
        let farm_id = FarmId::new();
        let product_id = ProductId::new();
        let listing_d_tag = super::d_tag_from_uuid(product_id.as_uuid());
        let projected_product_id = deterministic_cli_listing_product_id(
            Some(seller_pubkey.as_str()),
            listing_d_tag.as_str(),
        );
        let farm_payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "relay_ingest_farm"),
            farm_id,
            display_name: "Relay test farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let listing_payload = AppPublishPayload::Listing(AppListingPublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "relay_ingest_listing"),
            product_id,
            listing_d_tag: Some(listing_d_tag),
            farm_id: Some(farm_id),
            farm_pubkey: Some(seller_pubkey),
            farm_d_tag: Some(super::d_tag_from_uuid(farm_id.as_uuid())),
            title: "Relay ingest lettuce".to_owned(),
            subtitle: Some("Pulled into a fresh buyer app".to_owned()),
            category: Some("greens".to_owned()),
            unit_label: "each".to_owned(),
            price_minor_units: Some(450),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(6),
            availability_window_id: Some(FulfillmentWindowId::new()),
            availability_starts_at: Some("2099-04-25T14:00:00Z".to_owned()),
            availability_ends_at: Some("2099-04-25T18:00:00Z".to_owned()),
            fulfillment_method: Some("pickup".to_owned()),
            fulfillment_location: Some("Relay barn".to_owned()),
            status: ProductStatus::Published,
        });
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);
        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![
                    PendingSyncOperation::from_publish_payload(
                        farm_payload,
                        "2026-05-25T07:00:00Z",
                    )
                    .expect("farm publish payload should serialize"),
                    PendingSyncOperation::from_publish_payload(
                        listing_payload,
                        "2026-05-25T07:00:01Z",
                    )
                    .expect("listing publish payload should serialize"),
                ],
                known_conflicts: Vec::new(),
            })
            .expect("seller relay publish should succeed");
        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.published_receipts.len(), 2);

        projected_product_id
    }

    fn assert_fresh_buyer_relay_ingest(
        relay_url: &str,
        label: &str,
        trigger: SyncTrigger,
        projected_product_id: ProductId,
    ) {
        let (runtime, paths) = bootstrapped_runtime(label);
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("buyer account should generate")
        );
        runtime.lock_state_mut().nostr_relay_urls = vec![relay_url.to_owned()];

        let changed = match trigger {
            SyncTrigger::ManualRefresh => runtime
                .sync_on_manual_refresh()
                .expect("manual relay ingest should complete"),
            SyncTrigger::AppLaunch => runtime
                .sync_on_app_launch()
                .expect("launch relay ingest should complete"),
            SyncTrigger::ForegroundResume => runtime
                .sync_on_foreground_resume()
                .expect("foreground relay ingest should complete"),
            SyncTrigger::LocalMutation => panic!("local mutation is not a relay ingest trigger"),
        };
        assert!(changed);

        let summary = runtime.summary();
        let listing = summary
            .personal_projection
            .browse
            .listings
            .rows
            .iter()
            .find(|listing| listing.product_id == projected_product_id)
            .expect("fresh buyer app should project relay listing");
        assert_eq!(listing.title, "Relay ingest lettuce");
        assert_eq!(listing.farm_display_name, "Relay test farm");
        assert_eq!(listing.listing_relays, vec![relay_url.to_owned()]);
        let relay_ingest = runtime
            .lock_state()
            .selected_account_relay_ingest_freshness
            .clone();
        assert_eq!(relay_ingest.status, AppRelayIngestScopeStatus::Fresh);
        assert_eq!(relay_ingest.relays.len(), 1);
        assert_eq!(relay_ingest.relays[0].relay_url, relay_url);
        assert!(relay_ingest.relays[0].cursor_since_unix_seconds.is_some());

        let product_id_string = projected_product_id.to_string();
        let imports = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_local_interop_records()
            .expect("local interop records should load");
        let listing_import = imports
            .iter()
            .find(|record| {
                record.projected_kind == "listing"
                    && record.projected_id.as_deref() == Some(product_id_string.as_str())
            })
            .expect("listing import");
        let delivery = serde_json::from_str::<serde_json::Value>(
            listing_import
                .relay_delivery_json
                .as_deref()
                .expect("listing delivery evidence"),
        )
        .expect("delivery json");

        assert_eq!(
            listing_import.source_runtime,
            SourceRuntime::Network.as_str()
        );
        assert_eq!(listing_import.outbox_status, "none");
        assert_eq!(delivery["state"], json!("observed"));
        assert_eq!(delivery["acknowledged_relays"], json!([]));
        assert_eq!(delivery["observed_relays"], json!([relay_url]));
        assert_eq!(
            imports
                .iter()
                .filter(|record| record.projected_kind == "listing"
                    && record.projected_id.as_deref() == Some(product_id_string.as_str()))
                .count(),
            1
        );
        assert!(
            runtime
                .sync_on_manual_refresh()
                .expect("repeat relay ingest should complete")
        );
        let repeated_imports = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_local_interop_records()
            .expect("repeated local interop records should load");
        assert_eq!(
            repeated_imports
                .iter()
                .filter(|record| record.projected_kind == "listing"
                    && record.projected_id.as_deref() == Some(product_id_string.as_str()))
                .count(),
            1
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_relay_ingest_runs_after_outbound_sync_failure() {
        let relay = ThreadedAckRelay::spawn();
        let projected_product_id = publish_relay_ingest_listing_fixture(&relay);

        assert_relay_ingest_after_outbound_failure(
            relay.url(),
            "relay_ingest_after_manual_failure",
            SyncTrigger::ManualRefresh,
            projected_product_id,
        );
        assert_relay_ingest_after_outbound_failure(
            relay.url(),
            "relay_ingest_after_launch_failure",
            SyncTrigger::AppLaunch,
            projected_product_id,
        );
        assert_relay_ingest_after_outbound_failure(
            relay.url(),
            "relay_ingest_after_foreground_failure",
            SyncTrigger::ForegroundResume,
            projected_product_id,
        );
    }

    fn assert_relay_ingest_after_outbound_failure(
        relay_url: &str,
        label: &str,
        trigger: SyncTrigger,
        projected_product_id: ProductId,
    ) {
        let (runtime, paths) = bootstrapped_runtime(label);
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("buyer account should generate")
        );
        runtime.lock_state_mut().nostr_relay_urls = vec![relay_url.to_owned()];
        let buyer_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let pending_farm_id = FarmId::new();
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Farm(pending_farm_id),
                farm_sync_payload(
                    pending_farm_id,
                    "Pending outbound farm",
                    Some(FarmReadiness::Ready),
                    "relay_ingest_after_outbound_failure",
                ),
            )])
            .expect("pending sync should enqueue");
        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::fail(AppSyncTransportError::unavailable(
                "test outbound sync unavailable",
            )),
        );

        let changed = match trigger {
            SyncTrigger::ManualRefresh => runtime
                .sync_on_manual_refresh()
                .expect("manual refresh should complete"),
            SyncTrigger::AppLaunch => runtime
                .sync_on_app_launch()
                .expect("launch sync should complete"),
            SyncTrigger::ForegroundResume => runtime
                .sync_on_foreground_resume()
                .expect("foreground sync should complete"),
            SyncTrigger::LocalMutation => panic!("local mutation is not a relay ingest trigger"),
        };

        assert!(changed);
        assert_eq!(recorded.lock().expect("recorded transport").call_count(), 1);
        let summary = runtime.summary();
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Failed
        );
        assert_eq!(
            summary.sync_status.projection.checkpoint.state,
            SyncCheckpointState::Failed
        );
        assert_eq!(
            runtime
                .lock_state()
                .selected_account_relay_ingest_freshness
                .status,
            AppRelayIngestScopeStatus::Fresh
        );
        assert_eq!(summary.sync_status.pending_write_count, 1);
        let listing = summary
            .personal_projection
            .browse
            .listings
            .rows
            .iter()
            .find(|listing| listing.product_id == projected_product_id)
            .expect("relay listing should still project after outbound failure");
        assert_eq!(listing.title, "Relay ingest lettuce");
        assert_eq!(listing.listing_relays, vec![relay_url.to_owned()]);

        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(buyer_account_id.as_str())
            .expect("pending sync operations should load");
        assert_eq!(pending_operations.len(), 1);
        assert_eq!(pending_operations[0].operation.attempt_count, 1);
        assert!(
            pending_operations[0]
                .operation
                .last_error_message
                .as_deref()
                .is_some_and(|message| message.contains("test outbound sync unavailable"))
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_direct_relay_transport_returns_partial_failure_after_successful_prefix() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Farmer".to_owned()), true)
            .expect("local signing account should generate");
        let farm_id = FarmId::new();
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "farm_setup"),
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let successful_operation =
            PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
                .expect("typed farm publish work should serialize");
        let unsupported_operation = PendingSyncOperation::new(
            SyncAggregateRef::Product(ProductId::new()),
            SyncOperationKind::Delete,
            "{}",
            "2026-05-24T12:01:00Z",
        );
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![successful_operation, unsupported_operation],
                known_conflicts: Vec::new(),
            })
            .expect("successful prefix should return a partial result");

        assert_eq!(result.run_status, AppSyncRunStatus::Failed);
        assert_eq!(result.pushed_operation_count, 1);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(result.checkpoint.state, SyncCheckpointState::Failed);
        assert!(
            result
                .checkpoint
                .last_error_message
                .as_deref()
                .is_some_and(|message| message.contains("supports upsert"))
        );
    }

    #[test]
    fn runtime_direct_relay_transport_normalizes_configured_relay_set() {
        let relay_urls = super::normalized_app_sync_relay_urls(&[
            " ws://127.0.0.1:8081 ".to_owned(),
            "ws://127.0.0.1:8080".to_owned(),
            "ws://127.0.0.1:8081".to_owned(),
        ])
        .expect("relay set should normalize");

        assert_eq!(
            relay_urls,
            vec!["ws://127.0.0.1:8081", "ws://127.0.0.1:8080"]
        );
    }

    #[test]
    fn runtime_direct_relay_transport_rejects_invalid_configured_relay_urls() {
        for relay_url in [
            " ",
            "https://relay.example",
            "wss://",
            "wss://user@relay.example",
            "wss://relay.example:abc",
        ] {
            let error = super::normalized_app_sync_relay_urls(&[relay_url.to_owned()])
                .expect_err("invalid app sync relay url");
            assert!(
                error.to_string().contains("relay url"),
                "unexpected error for {relay_url}: {error}"
            );
        }
    }

    #[test]
    fn order_request_listing_pointer_prefers_configured_listing_relay() {
        let selected = super::selected_listing_relay(
            &[
                "wss://relay-b.example".to_owned(),
                "wss://relay-a.example".to_owned(),
            ],
            &[
                "wss://relay-a.example".to_owned(),
                "wss://relay-c.example".to_owned(),
            ],
        )
        .expect("configured listing relay should be selected");

        assert_eq!(selected.as_str(), "wss://relay-a.example");
    }

    #[test]
    fn order_request_listing_pointer_rejects_missing_configured_provenance_relay() {
        let error = super::selected_listing_relay(
            &["wss://listing.example".to_owned()],
            &["wss://target.example".to_owned()],
        )
        .expect_err("missing listing provenance relay should fail");

        assert_missing_listing_provenance_relay_error(&error, "wss://listing.example");
    }

    #[test]
    fn runtime_direct_relay_transport_rejects_order_request_missing_listing_provenance_target() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let account_id = manager
            .generate_identity(Some("Buyer".to_owned()), true)
            .expect("buyer account should generate");
        let identity = manager
            .get_signing_identity(&account_id)
            .expect("buyer signer lookup should succeed")
            .expect("buyer account should have local signer");
        let seller_pubkey = "2222222222222222222222222222222222222222222222222222222222222222";
        let payload = AppPublishPayload::OrderRequest(AppOrderRequestPublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "order_missing_listing_relay"),
            order_id: OrderId::new(),
            farm_id: FarmId::new(),
            status: Some("needs_action".to_owned()),
            order_document_json: Some(json!({"document": {"order": {}}})),
            listing_addr: Some(format!("30402:{seller_pubkey}:listing-key")),
            listing_event_id: Some("listing-event-id".to_owned()),
            listing_relays: vec!["wss://listing.example".to_owned()],
            buyer_pubkey: Some(identity.public_key_hex()),
            seller_pubkey: Some(seller_pubkey.to_owned()),
            items: vec![AppOrderRequestItemPayload {
                product_id: ProductId::new(),
                quantity: 1,
            }],
            currency_code: Some("USD".to_owned()),
            total_minor_units: Some(450),
            note: None,
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-25T07:00:00Z")
            .expect("order publish payload should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let error = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect_err("missing listing provenance relay should fail");

        assert_missing_listing_provenance_relay_error(&error, "wss://listing.example");
        assert_eq!(relay.event_count(), 0);
    }

    #[test]
    fn runtime_direct_relay_transport_signs_with_payload_account_context() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let first_account_id = manager
            .generate_identity(Some("First".to_owned()), true)
            .expect("first account");
        let second_account_id = manager
            .generate_identity(Some("Second".to_owned()), true)
            .expect("second account");
        let first_identity = manager
            .get_signing_identity(&first_account_id)
            .expect("first signer")
            .expect("first local signer");
        let second_identity = manager
            .get_signing_identity(&second_account_id)
            .expect("second signer")
            .expect("second local signer");
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(first_account_id.to_string(), "farm_setup"),
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed farm publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("payload account signer should publish");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.published_receipts.len(), 1);
        assert_eq!(
            result.published_receipts[0].event_pubkey,
            first_identity.public_key_hex()
        );
        assert_ne!(
            result.published_receipts[0].event_pubkey,
            second_identity.public_key_hex()
        );
    }

    #[test]
    fn runtime_direct_relay_transport_rejects_missing_account_context() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let farm_id = FarmId::new();
        let missing_account_id = RadrootsIdentity::generate().id();
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(missing_account_id.to_string(), "farm_setup"),
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed farm publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let error = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect_err("watch-only or missing custody should not publish");

        assert!(matches!(error, AppSyncTransportError::Unavailable { .. }));
        assert!(
            error
                .to_string()
                .contains("publish account is not configured locally")
        );
    }

    #[test]
    fn runtime_direct_relay_transport_rejects_watch_only_account_context() {
        let relay = ThreadedAckRelay::spawn();
        let manager = RadrootsNostrAccountsManager::new_in_memory();
        let identity = RadrootsIdentity::generate();
        let account_id = manager
            .upsert_public_identity(identity.to_public(), Some("Watch".to_owned()), true)
            .expect("watch-only account");
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "farm_setup"),
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed farm publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let error = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect_err("watch-only account should not publish");

        assert!(matches!(error, AppSyncTransportError::Unavailable { .. }));
        assert!(
            error
                .to_string()
                .contains("publish account is not backed by a local signing key")
        );
    }

    #[test]
    fn runtime_direct_relay_transport_rejects_mismatched_local_signing_custody() {
        let relay = ThreadedAckRelay::spawn();
        let store = Arc::new(RadrootsNostrMemoryAccountStore::new());
        let vault = Arc::new(RadrootsNostrSecretVaultMemory::new());
        let manager =
            RadrootsNostrAccountsManager::new(store, vault.clone()).expect("accounts manager");
        let account_identity = RadrootsIdentity::generate();
        let secret_identity = RadrootsIdentity::generate();
        let account_id = manager
            .upsert_public_identity(
                account_identity.to_public(),
                Some("Mismatched".to_owned()),
                true,
            )
            .expect("public account");
        vault
            .store_secret(
                account_secret_slot(&account_id).as_str(),
                secret_identity.secret_key_hex().as_str(),
            )
            .expect("mismatched secret");
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new(account_id.to_string(), "farm_setup"),
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("typed farm publish work should serialize");
        let mut transport =
            SdkDirectRelayAppSyncTransport::with_relay_urls(manager, vec![relay.url().to_owned()]);

        let error = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect_err("mismatched custody should not publish");

        assert!(matches!(error, AppSyncTransportError::Failed { .. }));
        assert!(
            error
                .to_string()
                .contains("public key does not match secret key")
        );
    }

    #[test]
    fn desktop_namespace_uses_canonical_app_and_shared_runtime_roots() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(PathBuf::from("/Users/treesap")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("interactive user roots should resolve");

        assert_eq!(
            paths.app.data,
            PathBuf::from("/Users/treesap/.radroots/data/apps/app")
        );
        assert_eq!(
            paths.app.logs,
            PathBuf::from("/Users/treesap/.radroots/logs/apps/app")
        );
        assert_eq!(
            paths.app.data.join(APP_DATABASE_FILE_NAME),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app/app.sqlite3")
        );
        assert_eq!(
            paths.shared_accounts.data_root,
            PathBuf::from("/Users/treesap/.radroots/data/shared/accounts")
        );
        assert_eq!(
            paths.shared_accounts.secrets_root,
            PathBuf::from("/Users/treesap/.radroots/secrets/shared/accounts")
        );
        assert_eq!(
            paths.shared_accounts.store_path,
            PathBuf::from("/Users/treesap/.radroots/data/shared/accounts")
                .join(SHARED_ACCOUNTS_STORE_FILE_NAME)
        );
        assert_eq!(
            paths.shared_identity.default_identity_path,
            PathBuf::from("/Users/treesap/.radroots/secrets/shared/identities")
                .join(SHARED_IDENTITY_FILE_NAME)
        );
    }

    #[test]
    fn cloned_runtime_handles_shared_settings_state() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();

        assert!(runtime.sync_settings_section(SettingsSection::About));
        assert!(cloned_runtime.set_settings_preference(SettingsPreference::LaunchAtLogin, true));

        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::About
        );
        assert!(summary.shell_projection.settings.general.launch_at_login);
        assert_eq!(
            cloned_runtime.selected_settings_section(),
            SettingsSection::About
        );
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert_eq!(summary.home_route, HomeRoute::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert!(
            summary
                .settings_account_projection
                .selected_account
                .is_none()
        );
        assert_eq!(
            summary.logged_out_startup,
            LoggedOutStartupProjection::default()
        );
    }

    #[test]
    fn cloned_runtime_handles_shared_startup_identity_choice_state() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();

        assert!(runtime.show_startup_identity_choice());
        assert!(cloned_runtime.show_startup_signer_entry());
        assert!(cloned_runtime.set_startup_signer_source_input(
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example"
        ));
        assert!(runtime.begin_generate_key_startup());

        let summary = runtime.summary();

        assert_eq!(
            summary.logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::GenerateKeyStarting
        );
        assert_eq!(
            summary.logged_out_startup.signer_entry.source_input,
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example"
        );
    }

    #[test]
    fn runtime_summary_keeps_sync_disabled_without_a_selected_account() {
        let runtime = memory_runtime();
        let summary = runtime.summary();

        assert_eq!(summary.sync_status, DesktopAppSyncStatusSummary::default());
        assert!(!summary.sync_status.is_enabled());
    }

    #[test]
    fn runtime_summary_refreshes_selected_account_sync_status_from_sqlite() {
        let (runtime, paths) = bootstrapped_runtime("selected_account_sync_status");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        {
            let state = runtime.lock_state();
            let sqlite_store = state
                .sqlite_store
                .as_ref()
                .expect("sqlite store should exist");

            sqlite_store
                .save_sync_checkpoint(
                    &account_id,
                    &SyncCheckpointStatus::current(
                        None,
                        "2026-04-20T19:00:00Z",
                        Some("cursor-3".to_owned()),
                    ),
                )
                .expect("sync checkpoint should save");
            sqlite_store
                .record_sync_conflict(
                    &account_id,
                    &SyncConflict {
                        aggregate: SyncAggregateRef::Farm(farm_id),
                        kind: SyncConflictKind::RevisionMismatch,
                        severity: SyncConflictSeverity::Blocking,
                        resolution: SyncConflictResolutionStatus::Unresolved,
                        local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                        remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                        detected_at: "2026-04-20T19:01:00Z".to_owned(),
                        resolved_at: None,
                    },
                )
                .expect("sync conflict should save");
            sqlite_store
                .enqueue_pending_sync_operation(
                    &account_id,
                    &PendingSyncOperation::new(
                        SyncAggregateRef::Farm(farm_id),
                        SyncOperationKind::Upsert,
                        "{\"farm\":\"queued\"}",
                        "2026-04-20T19:02:00Z",
                    ),
                )
                .expect("pending sync operation should save");
        }

        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        let summary = runtime.summary();

        assert_eq!(
            summary.sync_status.account_id.as_deref(),
            Some(account_id.as_str())
        );
        assert!(summary.sync_status.is_enabled());
        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Conflicted
        );
        assert_eq!(
            summary
                .sync_status
                .projection
                .conflict_status
                .unresolved_count,
            1
        );
        assert_eq!(
            summary
                .sync_status
                .projection
                .checkpoint
                .last_remote_cursor
                .as_deref(),
            Some("cursor-3")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_product_incomplete_save_does_not_enqueue_publish_work() {
        let runtime = memory_runtime();
        let (account_id, _) = provision_ready_farmer_account(&runtime);

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );
        let product_id = match runtime.summary().products_projection.editor {
            radroots_studio_app_state::ProductEditorState::Open(session) => session
                .selected_product_id
                .expect("open product editor should select a product"),
            radroots_studio_app_state::ProductEditorState::Closed => {
                panic!("product editor should be open")
            }
        };
        let first_draft = ProductEditorDraft {
            title: "Salad mix".to_owned(),
            subtitle: "Spring blend".to_owned(),
            category: String::new(),
            unit_label: "bag".to_owned(),
            price_minor_units: Some(700),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(8),
            availability_window_id: None,
            status: ProductStatus::Draft,
        };
        let second_draft = ProductEditorDraft {
            title: "Winter greens".to_owned(),
            subtitle: "Cut this morning".to_owned(),
            category: "greens".to_owned(),
            unit_label: "bag".to_owned(),
            price_minor_units: Some(900),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(11),
            availability_window_id: None,
            status: ProductStatus::Published,
        };

        assert!(
            runtime
                .save_product_editor_draft(first_draft)
                .expect("first product editor save should succeed")
        );
        assert!(
            runtime
                .save_product_editor_draft(second_draft.clone())
                .expect("second product editor save should succeed")
        );

        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");

        assert_eq!(pending_operations.len(), 0);
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_product_editor_draft(product_id)
                .expect("saved product draft should load"),
            Some(second_draft)
        );
    }

    #[test]
    fn runtime_product_publishable_save_enqueues_typed_listing_publish_work() {
        let (runtime, paths) = bootstrapped_runtime("publishable_product_listing_work");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();

        runtime
            .save_farm_rules_projection(FarmRulesProjection {
                farm_profile: Some(FarmProfileRecord {
                    farm_id,
                    display_name: "North field farm".to_owned(),
                    timezone: "UTC".to_owned(),
                    currency_code: "USD".to_owned(),
                }),
                pickup_locations: vec![PickupLocationRecord {
                    pickup_location_id,
                    farm_id,
                    label: "Barn pickup".to_owned(),
                    address_line: "14 Orchard Lane".to_owned(),
                    directions: None,
                    is_default: true,
                }],
                operating_rules: Some(FarmOperatingRulesRecord {
                    farm_id,
                    promise_lead_hours: 24,
                    substitution_policy: "ask_customer".to_owned(),
                    missed_pickup_policy: "hold_next_window".to_owned(),
                }),
                fulfillment_windows: vec![FulfillmentWindowRecord {
                    fulfillment_window_id,
                    farm_id,
                    pickup_location_id,
                    label: "Friday pickup".to_owned(),
                    starts_at: "2099-04-25T14:00:00Z".to_owned(),
                    ends_at: "2099-04-25T18:00:00Z".to_owned(),
                    order_cutoff_at: "2099-04-24T18:00:00Z".to_owned(),
                }],
                blackout_periods: Vec::new(),
                ..runtime
                    .load_farm_rules_projection()
                    .expect("farm rules projection should load")
            })
            .expect("farm rules should save");

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );
        let product_id = match runtime.summary().products_projection.editor {
            radroots_studio_app_state::ProductEditorState::Open(session) => session
                .selected_product_id
                .expect("open product editor should select a product"),
            radroots_studio_app_state::ProductEditorState::Closed => {
                panic!("product editor should be open")
            }
        };

        assert!(
            runtime
                .save_product_editor_draft(ProductEditorDraft {
                    title: "Salad mix".to_owned(),
                    subtitle: "Cut this morning".to_owned(),
                    category: "greens".to_owned(),
                    unit_label: "bag".to_owned(),
                    price_minor_units: Some(900),
                    price_currency: "usd".to_owned(),
                    stock_quantity: Some(11),
                    availability_window_id: Some(fulfillment_window_id),
                    status: ProductStatus::Published,
                })
                .expect("publishable product save should succeed")
        );

        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");
        let product_pending_operations = pending_operations
            .iter()
            .filter(|pending| pending.operation.aggregate == SyncAggregateRef::Product(product_id))
            .collect::<Vec<_>>();
        assert_eq!(product_pending_operations.len(), 1);
        assert_eq!(
            product_pending_operations[0].operation.operation_key,
            format!("product:{product_id}:upsert")
        );
        assert_eq!(
            product_pending_operations[0].operation.state,
            PendingSyncOperationState::Pending
        );
        let publish_payload = product_pending_operations[0]
            .operation
            .publish_payload()
            .expect("product publish operation should be typed");
        let AppPublishPayload::Listing(payload) = publish_payload else {
            panic!("product publish operation should carry listing payload")
        };
        assert_eq!(payload.product_id, product_id);
        assert_eq!(payload.category.as_deref(), Some("greens"));
        assert_eq!(payload.unit_label, "bag");
        assert_eq!(payload.price_minor_units, Some(900));
        assert_eq!(payload.price_currency, "USD");
        assert_eq!(payload.stock_quantity, Some(11));
        assert_eq!(
            payload.availability_starts_at.as_deref(),
            Some("2099-04-25T14:00:00Z")
        );
        assert_eq!(
            payload.availability_ends_at.as_deref(),
            Some("2099-04-25T18:00:00Z")
        );
        assert_eq!(payload.fulfillment_method.as_deref(), Some("pickup"));
        assert_eq!(
            payload.fulfillment_location.as_deref(),
            Some("14 Orchard Lane")
        );
        assert!(payload.farm_pubkey.as_deref().is_some_and(super::is_hex_64));
        assert!(
            payload
                .context
                .source_local_event_id
                .as_deref()
                .is_some_and(|value| value.starts_with("app:local_work:listing:"))
        );

        let records = shared_local_event_records(&paths);
        let listing_record = records
            .iter()
            .find(|record| {
                record
                    .local_work_json
                    .as_ref()
                    .and_then(|payload| payload["record_kind"].as_str())
                    == Some("listing_draft_v1")
            })
            .expect("listing local work record");
        let listing_payload = listing_record
            .local_work_json
            .as_ref()
            .expect("listing local work payload");
        assert_eq!(listing_payload["publishability"]["state"], "publishable");
        assert_eq!(listing_payload["document"]["product"]["category"], "greens");
        assert_eq!(
            listing_payload["document"]["primary_bin"]["bin_id"]
                .as_str()
                .expect("primary bin id should be present"),
            super::listing_primary_bin_id(
                payload
                    .listing_d_tag
                    .as_deref()
                    .expect("listing d tag should exist")
            )
        );
        assert_eq!(listing_payload["document"]["delivery"]["method"], "pickup");
        assert_eq!(
            listing_payload["document"]["location"]["primary"],
            "14 Orchard Lane"
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_product_stale_availability_save_records_blocker_without_publish_work() {
        let (runtime, paths) = bootstrapped_runtime("stale_product_listing_work");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        let pickup_location_id = PickupLocationId::new();
        let active_window_id = FulfillmentWindowId::new();
        let stale_window_id = FulfillmentWindowId::new();

        runtime
            .save_farm_rules_projection(FarmRulesProjection {
                farm_profile: Some(FarmProfileRecord {
                    farm_id,
                    display_name: "North field farm".to_owned(),
                    timezone: "UTC".to_owned(),
                    currency_code: "USD".to_owned(),
                }),
                pickup_locations: vec![PickupLocationRecord {
                    pickup_location_id,
                    farm_id,
                    label: "Barn pickup".to_owned(),
                    address_line: "14 Orchard Lane".to_owned(),
                    directions: None,
                    is_default: true,
                }],
                operating_rules: Some(FarmOperatingRulesRecord {
                    farm_id,
                    promise_lead_hours: 24,
                    substitution_policy: "ask_customer".to_owned(),
                    missed_pickup_policy: "hold_next_window".to_owned(),
                }),
                fulfillment_windows: vec![FulfillmentWindowRecord {
                    fulfillment_window_id: active_window_id,
                    farm_id,
                    pickup_location_id,
                    label: "Friday pickup".to_owned(),
                    starts_at: "2099-04-25T14:00:00Z".to_owned(),
                    ends_at: "2099-04-25T18:00:00Z".to_owned(),
                    order_cutoff_at: "2099-04-24T18:00:00Z".to_owned(),
                }],
                blackout_periods: Vec::new(),
                ..runtime
                    .load_farm_rules_projection()
                    .expect("farm rules projection should load")
            })
            .expect("farm rules should save");

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );
        let product_id = match runtime.summary().products_projection.editor {
            radroots_studio_app_state::ProductEditorState::Open(session) => session
                .selected_product_id
                .expect("open product editor should select a product"),
            radroots_studio_app_state::ProductEditorState::Closed => {
                panic!("product editor should be open")
            }
        };

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("foreign keys should disable for stale fixture");
        let save_result = runtime.save_product_editor_draft(ProductEditorDraft {
            title: "Salad mix".to_owned(),
            subtitle: "Cut this morning".to_owned(),
            category: "greens".to_owned(),
            unit_label: "bag".to_owned(),
            price_minor_units: Some(900),
            price_currency: "usd".to_owned(),
            stock_quantity: Some(11),
            availability_window_id: Some(stale_window_id),
            status: ProductStatus::Published,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("foreign keys should restore");
        assert!(save_result.expect("stale product editor save should succeed"));

        let summary = runtime.summary();
        let radroots_studio_app_state::ProductEditorState::Open(session) =
            summary.products_projection.editor
        else {
            panic!("product editor should stay open")
        };
        assert_eq!(
            session.publish_blockers,
            vec![ProductPublishBlocker::AttachAvailability]
        );

        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");
        let product_pending_operations = pending_operations
            .iter()
            .filter(|pending| pending.operation.aggregate == SyncAggregateRef::Product(product_id))
            .collect::<Vec<_>>();
        assert!(product_pending_operations.is_empty());

        let records = shared_local_event_records(&paths);
        let listing_record = records
            .iter()
            .find(|record| {
                record
                    .local_work_json
                    .as_ref()
                    .and_then(|payload| payload["record_kind"].as_str())
                    == Some("listing_draft_v1")
            })
            .expect("listing local work record");
        let listing_payload = listing_record
            .local_work_json
            .as_ref()
            .expect("listing local work payload");
        assert_eq!(listing_payload["publishability"]["state"], "blocked");
        assert_eq!(
            listing_payload["publishability"]["blockers"],
            json!(["attach_availability"])
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_product_local_drafts_do_not_enqueue_publish_work_without_required_fields() {
        let runtime = memory_runtime();
        let (account_id, _) = provision_ready_farmer_account(&runtime);
        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    None,
                    "2026-04-20T19:30:00Z",
                    Some("cursor-product".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );

        let summary = runtime.summary();
        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");

        assert_eq!(recorded.lock().expect("recorded transport").call_count(), 0);
        assert_eq!(summary.sync_status.pending_write_count, 0);
        assert_eq!(pending_operations.len(), 0);
    }

    #[test]
    fn runtime_launch_sync_attempt_dequeues_pushed_operations() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Farm(farm_id),
                farm_sync_payload(
                    farm_id,
                    "North field farm",
                    Some(FarmReadiness::Ready),
                    "launch_sync_attempt_dequeues_pushed_operations",
                ),
            )])
            .expect("pending farm sync should enqueue");

        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    Some("2026-04-20T19:40:00Z".to_owned()),
                    "2026-04-20T19:40:05Z",
                    Some("cursor-launch".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            runtime
                .sync_on_app_launch()
                .expect("launch sync should succeed")
        );

        let summary = runtime.summary();
        let recorded = recorded.lock().expect("recorded transport");
        let request = recorded
            .last_request()
            .cloned()
            .expect("launch sync request should record");

        assert_eq!(recorded.call_count(), 1);
        assert_eq!(request.trigger, SyncTrigger::AppLaunch);
        assert_eq!(request.pending_operations.len(), 1);
        assert_eq!(summary.sync_status.pending_write_count, 0);
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Succeeded
        );
        assert_eq!(
            summary.sync_status.projection.checkpoint.state,
            SyncCheckpointState::Current
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_pending_sync_operations(account_id.as_str())
                .expect("pending sync operations should load")
                .len(),
            0
        );
    }

    #[test]
    fn runtime_sync_result_refreshes_sync_status_after_receipt_import_changes() {
        let (runtime, paths) = bootstrapped_runtime("sync_status_after_receipt_import");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Farm(farm_id),
                farm_sync_payload(
                    farm_id,
                    "Receipt import farm",
                    Some(FarmReadiness::Ready),
                    "sync_result_refreshes_after_receipt_import",
                ),
            )])
            .expect("pending farm sync should enqueue");

        install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    Some("2026-04-20T19:41:00Z".to_owned()),
                    "2026-04-20T19:41:05Z",
                    Some("cursor-receipt-import".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: vec![published_operation_receipt_fixture(
                    account_id.to_string(),
                    None,
                    "1111111111111111111111111111111111111111111111111111111111111111",
                )],
            }),
        );

        assert!(
            runtime
                .sync_on_app_launch()
                .expect("launch sync should import published receipt")
        );

        let summary = runtime.summary();
        assert_eq!(summary.sync_status.pending_write_count, 0);
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Succeeded
        );
        assert_eq!(
            summary.sync_status.projection.checkpoint.state,
            SyncCheckpointState::Current
        );
        assert_eq!(
            shared_local_event_records(&paths)
                .into_iter()
                .filter(|record| record.family == LocalRecordFamily::SignedEvent)
                .count(),
            1
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_partial_sync_result_dequeues_successful_prefix_only() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        let product_id = ProductId::new();
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![
                pending_sync_upsert(
                    SyncAggregateRef::Farm(farm_id),
                    farm_sync_payload(
                        farm_id,
                        "North field farm",
                        Some(FarmReadiness::Ready),
                        "partial_sync_prefix",
                    ),
                ),
                pending_sync_upsert(SyncAggregateRef::Product(product_id), "{}".to_owned()),
            ])
            .expect("pending sync should enqueue");

        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Failed,
                checkpoint: SyncCheckpointStatus::failed(
                    Some("2026-04-20T19:45:00Z".to_owned()),
                    Some("2026-04-20T19:45:05Z".to_owned()),
                    Some("cursor-partial".to_owned()),
                    "relay refused second operation",
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            runtime
                .sync_on_app_launch()
                .expect("partial launch sync should apply")
        );

        let summary = runtime.summary();
        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending operations should load");

        assert_eq!(recorded.lock().expect("recorded transport").call_count(), 1);
        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Failed
        );
        assert_eq!(pending_operations.len(), 1);
        assert_eq!(
            pending_operations[0].operation.aggregate,
            SyncAggregateRef::Product(product_id)
        );
        assert_eq!(
            pending_operations[0].operation.state,
            PendingSyncOperationState::Retryable
        );
        assert_eq!(pending_operations[0].operation.attempt_count, 1);
        assert_eq!(
            pending_operations[0]
                .operation
                .last_error_message
                .as_deref(),
            Some("relay refused second operation")
        );
    }

    #[test]
    fn runtime_foreground_resume_sync_uses_the_resume_trigger() {
        let runtime = memory_runtime();
        let (_, _) = provision_ready_farmer_account(&runtime);

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );

        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    Some("2026-04-20T19:50:00Z".to_owned()),
                    "2026-04-20T19:50:03Z",
                    Some("cursor-resume".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            runtime
                .sync_on_foreground_resume()
                .expect("resume sync should succeed")
        );

        let request = recorded
            .lock()
            .expect("recorded transport")
            .last_request()
            .cloned()
            .expect("resume sync request should record");

        assert_eq!(request.trigger, SyncTrigger::ForegroundResume);
    }

    #[test]
    fn runtime_shared_local_events_refresh_reports_and_reloads_products() {
        let (runtime, paths) = bootstrapped_runtime("shared_local_events_refresh");
        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        append_cli_local_listing_records(&paths, account_id.as_str());

        let report = runtime
            .refresh_shared_local_events()
            .expect("shared local events should refresh");
        let summary = runtime.summary();

        assert_eq!(report.scanned_records, 2);
        assert_eq!(report.imported_records, 2);
        assert_eq!(report.skipped_records, 0);
        assert_eq!(summary.farm_setup_projection.draft.farm_name, "Green Farm");
        let saved_farm_id = summary
            .farm_setup_projection
            .saved_farm
            .as_ref()
            .expect("saved farm should import")
            .farm_id;
        let direct_products = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_products(
                saved_farm_id,
                "",
                ProductsFilter::Drafts,
                ProductsSort::default(),
            )
            .expect("imported products should load directly");
        assert_eq!(direct_products.rows.len(), 1);
        assert!(
            runtime
                .select_products_filter(ProductsFilter::Drafts)
                .expect("draft products filter should reload")
        );
        let summary = runtime.summary();
        assert_eq!(summary.products_projection.list.rows.len(), 1);
        assert_eq!(summary.products_projection.list.rows[0].title, "Eggs");
        assert_eq!(
            summary.products_projection.list.rows[0].status,
            ProductStatus::Draft
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_search_imports_shared_local_events_before_read() {
        let (runtime, paths) = bootstrapped_runtime("buyer_search_shared_local_events_refresh");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        assert_eq!(
            runtime
                .summary()
                .personal_projection
                .search
                .listings
                .rows
                .len(),
            0
        );

        append_cli_signed_buyer_listing_record(&paths);

        assert!(
            runtime
                .set_personal_search_query("eggs")
                .expect("buyer search query should refresh")
        );
        let summary = runtime.summary();
        assert_eq!(summary.personal_projection.search.listings.rows.len(), 1);
        assert_eq!(
            summary.personal_projection.search.listings.rows[0].title,
            "Buyer Visible Eggs"
        );
        assert_eq!(
            summary.personal_projection.search.listings.rows[0].fulfillment_methods,
            BTreeSet::from([FarmOrderMethod::Pickup])
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_search_repeated_query_refreshes_shared_local_events() {
        let (runtime, paths) =
            bootstrapped_runtime("buyer_search_same_query_shared_local_events_refresh");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );

        append_cli_signed_buyer_listing_record_with(
            &paths,
            "first-buyer-visible-listing",
            "DDDDDDDDDDDDDDDDDDDDDD",
            "Buyer Visible Eggs",
            1100,
        );

        assert!(
            runtime
                .set_personal_search_query("eggs")
                .expect("buyer search query should refresh")
        );
        let first_summary = runtime.summary();
        assert_eq!(
            first_summary.personal_projection.search.listings.rows.len(),
            1
        );

        append_cli_signed_buyer_listing_record_with(
            &paths,
            "second-buyer-visible-listing",
            "EEEEEEEEEEEEEEEEEEEEEE",
            "Buyer Visible Eggs Two",
            1200,
        );

        assert!(
            runtime
                .set_personal_search_query("eggs")
                .expect("same buyer search query should refresh")
        );
        let refreshed_summary = runtime.summary();
        let titles = refreshed_summary
            .personal_projection
            .search
            .listings
            .rows
            .iter()
            .map(|row| row.title.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            titles,
            BTreeSet::from(["Buyer Visible Eggs", "Buyer Visible Eggs Two"])
        );

        assert!(
            !runtime
                .set_personal_search_query("eggs")
                .expect("idempotent same buyer search query should refresh")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_shared_local_events_refresh_reloads_buyer_browse_idempotently() {
        let (runtime, paths) = bootstrapped_runtime("buyer_browse_shared_local_events_refresh");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        append_cli_signed_buyer_listing_record(&paths);

        let report = runtime
            .refresh_shared_local_events()
            .expect("shared local events should refresh");
        let summary = runtime.summary();
        assert_eq!(report.scanned_records, 1);
        assert_eq!(report.imported_records, 1);
        assert_eq!(report.skipped_records, 0);
        assert_eq!(summary.personal_projection.browse.listings.rows.len(), 1);
        assert_eq!(
            summary.personal_projection.browse.listings.rows[0].title,
            "Buyer Visible Eggs"
        );

        let second_report = runtime
            .refresh_shared_local_events()
            .expect("second shared local events refresh should succeed");
        assert_eq!(second_report.scanned_records, 0);
        assert_eq!(second_report.imported_records, 0);
        assert_eq!(second_report.skipped_records, 0);
        assert_eq!(
            runtime
                .summary()
                .personal_projection
                .browse
                .listings
                .rows
                .len(),
            1
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_browse_selection_refreshes_shared_local_events() {
        let (runtime, paths) = bootstrapped_runtime("buyer_browse_selection_shared_events_refresh");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        assert_eq!(
            runtime
                .summary()
                .personal_projection
                .browse
                .listings
                .rows
                .len(),
            0
        );

        append_cli_signed_buyer_listing_record_with(
            &paths,
            "browse-selection-first-listing",
            "DDDDDDDDDDDDDDDDDDDDDD",
            "Buyer Visible Eggs",
            1100,
        );

        assert!(
            runtime
                .select_personal_section(PersonalSection::Browse)
                .expect("buyer Browse selection should refresh")
        );
        let first_summary = runtime.summary();
        assert_eq!(
            first_summary.personal_projection.browse.listings.rows.len(),
            1
        );

        append_cli_signed_buyer_listing_record_with(
            &paths,
            "browse-selection-second-listing",
            "EEEEEEEEEEEEEEEEEEEEEE",
            "Buyer Visible Eggs Two",
            1200,
        );

        assert!(
            runtime
                .select_personal_section(PersonalSection::Browse)
                .expect("same buyer Browse selection should refresh")
        );
        let refreshed_summary = runtime.summary();
        let titles = refreshed_summary
            .personal_projection
            .browse
            .listings
            .rows
            .iter()
            .map(|row| row.title.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            titles,
            BTreeSet::from(["Buyer Visible Eggs", "Buyer Visible Eggs Two"])
        );

        assert!(
            !runtime
                .select_personal_section(PersonalSection::Browse)
                .expect("idempotent buyer Browse selection should refresh")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_browse_selection_surfaces_shared_local_events_import_errors() {
        let (runtime, paths) = bootstrapped_runtime("buyer_browse_selection_import_error");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events parent directory");
        }
        if database_path.is_file() {
            fs::remove_file(&database_path).expect("shared local events file should be removable");
        } else if database_path.is_dir() {
            fs::remove_dir_all(&database_path)
                .expect("shared local events directory should be removable");
        }
        fs::create_dir(&database_path).expect("directory should block sqlite open");

        let error = runtime
            .select_personal_section(PersonalSection::Browse)
            .expect_err("buyer Browse selection should surface import errors");
        match error {
            AppSqliteError::LocalEventsSql { operation, .. } => {
                assert_eq!(operation, "open shared local events database");
            }
            unexpected => panic!("unexpected Browse selection error: {unexpected:?}"),
        }

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_detail_open_imports_shared_local_events_before_lookup() {
        assert_detail_open_imports_shared_local_events_before_lookup(
            "buyer_browse_detail_shared_local_events_refresh",
            PersonalSection::Browse,
        );
        assert_detail_open_imports_shared_local_events_before_lookup(
            "buyer_search_detail_shared_local_events_refresh",
            PersonalSection::Search,
        );
    }

    #[test]
    fn runtime_app_farm_and_listing_writes_append_shared_local_work_records() {
        let (runtime, paths) = bootstrapped_runtime("app_local_work_records");
        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();

        runtime
            .save_farm_setup_draft(FarmSetupDraft::new(
                "Green Farm",
                "farmstand",
                [FarmOrderMethod::Pickup],
            ))
            .expect("farm setup draft should save");
        runtime
            .finish_farm_setup()
            .expect("farm setup should finish");
        assert!(
            runtime
                .open_new_product_editor()
                .expect("product editor should open")
        );
        assert!(
            runtime
                .save_product_editor_draft(ProductEditorDraft {
                    title: "Eggs".to_owned(),
                    subtitle: "Fresh eggs".to_owned(),
                    category: "eggs".to_owned(),
                    unit_label: "dozen".to_owned(),
                    price_minor_units: Some(750),
                    price_currency: "USD".to_owned(),
                    stock_quantity: Some(12),
                    availability_window_id: None,
                    status: ProductStatus::Draft,
                })
                .expect("product draft should save")
        );

        let records = shared_local_event_records(&paths);
        let app_records = records
            .iter()
            .filter(|record| record.source_runtime == SourceRuntime::App)
            .collect::<Vec<_>>();
        assert_eq!(app_records.len(), 2);

        let farm_record = app_records
            .iter()
            .find(|record| {
                record
                    .local_work_json
                    .as_ref()
                    .and_then(|payload| payload["record_kind"].as_str())
                    == Some("farm_config_v1")
            })
            .expect("farm local work record");
        assert_eq!(farm_record.family, LocalRecordFamily::LocalWork);
        assert_eq!(farm_record.status, LocalRecordStatus::LocalSaved);
        assert_eq!(farm_record.outbox_status, PublishOutboxStatus::None);
        assert_eq!(
            farm_record.owner_account_id.as_deref(),
            Some(account_id.as_str())
        );
        let owner_pubkey = farm_record
            .owner_pubkey
            .as_deref()
            .expect("farm owner pubkey");
        assert!(is_hex_64(owner_pubkey));
        assert!(
            farm_record
                .farm_id
                .as_ref()
                .is_some_and(|value| value.len() == 22)
        );
        assert_eq!(farm_record.listing_addr, None);
        let farm_payload = farm_record
            .local_work_json
            .as_ref()
            .expect("farm local work payload");
        assert_eq!(farm_payload["scope"], "app");
        assert_eq!(farm_payload["exportability"]["state"], "exportable");
        assert_eq!(farm_payload["document"]["farm"]["name"], "Green Farm");
        assert_eq!(
            farm_payload["document"]["listing_defaults"]["delivery_method"],
            "pickup"
        );
        assert!(farm_payload.get("draft").is_none());
        assert!(farm_payload.get("editor").is_none());

        let listing_record = app_records
            .iter()
            .find(|record| {
                record
                    .local_work_json
                    .as_ref()
                    .and_then(|payload| payload["record_kind"].as_str())
                    == Some("listing_draft_v1")
            })
            .expect("listing local work record");
        assert_eq!(listing_record.family, LocalRecordFamily::LocalWork);
        assert_eq!(listing_record.status, LocalRecordStatus::LocalSaved);
        assert_eq!(listing_record.outbox_status, PublishOutboxStatus::None);
        assert_eq!(
            listing_record.owner_account_id.as_deref(),
            Some(account_id.as_str())
        );
        assert_eq!(listing_record.owner_pubkey.as_deref(), Some(owner_pubkey));
        assert_eq!(listing_record.farm_id, farm_record.farm_id);
        let expected_listing_addr_prefix = format!("30402:{owner_pubkey}:");
        assert!(
            listing_record
                .listing_addr
                .as_deref()
                .expect("listing address")
                .starts_with(expected_listing_addr_prefix.as_str())
        );
        let listing_payload = listing_record
            .local_work_json
            .as_ref()
            .expect("listing local work payload");
        assert_eq!(listing_payload["exportability"]["state"], "exportable");
        assert_eq!(listing_payload["publishability"]["state"], "blocked");
        assert_eq!(listing_payload["document"]["kind"], "listing_draft_v1");
        assert_eq!(
            listing_payload["document"]["seller_actor"]["pubkey"],
            owner_pubkey
        );
        assert_eq!(listing_payload["document"]["product"]["title"], "Eggs");
        assert_eq!(listing_payload["document"]["product"]["category"], "eggs");
        assert!(
            listing_payload["document"]["primary_bin"]["bin_id"]
                .as_str()
                .is_some_and(|value| value.ends_with(":primary"))
        );
        assert_eq!(
            listing_payload["document"]["primary_bin"]["price_amount"],
            "7.50"
        );
        assert_eq!(listing_payload["document"]["inventory"]["available"], "12");
        assert_eq!(listing_payload["document"]["delivery"]["method"], "pickup");
        assert_eq!(
            listing_payload["document"]["location"]["primary"],
            "farmstand"
        );
        assert!(listing_payload.get("draft").is_none());
        assert!(listing_payload.get("editor").is_none());

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_published_receipts_record_payload_account_owner() {
        let (runtime, paths) = bootstrapped_runtime("published_receipt_payload_owner");
        assert!(
            runtime
                .generate_local_account(Some("First".to_owned()))
                .expect("first account should generate")
        );
        let payload_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();
        assert!(
            runtime
                .generate_local_account(Some("Second".to_owned()))
                .expect("second account should generate")
        );
        let selected_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();
        assert_ne!(payload_account_id, selected_account_id);

        let receipt = published_operation_receipt_fixture(
            payload_account_id.clone(),
            None,
            "event-app-owner",
        );
        runtime
            .lock_state()
            .record_published_sync_receipts(&[receipt])
            .expect("published receipt should record");

        let records = shared_local_event_records(&paths);
        let signed_record = records
            .iter()
            .find(|record| record.record_id == "app:signed_event:event-app-owner")
            .expect("signed event record");
        assert_eq!(
            signed_record.owner_account_id.as_deref(),
            Some(payload_account_id.as_str())
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_published_receipts_reject_conflicting_source_owner() {
        let (runtime, paths) = bootstrapped_runtime("published_receipt_owner_conflict");
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        store
            .append_record(&local_work_record(
                "app:local_work:conflict-source",
                "other-account",
                "farm-key",
                None,
                json!({"record_kind": "farm_config_v1"}),
            ))
            .expect("append conflicting source record");
        let receipt = published_operation_receipt_fixture(
            "payload-account".to_owned(),
            Some("app:local_work:conflict-source".to_owned()),
            "event-app-owner-conflict",
        );

        let error = runtime
            .lock_state()
            .record_published_sync_receipts(&[receipt])
            .expect_err("conflicting source owner should fail closed");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "published operation source account does not match local event owner"
            }
        ));
        assert!(
            shared_local_event_records(&paths)
                .iter()
                .all(|record| record.record_id != "app:signed_event:event-app-owner-conflict")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_app_local_work_without_resolved_pubkey_is_non_exportable() {
        let (runtime, paths) = bootstrapped_runtime("app_local_work_unresolved_pubkey");
        let farm_id = FarmId::new();
        let account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_unresolved".to_owned(),
                npub: "npub1unresolved".to_owned(),
                label: Some("Unresolved".to_owned()),
                custody: AccountCustody::RemoteSigner,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::active(farm_id),
        );
        let saved_farm = FarmSummary {
            farm_id,
            display_name: "Green Farm".to_owned(),
            readiness: FarmReadiness::Ready,
        };
        let farm_projection = FarmSetupProjection::from_saved_farm(saved_farm.clone());

        {
            let mut state = runtime.lock_state_mut();
            let identity =
                AppIdentityProjection::ready(vec![account.account.clone()], account.clone());
            let _ = state
                .state_store
                .apply_in_memory(AppStateCommand::replace_identity_projection(identity));
            state
                .append_app_farm_local_work_record(&account, &farm_projection, &saved_farm)
                .expect("unresolved farm local work should append");
            state
                .append_app_listing_local_work_record(
                    ProductId::new(),
                    &ProductEditorDraft {
                        title: "Eggs".to_owned(),
                        subtitle: "Fresh eggs".to_owned(),
                        category: "eggs".to_owned(),
                        unit_label: "dozen".to_owned(),
                        price_minor_units: Some(750),
                        price_currency: "USD".to_owned(),
                        stock_quantity: Some(12),
                        availability_window_id: None,
                        status: ProductStatus::Draft,
                    },
                )
                .expect("unresolved listing local work should append");
        }

        let records = shared_local_event_records(&paths);
        let app_records = records
            .iter()
            .filter(|record| record.source_runtime == SourceRuntime::App)
            .collect::<Vec<_>>();
        assert_eq!(app_records.len(), 2);
        assert!(
            app_records
                .iter()
                .all(|record| record.owner_account_id.as_deref() == Some("acct_unresolved"))
        );
        assert!(
            app_records
                .iter()
                .all(|record| record.owner_pubkey.is_none())
        );
        assert!(
            app_records
                .iter()
                .all(|record| record
                    .local_work_json
                    .as_ref()
                    .is_some_and(|payload| payload["exportability"]["state"]
                        == "identity_unresolved"
                        && payload["exportability"]["reason"] == "canonical_hex_pubkey_required"))
        );
        let listing_record = app_records
            .iter()
            .find(|record| {
                record
                    .local_work_json
                    .as_ref()
                    .and_then(|payload| payload["record_kind"].as_str())
                    == Some("listing_draft_v1")
            })
            .expect("listing local work record");
        assert_eq!(listing_record.listing_addr, None);
        assert!(
            listing_record
                .local_work_json
                .as_ref()
                .expect("listing payload")["document"]["seller_actor"]["pubkey"]
                .is_null()
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_manual_refresh_marks_failed_checkpoint_when_transport_is_unavailable() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Farm(farm_id),
                farm_sync_payload(
                    farm_id,
                    "North field farm",
                    Some(FarmReadiness::Ready),
                    "manual_refresh_unavailable_transport",
                ),
            )])
            .expect("pending farm sync should enqueue");

        assert!(
            runtime
                .sync_on_manual_refresh()
                .expect("manual refresh should complete")
        );

        let summary = runtime.summary();
        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");

        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Failed
        );
        assert_eq!(
            summary.sync_status.projection.checkpoint.state,
            SyncCheckpointState::Failed
        );
        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert!(
            summary
                .sync_status
                .projection
                .checkpoint
                .last_error_message
                .as_deref()
                .is_some_and(|message| { message.contains(SYNC_TRANSPORT_UNAVAILABLE_MESSAGE) })
        );
        assert_eq!(pending_operations.len(), 1);
        assert_eq!(pending_operations[0].operation.attempt_count, 1);
    }

    #[test]
    fn runtime_sync_attempts_stop_when_blocking_conflicts_are_present() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        runtime
            .lock_state_mut()
            .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Farm(farm_id),
                farm_sync_payload(
                    farm_id,
                    "North field farm",
                    Some(FarmReadiness::Ready),
                    "blocking_conflict_stops_sync",
                ),
            )])
            .expect("pending farm sync should enqueue");

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .record_sync_conflict(
                account_id.as_str(),
                &SyncConflict {
                    aggregate: SyncAggregateRef::Farm(farm_id),
                    kind: SyncConflictKind::RevisionMismatch,
                    severity: SyncConflictSeverity::Blocking,
                    resolution: SyncConflictResolutionStatus::Unresolved,
                    local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                    remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                    detected_at: "2026-04-20T20:00:00Z".to_owned(),
                    resolved_at: None,
                },
            )
            .expect("blocking conflict should save");
        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    None,
                    "2026-04-20T20:00:05Z",
                    Some("cursor-blocked".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            !runtime
                .sync_on_app_launch()
                .expect("blocked launch sync should skip")
        );

        let summary = runtime.summary();

        assert_eq!(recorded.lock().expect("recorded transport").call_count(), 0);
        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert_eq!(
            summary.sync_status.projection.run_status,
            AppSyncRunStatus::Conflicted
        );
        assert_eq!(
            summary
                .sync_status
                .projection
                .conflict_status
                .blocking_count,
            1
        );
    }

    #[test]
    fn runtime_resolving_a_blocking_conflict_refreshes_sync_summary() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        let conflict_id = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .record_sync_conflict(
                account_id.as_str(),
                &SyncConflict {
                    aggregate: SyncAggregateRef::Farm(farm_id),
                    kind: SyncConflictKind::RevisionMismatch,
                    severity: SyncConflictSeverity::Blocking,
                    resolution: SyncConflictResolutionStatus::Unresolved,
                    local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                    remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                    detected_at: "2026-04-20T20:05:00Z".to_owned(),
                    resolved_at: None,
                },
            )
            .expect("blocking conflict should save");
        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        assert!(
            runtime
                .resolve_sync_conflict(
                    conflict_id.as_str(),
                    SyncConflictResolutionStatus::AcceptedLocal,
                )
                .expect("conflict resolution should succeed")
        );

        let summary = runtime.summary();

        assert_eq!(
            summary
                .sync_status
                .projection
                .conflict_status
                .unresolved_count,
            0
        );
        assert_eq!(
            summary
                .sync_status
                .projection
                .conflict_status
                .blocking_count,
            0
        );
        assert_eq!(summary.sync_status.conflicts.len(), 1);
        assert_eq!(
            summary.sync_status.conflicts[0].conflict.resolution,
            SyncConflictResolutionStatus::AcceptedLocal
        );
        assert!(
            summary.sync_status.conflicts[0]
                .conflict
                .resolved_at
                .as_deref()
                .is_some()
        );
    }

    #[test]
    fn runtime_review_required_conflicts_do_not_block_manual_refresh() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .record_sync_conflict(
                account_id.as_str(),
                &SyncConflict {
                    aggregate: SyncAggregateRef::Farm(farm_id),
                    kind: SyncConflictKind::RemoteValidationReject,
                    severity: SyncConflictSeverity::ReviewRequired,
                    resolution: SyncConflictResolutionStatus::Unresolved,
                    local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                    remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                    detected_at: "2026-04-20T20:10:00Z".to_owned(),
                    resolved_at: None,
                },
            )
            .expect("review-required conflict should save");
        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        let recorded = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::succeed(AppSyncResult {
                run_status: AppSyncRunStatus::Succeeded,
                checkpoint: SyncCheckpointStatus::current(
                    Some("2026-04-20T20:10:05Z".to_owned()),
                    "2026-04-20T20:10:08Z",
                    Some("cursor-review-required".to_owned()),
                ),
                pushed_operation_count: 1,
                pulled_record_count: 0,
                conflicts: Vec::new(),
                published_receipts: Vec::new(),
            }),
        );

        assert!(
            runtime
                .sync_on_manual_refresh()
                .expect("manual refresh should succeed")
        );

        let recorded = recorded.lock().expect("recorded transport");
        let request = recorded
            .last_request()
            .cloned()
            .expect("manual refresh request should record");

        assert_eq!(recorded.call_count(), 1);
        assert_eq!(request.trigger, SyncTrigger::ManualRefresh);
    }

    #[test]
    fn runtime_summary_surfaces_runtime_metadata_from_bootstrap() {
        let (runtime, paths) = bootstrapped_runtime("runtime_metadata");
        let summary = runtime.summary();

        assert_eq!(
            summary.runtime_metadata.snapshot.host.app_name,
            radroots_studio_app_core::APP_NAME
        );
        assert_eq!(
            summary.runtime_metadata.data_root.as_ref(),
            Some(&paths.app.data)
        );
        assert_eq!(
            summary.runtime_metadata.logs_root.as_ref(),
            Some(&paths.app.logs)
        );
        assert_eq!(
            summary.runtime_metadata.database_path.as_ref(),
            Some(&paths.app.data.join(APP_DATABASE_FILE_NAME))
        );
        assert_eq!(
            summary.runtime_metadata.database_schema_version,
            Some(latest_schema_version())
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn clearing_startup_pending_remote_signer_session_is_idempotent_without_record() {
        let paths = temp_remote_signer_paths("clear_pending_none");
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: Some(paths.clone()),
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        assert!(
            runtime
                .clear_startup_pending_remote_signer_session()
                .expect("clear pending should succeed"),
            "missing pending startup session should count as a successful cleanup"
        );

        cleanup_remote_signer_paths(&paths);
    }

    #[test]
    fn clean_startup_cleanup_allows_generate_key_phase_transition() {
        let paths = temp_remote_signer_paths("generate_key_after_clean_cleanup");
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: Some(paths.clone()),
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        assert!(
            runtime
                .clear_startup_pending_remote_signer_session()
                .expect("clear pending should succeed")
        );
        assert!(runtime.begin_generate_key_startup());
        assert_eq!(
            runtime.summary().logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::GenerateKeyStarting
        );

        cleanup_remote_signer_paths(&paths);
    }

    #[test]
    fn pending_startup_signer_session_recovers_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_pending_recovery");
        let pending_session = fixture_pending_session();

        assert!(
            runtime
                .store_startup_pending_remote_signer_session(&pending_session)
                .expect("store pending should succeed")
        );

        let restarted = restart_runtime(paths.clone());
        let restored = restarted
            .load_startup_pending_remote_signer_session()
            .expect("load pending should succeed")
            .expect("pending session should recover after restart");

        assert_eq!(
            restarted.summary().logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::SignerEntry
        );
        assert_eq!(
            restored.record.client_account_id(),
            pending_session.record.client_account_id()
        );
        assert_eq!(
            restored.record.signer_identity.id,
            pending_session.record.signer_identity.id
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn clearing_pending_startup_signer_session_prevents_restart_recovery() {
        let (runtime, paths) = bootstrapped_runtime("restart_after_explicit_cancel");
        let pending_session = fixture_pending_session();

        assert!(
            runtime
                .store_startup_pending_remote_signer_session(&pending_session)
                .expect("store pending should succeed")
        );
        assert!(
            runtime
                .clear_startup_pending_remote_signer_session()
                .expect("clear pending should succeed")
        );

        let restarted = restart_runtime(paths.clone());

        assert_eq!(
            restarted.summary().logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::ContinuePrompt
        );
        assert!(
            restarted
                .load_startup_pending_remote_signer_session()
                .expect("load pending should succeed")
                .is_none(),
            "explicit cancel should leave no pending startup session to recover"
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn startup_signer_entry_source_input_recovers_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_startup_signer_entry");

        assert!(runtime.show_startup_identity_choice());
        assert!(runtime.show_startup_signer_entry());
        assert!(runtime.set_startup_signer_source_input(
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example"
        ));

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::SignerEntry
        );
        assert_eq!(
            summary.logged_out_startup.signer_entry.source_input,
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example"
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn generate_key_startup_phase_fails_closed_to_identity_choice_after_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_generate_key_sanitize");

        assert!(runtime.show_startup_identity_choice());
        assert!(runtime.begin_generate_key_startup());

        let restarted = restart_runtime(paths.clone());

        assert_eq!(
            restarted.summary().logged_out_startup.phase,
            radroots_studio_app_view::LoggedOutStartupPhase::IdentityChoice
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn buyer_search_query_and_detail_recover_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_buyer_search_detail");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");

        assert!(
            runtime
                .set_personal_search_query("salad")
                .expect("buyer search query should update")
        );
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Search, product_id)
                .expect("buyer search detail should open")
        );

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Search)
        );
        assert_eq!(
            summary.personal_projection.search.query.search_query,
            "salad"
        );
        assert_eq!(
            summary
                .personal_projection
                .search
                .detail
                .as_ref()
                .map(|detail| detail.listing.product_id),
            Some(product_id)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn products_query_and_editor_recover_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_products_editor");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Pea shoots",
            "Tray",
            "draft",
            Some(4),
            "2026-04-20T09:30:00Z",
        );

        assert!(
            runtime
                .set_products_search_query("pea")
                .expect("products query should update")
        );
        assert!(
            runtime
                .select_products_filter(ProductsFilter::Drafts)
                .expect("products filter should update")
        );
        assert!(
            runtime
                .select_products_sort(ProductsSort::Name)
                .expect("products sort should update")
        );
        assert!(
            runtime
                .open_existing_product_editor(product_id)
                .expect("product editor should open")
        );

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Products)
        );
        assert_eq!(summary.products_projection.query.search_query, "pea");
        assert_eq!(
            summary.products_projection.query.filter,
            ProductsFilter::Drafts
        );
        assert_eq!(summary.products_projection.query.sort, ProductsSort::Name);
        match &summary.products_projection.editor {
            radroots_studio_app_state::ProductEditorState::Open(session) => {
                assert_eq!(session.selected_product_id, Some(product_id));
            }
            radroots_studio_app_state::ProductEditorState::Closed => {
                panic!("product editor should recover after restart")
            }
        }

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn orders_query_and_detail_recover_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_orders_detail");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, order_id) = seed_order_workspace(&runtime, farm_id);

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update orders
                 set status = 'packed', updated_at = '2026-04-20T09:45:00Z'
                 where id = '{order_id}' and farm_id = '{farm_id}'"
            ))
            .expect("order should update to packed");

        assert!(
            runtime
                .select_orders_filter(OrdersFilter::Packed)
                .expect("orders filter should update")
        );
        assert!(
            runtime
                .open_order_detail(order_id)
                .expect("order detail should open")
        );

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(summary.orders_projection.query.filter, OrdersFilter::Packed);
        assert_eq!(
            summary
                .orders_projection
                .detail
                .as_ref()
                .map(|detail| detail.order_id),
            Some(order_id)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn stale_orders_selection_clears_invalid_window_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_stale_orders");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, _) = seed_order_workspace(&runtime, farm_id);

        assert!(
            runtime
                .open_orders_fulfillment_window(fulfillment_window_id)
                .expect("orders window should open")
        );
        let mut persisted_state = runtime.lock_state().state_store.persisted_state().clone();
        persisted_state.seller.orders_query.fulfillment_window_id =
            Some(FulfillmentWindowId::new());
        let mut repository =
            FileBackedAppStateRepository::new(paths.app.data.join(APP_STATE_FILE_NAME));
        repository
            .save_persisted_state(&persisted_state)
            .expect("stale orders selection should persist");

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(summary.orders_projection.query.fulfillment_window_id, None);
        assert!(
            summary
                .orders_projection
                .list
                .rows
                .iter()
                .any(|row| { row.fulfillment_window_id == Some(fulfillment_window_id) })
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn stale_pack_day_selection_clears_invalid_window_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_stale_pack_day");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, _) = seed_order_workspace(&runtime, farm_id);

        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        let mut persisted_state = runtime.lock_state().state_store.persisted_state().clone();
        let stale_fulfillment_window_id = FulfillmentWindowId::new();
        persisted_state.seller.pack_day_query.fulfillment_window_id =
            Some(stale_fulfillment_window_id);
        let mut repository =
            FileBackedAppStateRepository::new(paths.app.data.join(APP_STATE_FILE_NAME));
        repository
            .save_persisted_state(&persisted_state)
            .expect("stale pack day selection should persist");

        let restarted = restart_runtime(paths.clone());
        let summary = restarted.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::PackDay)
        );
        assert_eq!(
            summary.pack_day_projection.query.fulfillment_window_id,
            None
        );
        assert!(
            summary
                .pack_day_projection
                .projection
                .fulfillment_window
                .is_some()
        );
        assert_ne!(
            summary.pack_day_projection.query.fulfillment_window_id,
            Some(stale_fulfillment_window_id)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn replacing_today_agenda_is_shared_without_clobbering_home_shell() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();
        let today_agenda = TodayAgendaProjection {
            farm: Some(FarmSummary {
                farm_id: radroots_studio_app_view::FarmId::new(),
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            }),
            summary: Some(TodaySummary {
                farm_id: radroots_studio_app_view::FarmId::new(),
                orders_needing_action: 2,
                low_stock_products: 1,
                draft_products: 3,
                reminders_due_soon: 0,
                recovery_actions_open: 0,
            }),
            setup_checklist: vec![TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: false,
            }],
            ..TodayAgendaProjection::default()
        };

        assert!(runtime.select_settings_section(SettingsSection::About));
        assert!(cloned_runtime.replace_today_agenda(today_agenda.clone()));

        let summary = runtime.summary();

        assert_eq!(summary.today_projection.farm, today_agenda.farm);
        assert_eq!(summary.today_projection.summary, today_agenda.summary);
        assert_eq!(summary.today_projection.setup_checklist.len(), 6);
        assert!(summary.today_projection.needs_setup());
        assert_eq!(summary.home_route, HomeRoute::SetupRequired);
        assert_eq!(
            summary.shell_projection.active_surface,
            radroots_studio_app_view::ActiveSurface::Personal
        );
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::About
        );
        assert!(summary.today_projection.needs_setup());
    }

    #[test]
    fn degraded_runtime_surfaces_startup_issue_with_default_today_projection() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState::degraded(
            super::DesktopAppRuntimeBootstrapError::State(AppStateStoreError::Repository(
                AppStateRepositoryError::load("state unavailable"),
            )),
        ));

        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.active_surface,
            radroots_studio_app_view::ActiveSurface::Personal
        );
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            summary.shell_projection.settings.selected_section,
            SettingsSection::Account
        );
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert_eq!(
            summary.logged_out_startup,
            LoggedOutStartupProjection::default()
        );
        assert!(summary.settings_account_projection.roster.is_empty());
        assert_eq!(summary.home_route, HomeRoute::SetupRequired);
        assert_eq!(summary.today_projection, TodayAgendaProjection::default());
        assert_eq!(
            summary.startup_issue.as_deref(),
            Some("app state repository load failed: state unavailable")
        );
    }

    #[test]
    fn runtime_records_activity_context_for_user_visible_actions() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        assert!(runtime.record_home_opened());
        assert!(runtime.sync_settings_section(SettingsSection::About));
        assert!(runtime.record_settings_opened(SettingsSection::About));
        assert!(runtime.select_settings_section(SettingsSection::Settings));
        assert!(runtime.set_settings_preference(SettingsPreference::LaunchAtLogin, true));

        let context = runtime
            .activity_context(Some(8))
            .expect("activity context should load");

        assert_eq!(context.recent_events.len(), 4);
        assert_eq!(
            context.recent_events[0].kind,
            AppActivityKind::SettingsPreferenceUpdated {
                preference: SettingsPreference::LaunchAtLogin,
                enabled: true,
            }
        );
        assert_eq!(
            context.recent_events[1].kind,
            AppActivityKind::SettingsSectionSelected {
                section: SettingsSection::Settings,
            }
        );
        assert_eq!(
            context.recent_events[2].kind,
            AppActivityKind::SettingsOpened {
                section: SettingsSection::About,
            }
        );
        assert_eq!(context.recent_events[3].kind, AppActivityKind::HomeOpened);
    }

    #[test]
    fn activity_context_distinguishes_empty_history_from_runtime_unavailable() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        let empty_context = runtime
            .activity_context(Some(8))
            .expect("empty activity history should still load");
        assert!(empty_context.recent_events.is_empty());

        let degraded = DesktopAppRuntime::from_state(DesktopAppRuntimeState::degraded(
            super::DesktopAppRuntimeBootstrapError::State(AppStateStoreError::Repository(
                AppStateRepositoryError::load("state unavailable"),
            )),
        ));

        assert!(matches!(
            degraded.activity_context(Some(8)),
            Err(DesktopAppRuntimeActivityContextError::RuntimeUnavailable)
        ));
    }

    #[test]
    fn activity_context_surfaces_store_load_failure() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch("DROP TABLE activity_events")
            .expect("activity table should drop");

        assert!(matches!(
            runtime.activity_context(Some(8)),
            Err(DesktopAppRuntimeActivityContextError::Sqlite(_))
        ));
    }

    #[test]
    fn selecting_farmer_section_requires_farmer_identity_gate() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        assert!(!runtime.select_farmer_section(FarmerSection::Products));
        assert!(!runtime.select_farmer_section(FarmerSection::Orders));
        assert!(!runtime.select_farmer_section(FarmerSection::PackDay));
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Home
        );
    }

    #[test]
    fn pack_day_stays_blocked_without_a_window_context() {
        let runtime = memory_runtime();
        let _ = provision_ready_farmer_account(&runtime);

        assert!(!runtime.select_farmer_section(FarmerSection::PackDay));
        assert!(
            !runtime
                .open_pack_day(None)
                .expect("pack day route should stay blocked")
        );
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );
        assert!(
            runtime
                .summary()
                .pack_day_projection
                .projection
                .fulfillment_window
                .is_none()
        );
    }

    #[test]
    fn runtime_routes_between_farmer_home_and_products_through_explicit_methods() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");
        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );

        assert!(runtime.select_farmer_section(FarmerSection::Products));
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Products)
        );

        assert!(runtime.select_home());
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );
    }

    #[test]
    fn guest_marketplace_entry_selects_personal_browse_without_an_account() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .select_personal_section(PersonalSection::Browse)
                .expect("guest Browse selection should succeed")
        );

        let summary = runtime.summary();
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Browse)
        );
        assert_eq!(
            summary.personal_projection.entry.state,
            radroots_studio_app_view::PersonalEntryState::Guest
        );
    }

    #[test]
    fn runtime_personal_search_queries_refresh_repository_backed_marketplace_projection() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let sql = format!(
            "insert into pickup_locations (
                id,
                farm_id,
                label,
                address_line,
                directions,
                is_default,
                created_at,
                updated_at
             ) values (
                '{pickup_location_id}',
                '{farm_id}',
                'North barn',
                '14 County Road',
                null,
                1,
                '2026-04-20T08:00:00Z',
                '2026-04-20T08:00:00Z'
             );
             insert into fulfillment_windows (
                id,
                farm_id,
                starts_at,
                ends_at,
                capacity_limit,
                created_at,
                updated_at,
                pickup_location_id,
                label,
                order_cutoff_at
             ) values (
                '{fulfillment_window_id}',
                '{farm_id}',
                '2099-04-18T16:00:00Z',
                '2099-04-18T18:00:00Z',
                null,
                '2099-04-18T16:00:00Z',
                '2099-04-18T16:00:00Z',
                '{pickup_location_id}',
                'Friday pickup',
                '2099-04-17T18:00:00Z'
             );
             update account_farm_setups
             set
                pickup_enabled = 1,
                delivery_enabled = 0,
                shipping_enabled = 0,
                saved_farm_id = '{farm_id}',
                saved_farm_display_name = 'North field farm',
                saved_farm_readiness = 'ready',
                updated_at = '2026-04-20T08:00:00Z'
             where account_id = '{account_id}';"
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("buyer search workspace should seed");
        let salad_mix_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        let pea_shoots_id = seed_product(
            &runtime,
            farm_id,
            "Pea shoots",
            "Tray-grown",
            "published",
            Some(4),
            "2026-04-20T09:30:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id in ('{salad_mix_id}', '{pea_shoots_id}')"
            ))
            .expect("buyer-visible products should attach a fulfillment window");

        let _ = runtime
            .select_local_account(account_id.as_str())
            .expect("account should refresh after buyer workspace seeding");
        let summary = runtime.summary();
        assert_eq!(summary.personal_projection.search.listings.rows.len(), 2);
        assert!(
            summary
                .personal_projection
                .search
                .query
                .fulfillment_methods
                .is_empty()
        );

        assert!(
            runtime
                .set_personal_search_query("pea")
                .expect("buyer search query should apply")
        );
        let searched = runtime.summary();
        assert_eq!(searched.personal_projection.search.listings.rows.len(), 1);
        assert_eq!(
            searched.personal_projection.search.listings.rows[0].title,
            "Pea shoots"
        );

        assert!(
            runtime
                .set_personal_search_fulfillment_method(FarmOrderMethod::Pickup, true)
                .expect("buyer fulfillment filter should apply")
        );
        let filtered = runtime.summary();
        assert_eq!(
            filtered
                .personal_projection
                .search
                .query
                .fulfillment_methods,
            BTreeSet::from([FarmOrderMethod::Pickup])
        );
        assert_eq!(filtered.personal_projection.search.listings.rows.len(), 1);
        assert_eq!(
            filtered.personal_projection.search.listings.rows[0]
                .next_fulfillment_window_label
                .as_deref(),
            Some("Friday pickup")
        );
    }

    #[test]
    fn runtime_personal_product_detail_adds_to_cart_and_routes_into_cart() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");

        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should open")
        );
        assert!(runtime.increase_personal_product_quantity(PersonalSection::Browse));
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Cart)
        );
        assert_eq!(summary.personal_projection.cart.cart.lines.len(), 1);
        assert_eq!(
            summary.personal_projection.cart.cart.lines[0].title,
            "Salad mix"
        );
        assert_eq!(summary.personal_projection.cart.cart.lines[0].quantity, 2);
        assert_eq!(
            summary.personal_projection.cart.cart.subtotal_minor_units,
            Some(1200)
        );
        assert_eq!(
            summary
                .personal_projection
                .cart
                .cart
                .farm_display_name
                .as_deref(),
            Some("North field farm")
        );
        assert!(
            summary
                .personal_projection
                .cart
                .cart
                .replace_confirmation
                .is_none()
        );
        assert_eq!(
            summary
                .personal_projection
                .browse
                .detail
                .as_ref()
                .expect("buyer detail should persist on browse")
                .selected_quantity,
            2
        );
    }

    #[test]
    fn runtime_cross_farm_buyer_add_requires_replace_confirmation() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let first_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let first_product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{first_window_id}'
                 where id = '{first_product_id}'"
            ))
            .expect("first product should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, first_product_id)
                .expect("first buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("first buyer product should add to cart")
        );

        let other_farm_id = FarmId::new();
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(&FarmSummary {
                farm_id: other_farm_id,
                display_name: "Willow Farm".to_owned(),
                readiness: FarmReadiness::Ready,
            })
            .expect("other farm summary should save");
        let second_window_id = seed_buyer_marketplace_support(
            &runtime,
            "acct_other_farmer",
            other_farm_id,
            "Willow Farm",
            "Saturday pickup",
        );
        let second_product_id = seed_product(
            &runtime,
            other_farm_id,
            "Pea shoots",
            "Tray-grown",
            "published",
            Some(5),
            "2026-04-20T10:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{second_window_id}'
                 where id = '{second_product_id}'"
            ))
            .expect("second product should attach a fulfillment window");

        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, second_product_id)
                .expect("second buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("cross-farm add should require confirmation")
        );

        let confirmation_summary = runtime.summary();
        assert_eq!(
            confirmation_summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Browse)
        );
        assert_eq!(
            confirmation_summary
                .personal_projection
                .cart
                .cart
                .lines
                .len(),
            1
        );
        assert_eq!(
            confirmation_summary.personal_projection.cart.cart.lines[0].title,
            "Salad mix"
        );
        assert_eq!(
            confirmation_summary
                .personal_projection
                .cart
                .cart
                .replace_confirmation
                .as_ref()
                .expect("replace confirmation should exist")
                .incoming_farm_display_name,
            "Willow Farm"
        );

        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, true)
                .expect("confirmed cross-farm add should replace the cart")
        );
        let replaced_summary = runtime.summary();
        assert_eq!(
            replaced_summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Cart)
        );
        assert_eq!(
            replaced_summary.personal_projection.cart.cart.lines.len(),
            1
        );
        assert_eq!(
            replaced_summary.personal_projection.cart.cart.lines[0].title,
            "Pea shoots"
        );
        assert_eq!(
            replaced_summary
                .personal_projection
                .cart
                .cart
                .farm_display_name
                .as_deref(),
            Some("Willow Farm")
        );
        assert!(
            replaced_summary
                .personal_projection
                .cart
                .cart
                .replace_confirmation
                .is_none()
        );
    }

    #[test]
    fn runtime_removing_buyer_cart_line_clears_cart_and_order_review_readiness() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );

        assert!(
            runtime
                .remove_personal_cart_line(product_id)
                .expect("buyer cart line should remove")
        );

        let summary = runtime.summary();
        assert!(summary.personal_projection.cart.cart.lines.is_empty());
        assert!(summary.personal_projection.cart.cart.farm_id.is_none());
        assert!(
            !summary
                .personal_projection
                .cart
                .order_review
                .can_place_order
        );
        assert_eq!(
            summary
                .personal_projection
                .cart
                .order_review
                .summary
                .line_count,
            0
        );
    }

    #[test]
    fn runtime_places_buyer_order_and_routes_into_personal_orders() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: "555-0101".to_owned(),
                    order_note: "Leave by the cooler".to_owned(),
                })
                .expect("buyer order review draft should save")
        );
        let order_review = runtime.summary().personal_projection.cart.order_review;
        assert!(order_review.can_place_order);
        assert_eq!(order_review.place_order_disabled_reason, None);
        assert!(
            runtime
                .place_personal_order()
                .expect("buyer order should place")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Orders)
        );
        assert!(summary.personal_projection.cart.cart.lines.is_empty());
        assert!(
            !summary
                .personal_projection
                .cart
                .order_review
                .can_place_order
        );
        assert_eq!(
            summary
                .personal_projection
                .cart
                .order_review
                .place_order_disabled_reason,
            Some(BuyerOrderReviewDisabledReason::EmptyCart)
        );
        assert_eq!(summary.personal_projection.orders.list.rows.len(), 1);
        assert_eq!(
            summary.personal_projection.orders.list.rows[0].farm_display_name,
            "North field farm"
        );
        assert_eq!(
            summary.personal_projection.orders.list.rows[0]
                .status
                .storage_key(),
            "placed"
        );
        assert_eq!(
            summary
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("buyer order detail should be selected")
                .order_id,
            summary.personal_projection.orders.list.rows[0].order_id
        );
        assert_eq!(
            summary
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("buyer order detail")
                .order_note
                .as_deref(),
            Some("Leave by the cooler")
        );
    }

    #[test]
    fn runtime_guest_order_review_requires_account_before_order_write() {
        let runtime = memory_runtime();
        let farm_id = FarmId::new();
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(&FarmSummary {
                farm_id,
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Ready,
            })
            .expect("farm summary should save");
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            "acct_farmer",
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: "555-0101".to_owned(),
                    order_note: "Leave by the cooler".to_owned(),
                })
                .expect("buyer order review draft should save")
        );

        let ready_summary = runtime.summary();
        assert!(
            !ready_summary
                .personal_projection
                .cart
                .order_review
                .can_place_order
        );
        assert_eq!(
            ready_summary
                .personal_projection
                .cart
                .order_review
                .place_order_disabled_reason,
            Some(BuyerOrderReviewDisabledReason::AccountRequired)
        );
        assert_eq!(
            ready_summary
                .personal_projection
                .cart
                .order_review
                .summary
                .line_count,
            1
        );

        let error = runtime
            .place_personal_order()
            .expect_err("guest order review should require an account");
        assert!(matches!(error, AppSqliteError::InvalidProjection { .. }));

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Cart)
        );
        assert_eq!(summary.personal_projection.cart.cart.lines.len(), 1);
        assert_eq!(summary.personal_projection.orders.list.rows.len(), 0);
        let order_count: i64 = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .query_row("select count(*) from orders", [], |row| row.get(0))
            .expect("order count should load");
        let coordination_count: i64 = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .query_row(
                "select count(*) from buyer_order_coordination_records",
                [],
                |row| row.get(0),
            )
            .expect("coordination count should load");
        assert_eq!(order_count, 0);
        assert_eq!(coordination_count, 0);
    }

    #[test]
    fn runtime_prepares_seller_order_accept_payload_from_signed_request() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_accept_payload", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);

        let payload = runtime
            .prepare_order_accept(order_id)
            .expect("seller order accept payload should prepare");
        let decision = order_decision_publish_payload_to_sdk_decision(&payload);

        assert_eq!(payload.app_order_id, order_id);
        assert_eq!(payload.trade_order_id, "seller-order-decision-1");
        assert_eq!(
            payload.request_event_id,
            "event-app:signed_event:order-request:seller-order-decision-1"
        );
        assert_eq!(
            payload.listing_event_id.as_deref(),
            Some("event-app:signed_event:listing:seller-order-decision")
        );
        assert_eq!(payload.buyer_pubkey, buyer_pubkey);
        assert_eq!(payload.seller_pubkey, seller_pubkey);
        assert_eq!(decision.order_id, "seller-order-decision-1");
        let RadrootsTradeOrderDecision::Accepted {
            inventory_commitments,
        } = decision.decision
        else {
            panic!("expected accepted decision");
        };
        assert_eq!(inventory_commitments.len(), 1);
        assert_eq!(inventory_commitments[0].bin_id, "seller-order-primary-bin");
        assert_eq!(inventory_commitments[0].bin_count, 2);

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_prepares_seller_order_decline_payload_with_trimmed_reason() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_decline_payload", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);

        let payload = runtime
            .prepare_order_decline(order_id, "  out of stock  ")
            .expect("seller order decline payload should prepare");
        let decision = order_decision_publish_payload_to_sdk_decision(&payload);

        assert_eq!(payload.buyer_pubkey, buyer_pubkey);
        assert_eq!(payload.seller_pubkey, seller_pubkey);
        assert_eq!(
            payload.decision,
            AppOrderDecisionPayload::Declined {
                reason: "out of stock".to_owned()
            }
        );
        let RadrootsTradeOrderDecision::Declined { reason } = decision.decision else {
            panic!("expected declined decision");
        };
        assert_eq!(reason, "out of stock");

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finds_seller_order_request_evidence_past_first_local_events_page() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, _seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_old_request_evidence", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);
        append_unrelated_signed_event_records(&paths, 1_005);

        let payload = runtime
            .prepare_order_accept(order_id)
            .expect("seller order accept payload should prepare from older evidence");

        assert_eq!(payload.trade_order_id, "seller-order-decision-1");
        assert_eq!(
            payload.request_event_id,
            "event-app:signed_event:order-request:seller-order-decision-1"
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_decision_with_unusable_request_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, _seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_unusable_request_evidence", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);
        mark_shared_seller_order_request_evidence_pending(&paths);

        let error = runtime
            .prepare_order_accept(order_id)
            .expect_err("seller order decision should require usable request evidence");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "seller order decision requires signed order request evidence"
            }
        ));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_refreshes_configured_relay_before_seller_order_decision_signing() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_relay_freshness_pre_sign", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);
        publish_prior_relay_seller_order_accept(
            &runtime,
            &relay,
            order_id,
            product_id,
            seller_pubkey.as_str(),
            buyer_pubkey.as_str(),
        );

        let error = runtime
            .prepare_order_accept(order_id)
            .expect_err("stale seller order decision should fail pre-signing");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "seller order decision requires an undecided order"
            }
        ));
        assert_eq!(persisted_order_status(&runtime, order_id), "scheduled");
        assert_eq!(relay.event_count(), 1);

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_decision_when_relay_freshness_fails() {
        let (runtime, paths, order_id, _product_id, _seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_relay_freshness_failure", 6, 2);
        runtime.lock_state_mut().nostr_relay_urls = vec!["ws://127.0.0.1:9".to_owned()];

        let error = runtime
            .prepare_order_accept(order_id)
            .expect_err("seller order decision should require fresh relay state");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "order lifecycle publish requires fresh configured relay state"
            }
        ));
        assert_eq!(persisted_order_status(&runtime, order_id), "needs_action");

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_decision_for_wrong_selected_account() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, _seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_wrong_account", 6, 2);
        configure_runtime_relay_ingest(&runtime, &relay);
        assert!(
            runtime
                .generate_local_account(Some("Other seller".to_owned()))
                .expect("other account should generate")
        );
        configure_runtime_relay_ingest(&runtime, &relay);

        let error = runtime
            .prepare_order_accept(order_id)
            .expect_err("wrong seller account should fail preflight");

        assert!(matches!(error, AppSqliteError::InvalidProjection { .. }));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_accept_that_would_over_reserve_inventory() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, _seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_over_reserved", 1, 2);
        configure_runtime_relay_ingest(&runtime, &relay);

        let error = runtime
            .prepare_order_accept(order_id)
            .expect_err("over-reserved seller order should fail preflight");

        assert!(matches!(error, AppSqliteError::InvalidProjection { .. }));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_publishes_seller_order_accept_and_projects_signed_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_accept_publish", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);

        assert!(
            runtime
                .publish_order_accept(order_id)
                .expect("seller order accept should publish")
        );

        assert_eq!(persisted_order_status(&runtime, order_id), "scheduled");
        assert_eq!(relay.event_count(), 1);
        assert!(shared_local_event_records(&paths).iter().any(|record| {
            record.family == LocalRecordFamily::SignedEvent
                && record.event_kind == Some(3423)
                && record.event_pubkey.as_deref() == Some(seller_pubkey.as_str())
        }));
        let decision_event = shared_seller_order_decision_event(&paths, seller_pubkey.as_str());
        let envelope = radroots_sdk::trade::parse_order_decision(&decision_event)
            .expect("app seller order accept should parse as canonical order decision");
        assert_eq!(envelope.payload.order_id, "seller-order-decision-1");
        assert!(matches!(
            envelope.payload.decision,
            RadrootsTradeOrderDecision::Accepted { .. }
        ));
        assert!(event_has_tag(
            &decision_event,
            "e_root",
            "event-app:signed_event:order-request:seller-order-decision-1"
        ));
        assert!(event_has_tag(
            &decision_event,
            "e_prev",
            "event-app:signed_event:order-request:seller-order-decision-1"
        ));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_publishes_seller_order_decline_and_projects_signed_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, _product_id, seller_pubkey, _buyer_pubkey) =
            seller_order_decision_runtime("seller_order_decline_publish", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);

        assert!(
            runtime
                .publish_order_decline(order_id, "not available")
                .expect("seller order decline should publish")
        );

        assert_eq!(persisted_order_status(&runtime, order_id), "declined");
        assert_eq!(relay.event_count(), 1);
        assert!(shared_local_event_records(&paths).iter().any(|record| {
            record.family == LocalRecordFamily::SignedEvent
                && record.event_kind == Some(3423)
                && record.event_pubkey.as_deref() == Some(seller_pubkey.as_str())
        }));
        let decision_event = shared_seller_order_decision_event(&paths, seller_pubkey.as_str());
        let envelope = radroots_sdk::trade::parse_order_decision(&decision_event)
            .expect("app seller order decline should parse as canonical order decision");
        let RadrootsTradeOrderDecision::Declined { reason } = envelope.payload.decision else {
            panic!("expected declined decision");
        };
        assert_eq!(reason, "not available");
        assert!(event_has_tag(
            &decision_event,
            "e_root",
            "event-app:signed_event:order-request:seller-order-decision-1"
        ));
        assert!(event_has_tag(
            &decision_event,
            "e_prev",
            "event-app:signed_event:order-request:seller-order-decision-1"
        ));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_publishes_all_seller_fulfillment_states_and_projects_signed_evidence() {
        for (label, action, expected_status, expected_order_status) in [
            (
                "preparing",
                OrderFulfillmentAction::Preparing,
                RadrootsActiveTradeFulfillmentState::Preparing,
                "scheduled",
            ),
            (
                "ready_for_pickup",
                OrderFulfillmentAction::ReadyForPickup,
                RadrootsActiveTradeFulfillmentState::ReadyForPickup,
                "packed",
            ),
            (
                "out_for_delivery",
                OrderFulfillmentAction::OutForDelivery,
                RadrootsActiveTradeFulfillmentState::OutForDelivery,
                "packed",
            ),
            (
                "delivered",
                OrderFulfillmentAction::Delivered,
                RadrootsActiveTradeFulfillmentState::Delivered,
                "packed",
            ),
            (
                "seller_cancelled",
                OrderFulfillmentAction::SellerCancelled,
                RadrootsActiveTradeFulfillmentState::SellerCancelled,
                "declined",
            ),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let runtime_label = format!("seller_order_fulfillment_publish_{label}");
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(runtime_label.as_str(), 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            publish_prior_relay_seller_order_accept(
                &runtime,
                &relay,
                order_id,
                product_id,
                seller_pubkey.as_str(),
                buyer_pubkey.as_str(),
            );

            assert!(
                runtime
                    .publish_order_fulfillment_update(order_id, action)
                    .expect("seller fulfillment update should publish")
            );

            assert_eq!(
                persisted_order_status(&runtime, order_id),
                expected_order_status
            );
            assert_eq!(relay.event_count(), 2);
            let fulfillment_events =
                shared_order_events_by_kind(&paths, 3433, seller_pubkey.as_str());
            assert_eq!(fulfillment_events.len(), 1);
            let fulfillment_event = fulfillment_events.first().expect("fulfillment event");
            let envelope = radroots_sdk::trade::parse_fulfillment_update(fulfillment_event)
                .expect("fulfillment should parse");
            assert_eq!(envelope.payload.status, expected_status);
            assert!(event_has_tag(
                fulfillment_event,
                "e_root",
                "event-app:signed_event:order-request:seller-order-decision-1"
            ));
            assert!(event_has_nonempty_value_tag(fulfillment_event, "e_prev"));

            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_publishes_seller_order_fulfillment_ready_from_revision_parent() {
        for (label, revision_decision) in [
            ("accepted", RadrootsTradeOrderRevisionDecision::Accepted),
            (
                "declined",
                RadrootsTradeOrderRevisionDecision::Declined {
                    reason: "keep original order".to_owned(),
                },
            ),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let runtime_label = format!("seller_order_fulfillment_revision_parent_{label}");
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(runtime_label.as_str(), 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
            let decision_event_id = append_signed_order_decision_record(
                &paths,
                "seller-order-decision-1",
                request_event_id,
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            let proposal_key = format!("seller-order-ready-revision-{label}-proposal");
            let proposal_event_id = append_signed_order_revision_proposal_record_with_prev(
                &paths,
                "seller-order-decision-1",
                proposal_key.as_str(),
                request_event_id,
                decision_event_id.as_str(),
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
            );
            let revision_id = format!("revision-{proposal_key}");
            let revision_decision_event_id = append_signed_order_revision_decision_record_with_prev(
                &paths,
                "seller-order-decision-1",
                format!("seller-order-ready-revision-{label}-decision").as_str(),
                request_event_id,
                proposal_event_id.as_str(),
                revision_id.as_str(),
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                revision_decision,
            );
            runtime
                .refresh_shared_local_events()
                .expect("seller revision fixture should import");
            set_persisted_order_status(&runtime, order_id, "scheduled");

            assert!(
                runtime
                    .publish_order_fulfillment_update(
                        order_id,
                        OrderFulfillmentAction::ReadyForPickup,
                    )
                    .expect("seller ready fulfillment should publish from revision parent")
            );

            assert_eq!(relay.event_count(), 1);
            let fulfillment_events =
                shared_order_events_by_kind(&paths, 3433, seller_pubkey.as_str());
            assert_eq!(fulfillment_events.len(), 1);
            let ready_event = fulfillment_events.first().expect("ready event");
            assert!(event_has_tag(
                ready_event,
                "e_prev",
                revision_decision_event_id.as_str()
            ));

            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_publishes_seller_order_fulfillment_delivered_when_coarse_status_lags() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_fulfillment_delivery_status_lag", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
        let decision_event_id = append_signed_order_decision_record(
            &paths,
            "seller-order-decision-1",
            request_event_id,
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        let ready_event_id = append_signed_order_fulfillment_record_with_status(
            &paths,
            "seller-order-decision-1",
            request_event_id,
            decision_event_id.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            RadrootsActiveTradeFulfillmentState::ReadyForPickup,
        );
        runtime
            .refresh_shared_local_events()
            .expect("seller ready fulfillment should import");
        assert_eq!(persisted_order_status(&runtime, order_id), "packed");
        set_persisted_order_status(&runtime, order_id, "scheduled");

        assert!(
            runtime
                .publish_order_fulfillment_update(order_id, OrderFulfillmentAction::Delivered)
                .expect("seller delivered fulfillment should publish from workflow evidence")
        );

        assert_eq!(persisted_order_status(&runtime, order_id), "packed");
        assert_eq!(relay.event_count(), 1);
        let fulfillment_events = shared_order_events_by_kind(&paths, 3433, seller_pubkey.as_str());
        assert_eq!(fulfillment_events.len(), 2);
        let delivered_event = fulfillment_events
            .iter()
            .find(|event| {
                radroots_sdk::trade::parse_fulfillment_update(event)
                    .map(|envelope| {
                        envelope.payload.status == RadrootsActiveTradeFulfillmentState::Delivered
                    })
                    .unwrap_or(false)
            })
            .expect("delivered fulfillment event should exist");
        assert!(event_has_tag(
            delivered_event,
            "e_prev",
            ready_event_id.as_str()
        ));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_publishes_seller_order_fulfillment_delivered_without_ready_evidence() {
        for (label, latest_fulfillment) in [
            ("seller_order_fulfillment_delivery_missing_ready", None),
            (
                "seller_order_fulfillment_delivery_preparing",
                Some(RadrootsActiveTradeFulfillmentState::Preparing),
            ),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(label, 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
            let decision_event_id = append_signed_order_decision_record(
                &paths,
                "seller-order-decision-1",
                request_event_id,
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            if let Some(status) = latest_fulfillment {
                append_signed_order_fulfillment_record_with_status(
                    &paths,
                    "seller-order-decision-1",
                    request_event_id,
                    decision_event_id.as_str(),
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    status,
                );
            }
            runtime
                .refresh_shared_local_events()
                .expect("seller fulfillment fixture should import");

            assert!(
                runtime
                    .publish_order_fulfillment_update(order_id, OrderFulfillmentAction::Delivered)
                    .expect("seller delivered fulfillment should publish")
            );

            assert_eq!(persisted_order_status(&runtime, order_id), "packed");
            assert_eq!(relay.event_count(), 1);
            let fulfillment_events =
                shared_order_events_by_kind(&paths, 3433, seller_pubkey.as_str());
            let delivered_event = fulfillment_events
                .iter()
                .find(|event| {
                    radroots_sdk::trade::parse_fulfillment_update(event)
                        .map(|envelope| {
                            envelope.payload.status
                                == RadrootsActiveTradeFulfillmentState::Delivered
                        })
                        .unwrap_or(false)
                })
                .expect("delivered fulfillment event should exist");
            assert!(event_has_tag(
                delivered_event,
                "e_prev",
                latest_fulfillment
                    .map(|_| {
                        "event-app:signed_event:fulfillment:seller-order-decision-1".to_owned()
                    })
                    .unwrap_or_else(|| decision_event_id.clone())
                    .as_str()
            ));
            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_rejects_seller_order_fulfillment_delivered_with_reducer_invalid_ready_evidence() {
        for label in [
            "seller_order_fulfillment_delivery_unchained_ready",
            "seller_order_fulfillment_delivery_forked_ready",
        ] {
            let relay = ThreadedAckRelay::spawn();
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(label, 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
            let decision_event_id = append_signed_order_decision_record(
                &paths,
                "seller-order-decision-1",
                request_event_id,
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            if label.ends_with("unchained_ready") {
                append_signed_order_fulfillment_record_with_status_and_key(
                    &paths,
                    "seller-order-decision-1",
                    "seller-order-decision-1-unchained-ready",
                    request_event_id,
                    request_event_id,
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    RadrootsActiveTradeFulfillmentState::ReadyForPickup,
                );
            } else {
                append_signed_order_fulfillment_record_with_status_and_key(
                    &paths,
                    "seller-order-decision-1",
                    "seller-order-decision-1-forked-preparing",
                    request_event_id,
                    decision_event_id.as_str(),
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    RadrootsActiveTradeFulfillmentState::Preparing,
                );
                append_signed_order_fulfillment_record_with_status_and_key(
                    &paths,
                    "seller-order-decision-1",
                    "seller-order-decision-1-forked-ready",
                    request_event_id,
                    decision_event_id.as_str(),
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    RadrootsActiveTradeFulfillmentState::ReadyForPickup,
                );
            }

            let error = runtime
                .publish_order_fulfillment_update(order_id, OrderFulfillmentAction::Delivered)
                .expect_err("seller delivered fulfillment should reject reducer-invalid evidence");

            assert_order_lifecycle_evidence_invalid(error);
            assert_eq!(relay.event_count(), 0);
            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_rejects_seller_order_fulfillment_ready_with_invalid_terminal_evidence() {
        for label in [
            "seller_order_fulfillment_invalid_cancellation",
            "seller_order_fulfillment_invalid_receipt",
        ] {
            let relay = ThreadedAckRelay::spawn();
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(label, 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
            append_signed_order_decision_record(
                &paths,
                "seller-order-decision-1",
                request_event_id,
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            if label.ends_with("invalid_cancellation") {
                append_signed_order_cancellation_record_with_prev(
                    &paths,
                    "seller-order-decision-1",
                    "seller-order-decision-1-invalid-cancellation",
                    request_event_id,
                    request_event_id,
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                );
            } else {
                append_signed_order_receipt_record_with_prev(
                    &paths,
                    "seller-order-decision-1",
                    "seller-order-decision-1-invalid-receipt",
                    request_event_id,
                    request_event_id,
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    true,
                );
            }

            let error = runtime
                .publish_order_fulfillment_update(order_id, OrderFulfillmentAction::ReadyForPickup)
                .expect_err("seller ready fulfillment should reject invalid terminal evidence");

            assert_order_lifecycle_evidence_invalid(error);
            assert_eq!(relay.event_count(), 0);
            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_rejects_seller_order_revision_with_reducer_invalid_parent_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_revision_invalid_parent", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
        append_signed_order_decision_record(
            &paths,
            "seller-order-decision-1",
            request_event_id,
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        append_signed_order_revision_proposal_record_with_prev(
            &paths,
            "seller-order-decision-1",
            "seller-order-decision-1-stale-revision",
            request_event_id,
            request_event_id,
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
        );

        let error = runtime
            .publish_order_revision_proposal(
                order_id,
                revision_test_order_items(),
                revision_test_order_economics(),
                "harvest count updated",
            )
            .expect_err("seller revision proposal should reject reducer-invalid parent evidence");

        assert_order_lifecycle_evidence_invalid(error);
        assert_eq!(relay.event_count(), 0);
        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_revision_after_recorded_or_settled_payment_evidence() {
        for (label, settlement) in [
            ("recorded", None),
            ("settled", Some(RadrootsTradeSettlementDecision::Accepted)),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let runtime_label = format!("seller_order_revision_payment_{label}");
            let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
                seller_order_decision_runtime(runtime_label.as_str(), 6, 2);
            install_direct_relay_sync_transport(&runtime, &relay);
            let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
            let decision_event_id = append_signed_order_decision_record(
                &paths,
                "seller-order-decision-1",
                request_event_id,
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            let payment_event_id = append_signed_payment_record(
                &paths,
                "seller-order-decision-1",
                format!("seller-order-revision-payment-{label}").as_str(),
                request_event_id,
                decision_event_id.as_str(),
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey.as_str(),
                2,
            );
            if let Some(decision) = settlement {
                append_signed_settlement_decision_record(
                    &paths,
                    "seller-order-decision-1",
                    format!("seller-order-revision-k3436-{label}").as_str(),
                    request_event_id,
                    decision_event_id.as_str(),
                    payment_event_id.as_str(),
                    listing_addr.as_str(),
                    buyer_pubkey.as_str(),
                    seller_pubkey.as_str(),
                    2,
                    decision,
                );
            }
            runtime
                .refresh_shared_local_events()
                .expect("seller payment evidence should import");
            set_persisted_order_status(&runtime, order_id, "scheduled");

            let error = runtime
                .publish_order_revision_proposal(
                    order_id,
                    revision_test_order_items(),
                    revision_test_order_economics(),
                    "harvest count updated",
                )
                .expect_err("seller revision proposal should reject payment evidence");

            assert!(matches!(
                error,
                AppSqliteError::InvalidProjection {
                    reason: "seller order revision requires no recorded or settled payment"
                }
            ));
            assert_eq!(relay.event_count(), 0);
            cleanup_bootstrapped_runtime_paths(&paths);
        }
    }

    #[test]
    fn runtime_publishes_seller_order_revision_after_rejected_settlement_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_revision_rejected_payment", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
        let decision_event_id = append_signed_order_decision_record(
            &paths,
            "seller-order-decision-1",
            request_event_id,
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        let payment_event_id = append_signed_payment_record(
            &paths,
            "seller-order-decision-1",
            "seller-order-revision-payment-rejected",
            request_event_id,
            decision_event_id.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        append_signed_settlement_decision_record(
            &paths,
            "seller-order-decision-1",
            "seller-order-revision-k3436-rejected",
            request_event_id,
            decision_event_id.as_str(),
            payment_event_id.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
            RadrootsTradeSettlementDecision::Rejected,
        );
        runtime
            .refresh_shared_local_events()
            .expect("seller rejected payment evidence should import");
        set_persisted_order_status(&runtime, order_id, "scheduled");

        assert!(
            runtime
                .publish_order_revision_proposal(
                    order_id,
                    revision_test_order_items(),
                    revision_test_order_economics(),
                    "harvest count updated",
                )
                .expect("seller revision proposal should publish after rejected 3436")
        );

        assert_eq!(relay.event_count(), 1);
        let revision_events = shared_order_events_by_kind(&paths, 3424, seller_pubkey.as_str());
        assert_eq!(revision_events.len(), 1);
        assert!(event_has_tag(
            revision_events.first().expect("seller revision event"),
            "e_prev",
            decision_event_id.as_str()
        ));
        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_rejects_seller_order_revision_with_invalid_payment_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let (runtime, paths, order_id, product_id, seller_pubkey, buyer_pubkey) =
            seller_order_decision_runtime("seller_order_revision_invalid_payment", 6, 2);
        install_direct_relay_sync_transport(&runtime, &relay);
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let request_event_id = "event-app:signed_event:order-request:seller-order-decision-1";
        let decision_event_id = append_signed_order_decision_record(
            &paths,
            "seller-order-decision-1",
            request_event_id,
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        append_signed_payment_record_with_prev(
            &paths,
            "seller-order-decision-1",
            "seller-order-revision-invalid-payment",
            request_event_id,
            request_event_id,
            decision_event_id.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            2,
        );
        runtime
            .refresh_shared_local_events()
            .expect("seller invalid payment evidence should import");
        set_persisted_order_status(&runtime, order_id, "scheduled");

        let error = runtime
            .publish_order_revision_proposal(
                order_id,
                revision_test_order_items(),
                revision_test_order_economics(),
                "harvest count updated",
            )
            .expect_err("seller revision proposal should reject invalid payment evidence");

        assert_order_lifecycle_evidence_invalid(error);
        assert_eq!(relay.event_count(), 0);
        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_places_supported_buyer_order_into_shared_local_events() {
        let (runtime, paths) = bootstrapped_runtime("buyer_order_local_event");
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let buyer_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let listing_key = "DDDDDDDDDDDDDDDDDDDDDD";
        append_cli_signed_buyer_listing_record_with_bin(
            &paths,
            "buyer-order-supported-listing",
            listing_key,
            "Buyer Visible Eggs",
            1100,
            "dozen-eggs",
        );
        let product_id =
            deterministic_cli_listing_product_id(Some("buyer-visible-seller-pubkey"), listing_key);

        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should import before lookup")
        );
        assert!(runtime.increase_personal_product_quantity(PersonalSection::Browse));
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute(
                "update products set listing_bin_id = 'mutated-bin' where id = ?1",
                [product_id.to_string()],
            )
            .expect("listing projection should mutate after cart snapshot");
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: "555-0101".to_owned(),
                    order_note: "Leave by the cooler".to_owned(),
                })
                .expect("buyer order review draft should save")
        );
        assert!(
            runtime
                .place_personal_order()
                .expect("buyer order should place")
        );
        let order_id = runtime.summary().personal_projection.orders.list.rows[0].order_id;
        let order_farm_id = runtime
            .summary()
            .personal_projection
            .orders
            .detail
            .as_ref()
            .expect("buyer order detail")
            .farm_id;
        let pending_payload = assert_single_order_request_publish_payload(
            &runtime,
            buyer_account_id.as_str(),
            order_id,
            order_farm_id,
            "needs_action",
        );
        assert_eq!(
            pending_payload.context.source_local_event_id.as_deref(),
            Some(format!("app:local_work:order_request:{order_id}").as_str())
        );
        assert_eq!(
            pending_payload.listing_addr.as_deref(),
            Some(format!("30402:buyer-visible-seller-pubkey:{listing_key}").as_str())
        );
        assert_eq!(
            pending_payload.listing_event_id.as_deref(),
            Some("event-cli:signed_event:buyer-order-supported-listing")
        );
        assert_eq!(pending_payload.listing_relays, vec!["ws://127.0.0.1:1234/"]);
        assert_eq!(
            pending_payload.seller_pubkey.as_deref(),
            Some("buyer-visible-seller-pubkey")
        );
        assert!(
            pending_payload
                .buyer_pubkey
                .as_deref()
                .is_some_and(is_hex_64)
        );
        assert_eq!(pending_payload.items.len(), 1);
        assert_eq!(pending_payload.items[0].product_id, product_id);
        assert_eq!(pending_payload.items[0].quantity, 2);
        assert_eq!(pending_payload.currency_code.as_deref(), Some("USD"));
        assert_eq!(pending_payload.total_minor_units, Some(1600));
        assert_eq!(pending_payload.note.as_deref(), Some("Leave by the cooler"));
        assert!(pending_payload.order_document_json.is_some());

        {
            let state = runtime.lock_state_mut();
            let buyer_context = state.state_store.identity_projection().buyer_context();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            let order_export = state
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_buyer_order_local_event_export(&buyer_context, order_id)
                .expect("order export should load")
                .expect("order export should exist");
            let coordination = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("order coordination should load")
                .expect("order coordination should exist");
            assert_eq!(coordination.state, BuyerOrderCoordinationState::Synced);
            assert_eq!(
                coordination.record_id.as_deref(),
                Some(format!("app:local_work:order_request:{order_id}").as_str())
            );
            assert!(coordination.payload_json.is_some());
            assert_eq!(coordination.attempt_count, 1);
            assert_eq!(coordination.last_error_message, None);
            assert!(
                state
                    .append_app_buyer_order_request_local_work_record(
                        sqlite_store,
                        &buyer_context,
                        &order_export,
                    )
                    .expect("order local event reappend should be idempotent")
                    .is_some()
            );
            let coordination_after = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("order coordination should reload")
                .expect("order coordination should still exist");
            assert_eq!(coordination_after.attempt_count, 1);
        }

        let records = shared_local_event_records(&paths);
        let order_records = records
            .iter()
            .filter(|record| {
                record.source_runtime == SourceRuntime::App
                    && record
                        .local_work_json
                        .as_ref()
                        .and_then(|payload| payload["record_kind"].as_str())
                        == Some(BUYER_ORDER_REQUEST_LOCAL_WORK_RECORD_KIND)
            })
            .collect::<Vec<_>>();
        assert_eq!(order_records.len(), 1);
        let order_record = order_records[0];
        assert_eq!(order_record.family, LocalRecordFamily::LocalWork);
        assert_eq!(order_record.status, LocalRecordStatus::LocalSaved);
        assert_eq!(order_record.outbox_status, PublishOutboxStatus::None);
        assert_eq!(
            order_record.record_id,
            format!("app:local_work:order_request:{order_id}")
        );
        assert_eq!(
            order_record.owner_account_id.as_deref(),
            Some(buyer_account_id.as_str())
        );
        assert!(order_record.owner_pubkey.as_deref().is_some_and(is_hex_64));
        assert_eq!(
            order_record.listing_addr.as_deref(),
            Some(format!("30402:buyer-visible-seller-pubkey:{listing_key}").as_str())
        );
        let payload = order_record
            .local_work_json
            .as_ref()
            .expect("order local work payload");
        assert_eq!(payload["support_status"]["state"], "supported");
        assert_eq!(payload["payment_display"]["state"], "not_recorded");
        assert_eq!(payload["payment_display"]["allows_payment_action"], false);
        assert_eq!(payload["currentness"]["current"], true);
        assert_eq!(payload["document"]["kind"], "order_draft_v1");
        assert_eq!(
            payload["document"]["order"]["order_id"],
            order_id.to_string()
        );
        assert_eq!(
            payload["document"]["order"]["listing_event_id"],
            "event-cli:signed_event:buyer-order-supported-listing"
        );
        assert_eq!(
            payload["document"]["order"]["listing_relays"],
            json!(["ws://127.0.0.1:1234/"])
        );
        assert_eq!(
            payload["document"]["order"]["seller_pubkey"],
            "buyer-visible-seller-pubkey"
        );
        assert_eq!(
            payload["document"]["order"]["items"][0]["bin_id"],
            "dozen-eggs"
        );
        assert_eq!(payload["document"]["order"]["items"][0]["bin_count"], 2);
        assert_eq!(
            payload["document"]["order"]["economics"]["items"][0]["quantity_amount"],
            "1"
        );
        assert_eq!(
            payload["document"]["order"]["economics"]["items"][0]["bin_id"],
            "dozen-eggs"
        );
        assert_eq!(
            payload["document"]["order"]["economics"]["pricing_basis"],
            "listing_event"
        );
        assert_eq!(
            payload["document"]["order"]["economics"]["total"]["amount"],
            "16.00"
        );
        assert_eq!(
            payload["app_order"]["buyer_order_note"],
            "Leave by the cooler"
        );
        assert_eq!(
            payload["app_order"]["lines"][0]["listing_bin_id"],
            "dozen-eggs"
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_order_shared_append_failure_is_recoverable_in_same_session() {
        let (runtime, paths, buyer_account_id, order_id, order_farm_id) =
            blocked_buyer_order_runtime("buyer_order_append_failure_same_session");
        {
            let state = runtime.lock_state_mut();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            sqlite_store
                .connection()
                .execute(
                    "update orders set status = 'scheduled' where id = ?1",
                    [order_id.to_string()],
                )
                .expect("buyer order status should mutate before retry refresh");
        }
        unblock_shared_local_events_database(&paths);
        assert!(
            runtime
                .retry_pending_personal_order_coordination()
                .expect("same-session buyer order recovery retry should sync")
        );
        let summary_after_retry = runtime.summary();
        assert!(
            !summary_after_retry
                .personal_projection
                .orders
                .has_recoverable_coordination
        );
        assert_single_order_request_publish_payload(
            &runtime,
            buyer_account_id.as_str(),
            order_id,
            order_farm_id,
            "scheduled",
        );
        assert_eq!(
            summary_after_retry
                .personal_projection
                .orders
                .list
                .rows
                .len(),
            1
        );
        assert_eq!(
            summary_after_retry.personal_projection.orders.list.rows[0].order_id,
            order_id
        );
        assert_eq!(
            summary_after_retry.personal_projection.orders.list.rows[0].status,
            BuyerOrderStatus::Scheduled
        );
        assert_eq!(
            summary_after_retry
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("buyer order detail should refresh after same-session retry")
                .status,
            BuyerOrderStatus::Scheduled
        );
        assert_eq!(
            buyer_order_local_work_record_ids(&paths),
            vec![format!("app:local_work:order_request:{order_id}")]
        );
        {
            let state = runtime.lock_state_mut();
            let buyer_context = state.state_store.identity_projection().buyer_context();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            let buyer_orders = sqlite_store
                .load_buyer_orders(&buyer_context)
                .expect("buyer orders should reload");
            assert_eq!(buyer_orders.rows.len(), 1);
            let coordination = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("buyer order coordination should reload")
                .expect("buyer order coordination should still exist");
            assert_eq!(coordination.state, BuyerOrderCoordinationState::Synced);
            assert_eq!(coordination.attempt_count, 2);
            assert_eq!(coordination.last_error_message, None);
        }
        assert!(
            !runtime
                .retry_pending_personal_order_coordination()
                .expect("same-session synced buyer order recovery retry should be idempotent")
        );
        assert_single_order_request_publish_payload(
            &runtime,
            buyer_account_id.as_str(),
            order_id,
            order_farm_id,
            "scheduled",
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_buyer_order_shared_append_failure_is_recoverable_after_restart() {
        let (runtime, paths, buyer_account_id, order_id, order_farm_id) =
            blocked_buyer_order_runtime("buyer_order_append_failure_restart");
        unblock_shared_local_events_database(&paths);
        drop(runtime);

        let restarted_runtime = restart_runtime(paths.clone());
        assert_eq!(
            buyer_order_local_work_record_ids(&paths),
            vec![format!("app:local_work:order_request:{order_id}")]
        );
        let summary = restarted_runtime.summary();
        assert!(
            !summary
                .personal_projection
                .orders
                .has_recoverable_coordination
        );
        assert_eq!(summary.personal_projection.orders.list.rows.len(), 1);
        assert_eq!(
            summary.personal_projection.orders.list.rows[0].order_id,
            order_id
        );
        assert_eq!(
            summary
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("buyer order detail should reload after restart")
                .order_id,
            order_id
        );
        {
            let state = restarted_runtime.lock_state_mut();
            let buyer_context = state.state_store.identity_projection().buyer_context();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            let buyer_orders = sqlite_store
                .load_buyer_orders(&buyer_context)
                .expect("buyer orders should reload");
            assert_eq!(buyer_orders.rows.len(), 1);
            let coordination = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("buyer order coordination should reload")
                .expect("buyer order coordination should still exist");
            assert_eq!(coordination.state, BuyerOrderCoordinationState::Synced);
            assert_eq!(coordination.attempt_count, 2);
            assert_eq!(coordination.last_error_message, None);
        }
        assert!(
            !restarted_runtime
                .retry_pending_personal_order_coordination()
                .expect("synced buyer order recovery retry should be idempotent")
        );
        assert_single_order_request_publish_payload(
            &restarted_runtime,
            buyer_account_id.as_str(),
            order_id,
            order_farm_id,
            "needs_action",
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_outbox_recovery_buyer_order_shared_append_failure_is_recoverable_on_foreground_resume()
     {
        let (runtime, paths, buyer_account_id, order_id, order_farm_id) =
            blocked_buyer_order_runtime("buyer_order_append_failure_foreground_resume");
        unblock_shared_local_events_database(&paths);
        assert!(
            runtime
                .sync_on_foreground_resume()
                .expect("foreground resume should repair buyer order coordination")
        );
        let summary = runtime.summary();
        assert!(
            !summary
                .personal_projection
                .orders
                .has_recoverable_coordination
        );
        assert_eq!(
            buyer_order_local_work_record_ids(&paths),
            vec![format!("app:local_work:order_request:{order_id}")]
        );
        assert_single_order_request_publish_payload(
            &runtime,
            buyer_account_id.as_str(),
            order_id,
            order_farm_id,
            "needs_action",
        );
        {
            let state = runtime.lock_state_mut();
            let buyer_context = state.state_store.identity_projection().buyer_context();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            let coordination = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("buyer order coordination should reload")
                .expect("buyer order coordination should still exist");
            assert_eq!(coordination.state, BuyerOrderCoordinationState::Synced);
            assert_eq!(coordination.attempt_count, 2);
            assert_eq!(coordination.last_error_message, None);
        }
        assert!(
            !runtime
                .retry_pending_personal_order_coordination()
                .expect("foreground-resumed buyer order retry should be idempotent")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_opens_buyer_order_detail_from_personal_orders() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id = '{product_id}'"
            ))
            .expect("buyer detail product should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                })
                .expect("buyer order review draft should save")
        );
        assert!(
            runtime
                .place_personal_order()
                .expect("buyer order should place")
        );
        let order_id = runtime.summary().personal_projection.orders.list.rows[0].order_id;
        assert!(
            runtime
                .select_personal_section(PersonalSection::Browse)
                .expect("buyer Browse selection should succeed")
        );
        assert!(runtime.lock_state_mut().set_personal_order_detail(None));

        assert!(
            runtime
                .open_personal_order_detail(order_id)
                .expect("buyer order detail should open")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Orders)
        );
        assert_eq!(
            summary
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("buyer order detail")
                .order_id,
            order_id
        );
    }

    #[test]
    fn runtime_opens_linked_buyer_order_detail_from_selected_account_nostr_scope() {
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_open", false);
        let report = fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(report.imported_records > 0);

        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );

        let summary = fixture.runtime.summary();
        let row = summary
            .personal_projection
            .orders
            .list
            .rows
            .iter()
            .find(|row| row.order_id == fixture.order_id)
            .expect("linked buyer order row should exist");
        let detail = summary
            .personal_projection
            .orders
            .detail
            .as_ref()
            .expect("linked buyer order detail should exist");
        assert_eq!(row.status, BuyerOrderStatus::Scheduled);
        assert_eq!(detail.order_id, fixture.order_id);
        assert_eq!(detail.status, BuyerOrderStatus::Scheduled);
        assert_eq!(
            detail.workflow.provenance.last_event_id.as_deref(),
            Some(fixture.decision_event_id.as_str())
        );

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_publishes_linked_buyer_cancellation_from_selected_account_nostr_scope() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_cancel", false);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_cancel(fixture.order_id)
                .expect("linked buyer cancellation should publish")
        );

        assert_eq!(
            persisted_order_status(&fixture.runtime, fixture.order_id),
            "declined"
        );
        assert_eq!(relay.event_count(), 1);
        let cancellation_events =
            shared_order_events_by_kind(&fixture.paths, 3432, fixture.buyer_pubkey.as_str());
        assert_eq!(cancellation_events.len(), 1);
        let cancellation_event = cancellation_events
            .first()
            .expect("linked buyer cancellation event");
        let cancellation = radroots_sdk::trade::parse_order_cancellation(cancellation_event)
            .expect("linked buyer cancellation should parse");
        assert_eq!(cancellation.payload.order_id, fixture.trade_order_id);
        assert_eq!(cancellation.payload.buyer_pubkey, fixture.buyer_pubkey);
        assert_eq!(cancellation.payload.seller_pubkey, fixture.seller_pubkey);
        assert!(event_has_tag(
            cancellation_event,
            "e_root",
            fixture.request_event_id.as_str()
        ));
        assert!(event_has_tag(
            cancellation_event,
            "e_prev",
            fixture.decision_event_id.as_str()
        ));

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_cancellation_after_recorded_or_settled_payment_evidence() {
        for (label, settlement) in [
            ("recorded", None),
            ("settled", Some(RadrootsTradeSettlementDecision::Accepted)),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let fixture_label = format!("linked_buyer_order_cancel_payment_{label}");
            let fixture = linked_buyer_lifecycle_runtime(fixture_label.as_str(), false);
            install_direct_relay_sync_transport(&fixture.runtime, &relay);
            let payment_event_id = append_signed_payment_record(
                &fixture.paths,
                fixture.trade_order_id.as_str(),
                format!("linked-buyer-cancel-payment-{label}").as_str(),
                fixture.request_event_id.as_str(),
                fixture.decision_event_id.as_str(),
                fixture.listing_addr.as_str(),
                fixture.buyer_pubkey.as_str(),
                fixture.seller_pubkey.as_str(),
                2,
            );
            if let Some(decision) = settlement {
                append_signed_settlement_decision_record(
                    &fixture.paths,
                    fixture.trade_order_id.as_str(),
                    format!("linked-buyer-cancel-k3436-{label}").as_str(),
                    fixture.request_event_id.as_str(),
                    fixture.decision_event_id.as_str(),
                    payment_event_id.as_str(),
                    fixture.listing_addr.as_str(),
                    fixture.buyer_pubkey.as_str(),
                    fixture.seller_pubkey.as_str(),
                    2,
                    decision,
                );
            }
            fixture
                .runtime
                .refresh_shared_local_events()
                .expect("linked buyer payment evidence should import");
            assert!(
                fixture
                    .runtime
                    .open_personal_order_detail(fixture.order_id)
                    .expect("linked buyer order detail should open")
            );

            let error = fixture
                .runtime
                .publish_buyer_order_cancel(fixture.order_id)
                .expect_err("linked buyer cancellation should reject payment evidence");

            assert!(matches!(
                error,
                AppSqliteError::InvalidProjection {
                    reason: "buyer order cancellation requires no recorded or settled payment"
                }
            ));
            assert_eq!(relay.event_count(), 0);
            cleanup_bootstrapped_runtime_paths(&fixture.paths);
        }
    }

    #[test]
    fn runtime_publishes_linked_buyer_cancellation_after_rejected_settlement_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let fixture =
            linked_buyer_lifecycle_runtime("linked_buyer_order_cancel_rejected_payment", false);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        let payment_event_id = append_signed_payment_record(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-cancel-payment-rejected",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
        );
        append_signed_settlement_decision_record(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-cancel-k3436-rejected",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            payment_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
            RadrootsTradeSettlementDecision::Rejected,
        );
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer rejected payment evidence should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_cancel(fixture.order_id)
                .expect("linked buyer cancellation should publish after rejected 3436")
        );

        assert_eq!(relay.event_count(), 1);
        let cancellation_events =
            shared_order_events_by_kind(&fixture.paths, 3432, fixture.buyer_pubkey.as_str());
        assert_eq!(cancellation_events.len(), 1);
        assert!(event_has_tag(
            cancellation_events
                .first()
                .expect("linked buyer cancellation event"),
            "e_prev",
            fixture.decision_event_id.as_str()
        ));
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_cancellation_with_invalid_payment_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let fixture =
            linked_buyer_lifecycle_runtime("linked_buyer_order_cancel_invalid_payment", false);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        append_signed_payment_record_with_prev(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-cancel-invalid-payment",
            fixture.request_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
        );
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer invalid payment evidence should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );

        let error = fixture
            .runtime
            .publish_buyer_order_cancel(fixture.order_id)
            .expect_err("linked buyer cancellation should reject invalid payment evidence");

        assert_order_lifecycle_evidence_invalid(error);
        assert_eq!(relay.event_count(), 0);
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_cancellation_after_relay_payment_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let fixture =
            linked_buyer_lifecycle_runtime("linked_buyer_order_cancel_relay_payment", false);
        configure_runtime_relay_ingest(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );
        let buyer_identity = selected_account_signing_identity(&fixture.runtime);
        let payment_event = signed_payment_recorded_relay_event(
            &buyer_identity,
            fixture.trade_order_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
        );
        let payment_event_id = payment_event.id.to_hex();
        relay.push_event(&payment_event);

        let error = fixture
            .runtime
            .publish_buyer_order_cancel(fixture.order_id)
            .expect_err("linked buyer cancellation should reject relay payment evidence");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires no recorded or settled payment"
            }
        ));
        let payment_events = fixture
            .runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_local_interop_signed_events_by_kind(3435)
            .expect("relay payment evidence should load from local interop");
        assert!(
            payment_events
                .iter()
                .any(|event| event.id == payment_event_id)
        );
        assert_eq!(relay.event_count(), 1);
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_cancellation_after_relay_settlement_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let seller_identity = RadrootsIdentity::generate();
        let seller_pubkey = seller_identity.public_key_hex();
        let fixture = linked_buyer_lifecycle_runtime_with_seller_pubkey(
            "linked_buyer_order_cancel_relay_k3436",
            false,
            seller_pubkey.as_str(),
        );
        configure_runtime_relay_ingest(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );
        let buyer_identity = selected_account_signing_identity(&fixture.runtime);
        let payment_event = signed_payment_recorded_relay_event(
            &buyer_identity,
            fixture.trade_order_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
        );
        let payment_event_id = payment_event.id.to_hex();
        let k3436_event = signed_settlement_decision_relay_event(
            &seller_identity,
            fixture.trade_order_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            payment_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            2,
            RadrootsTradeSettlementDecision::Accepted,
        );
        let k3436_event_id = k3436_event.id.to_hex();
        relay.push_event(&payment_event);
        relay.push_event(&k3436_event);

        let error = fixture
            .runtime
            .publish_buyer_order_cancel(fixture.order_id)
            .expect_err("linked buyer cancellation should reject relay k3436 evidence");

        assert!(matches!(
            error,
            AppSqliteError::InvalidProjection {
                reason: "buyer order cancellation requires no recorded or settled payment"
            }
        ));
        let k3436_events = fixture
            .runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_local_interop_signed_events_by_kind(3436)
            .expect("relay k3436 evidence should load from local interop");
        assert!(k3436_events.iter().any(|event| event.id == k3436_event_id));
        assert_eq!(relay.event_count(), 2);
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_publishes_linked_buyer_cancellation_from_revision_parent() {
        for (label, revision_decision) in [
            ("accepted", RadrootsTradeOrderRevisionDecision::Accepted),
            (
                "declined",
                RadrootsTradeOrderRevisionDecision::Declined {
                    reason: "keep original order".to_owned(),
                },
            ),
        ] {
            let relay = ThreadedAckRelay::spawn();
            let fixture_label = format!("linked_buyer_order_cancel_revision_{label}");
            let fixture = linked_buyer_lifecycle_runtime(fixture_label.as_str(), false);
            let proposal_key = format!("linked-buyer-order-cancel-revision-{label}-proposal");
            let proposal_event_id = append_signed_order_revision_proposal_record_with_prev(
                &fixture.paths,
                fixture.trade_order_id.as_str(),
                proposal_key.as_str(),
                fixture.request_event_id.as_str(),
                fixture.decision_event_id.as_str(),
                fixture.listing_addr.as_str(),
                fixture.buyer_pubkey.as_str(),
                fixture.seller_pubkey.as_str(),
            );
            let revision_id = format!("revision-{proposal_key}");
            let revision_decision_event_id = append_signed_order_revision_decision_record_with_prev(
                &fixture.paths,
                fixture.trade_order_id.as_str(),
                format!("linked-buyer-order-cancel-revision-{label}-decision").as_str(),
                fixture.request_event_id.as_str(),
                proposal_event_id.as_str(),
                revision_id.as_str(),
                fixture.listing_addr.as_str(),
                fixture.buyer_pubkey.as_str(),
                fixture.seller_pubkey.as_str(),
                revision_decision,
            );
            install_direct_relay_sync_transport(&fixture.runtime, &relay);
            fixture
                .runtime
                .refresh_shared_local_events()
                .expect("linked buyer revision events should import");
            assert!(
                fixture
                    .runtime
                    .open_personal_order_detail(fixture.order_id)
                    .expect("linked buyer order detail should open")
            );
            set_persisted_order_status(&fixture.runtime, fixture.order_id, "scheduled");

            assert!(
                fixture
                    .runtime
                    .publish_buyer_order_cancel(fixture.order_id)
                    .expect("linked buyer cancellation should publish from revision parent")
            );

            assert_eq!(relay.event_count(), 1);
            let cancellation_events =
                shared_order_events_by_kind(&fixture.paths, 3432, fixture.buyer_pubkey.as_str());
            assert_eq!(cancellation_events.len(), 1);
            let cancellation_event = cancellation_events
                .first()
                .expect("linked buyer cancellation event");
            assert!(event_has_tag(
                cancellation_event,
                "e_prev",
                revision_decision_event_id.as_str()
            ));

            cleanup_bootstrapped_runtime_paths(&fixture.paths);
        }
    }

    #[test]
    fn runtime_publishes_linked_buyer_receipt_from_selected_account_nostr_scope() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_receipt", true);
        let fulfillment_event_id = fixture
            .fulfillment_event_id
            .as_deref()
            .expect("ready fixture should include fulfillment event")
            .to_owned();
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked ready buyer order detail should open")
        );
        assert_eq!(
            fixture
                .runtime
                .summary()
                .personal_projection
                .orders
                .detail
                .as_ref()
                .expect("linked ready buyer detail")
                .status,
            BuyerOrderStatus::Ready
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_receipt(fixture.order_id, AppOrderReceiptOutcome::Received)
                .expect("linked buyer receipt should publish")
        );

        assert_eq!(
            persisted_order_status(&fixture.runtime, fixture.order_id),
            "completed"
        );
        assert_eq!(relay.event_count(), 1);
        let receipt_events =
            shared_order_events_by_kind(&fixture.paths, 3434, fixture.buyer_pubkey.as_str());
        assert_eq!(receipt_events.len(), 1);
        let receipt_event = receipt_events.first().expect("linked buyer receipt event");
        let receipt = radroots_sdk::trade::parse_buyer_receipt(receipt_event)
            .expect("linked buyer receipt should parse");
        assert_eq!(receipt.payload.order_id, fixture.trade_order_id);
        assert_eq!(receipt.payload.buyer_pubkey, fixture.buyer_pubkey);
        assert_eq!(receipt.payload.seller_pubkey, fixture.seller_pubkey);
        assert!(receipt.payload.received);
        assert!(event_has_tag(
            receipt_event,
            "e_root",
            fixture.request_event_id.as_str()
        ));
        assert!(event_has_tag(
            receipt_event,
            "e_prev",
            fulfillment_event_id.as_str()
        ));

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_publishes_linked_buyer_receipt_after_direct_delivered_fulfillment() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_receipt_delivered", false);
        let fulfillment_event_id = append_signed_order_fulfillment_record_with_status(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            RadrootsActiveTradeFulfillmentState::Delivered,
        );
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer delivered local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked delivered buyer order detail should open")
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_receipt(fixture.order_id, AppOrderReceiptOutcome::Received)
                .expect("linked delivered buyer receipt should publish")
        );

        assert_eq!(
            persisted_order_status(&fixture.runtime, fixture.order_id),
            "completed"
        );
        assert_eq!(relay.event_count(), 1);
        let receipt_events =
            shared_order_events_by_kind(&fixture.paths, 3434, fixture.buyer_pubkey.as_str());
        assert_eq!(receipt_events.len(), 1);
        let receipt_event = receipt_events.first().expect("linked buyer receipt event");
        let receipt = radroots_sdk::trade::parse_buyer_receipt(receipt_event)
            .expect("linked buyer delivered receipt should parse");
        assert!(receipt.payload.received);
        assert!(event_has_tag(
            receipt_event,
            "e_prev",
            fulfillment_event_id.as_str()
        ));

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_publishes_linked_buyer_issue_receipt_from_selected_account_nostr_scope() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_issue_receipt", true);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked ready buyer order detail should open")
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_receipt(
                    fixture.order_id,
                    AppOrderReceiptOutcome::issue("items need review")
                        .expect("issue receipt text should be accepted"),
                )
                .expect("linked buyer issue receipt should publish")
        );

        assert_eq!(
            persisted_order_status(&fixture.runtime, fixture.order_id),
            "needs_review"
        );
        assert_eq!(relay.event_count(), 1);
        let receipt_events =
            shared_order_events_by_kind(&fixture.paths, 3434, fixture.buyer_pubkey.as_str());
        assert_eq!(receipt_events.len(), 1);
        let receipt_event = receipt_events
            .first()
            .expect("linked buyer issue receipt event");
        let receipt = radroots_sdk::trade::parse_buyer_receipt(receipt_event)
            .expect("linked buyer issue receipt should parse");
        assert!(!receipt.payload.received);
        assert_eq!(receipt.payload.issue.as_deref(), Some("items need review"));
        let buyer_detail = fixture
            .runtime
            .summary()
            .personal_projection
            .orders
            .detail
            .as_ref()
            .expect("linked buyer issue receipt detail")
            .clone();
        assert_eq!(buyer_detail.status, BuyerOrderStatus::NeedsReview);
        let projected_receipt = buyer_detail
            .workflow
            .receipt
            .as_ref()
            .expect("receipt projection should be present");
        assert!(!projected_receipt.received);
        assert_eq!(
            projected_receipt.issue.as_deref(),
            Some("items need review")
        );

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_cancellation_with_reducer_invalid_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_cancel_invalid", false);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );
        append_signed_order_cancellation_record_with_prev(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-order-cancel-invalid-a",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
        );
        append_signed_order_cancellation_record_with_prev(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-order-cancel-invalid-b",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
        );

        let error = fixture
            .runtime
            .publish_buyer_order_cancel(fixture.order_id)
            .expect_err("linked buyer cancellation should reject reducer-invalid evidence");

        assert_order_lifecycle_evidence_invalid(error);
        assert_eq!(relay.event_count(), 0);
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_rejects_linked_buyer_receipt_with_reducer_invalid_fulfillment_evidence() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_receipt_invalid", true);
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );
        append_signed_order_fulfillment_record_with_status_and_key(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-order-receipt-forked-ready",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
            RadrootsActiveTradeFulfillmentState::ReadyForPickup,
        );
        let resolver_error = {
            let state = fixture.runtime.lock_state();
            let request = state
                .resolve_seller_order_request_evidence(fixture.order_id)
                .expect("linked buyer request evidence should resolve");
            state
                .resolve_order_lifecycle_evidence(&request)
                .expect_err("linked buyer receipt evidence should be reducer-invalid")
        };
        assert_order_lifecycle_evidence_invalid(resolver_error);

        let error = fixture
            .runtime
            .publish_buyer_order_receipt(fixture.order_id, AppOrderReceiptOutcome::Received)
            .expect_err("linked buyer receipt should reject reducer-invalid fulfillment evidence");

        assert!(
            matches!(error, AppSqliteError::InvalidProjection { .. }),
            "{error:?}"
        );
        assert_eq!(relay.event_count(), 0);
        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_publishes_linked_buyer_revision_decision_from_reducer_valid_parent() {
        let relay = ThreadedAckRelay::spawn();
        let fixture = linked_buyer_lifecycle_runtime("linked_buyer_order_revision", false);
        let proposal_event_id = append_signed_order_revision_proposal_record_with_prev(
            &fixture.paths,
            fixture.trade_order_id.as_str(),
            "linked-buyer-order-revision-proposal",
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            fixture.listing_addr.as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.seller_pubkey.as_str(),
        );
        install_direct_relay_sync_transport(&fixture.runtime, &relay);
        fixture
            .runtime
            .refresh_shared_local_events()
            .expect("linked buyer local events should import");
        assert!(
            fixture
                .runtime
                .open_personal_order_detail(fixture.order_id)
                .expect("linked buyer order detail should open")
        );

        assert!(
            fixture
                .runtime
                .publish_buyer_order_revision_accept(fixture.order_id)
                .expect("linked buyer revision decision should publish")
        );

        assert_eq!(relay.event_count(), 1);
        let revision_decision_events =
            shared_order_events_by_kind(&fixture.paths, 3425, fixture.buyer_pubkey.as_str());
        assert_eq!(revision_decision_events.len(), 1);
        let revision_decision_event = revision_decision_events
            .first()
            .expect("linked buyer revision decision event");
        let revision_decision =
            radroots_sdk::trade::parse_order_revision_decision(revision_decision_event)
                .expect("linked buyer revision decision should parse");
        assert_eq!(
            revision_decision.payload.decision,
            RadrootsTradeOrderRevisionDecision::Accepted
        );
        assert!(event_has_tag(
            revision_decision_event,
            "e_root",
            fixture.request_event_id.as_str()
        ));
        assert!(event_has_tag(
            revision_decision_event,
            "e_prev",
            proposal_event_id.as_str()
        ));

        cleanup_bootstrapped_runtime_paths(&fixture.paths);
    }

    #[test]
    fn runtime_repeat_personal_order_readds_only_currently_eligible_items() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let fulfillment_window_id = seed_buyer_marketplace_support(
            &runtime,
            account_id.as_str(),
            farm_id,
            "North field farm",
            "Friday pickup",
        );
        let available_product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(8),
            "2026-04-20T09:00:00Z",
        );
        let unavailable_product_id = seed_product(
            &runtime,
            farm_id,
            "Pea shoots",
            "Tray-grown",
            "published",
            Some(6),
            "2026-04-20T10:00:00Z",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&format!(
                "update products
                 set availability_window_id = '{fulfillment_window_id}'
                 where id in ('{available_product_id}', '{unavailable_product_id}')"
            ))
            .expect("buyer detail products should attach a fulfillment window");
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, available_product_id)
                .expect("available buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("available buyer product should add to cart")
        );
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, unavailable_product_id)
                .expect("unavailable buyer detail should open")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("second buyer product should add to cart")
        );
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                })
                .expect("buyer order review draft should save")
        );
        assert!(
            runtime
                .place_personal_order()
                .expect("buyer order should place")
        );
        let order_id = runtime.summary().personal_projection.orders.list.rows[0].order_id;

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute(
                "update products set status = 'archived' where id = ?1",
                [unavailable_product_id.to_string()],
            )
            .expect("product should archive");

        assert!(
            runtime
                .open_personal_order_detail(order_id)
                .expect("buyer order detail should reopen")
        );
        let detail_summary = runtime.summary();
        let repeat_demand = detail_summary
            .personal_projection
            .orders
            .detail
            .as_ref()
            .and_then(|detail| detail.repeat_demand.as_ref())
            .expect("repeat demand should derive from buyer order detail");
        assert_eq!(repeat_demand.eligibility.storage_key(), "partial");
        assert_eq!(repeat_demand.available_item_count, 1);
        assert_eq!(repeat_demand.unavailable_item_count, 1);

        assert!(
            runtime
                .repeat_personal_order(order_id, false)
                .expect("repeat demand should add available items to cart")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Cart)
        );
        assert_eq!(summary.personal_projection.cart.cart.lines.len(), 1);
        assert_eq!(
            summary.personal_projection.cart.cart.lines[0].product_id,
            available_product_id
        );
        assert_eq!(summary.personal_projection.cart.cart.lines[0].quantity, 1);
        assert!(
            summary
                .personal_projection
                .cart
                .cart
                .replace_confirmation
                .is_none()
        );
    }

    #[test]
    fn runtime_products_queries_refresh_the_repository_backed_projection() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");
        seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(2),
            "2026-04-18T10:00:00Z",
        );
        seed_product(
            &runtime,
            farm_id,
            "Pea shoots",
            "Tray-grown",
            "draft",
            None,
            "2026-04-18T09:00:00Z",
        );

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );

        let summary = runtime.summary();
        assert_eq!(summary.products_projection.list.summary.total_products, 2);
        assert_eq!(summary.products_projection.list.rows[0].title, "Salad mix");
        assert_eq!(
            summary.products_projection.query.filter,
            ProductsFilter::default()
        );
        assert_eq!(
            summary.products_projection.query.sort,
            ProductsSort::default()
        );

        assert!(
            runtime
                .select_products_filter(ProductsFilter::NeedAttention)
                .expect("filter should apply")
        );
        assert_eq!(runtime.summary().products_projection.list.rows.len(), 2);

        assert!(
            runtime
                .set_products_search_query("pea")
                .expect("search should apply")
        );
        let searched = runtime.summary();
        assert_eq!(searched.products_projection.list.rows.len(), 1);
        assert_eq!(
            searched.products_projection.list.rows[0].title,
            "Pea shoots"
        );

        assert!(
            runtime
                .select_products_sort(ProductsSort::Name)
                .expect("sort should apply")
        );
        assert_eq!(
            runtime.summary().products_projection.query.sort,
            ProductsSort::Name
        );
    }

    #[test]
    fn runtime_open_products_filter_routes_today_follow_ons_into_products() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );

        assert!(
            runtime
                .open_products_filter(ProductsFilter::Drafts)
                .expect("products follow-on should route")
        );
        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Products)
        );
        assert_eq!(
            summary.products_projection.query.filter,
            ProductsFilter::Drafts
        );
    }

    #[test]
    fn runtime_opens_orders_detail_and_pack_day_through_shared_farmer_routing() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, order_id) = seed_order_workspace(&runtime, farm_id);

        assert_eq!(
            runtime.summary().orders_projection.query.filter,
            OrdersFilter::NeedsAction
        );

        assert!(runtime.open_orders().expect("orders should open"));
        let orders_summary = runtime.summary();
        assert_eq!(
            orders_summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(orders_summary.orders_projection.list.rows.len(), 1);
        assert_eq!(
            orders_summary.orders_projection.list.rows[0].order_id,
            order_id
        );
        assert!(orders_summary.orders_projection.detail.is_none());

        assert!(
            runtime
                .open_order_detail(order_id)
                .expect("order detail should open")
        );
        let detail_summary = runtime.summary();
        assert_eq!(
            detail_summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(
            detail_summary
                .orders_projection
                .detail
                .as_ref()
                .expect("order detail")
                .order_id,
            order_id
        );

        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        let pack_day_summary = runtime.summary();
        assert_eq!(
            pack_day_summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::PackDay)
        );
        assert_eq!(
            pack_day_summary
                .pack_day_projection
                .query
                .fulfillment_window_id,
            None
        );
        assert_eq!(
            pack_day_summary
                .pack_day_projection
                .projection
                .fulfillment_window
                .as_ref()
                .expect("pack day fulfillment window")
                .fulfillment_window_id,
            fulfillment_window_id
        );
    }

    #[test]
    fn runtime_export_pack_day_requires_a_current_window_context() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_export_requires_context");
        let (_, _farm_id) = provision_ready_farmer_account(&runtime);

        assert!(
            !runtime
                .export_pack_day()
                .expect("missing pack day context should no-op")
        );
        assert_eq!(
            runtime.summary().pack_day_projection.export.status,
            PackDayExportStatus::Idle
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_export_pack_day_uses_repository_source_truth_and_writes_bundle() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_export_bundle");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, order_id) = seed_order_workspace(&runtime, farm_id);

        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        let fulfillment_window = runtime
            .summary()
            .pack_day_projection
            .projection
            .fulfillment_window
            .clone()
            .expect("pack day fulfillment window");
        let _ = runtime.lock_state_mut().state_store.apply_in_memory(
            AppStateCommand::replace_pack_day_projection(PackDayProjection {
                fulfillment_window: Some(fulfillment_window.clone()),
                reminders: ReminderFeedProjection::default(),
                totals_by_product: vec![PackDayProductTotalRow {
                    title: "Bogus totals".to_owned(),
                    quantity_display: "999 crates".to_owned(),
                }],
                pack_list: vec![PackDayPackListRow {
                    title: "Bogus pack list".to_owned(),
                    quantity_display: "Do not trust screen strings".to_owned(),
                }],
                pickup_roster: vec![PackDayRosterRow {
                    order_id: OrderId::new(),
                    order_number: "R-999".to_owned(),
                    customer_display_name: "Bogus".to_owned(),
                }],
            }),
        );

        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let summary = runtime.summary();
        let export = &summary.pack_day_projection.export;
        assert_eq!(export.status, PackDayExportStatus::Succeeded);
        assert_eq!(
            export
                .request
                .as_ref()
                .expect("export request")
                .fulfillment_window_id,
            fulfillment_window_id
        );
        let bundle = export.bundle.as_ref().expect("export bundle");
        assert_eq!(bundle.fulfillment_window_id, fulfillment_window_id);
        assert_eq!(bundle.artifact_count(), 3);

        let pack_sheet_path = PathBuf::from(&bundle.bundle_directory).join("pack_sheet.txt");
        let pickup_roster_path = PathBuf::from(&bundle.bundle_directory).join("pickup_roster.txt");
        let customer_labels_path =
            PathBuf::from(&bundle.bundle_directory).join("customer_labels.txt");

        let pack_sheet = fs::read_to_string(&pack_sheet_path).expect("pack sheet should exist");
        let pickup_roster =
            fs::read_to_string(&pickup_roster_path).expect("pickup roster should exist");
        let customer_labels =
            fs::read_to_string(&customer_labels_path).expect("customer labels should exist");

        assert!(pack_sheet.contains("Farm: North field farm"));
        assert!(pack_sheet.contains("Casey | R-100 | needs_action | Salad mix | 2 bags"));
        assert!(!pack_sheet.contains("Bogus"));
        assert!(pickup_roster.contains("Casey | R-100 | needs_action"));
        assert!(customer_labels.contains("North field farm"));
        assert!(customer_labels.contains("Casey"));
        assert!(customer_labels.contains("Order: R-100"));
        assert!(!customer_labels.contains("Bogus"));
        assert!(!pickup_roster.contains(&order_id.to_string()));

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_bootstrap_sweeps_prepared_pack_day_print_assets() {
        let paths = temp_desktop_runtime_paths("pack_day_print_bootstrap_sweep");
        let stale_root = prepared_customer_label_asset_root();
        let stale_directory = stale_root.join(PackDayExportInstanceId::new().to_string());
        let _ = fs::remove_file(&stale_root);
        let _ = fs::remove_dir_all(&stale_root);
        fs::create_dir_all(&stale_directory).expect("stale prepared directory should create");
        fs::write(stale_directory.join("stale.ps"), "stale").expect("stale asset should write");

        let _ = restart_runtime(paths.clone());

        assert!(!stale_root.exists());

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_bootstrap_keeps_running_when_prepared_pack_day_print_root_sweep_fails() {
        let paths = temp_desktop_runtime_paths("pack_day_print_bootstrap_best_effort");
        let stale_root = prepared_customer_label_asset_root();
        let _ = fs::remove_file(&stale_root);
        let _ = fs::remove_dir_all(&stale_root);
        if let Some(parent) = stale_root.parent() {
            fs::create_dir_all(parent).expect("prepared asset root parent should create");
        }
        fs::write(&stale_root, "blocked").expect("prepared asset root blocker should write");

        let _ = restart_runtime(paths.clone());

        assert!(stale_root.is_file());

        let _ = fs::remove_file(&stale_root);
        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_prepare_pack_day_host_handoff_uses_the_current_export_bundle_for_file_actions() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_host_handoff_prepare");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        for (kind, suffix) in [
            (PackDayHostHandoffKind::OpenPackSheet, "pack_sheet.txt"),
            (
                PackDayHostHandoffKind::OpenPickupRoster,
                "pickup_roster.txt",
            ),
            (
                PackDayHostHandoffKind::OpenCustomerLabels,
                "customer_labels.txt",
            ),
        ] {
            let prepared = runtime
                .prepare_pack_day_host_handoff(kind)
                .expect("host handoff should prepare")
                .expect("host handoff should produce a plan");

            let summary = runtime.summary();
            assert_eq!(
                summary.pack_day_projection.host_handoff.status,
                PackDayHostHandoffStatus::Running
            );
            assert_eq!(
                summary.pack_day_projection.host_handoff.request,
                Some(prepared.0.clone())
            );
            assert_eq!(prepared.0.kind, kind);
            assert_eq!(
                prepared.0.bundle_directory,
                summary
                    .pack_day_projection
                    .export
                    .bundle
                    .as_ref()
                    .expect("pack day export bundle")
                    .bundle_directory
            );
            assert_eq!(prepared.1.kind, kind);
            assert!(prepared.1.target_path.ends_with(suffix));

            assert!(
                runtime
                    .finish_pack_day_host_handoff(prepared.0, Ok(()))
                    .expect("host handoff success should apply")
            );
        }

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_host_handoff_records_failures_in_state() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_host_handoff_failure");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, _) = runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff should prepare")
            .expect("host handoff should produce a plan");

        let error = runtime
            .finish_pack_day_host_handoff(
                request.clone(),
                Err(PackDayHostHandoffError::UnsupportedPlatform),
            )
            .expect_err("host handoff failure should surface");
        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::PackDayHostHandoff(
                PackDayHostHandoffError::UnsupportedPlatform
            )
        ));

        let summary = runtime.summary();
        assert_eq!(
            summary.pack_day_projection.host_handoff.status,
            PackDayHostHandoffStatus::Failed
        );
        assert_eq!(
            summary.pack_day_projection.host_handoff.request,
            Some(request)
        );
        assert_eq!(
            summary
                .pack_day_projection
                .host_handoff
                .error_message
                .as_deref(),
            Some("pack day host handoff is only supported on macos")
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_host_handoff_ignores_stale_background_completion() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_host_handoff_stale");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, _) = runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff should prepare")
            .expect("host handoff should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_host_handoff());

        assert!(
            !runtime
                .finish_pack_day_host_handoff(request, Ok(()))
                .expect("stale completion should no-op")
        );
        assert_eq!(
            runtime.summary().pack_day_projection.host_handoff.status,
            PackDayHostHandoffStatus::Idle
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_prepare_pack_day_print_uses_the_current_export_bundle_for_all_v1_documents() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_prepare");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        for (kind, expected_exported_suffix) in [
            (PackDayPrintKind::PrintPackSheet, Some("pack_sheet.txt")),
            (
                PackDayPrintKind::PrintPickupRoster,
                Some("pickup_roster.txt"),
            ),
            (PackDayPrintKind::PrintCustomerLabels, None),
        ] {
            let prepared = runtime
                .prepare_pack_day_print(kind)
                .expect("print should prepare")
                .expect("print should produce a plan");

            let summary = runtime.summary();
            assert_eq!(
                summary.pack_day_projection.print.status,
                PackDayPrintStatus::Running
            );
            assert_eq!(
                summary.pack_day_projection.print.request,
                Some(prepared.0.clone())
            );
            assert_eq!(prepared.0.kind, kind);
            assert_eq!(
                prepared.0.export_instance_id,
                summary
                    .pack_day_projection
                    .export
                    .bundle
                    .as_ref()
                    .expect("pack day export bundle")
                    .export_instance_id
            );
            assert_eq!(prepared.0.label_stock, kind.label_stock());
            assert_eq!(prepared.1.kind, kind);
            assert_eq!(prepared.1.command_program, "lp");
            match expected_exported_suffix {
                Some(suffix) => {
                    assert!(prepared.1.target_path.ends_with(suffix));
                    assert_eq!(
                        prepared.1.command_args,
                        vec![prepared.1.target_path.to_string_lossy().into_owned()]
                    );
                }
                None => {
                    let export_bundle = summary
                        .pack_day_projection
                        .export
                        .bundle
                        .as_ref()
                        .expect("pack day export bundle");
                    assert!(
                        prepared
                            .1
                            .target_path
                            .ends_with("customer_labels_avery_5160_letter_30_up.ps")
                    );
                    assert!(
                        !prepared
                            .1
                            .target_path
                            .starts_with(PathBuf::from(&export_bundle.bundle_directory))
                    );
                    assert!(
                        prepared
                            .1
                            .target_path
                            .to_string_lossy()
                            .contains(export_bundle.export_instance_id.to_string().as_str())
                    );
                    assert_eq!(
                        prepared.1.command_args,
                        vec![
                            "-o".to_owned(),
                            "media=Letter".to_owned(),
                            prepared.1.target_path.to_string_lossy().into_owned()
                        ]
                    );
                }
            }

            assert!(
                runtime
                    .finish_pack_day_print(prepared.0, Ok(()))
                    .expect("print success should apply")
            );

            if let PackDayPrintKind::PrintCustomerLabels = kind {
                if let Some(parent) = prepared.1.target_path.parent() {
                    let _ = fs::remove_dir_all(parent);
                }
            }
        }

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_prepare_pack_day_batch_print_uses_the_current_export_bundle_for_all_v1_documents() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_print_prepare");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, plan) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");

        let summary = runtime.summary();
        let bundle = summary
            .pack_day_projection
            .export
            .bundle
            .as_ref()
            .expect("pack day export bundle");
        assert_eq!(
            summary.pack_day_projection.batch_print.status,
            PackDayBatchPrintStatus::Running
        );
        assert_eq!(
            summary.pack_day_projection.batch_print.request,
            Some(request.clone())
        );
        assert_eq!(request.export_instance_id, bundle.export_instance_id);
        assert_eq!(
            request.artifacts,
            Vec::from(PackDayBatchPrintArtifact::all_v1())
        );
        assert_eq!(plan.export_instance_id, bundle.export_instance_id);
        assert_eq!(
            plan.plans
                .iter()
                .map(|plan| PackDayBatchPrintArtifact::from_print_kind(plan.kind))
                .collect::<Vec<_>>(),
            request.artifacts.clone()
        );
        assert!(plan.plans.iter().all(|plan| plan.command_program == "lp"));

        assert!(
            runtime
                .finish_pack_day_batch_print(request, Ok(()))
                .expect("batch print success should apply")
        );
        assert_eq!(
            runtime.summary().pack_day_projection.batch_print.status,
            PackDayBatchPrintStatus::Succeeded
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_pack_day_batch_print_blocks_conflicting_pack_day_actions() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_print_conflicts");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (_, _) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");
        assert!(
            runtime
                .prepare_pack_day_print(PackDayPrintKind::PrintPackSheet)
                .expect("print prepare should not fail")
                .is_none()
        );
        assert!(
            runtime
                .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
                .expect("host handoff prepare should not fail")
                .is_none()
        );

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_batch_print());

        let (print_request, _) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPackSheet)
            .expect("print should prepare")
            .expect("print should produce a plan");
        assert!(
            runtime
                .prepare_pack_day_batch_print()
                .expect("batch print prepare should not fail")
                .is_none()
        );
        assert!(
            runtime
                .finish_pack_day_print(print_request, Ok(()))
                .expect("print success should apply")
        );
        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        let (_, _) = runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff should prepare")
            .expect("host handoff should produce a plan");
        assert!(
            runtime
                .prepare_pack_day_batch_print()
                .expect("batch print prepare should not fail")
                .is_none()
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_batch_print_records_failures_and_cleans_prepared_assets() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_print_failure_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, plan) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");
        let prepared_directory = plan
            .plans
            .iter()
            .find(|plan| plan.kind == PackDayPrintKind::PrintCustomerLabels)
            .and_then(|plan| plan.target_path.parent())
            .expect("prepared customer labels parent")
            .to_path_buf();
        assert!(prepared_directory.is_dir());

        let failed_artifact =
            PackDayBatchPrintArtifact::from_print_kind(PackDayPrintKind::PrintPickupRoster);
        let error = runtime
            .finish_pack_day_batch_print(
                request.clone(),
                Err(PackDayBatchPrintError::QueueExit {
                    submitted_artifacts: vec![PackDayBatchPrintArtifact::from_print_kind(
                        PackDayPrintKind::PrintPackSheet,
                    )],
                    failed_artifact,
                    source: PackDayPrintError::UnsupportedPlatform,
                }),
            )
            .expect_err("batch print failure should surface");
        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::PackDayBatchPrint(
                PackDayBatchPrintError::QueueExit { .. }
            )
        ));
        assert!(!prepared_directory.exists());

        let summary = runtime.summary();
        let batch_print = &summary.pack_day_projection.batch_print;
        assert_eq!(batch_print.status, PackDayBatchPrintStatus::Failed);
        assert_eq!(batch_print.request, Some(request));
        assert_eq!(batch_print.failed_artifact, Some(failed_artifact));
        assert_eq!(
            batch_print.failure,
            Some(PackDayBatchPrintFailureKind::QueueExit)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_batch_print_ignores_stale_background_completion() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_print_stale");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, _) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_batch_print());

        assert!(
            !runtime
                .finish_pack_day_batch_print(request, Ok(()))
                .expect("stale completion should no-op")
        );
        assert_eq!(
            runtime.summary().pack_day_projection.batch_print.status,
            PackDayBatchPrintStatus::Idle
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn pack_day_batch_workflow_success_submits_frozen_v1_and_records_success() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_workflow_success");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, plan) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");
        let mut submitted = Vec::new();

        execute_pack_day_batch_print_plan_with(&plan, |print_plan| {
            submitted.push(PackDayBatchPrintArtifact::from_print_kind(print_plan.kind));
            Ok(PackDayPrintCommandResult::succeeded())
        })
        .expect("batch print execution should succeed");

        assert_eq!(submitted, Vec::from(PackDayBatchPrintArtifact::all_v1()));
        assert!(
            runtime
                .finish_pack_day_batch_print(request.clone(), Ok(()))
                .expect("batch print success should apply")
        );

        let summary = runtime.summary();
        let batch_print = &summary.pack_day_projection.batch_print;
        assert_eq!(batch_print.status, PackDayBatchPrintStatus::Succeeded);
        assert_eq!(batch_print.request, Some(request));
        assert_eq!(batch_print.failed_artifact, None);
        assert_eq!(batch_print.failure, None);

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn pack_day_batch_workflow_queue_failure_records_failed_artifact_state() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_workflow_queue_failure");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, plan) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");
        let mut submitted = Vec::new();

        let execution_error = execute_pack_day_batch_print_plan_with(&plan, |print_plan| {
            submitted.push(PackDayBatchPrintArtifact::from_print_kind(print_plan.kind));
            match print_plan.kind {
                PackDayPrintKind::PrintPackSheet => Ok(PackDayPrintCommandResult::succeeded()),
                PackDayPrintKind::PrintPickupRoster => Ok(PackDayPrintCommandResult::failed(
                    Some(2),
                    "lp stopped before submit",
                )),
                PackDayPrintKind::PrintCustomerLabels => {
                    panic!("batch should stop before customer labels")
                }
            }
        })
        .expect_err("batch print execution should fail");

        assert_eq!(
            submitted,
            vec![
                PackDayBatchPrintArtifact::from_print_kind(PackDayPrintKind::PrintPackSheet),
                PackDayBatchPrintArtifact::from_print_kind(PackDayPrintKind::PrintPickupRoster),
            ]
        );
        let failed_artifact =
            PackDayBatchPrintArtifact::from_print_kind(PackDayPrintKind::PrintPickupRoster);
        let runtime_error = runtime
            .finish_pack_day_batch_print(request.clone(), Err(execution_error))
            .expect_err("batch print failure should surface");
        assert!(matches!(
            runtime_error,
            DesktopAppRuntimeCommandError::PackDayBatchPrint(
                PackDayBatchPrintError::QueueExit { .. }
            )
        ));

        let summary = runtime.summary();
        let batch_print = &summary.pack_day_projection.batch_print;
        assert_eq!(batch_print.status, PackDayBatchPrintStatus::Failed);
        assert_eq!(batch_print.request, Some(request));
        assert_eq!(batch_print.failed_artifact, Some(failed_artifact));
        assert_eq!(
            batch_print.failure,
            Some(PackDayBatchPrintFailureKind::QueueExit)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_print_records_failures_in_state() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_failure");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, _) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPackSheet)
            .expect("print should prepare")
            .expect("print should produce a plan");

        let error = runtime
            .finish_pack_day_print(request.clone(), Err(PackDayPrintError::UnsupportedPlatform))
            .expect_err("print failure should surface");
        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::PackDayPrint(PackDayPrintError::UnsupportedPlatform)
        ));

        let summary = runtime.summary();
        assert_eq!(
            summary.pack_day_projection.print.status,
            PackDayPrintStatus::Failed
        );
        assert_eq!(summary.pack_day_projection.print.request, Some(request));
        assert_eq!(summary.pack_day_projection.print.failure, None);

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_prepare_pack_day_print_surfaces_customer_label_overflow_as_a_typed_failure() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_overflow_failure");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let bundle = runtime
            .summary()
            .pack_day_projection
            .export
            .bundle
            .clone()
            .expect("pack day export bundle");
        let customer_labels_path =
            PathBuf::from(&bundle.bundle_directory).join("customer_labels.txt");
        fs::write(
            &customer_labels_path,
            "Willow farm\nCasey\nOrder R-1001\nPickup barn\nThursday\nKeep cold\nOverflow note\n",
        )
        .expect("overflowing customer labels should write");

        let error = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintCustomerLabels)
            .expect_err("overflowing customer labels should fail");
        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::PackDayPrint(
                PackDayPrintError::CustomerLabelsAvery5160Overflow
            )
        ));

        let summary = runtime.summary();
        let print = &summary.pack_day_projection.print;
        assert_eq!(print.status, PackDayPrintStatus::Failed);
        assert_eq!(
            print.request.as_ref().map(|request| request.kind),
            Some(PackDayPrintKind::PrintCustomerLabels)
        );
        assert_eq!(
            print
                .request
                .as_ref()
                .map(|request| request.export_instance_id),
            Some(bundle.export_instance_id)
        );
        assert_eq!(
            print.failure,
            Some(PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_print_cleans_customer_label_assets_and_keeps_cleanup_failures_best_effort()
     {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (success_request, success_plan) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintCustomerLabels)
            .expect("customer labels should prepare")
            .expect("customer labels plan should exist");
        let success_directory = success_plan
            .target_path
            .parent()
            .expect("prepared asset parent")
            .to_path_buf();
        assert!(success_directory.is_dir());

        assert!(
            runtime
                .finish_pack_day_print(success_request, Ok(()))
                .expect("print success should apply")
        );
        assert!(!success_directory.exists());

        let (failure_request, failure_plan) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintCustomerLabels)
            .expect("customer labels should prepare again")
            .expect("customer labels plan should exist again");
        let failure_directory = failure_plan
            .target_path
            .parent()
            .expect("prepared asset parent")
            .to_path_buf();
        fs::remove_file(&failure_plan.target_path).expect("prepared asset should remove");
        fs::remove_dir_all(&failure_directory).expect("prepared asset directory should remove");
        fs::write(&failure_directory, "blocked").expect("cleanup blocker should write");

        let error = runtime
            .finish_pack_day_print(
                failure_request.clone(),
                Err(PackDayPrintError::UnsupportedPlatform),
            )
            .expect_err("print failure should surface");
        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::PackDayPrint(PackDayPrintError::UnsupportedPlatform)
        ));
        assert!(failure_directory.is_file());

        let summary = runtime.summary();
        assert_eq!(
            summary.pack_day_projection.print.status,
            PackDayPrintStatus::Failed
        );
        assert_eq!(
            summary.pack_day_projection.print.request,
            Some(failure_request)
        );

        let _ = fs::remove_file(&failure_directory);
        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_reexport_pack_day_cleans_previous_customer_label_prepared_assets() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_reexport_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("initial pack day export should succeed")
        );
        let first_bundle = runtime
            .summary()
            .pack_day_projection
            .export
            .bundle
            .clone()
            .expect("initial export bundle");

        let (_request, plan) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintCustomerLabels)
            .expect("customer labels should prepare")
            .expect("customer labels plan should exist");
        let prepared_directory = plan
            .target_path
            .parent()
            .expect("prepared asset parent")
            .to_path_buf();
        assert!(prepared_directory.is_dir());
        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        assert!(
            runtime
                .export_pack_day()
                .expect("replacement pack day export should succeed")
        );

        let summary = runtime.summary();
        let replacement_bundle = summary
            .pack_day_projection
            .export
            .bundle
            .as_ref()
            .expect("replacement export bundle");
        assert_ne!(
            replacement_bundle.export_instance_id,
            first_bundle.export_instance_id
        );
        assert!(!prepared_directory.exists());

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_pack_day_window_change_cleans_previous_customer_label_prepared_assets() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_window_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, _) = seed_order_workspace(&runtime, farm_id);
        let (other_fulfillment_window_id, _) =
            seed_second_order_workspace(&runtime, farm_id, fulfillment_window_id);

        assert!(
            runtime
                .open_pack_day(Some(fulfillment_window_id))
                .expect("first pack day window should open")
        );
        assert!(
            runtime
                .export_pack_day()
                .expect("initial pack day export should succeed")
        );

        let (_request, plan) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintCustomerLabels)
            .expect("customer labels should prepare")
            .expect("customer labels plan should exist");
        let prepared_directory = plan
            .target_path
            .parent()
            .expect("prepared asset parent")
            .to_path_buf();
        assert!(prepared_directory.is_dir());
        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        assert!(
            runtime
                .open_pack_day(Some(other_fulfillment_window_id))
                .expect("second pack day window should open")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.pack_day_projection.query.fulfillment_window_id,
            Some(other_fulfillment_window_id)
        );
        assert_eq!(
            summary.pack_day_projection.export.status,
            PackDayExportStatus::Idle
        );
        assert!(!prepared_directory.exists());

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_print_ignores_stale_background_completion() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_stale");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(
            runtime
                .export_pack_day()
                .expect("pack day export should succeed")
        );

        let (request, _) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPickupRoster)
            .expect("print should prepare")
            .expect("print should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        assert!(
            !runtime
                .finish_pack_day_print(request, Ok(()))
                .expect("stale completion should no-op")
        );
        assert_eq!(
            runtime.summary().pack_day_projection.print.status,
            PackDayPrintStatus::Idle
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_threads_canonical_seller_reminders_across_today_orders_and_pack_day() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        seed_order_workspace(&runtime, farm_id);

        assert!(runtime.open_orders().expect("orders should open"));
        let summary = runtime.summary();

        assert_eq!(summary.today_projection.reminders.items.len(), 1);
        assert_eq!(
            summary.today_projection.reminders.items[0].kind,
            ReminderKind::FulfillmentWindow
        );
        assert_eq!(summary.orders_projection.reminders.items.len(), 1);
        assert_eq!(
            summary.orders_projection.reminders.items[0].kind,
            ReminderKind::OrderAction
        );
        assert_eq!(
            summary.pack_day_projection.projection.reminders.items.len(),
            1
        );
        assert_eq!(
            summary.pack_day_projection.projection.reminders.items[0].kind,
            ReminderKind::FulfillmentWindow
        );
        assert_eq!(
            summary
                .today_projection
                .summary
                .as_ref()
                .expect("today summary")
                .recovery_actions_open,
            0
        );
    }

    #[test]
    fn runtime_sync_refresh_threads_sync_reminders_into_orders_projection() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, order_id) = seed_order_workspace(&runtime, farm_id);

        assert!(runtime.open_orders().expect("orders should open"));
        assert!(
            runtime
                .lock_state_mut()
                .enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                    SyncAggregateRef::Order(order_id),
                    json!({
                        "aggregate_kind": "order",
                        "order_id": order_id.to_string(),
                        "source": "test_pending_order_sync",
                    })
                    .to_string(),
                )])
                .expect("pending order sync should enqueue")
        );
        let summary = runtime.summary();

        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert!(
            summary
                .orders_projection
                .reminders
                .items
                .iter()
                .any(|item| item.kind == ReminderKind::SyncImpact
                    && item.title == "Pending local changes")
        );
    }

    #[test]
    fn runtime_refresh_promotes_blocking_sync_reminders_into_presented_log_entries() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .record_sync_conflict(
                account_id.as_str(),
                &SyncConflict {
                    aggregate: SyncAggregateRef::Farm(farm_id),
                    kind: SyncConflictKind::RevisionMismatch,
                    severity: SyncConflictSeverity::Blocking,
                    resolution: SyncConflictResolutionStatus::Unresolved,
                    local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                    remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                    detected_at: "2026-04-20T20:10:00Z".to_owned(),
                    resolved_at: None,
                },
            )
            .expect("blocking conflict should save");

        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        let summary = runtime.summary();
        let reminder = summary
            .orders_projection
            .reminders
            .items
            .iter()
            .find(|item| item.kind == ReminderKind::SyncImpact)
            .expect("sync reminder");

        assert_eq!(reminder.delivery_state, ReminderDeliveryState::Presented);
        assert!(summary.reminder_log.entries.iter().any(|entry| {
            entry.reminder_id == reminder.reminder_id
                && entry.delivery_state == ReminderDeliveryState::Presented
        }));
    }

    #[test]
    fn runtime_resolving_an_acknowledged_reminder_records_the_resolved_log_entry() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        let conflict_id = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .record_sync_conflict(
                account_id.as_str(),
                &SyncConflict {
                    aggregate: SyncAggregateRef::Farm(farm_id),
                    kind: SyncConflictKind::RevisionMismatch,
                    severity: SyncConflictSeverity::Blocking,
                    resolution: SyncConflictResolutionStatus::Unresolved,
                    local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                    remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                    detected_at: "2026-04-20T20:15:00Z".to_owned(),
                    resolved_at: None,
                },
            )
            .expect("blocking conflict should save");
        assert!(
            runtime
                .lock_state_mut()
                .refresh_selected_account_sync()
                .expect("sync status should refresh")
        );

        let reminder_id = runtime
            .summary()
            .orders_projection
            .reminders
            .items
            .iter()
            .find(|item| item.kind == ReminderKind::SyncImpact)
            .expect("sync reminder")
            .reminder_id;
        assert!(
            runtime
                .acknowledge_reminder(reminder_id)
                .expect("reminder should acknowledge")
        );

        let acknowledged_summary = runtime.summary();
        assert!(
            acknowledged_summary
                .orders_projection
                .reminders
                .items
                .iter()
                .any(|item| {
                    item.reminder_id == reminder_id
                        && item.delivery_state == ReminderDeliveryState::Acknowledged
                })
        );
        assert!(
            acknowledged_summary
                .reminder_log
                .entries
                .iter()
                .any(|entry| {
                    entry.reminder_id == reminder_id
                        && entry.delivery_state == ReminderDeliveryState::Acknowledged
                })
        );

        assert!(
            runtime
                .resolve_sync_conflict(
                    conflict_id.as_str(),
                    SyncConflictResolutionStatus::AcceptedLocal,
                )
                .expect("conflict resolution should succeed")
        );

        let resolved_summary = runtime.summary();
        assert!(
            resolved_summary
                .orders_projection
                .reminders
                .items
                .iter()
                .all(|item| { item.reminder_id != reminder_id })
        );
        assert!(resolved_summary.reminder_log.entries.iter().any(|entry| {
            entry.reminder_id == reminder_id
                && entry.delivery_state == ReminderDeliveryState::Resolved
        }));
    }

    #[test]
    fn runtime_threads_recovery_queue_into_today_counts_and_order_detail() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, order_id) = seed_order_workspace(&runtime, farm_id);
        let recovery_record_id = RecoveryRecordId::new();
        let sql = format!(
            "insert into order_recovery_records (
                recovery_record_id,
                account_id,
                farm_id,
                order_id,
                recovery_kind,
                recovery_state,
                summary,
                note,
                last_updated_at
             ) values (
                '{recovery_record_id}',
                '{}',
                '{farm_id}',
                '{order_id}',
                'missed_pickup',
                'open',
                'Follow up on the missed pickup',
                'Confirm a new pickup time.',
                '2026-04-18T18:30:00Z'
             )",
            runtime
                .summary()
                .settings_account_projection
                .selected_account
                .as_ref()
                .expect("selected account")
                .account
                .account_id
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("recovery record should seed");

        assert!(
            runtime
                .open_order_detail(order_id)
                .expect("order detail should open")
        );
        let summary = runtime.summary();

        assert_eq!(summary.orders_projection.recovery_queue.items.len(), 1);
        assert_eq!(
            summary
                .today_projection
                .summary
                .as_ref()
                .expect("today summary")
                .recovery_actions_open,
            1
        );
        assert_eq!(
            summary
                .orders_projection
                .detail
                .as_ref()
                .and_then(|detail| detail.recoveries.first())
                .expect("order recovery")
                .kind,
            RecoveryKind::MissedPickup
        );
    }

    #[test]
    fn reminder_urgency_marks_due_soon_and_overdue_deadlines() {
        let due_soon = (Utc::now() + Duration::hours(24))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let overdue = (Utc::now() - Duration::hours(2))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        assert_eq!(
            super::reminder_urgency(due_soon.as_str()),
            super::ReminderUrgency::DueSoon
        );
        assert_eq!(
            super::reminder_urgency(overdue.as_str()),
            super::ReminderUrgency::Overdue
        );
    }

    #[test]
    fn runtime_open_orders_resets_to_default_queue_and_clears_detail() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, order_id) = seed_order_workspace(&runtime, farm_id);

        assert!(
            runtime
                .select_orders_filter(OrdersFilter::Packed)
                .expect("orders filter should update")
        );
        assert!(
            runtime
                .open_order_detail(order_id)
                .expect("order detail should open")
        );

        assert!(runtime.open_orders().expect("orders should reopen"));
        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(
            summary.orders_projection.query.filter,
            OrdersFilter::NeedsAction
        );
        assert_eq!(summary.orders_projection.list.rows.len(), 1);
        assert!(summary.orders_projection.detail.is_none());
    }

    #[test]
    fn runtime_open_orders_fulfillment_window_filters_the_queue_to_one_window() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, order_id) = seed_order_workspace(&runtime, farm_id);
        let other_fulfillment_window_id = FulfillmentWindowId::new();
        let other_order_id = OrderId::new();
        let sql = format!(
            "insert into fulfillment_windows (
                id,
                farm_id,
                starts_at,
                ends_at,
                capacity_limit,
                created_at,
                updated_at,
                pickup_location_id,
                label,
                order_cutoff_at
             )
             select
                '{other_fulfillment_window_id}',
                farm_id,
                '2099-04-19T16:00:00Z',
                '2099-04-19T18:00:00Z',
                capacity_limit,
                '2099-04-19T16:00:00Z',
                '2099-04-19T16:00:00Z',
                pickup_location_id,
                'Saturday pickup',
                '2099-04-18T18:00:00Z'
             from fulfillment_windows
             where id = '{fulfillment_window_id}' and farm_id = '{farm_id}';
             insert into orders (
                id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                updated_at
             ) values (
                '{other_order_id}',
                '{farm_id}',
                '{other_fulfillment_window_id}',
                'R-101',
                'Robin',
                'scheduled',
                '2026-04-17T11:00:00Z'
             )"
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("second orders workspace should seed");

        assert!(
            runtime
                .open_orders_fulfillment_window(fulfillment_window_id)
                .expect("orders window follow-on should route")
        );
        let summary = runtime.summary();

        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Orders)
        );
        assert_eq!(summary.orders_projection.query.filter, OrdersFilter::All);
        assert_eq!(
            summary.orders_projection.query.fulfillment_window_id,
            Some(fulfillment_window_id)
        );
        assert_eq!(summary.orders_projection.list.rows.len(), 1);
        assert_eq!(summary.orders_projection.list.rows[0].order_id, order_id);
        assert!(summary.orders_projection.detail.is_none());
    }

    #[test]
    fn runtime_order_filters_refresh_repository_backed_orders_projection() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (fulfillment_window_id, scheduled_order_id) = seed_order_workspace(&runtime, farm_id);
        let packed_order_id = OrderId::new();
        let completed_order_id = OrderId::new();

        let sql = format!(
            "update orders
             set status = 'scheduled', updated_at = '2026-04-17T12:00:00Z'
             where id = '{scheduled_order_id}' and farm_id = '{farm_id}';
             insert into orders (
                id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                updated_at
             ) values (
                '{packed_order_id}',
                '{farm_id}',
                '{fulfillment_window_id}',
                'R-101',
                'Taylor',
                'packed',
                '2026-04-17T12:30:00Z'
             );
             insert into orders (
                id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                updated_at
             ) values (
                '{completed_order_id}',
                '{farm_id}',
                '{fulfillment_window_id}',
                'R-102',
                'Morgan',
                'completed',
                '2026-04-17T13:00:00Z'
             )"
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("order should update to scheduled");

        assert!(
            runtime
                .select_orders_filter(OrdersFilter::Scheduled)
                .expect("scheduled filter should apply")
        );
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Scheduled
        );

        assert!(
            runtime
                .open_order_detail(scheduled_order_id)
                .expect("order detail should open")
        );
        let scheduled_detail_summary = runtime.summary();
        assert_eq!(
            scheduled_detail_summary
                .orders_projection
                .detail
                .as_ref()
                .expect("scheduled detail")
                .status,
            OrderStatus::Scheduled
        );
        assert_eq!(
            scheduled_detail_summary
                .orders_projection
                .list
                .summary
                .scheduled_orders,
            1
        );
        assert_eq!(
            scheduled_detail_summary
                .orders_projection
                .list
                .summary
                .packed_orders,
            1
        );

        assert!(
            runtime
                .select_orders_filter(OrdersFilter::Packed)
                .expect("packed filter should apply")
        );
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Packed
        );

        assert!(
            runtime
                .open_order_detail(packed_order_id)
                .expect("packed detail should open")
        );
        let packed_detail_summary = runtime.summary();
        assert_eq!(
            packed_detail_summary
                .orders_projection
                .detail
                .as_ref()
                .expect("packed detail")
                .status,
            OrderStatus::Packed
        );

        assert!(
            runtime
                .select_orders_filter(OrdersFilter::Completed)
                .expect("completed filter should apply")
        );
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Completed
        );

        assert!(
            runtime
                .open_order_detail(completed_order_id)
                .expect("completed detail should open")
        );
        assert_eq!(
            runtime
                .summary()
                .orders_projection
                .detail
                .as_ref()
                .expect("completed detail")
                .status,
            OrderStatus::Completed
        );
    }

    #[test]
    fn runtime_stock_updates_refresh_today_and_products_projections() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");
        seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "published",
            Some(2),
            "2026-04-18T10:00:00Z",
        );

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        let product_id = runtime.summary().products_projection.list.rows[0].product_id;

        assert_eq!(
            runtime.summary().today_projection.low_stock_products.len(),
            1
        );
        assert!(
            runtime
                .update_product_stock(product_id, 12)
                .expect("stock update should succeed")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.products_projection.list.rows[0].stock.quantity,
            Some(12)
        );
        assert!(summary.today_projection.low_stock_products.is_empty());
    }

    #[test]
    fn runtime_open_new_product_editor_creates_a_local_draft_and_opens_it() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert_eq!(
            runtime
                .summary()
                .products_projection
                .list
                .summary
                .total_products,
            0
        );

        assert!(
            runtime
                .open_new_product_editor()
                .expect("new product editor should open")
        );

        let summary = runtime.summary();
        assert_eq!(summary.products_projection.list.summary.total_products, 1);
        assert!(matches!(
            summary.products_projection.editor,
            radroots_studio_app_state::ProductEditorState::Open(_)
        ));
        assert_eq!(
            summary.products_projection.list.rows[0].status,
            ProductStatus::Draft
        );
    }

    #[test]
    fn runtime_open_existing_and_save_product_editor_refreshes_products_projection() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");
        let product_id = seed_product(
            &runtime,
            farm_id,
            "Salad mix",
            "Spring blend",
            "draft",
            Some(2),
            "2026-04-18T10:00:00Z",
        );

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert!(
            runtime
                .open_existing_product_editor(product_id)
                .expect("existing product editor should open")
        );

        let saved_draft = ProductEditorDraft {
            title: "Salad mix".to_owned(),
            subtitle: "Washed and boxed".to_owned(),
            category: "greens".to_owned(),
            unit_label: "box".to_owned(),
            price_minor_units: Some(900),
            price_currency: "usd".to_owned(),
            stock_quantity: Some(14),
            availability_window_id: None,
            status: radroots_studio_app_view::ProductStatus::Published,
        };

        assert!(
            runtime
                .save_product_editor_draft(saved_draft.clone())
                .expect("product editor draft should save")
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.products_projection.list.rows[0].subtitle.as_deref(),
            Some("Washed and boxed")
        );
        assert_eq!(
            summary.products_projection.list.rows[0]
                .price
                .as_ref()
                .map(|price| price.amount_minor_units),
            Some(900)
        );
        assert_eq!(
            summary.products_projection.list.rows[0].stock.quantity,
            Some(14)
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_product_editor_draft(product_id)
                .expect("saved draft should load"),
            Some(ProductEditorDraft {
                price_currency: "USD".to_owned(),
                ..saved_draft
            })
        );
    }

    #[test]
    fn runtime_account_commands_refresh_identity_projection() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("First".to_owned()))
                .expect("first account should generate")
        );
        let first_summary = runtime.summary();
        let first_account_id = first_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();

        assert!(
            runtime
                .generate_local_account(Some("Second".to_owned()))
                .expect("second account should generate")
        );
        let second_summary = runtime.summary();
        let second_account_id = second_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();
        assert_eq!(second_summary.settings_account_projection.roster.len(), 2);
        assert_eq!(
            second_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .and_then(|account| account.account.label.as_deref()),
            Some("Second")
        );

        save_surface_activation(
            &runtime,
            second_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        assert!(
            runtime
                .select_local_account(second_account_id.as_str())
                .expect("selection should succeed")
        );
        let selected_summary = runtime.summary();
        assert_eq!(selected_summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(selected_summary.home_route, HomeRoute::FarmSetupOnboarding);
        assert_eq!(
            selected_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );

        assert!(
            runtime
                .remove_selected_local_key()
                .expect("selected local key should remove")
        );
        let removed_summary = runtime.summary();
        assert_eq!(removed_summary.settings_account_projection.roster.len(), 1);
        assert_eq!(
            removed_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(first_account_id.as_str())
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(second_account_id.as_str())
                .expect("removed activation should load"),
            None
        );

        let imported_identity = RadrootsIdentity::generate();
        assert!(
            runtime
                .import_local_account(DesktopLocalIdentityImportRequest::raw_secret_key(
                    imported_identity.nsec(),
                ))
                .expect("raw import should succeed")
        );
        let imported_summary = runtime.summary();
        assert_eq!(imported_summary.settings_account_projection.roster.len(), 2);
        assert_eq!(
            imported_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.as_str()),
            Some(imported_identity.id().as_str())
        );
    }

    #[test]
    fn runtime_select_active_surface_persists_selected_surface() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        save_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer, true);
        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert_eq!(runtime.summary().startup_gate, AppStartupGate::Farmer);

        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should select")
        );
        let personal_summary = runtime.summary();
        assert_eq!(personal_summary.startup_gate, AppStartupGate::Personal);
        assert_eq!(
            personal_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Personal)
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(account_id.as_str())
                .expect("surface activation should load")
                .expect("surface activation should exist")
                .active_surface(),
            ActiveSurface::Personal
        );

        assert!(
            runtime
                .select_active_surface(ActiveSurface::Farmer)
                .expect("surface should reselect")
        );
        let farmer_summary = runtime.summary();
        assert_eq!(farmer_summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(
            farmer_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.active_surface()),
            Some(ActiveSurface::Farmer)
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(account_id.as_str())
                .expect("surface activation should load")
                .expect("surface activation should exist")
                .active_surface(),
            ActiveSurface::Farmer
        );
    }

    #[test]
    fn selecting_farmer_account_loads_persisted_farm_setup_draft() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let projection = FarmSetupProjection::from_draft(FarmSetupDraft::new(
            "North field farm",
            "Stockholm County",
            [FarmOrderMethod::Pickup],
        ));
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &projection)
            .expect("farm setup should save");
        save_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer, true);

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        let summary = runtime.summary();

        assert_eq!(summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(summary.home_route, HomeRoute::FarmSetupForm);
        assert_eq!(summary.farm_setup_projection, projection);
    }

    #[test]
    fn finishing_farm_setup_persists_saved_farm_and_today_projection() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );
        assert_eq!(runtime.summary().home_route, HomeRoute::FarmSetupOnboarding);

        let draft = FarmSetupDraft::new(
            "North field farm",
            "Stockholm County",
            [FarmOrderMethod::Pickup, FarmOrderMethod::Delivery],
        );
        assert_eq!(
            runtime
                .save_farm_setup_draft(draft.clone())
                .expect("draft should save")
                .draft,
            draft
        );
        assert_eq!(runtime.summary().home_route, HomeRoute::FarmSetupForm);

        let finished_projection = runtime
            .finish_farm_setup()
            .expect("farm setup should finish");
        let summary = runtime.summary();

        assert_eq!(summary.home_route, HomeRoute::Today);
        assert_eq!(
            finished_projection.saved_farm,
            Some(FarmSummary {
                farm_id,
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            })
        );
        assert_eq!(
            summary.today_projection.farm,
            finished_projection.saved_farm.clone()
        );
        assert_eq!(summary.today_projection.setup_checklist.len(), 6);
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_farm_setup(account_id.as_str())
                .expect("farm setup should load"),
            finished_projection
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_today_agenda(Some(farm_id))
                .expect("today agenda should load")
                .farm,
            finished_projection.saved_farm
        );
    }

    #[test]
    fn loading_farm_rules_projection_seeds_profile_from_saved_farm() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Incomplete,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );

        let projection = runtime
            .load_farm_rules_projection()
            .expect("farm rules projection should load");

        assert_eq!(
            projection.farm_profile,
            Some(FarmProfileRecord {
                farm_id,
                display_name: "North field farm".to_owned(),
                timezone: "UTC".to_owned(),
                currency_code: "USD".to_owned(),
            })
        );
        assert_eq!(
            projection.readiness.blockers,
            vec![
                FarmReadinessBlocker::MissingPickupLocation,
                FarmReadinessBlocker::MissingOperatingRules,
                FarmReadinessBlocker::MissingFulfillmentWindow,
            ]
        );
    }

    #[test]
    fn saving_farm_rules_projection_refreshes_saved_farm_summary_and_pickup_defaults() {
        let runtime = memory_runtime();

        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(&runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Incomplete,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");

        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );

        let default_pickup_location_id = PickupLocationId::new();
        let market_pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let blackout_period_id = BlackoutPeriodId::new();

        let saved_projection = runtime
            .save_farm_rules_projection(radroots_studio_app_view::FarmRulesProjection {
                farm_profile: Some(FarmProfileRecord {
                    farm_id,
                    display_name: "Harbor farm".to_owned(),
                    timezone: "Europe/Stockholm".to_owned(),
                    currency_code: "sek".to_owned(),
                }),
                pickup_locations: vec![
                    PickupLocationRecord {
                        pickup_location_id: default_pickup_location_id,
                        farm_id,
                        label: "   Barn pickup   ".to_owned(),
                        address_line: " 14 Orchard Lane ".to_owned(),
                        directions: Some("  Drive to the red barn.  ".to_owned()),
                        is_default: false,
                    },
                    PickupLocationRecord {
                        pickup_location_id: market_pickup_location_id,
                        farm_id,
                        label: "Market stall".to_owned(),
                        address_line: "2 Harbor Road".to_owned(),
                        directions: None,
                        is_default: false,
                    },
                ],
                operating_rules: Some(FarmOperatingRulesRecord {
                    farm_id,
                    promise_lead_hours: 24,
                    substitution_policy: "  ask_customer  ".to_owned(),
                    missed_pickup_policy: "  hold_next_window  ".to_owned(),
                }),
                fulfillment_windows: vec![FulfillmentWindowRecord {
                    fulfillment_window_id,
                    farm_id,
                    pickup_location_id: default_pickup_location_id,
                    label: "  Friday pickup  ".to_owned(),
                    starts_at: " 2026-04-25T14:00:00Z ".to_owned(),
                    ends_at: " 2026-04-25T18:00:00Z ".to_owned(),
                    order_cutoff_at: " 2026-04-24T18:00:00Z ".to_owned(),
                }],
                blackout_periods: vec![BlackoutPeriodRecord {
                    blackout_period_id,
                    farm_id,
                    label: "  Spring break  ".to_owned(),
                    starts_at: " 2026-05-01T00:00:00Z ".to_owned(),
                    ends_at: " 2026-05-03T23:59:59Z ".to_owned(),
                }],
                ..runtime
                    .load_farm_rules_projection()
                    .expect("farm rules projection should load")
            })
            .expect("farm rules projection should save");

        assert_eq!(
            saved_projection.farm_profile,
            Some(FarmProfileRecord {
                farm_id,
                display_name: "Harbor farm".to_owned(),
                timezone: "Europe/Stockholm".to_owned(),
                currency_code: "SEK".to_owned(),
            })
        );
        assert_eq!(saved_projection.pickup_locations.len(), 2);
        assert!(saved_projection.pickup_locations[0].is_default);
        assert_eq!(saved_projection.pickup_locations[0].label, "Barn pickup");
        assert_eq!(
            saved_projection.pickup_locations[0].address_line,
            "14 Orchard Lane"
        );
        assert_eq!(
            saved_projection.pickup_locations[0].directions.as_deref(),
            Some("Drive to the red barn.")
        );
        assert_eq!(
            saved_projection.operating_rules,
            Some(FarmOperatingRulesRecord {
                farm_id,
                promise_lead_hours: 24,
                substitution_policy: "ask_customer".to_owned(),
                missed_pickup_policy: "hold_next_window".to_owned(),
            })
        );
        assert_eq!(
            saved_projection.fulfillment_windows,
            vec![FulfillmentWindowRecord {
                fulfillment_window_id,
                farm_id,
                pickup_location_id: default_pickup_location_id,
                label: "Friday pickup".to_owned(),
                starts_at: "2026-04-25T14:00:00Z".to_owned(),
                ends_at: "2026-04-25T18:00:00Z".to_owned(),
                order_cutoff_at: "2026-04-24T18:00:00Z".to_owned(),
            }]
        );
        assert_eq!(
            saved_projection.blackout_periods,
            vec![BlackoutPeriodRecord {
                blackout_period_id,
                farm_id,
                label: "Spring break".to_owned(),
                starts_at: "2026-05-01T00:00:00Z".to_owned(),
                ends_at: "2026-05-03T23:59:59Z".to_owned(),
            }]
        );

        let summary = runtime.summary();
        assert_eq!(
            summary.farm_setup_projection.saved_farm,
            Some(FarmSummary {
                farm_id,
                display_name: "Harbor farm".to_owned(),
                readiness: FarmReadiness::Ready,
            })
        );
        assert_eq!(summary.farm_setup_projection.draft.farm_name, "Harbor farm");
        assert_eq!(
            summary.today_projection.farm,
            summary.farm_setup_projection.saved_farm
        );
    }

    #[test]
    fn runtime_reset_local_device_state_clears_store_file_and_projection() {
        let (runtime, paths) = file_backed_runtime("reset");

        assert!(
            runtime
                .generate_local_account(Some("First".to_owned()))
                .expect("first account should generate")
        );
        let first_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();
        assert!(
            runtime
                .generate_local_account(Some("Second".to_owned()))
                .expect("second account should generate")
        );
        let second_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("second selected account")
            .account
            .account_id
            .clone();
        save_surface_activation(
            &runtime,
            first_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        save_surface_activation(
            &runtime,
            second_account_id.as_str(),
            ActiveSurface::Farmer,
            true,
        );
        assert!(paths.store_path.exists());

        assert!(
            runtime
                .reset_local_device_state()
                .expect("device state should reset")
        );
        let summary = runtime.summary();

        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert!(
            summary
                .settings_account_projection
                .selected_account
                .is_none()
        );
        assert!(!paths.store_path.exists());
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(first_account_id.as_str())
                .expect("first activation should load"),
            None
        );
        assert_eq!(
            runtime
                .lock_state()
                .sqlite_store
                .as_ref()
                .expect("sqlite store")
                .load_surface_activation(second_account_id.as_str())
                .expect("second activation should load"),
            None
        );

        cleanup_paths(&paths);
    }

    #[test]
    fn runtime_account_commands_fail_closed_without_accounts_manager() {
        let paths = temp_shared_accounts_paths("blocked");
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: Some(paths),
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        let error = runtime
            .generate_local_account(Some("Blocked".to_owned()))
            .expect_err("blocked runtime should fail closed");

        assert!(matches!(
            error,
            DesktopAppRuntimeCommandError::RuntimeUnavailable
        ));
    }

    fn memory_runtime() -> DesktopAppRuntime {
        DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: Some(
                RadrootsNostrAccountsManager::new(
                    Arc::new(RadrootsNostrMemoryAccountStore::new()),
                    Arc::new(RadrootsNostrSecretVaultMemory::new()),
                )
                .expect("memory manager should build"),
            ),
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
            selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        })
    }

    fn file_backed_runtime(label: &str) -> (DesktopAppRuntime, AppSharedAccountsPaths) {
        let paths = temp_shared_accounts_paths(label);
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");

        (
            DesktopAppRuntime::from_state(DesktopAppRuntimeState {
                state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                    .expect("in-memory state store should load"),
                nostr_relay_urls: vec!["ws://127.0.0.1:8080".to_owned()],
                shared_accounts_paths: Some(paths.clone()),
                remote_signer_paths: None,
                accounts_manager: Some(
                    RadrootsNostrAccountsManager::new(
                        Arc::new(RadrootsNostrFileAccountStore::new(
                            paths.store_path.as_path(),
                        )),
                        Arc::new(RadrootsNostrSecretVaultMemory::new()),
                    )
                    .expect("file-backed manager should build"),
                ),
                sqlite_store: Some(
                    AppSqliteStore::open(DatabaseTarget::InMemory)
                        .expect("in-memory sqlite store should open"),
                ),
                sync_transport: default_sync_transport(),
                runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
                selected_account_pending_sync_write_count: 0,
                selected_account_relay_ingest_freshness: AppRelayIngestScopeFreshness::default(),
                selected_account_sync_conflicts: Vec::new(),
                startup_issue: None,
            }),
            paths,
        )
    }

    fn bootstrapped_runtime(label: &str) -> (DesktopAppRuntime, AppDesktopRuntimePaths) {
        let paths = temp_desktop_runtime_paths(label);
        let runtime = restart_runtime(paths.clone());
        (runtime, paths)
    }

    fn restart_runtime(paths: AppDesktopRuntimePaths) -> DesktopAppRuntime {
        DesktopAppRuntime::from_state(
            DesktopAppRuntimeState::bootstrap_from_paths(
                paths,
                vec!["ws://127.0.0.1:8080".to_owned()],
                super::default_runtime_snapshot(),
            )
            .expect("runtime bootstrap should succeed"),
        )
    }

    fn temp_shared_accounts_paths(label: &str) -> AppSharedAccountsPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("radroots_runtime_accounts_{label}_{suffix}"));

        AppSharedAccountsPaths {
            data_root: base.join("data/shared/accounts"),
            secrets_root: base.join("secrets/shared/accounts"),
            store_path: base.join("data/shared/accounts/store.json"),
        }
    }

    fn temp_desktop_runtime_paths(label: &str) -> AppDesktopRuntimePaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let home_dir = std::env::temp_dir().join(format!("radroots_runtime_home_{label}_{suffix}"));
        AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(home_dir),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("desktop runtime paths should resolve")
    }

    fn temp_remote_signer_paths(label: &str) -> DesktopRemoteSignerPaths {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let base =
            std::env::temp_dir().join(format!("radroots_runtime_remote_signer_{label}_{suffix}"));
        DesktopRemoteSignerPaths {
            sessions_path: base.join("data/apps/app/nostr/remote-signer-sessions.json"),
            client_secret_root: base.join("secrets/shared/accounts/remote_signer"),
        }
    }

    fn cleanup_remote_signer_paths(paths: &DesktopRemoteSignerPaths) {
        if let Some(base) = paths.sessions_path.ancestors().nth(5) {
            let _ = fs::remove_dir_all(base);
        }
    }

    fn cleanup_bootstrapped_runtime_paths(paths: &AppDesktopRuntimePaths) {
        if let Some(home_dir) = paths.app.data.ancestors().nth(4) {
            let _ = fs::remove_dir_all(home_dir);
        }
    }

    fn append_cli_local_listing_records(paths: &AppDesktopRuntimePaths, account_id: &str) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        store
            .append_record(&local_work_record(
                "cli:local_work:farm",
                account_id,
                farm_key,
                None,
                json!({
                    "record_kind": "farm_config_v1",
                    "document": {
                        "selection": {
                            "account": account_id,
                            "farm_d_tag": farm_key
                        },
                        "profile": {
                            "name": "Green Farm",
                            "display_name": "Green Farm"
                        },
                        "farm": {
                            "d_tag": farm_key,
                            "name": "Green Farm",
                            "location": {
                                "primary": "farmstand"
                            }
                        },
                        "listing_defaults": {
                            "delivery_method": "pickup",
                            "location": {
                                "primary": "farmstand"
                            }
                        }
                    }
                }),
            ))
            .expect("append farm local work");
        store
            .append_record(&local_work_record(
                "cli:local_work:listing",
                account_id,
                farm_key,
                Some(format!("30402:seller-pubkey:{listing_key}")),
                json!({
                    "record_kind": "listing_draft_v1",
                    "document": {
                        "listing": {
                            "d_tag": listing_key,
                            "farm_d_tag": farm_key
                        },
                        "seller_actor": {
                            "account_id": account_id,
                            "pubkey": "seller-pubkey"
                        },
                        "product": {
                            "key": "eggs",
                            "title": "Eggs",
                            "summary": "Fresh eggs"
                        },
                        "primary_bin": {
                            "quantity_unit": "each",
                            "price_amount": "6",
                            "price_currency": "USD"
                        },
                        "inventory": {
                            "available": "10"
                        }
                    }
                }),
            ))
            .expect("append listing local work");
    }

    fn append_cli_signed_buyer_listing_record(paths: &AppDesktopRuntimePaths) {
        append_cli_signed_buyer_listing_record_with(
            paths,
            "buyer-visible-listing",
            "DDDDDDDDDDDDDDDDDDDDDD",
            "Buyer Visible Eggs",
            1100,
        );
    }

    fn append_cli_signed_buyer_listing_record_with(
        paths: &AppDesktopRuntimePaths,
        record_suffix: &str,
        listing_key: &str,
        title: &str,
        created_at_ms: i64,
    ) {
        append_cli_signed_buyer_listing_record_with_bin(
            paths,
            record_suffix,
            listing_key,
            title,
            created_at_ms,
            "bin-1",
        );
    }

    fn append_cli_signed_buyer_listing_record_with_bin(
        paths: &AppDesktopRuntimePaths,
        record_suffix: &str,
        listing_key: &str,
        title: &str,
        created_at_ms: i64,
        bin_id: &str,
    ) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let farm_key = "CCCCCCCCCCCCCCCCCCCCCC";
        let owner_pubkey = "buyer-visible-seller-pubkey";
        let record_id = format!("cli:signed_event:{record_suffix}");
        let content = json!({
            "d_tag": listing_key,
            "status": "active",
            "farm": {
                "pubkey": owner_pubkey,
                "d_tag": farm_key
            },
            "product": {
                "key": listing_key,
                "title": title,
                "summary": "Published local eggs"
            },
            "availability": {
                "kind": "window",
                "amount": {
                    "start": 4_102_444_800u64,
                    "end": 4_102_531_200u64
                }
            },
            "delivery_method": {
                "kind": "pickup"
            },
            "location": {
                "primary": "North barn pickup"
            }
        });
        store
            .append_record(&LocalEventRecordInput {
                record_id: record_id.to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms,
                inserted_at_ms: created_at_ms + 1,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some(owner_pubkey.to_owned()),
                farm_id: Some(farm_key.to_owned()),
                listing_addr: Some(format!("30402:{owner_pubkey}:{listing_key}")),
                local_work_json: None,
                event_id: Some(format!("event-{record_id}")),
                event_kind: Some(30402),
                event_pubkey: Some(owner_pubkey.to_owned()),
                event_created_at: Some(created_at_ms),
                event_tags_json: Some(json!([
                    ["d", listing_key],
                    ["a", format!("30340:{owner_pubkey}:{farm_key}")],
                    ["key", listing_key],
                    ["title", title],
                    ["summary", "Published local eggs"],
                    ["radroots:bin", bin_id, "1", "each"],
                    ["radroots:price", bin_id, "8", "USD", "1", "each"],
                    ["inventory", "9"],
                    ["status", "active"],
                    ["radroots:availability_start", "4102444800"],
                    ["expires_at", "4102531200"],
                    ["delivery", "pickup"],
                    ["location", "North barn pickup"]
                ])),
                event_content: Some(content.to_string()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": format!("event-{record_id}"),
                    "kind": 30402,
                    "pubkey": owner_pubkey,
                    "content": content.to_string()
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append signed buyer listing");
    }

    struct LinkedBuyerLifecycleFixture {
        runtime: DesktopAppRuntime,
        paths: AppDesktopRuntimePaths,
        order_id: OrderId,
        trade_order_id: String,
        request_event_id: String,
        decision_event_id: String,
        fulfillment_event_id: Option<String>,
        listing_addr: String,
        buyer_pubkey: String,
        seller_pubkey: String,
    }

    fn linked_buyer_lifecycle_runtime(
        label: &str,
        include_ready_fulfillment: bool,
    ) -> LinkedBuyerLifecycleFixture {
        linked_buyer_lifecycle_runtime_with_seller_pubkey(
            label,
            include_ready_fulfillment,
            "2222222222222222222222222222222222222222222222222222222222222222",
        )
    }

    fn linked_buyer_lifecycle_runtime_with_seller_pubkey(
        label: &str,
        include_ready_fulfillment: bool,
        seller_pubkey: &str,
    ) -> LinkedBuyerLifecycleFixture {
        let (runtime, paths) = bootstrapped_runtime(label);
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("buyer account should generate")
        );
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("buyer surface should select")
        );
        let buyer_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected buyer account")
            .account
            .account_id
            .clone();
        let buyer_pubkey = runtime
            .lock_state()
            .accounts_manager
            .as_ref()
            .expect("accounts manager")
            .resolve_account_selector(buyer_account_id.as_str())
            .expect("selected buyer account should resolve")
            .public_identity
            .public_key_hex;
        let farm_key = super::d_tag_from_uuid(FarmId::new().as_uuid());
        let listing_key = super::d_tag_from_uuid(ProductId::new().as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let listing_event_id = format!("event-app:signed_event:listing:{label}");
        let trade_order_id = format!("{label}-trade-order");
        let order_id =
            projected_order_id_from_trade_request(trade_order_id.as_str(), buyer_pubkey.as_str());
        append_app_signed_listing_record(
            &paths,
            "linked-seller-account",
            seller_pubkey,
            farm_key.as_str(),
            listing_key.as_str(),
            listing_event_id.as_str(),
            6,
        );
        append_signed_order_request_record(
            &paths,
            trade_order_id.as_str(),
            listing_addr.as_str(),
            listing_event_id.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey,
            2,
        );
        let request_event_id = format!("event-app:signed_event:order-request:{trade_order_id}");
        let decision_event_id = append_signed_order_decision_record(
            &paths,
            trade_order_id.as_str(),
            request_event_id.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey,
            2,
        );
        let fulfillment_event_id = if include_ready_fulfillment {
            Some(append_signed_order_fulfillment_record(
                &paths,
                trade_order_id.as_str(),
                request_event_id.as_str(),
                decision_event_id.as_str(),
                listing_addr.as_str(),
                buyer_pubkey.as_str(),
                seller_pubkey,
            ))
        } else {
            None
        };

        LinkedBuyerLifecycleFixture {
            runtime,
            paths,
            order_id,
            trade_order_id,
            request_event_id,
            decision_event_id,
            fulfillment_event_id,
            listing_addr,
            buyer_pubkey,
            seller_pubkey: seller_pubkey.to_owned(),
        }
    }

    fn seller_order_decision_runtime(
        label: &str,
        stock_count: u32,
        order_quantity: u32,
    ) -> (
        DesktopAppRuntime,
        AppDesktopRuntimePaths,
        OrderId,
        ProductId,
        String,
        String,
    ) {
        let (runtime, paths) = bootstrapped_runtime(label);
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        runtime.lock_state_mut().nostr_relay_urls = vec!["wss://relay.example".to_owned()];
        let seller_pubkey = runtime
            .lock_state()
            .accounts_manager
            .as_ref()
            .expect("accounts manager")
            .resolve_account_selector(account_id.as_str())
            .expect("selected seller account should resolve")
            .public_identity
            .public_key_hex;
        let buyer_pubkey =
            "1111111111111111111111111111111111111111111111111111111111111111".to_owned();
        let product_id = ProductId::new();
        let trade_order_id = "seller-order-decision-1";
        let order_id = projected_order_id_from_trade_request(trade_order_id, buyer_pubkey.as_str());
        let farm_key = super::d_tag_from_uuid(farm_id.as_uuid());
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let listing_event_id = "event-app:signed_event:listing:seller-order-decision";
        append_app_signed_listing_record(
            &paths,
            account_id.as_str(),
            seller_pubkey.as_str(),
            farm_key.as_str(),
            listing_key.as_str(),
            listing_event_id,
            stock_count,
        );
        append_signed_order_request_record(
            &paths,
            trade_order_id,
            listing_addr.as_str(),
            listing_event_id,
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
            order_quantity,
        );

        (
            runtime,
            paths,
            order_id,
            product_id,
            seller_pubkey,
            buyer_pubkey,
        )
    }

    fn publish_prior_relay_seller_order_accept(
        runtime: &DesktopAppRuntime,
        relay: &ThreadedAckRelay,
        order_id: OrderId,
        product_id: ProductId,
        seller_pubkey: &str,
        buyer_pubkey: &str,
    ) {
        let (account_id, farm_id, accounts_manager) = {
            let state = runtime.lock_state();
            let selected_account = state
                .state_store
                .identity_projection()
                .selected_account
                .as_ref()
                .expect("selected seller account");
            (
                selected_account.account.account_id.clone(),
                state.selected_farm_id().expect("selected farm"),
                state
                    .accounts_manager
                    .as_ref()
                    .expect("accounts manager")
                    .clone(),
            )
        };
        let listing_key = super::d_tag_from_uuid(product_id.as_uuid());
        let payload = AppPublishPayload::OrderDecision(AppOrderDecisionPublishPayload {
            context: AppPublishContext::new(account_id, "seller_order_decision"),
            app_order_id: order_id,
            farm_id,
            trade_order_id: "seller-order-decision-1".to_owned(),
            request_event_id: "event-app:signed_event:order-request:seller-order-decision-1"
                .to_owned(),
            listing_event_id: Some(
                "event-app:signed_event:listing:seller-order-decision".to_owned(),
            ),
            listing_addr: format!("30402:{seller_pubkey}:{listing_key}"),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            decision: AppOrderDecisionPayload::Accepted {
                inventory_commitments: vec![AppOrderDecisionInventoryCommitment {
                    bin_id: "seller-order-primary-bin".to_owned(),
                    bin_count: 2,
                }],
            },
        });
        let operation = PendingSyncOperation::from_publish_payload(payload, "2026-05-24T12:00:00Z")
            .expect("prior order decision publish work should serialize");
        let mut transport = SdkDirectRelayAppSyncTransport::with_relay_urls(
            accounts_manager,
            vec![relay.url().to_owned()],
        );
        let result = transport
            .sync(AppSyncRequest {
                trigger: SyncTrigger::ManualRefresh,
                checkpoint: SyncCheckpointStatus::never_synced(),
                pending_operations: vec![operation],
                known_conflicts: Vec::new(),
            })
            .expect("prior seller decision relay publish should succeed");

        assert_eq!(result.run_status, AppSyncRunStatus::Succeeded);
        assert_eq!(result.pushed_operation_count, 1);
    }

    fn append_app_signed_listing_record(
        paths: &AppDesktopRuntimePaths,
        account_id: &str,
        seller_pubkey: &str,
        farm_key: &str,
        listing_key: &str,
        listing_event_id: &str,
        stock_count: u32,
    ) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let content = json!({
            "d_tag": listing_key,
            "status": "active",
            "farm": {
                "pubkey": seller_pubkey,
                "d_tag": farm_key
            },
            "product": {
                "key": listing_key,
                "title": "Seller decision lettuce",
                "summary": "Signed listing for seller decision tests"
            },
            "availability": {
                "kind": "window",
                "amount": {
                    "start": 4_102_444_800u64,
                    "end": 4_102_531_200u64
                }
            },
            "delivery_method": {
                "kind": "pickup"
            },
            "location": {
                "primary": "North barn pickup"
            }
        });
        store
            .append_record(&LocalEventRecordInput {
                record_id: "app:signed_event:listing:seller-order-decision".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::App,
                created_at_ms: 1_774_000_000_000,
                inserted_at_ms: 1_774_000_000_001,
                owner_account_id: Some(account_id.to_owned()),
                owner_pubkey: Some(seller_pubkey.to_owned()),
                farm_id: Some(farm_key.to_owned()),
                listing_addr: Some(listing_addr),
                local_work_json: None,
                event_id: Some(listing_event_id.to_owned()),
                event_kind: Some(30402),
                event_pubkey: Some(seller_pubkey.to_owned()),
                event_created_at: Some(1_774_000_000),
                event_tags_json: Some(json!([
                    ["d", listing_key],
                    ["a", format!("30340:{seller_pubkey}:{farm_key}")],
                    ["key", listing_key],
                    ["title", "Seller decision lettuce"],
                    ["summary", "Signed listing for seller decision tests"],
                    ["radroots:bin", "seller-order-primary-bin", "1", "each"],
                    [
                        "radroots:price",
                        "seller-order-primary-bin",
                        "8",
                        "USD",
                        "1",
                        "each"
                    ],
                    ["inventory", stock_count.to_string()],
                    ["status", "active"],
                    ["radroots:availability_start", "4102444800"],
                    ["expires_at", "4102531200"],
                    ["delivery", "pickup"],
                    ["location", "North barn pickup"]
                ])),
                event_content: Some(content.to_string()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": listing_event_id,
                    "kind": 30402,
                    "pubkey": seller_pubkey,
                    "content": content.to_string()
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "target_relays": ["wss://relay.example"],
                    "connected_relays": ["wss://relay.example"],
                    "acknowledged_relays": ["wss://relay.example"]
                })),
            })
            .expect("append app signed listing");
    }

    fn append_signed_order_request_record(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        listing_addr: &str,
        listing_event_id: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
    ) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let order = RadrootsTradeOrderRequested {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            items: vec![RadrootsTradeOrderItem {
                bin_id: "seller-order-primary-bin".to_owned(),
                bin_count: order_quantity,
            }],
            economics: signed_order_request_economics(trade_order_id, order_quantity),
        };
        let parts = radroots_sdk::trade::build_order_request_draft(
            &RadrootsNostrEventPtr {
                id: listing_event_id.to_owned(),
                relays: Some("wss://relay.example".to_owned()),
            },
            &order,
        )
        .expect("order request draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:order-request:{trade_order_id}");
        let event_id = format!("event-{record_id}");
        let relay_delivery_json = RelayDeliveryEvidence::acknowledged(
            ["wss://relay.example"],
            ["wss://relay.example"],
            ["wss://relay.example"],
            Vec::new(),
        )
        .expect("acknowledged relay delivery evidence")
        .to_json_value()
        .expect("acknowledged relay delivery json");
        store
            .append_record(&LocalEventRecordInput {
                record_id,
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Test,
                created_at_ms: 1_774_000_010_000,
                inserted_at_ms: 1_774_000_010_001,
                owner_account_id: None,
                owner_pubkey: Some(buyer_pubkey.to_owned()),
                farm_id: None,
                listing_addr: Some(listing_addr.to_owned()),
                local_work_json: None,
                event_id: Some(event_id.clone()),
                event_kind: Some(i64::from(parts.kind)),
                event_pubkey: Some(buyer_pubkey.to_owned()),
                event_created_at: Some(1_774_000_010),
                event_tags_json: Some(json!(parts.tags)),
                event_content: Some(parts.content.clone()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": event_id,
                    "kind": parts.kind,
                    "pubkey": buyer_pubkey,
                    "content": parts.content
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(relay_delivery_json),
            })
            .expect("append signed order request");
    }

    fn signed_order_request_economics(
        trade_order_id: &str,
        order_quantity: u32,
    ) -> RadrootsTradeOrderEconomics {
        let currency = RadrootsCoreCurrency::USD;
        let unit_price_minor_units = 800_u32;
        let total_minor_units = unit_price_minor_units
            .checked_mul(order_quantity)
            .expect("order total should fit");

        RadrootsTradeOrderEconomics {
            quote_id: format!("{trade_order_id}-quote"),
            quote_version: 1,
            pricing_basis: RadrootsTradePricingBasis::ListingEvent,
            currency,
            items: vec![RadrootsTradeOrderEconomicItem {
                bin_id: "seller-order-primary-bin".to_owned(),
                bin_count: order_quantity,
                quantity_amount: RadrootsCoreDecimal::from(1u32),
                quantity_unit: RadrootsCoreUnit::Each,
                unit_price_amount: RadrootsCoreDecimal::from(8u32),
                unit_price_currency: currency,
                line_subtotal: RadrootsCoreMoney::from_minor_units_u32(total_minor_units, currency),
            }],
            discounts: Vec::new(),
            adjustments: Vec::new(),
            subtotal: RadrootsCoreMoney::from_minor_units_u32(total_minor_units, currency),
            discount_total: RadrootsCoreMoney::zero(currency),
            adjustment_total: RadrootsCoreMoney::zero(currency),
            total: RadrootsCoreMoney::from_minor_units_u32(total_minor_units, currency),
        }
    }

    fn append_signed_order_decision_record(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        request_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
    ) -> String {
        let payload = RadrootsTradeOrderDecisionEvent {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            decision: RadrootsTradeOrderDecision::Accepted {
                inventory_commitments: vec![RadrootsTradeInventoryCommitment {
                    bin_id: "seller-order-primary-bin".to_owned(),
                    bin_count: order_quantity,
                }],
            },
        };
        let parts = radroots_sdk::trade::build_order_decision_draft(
            request_event_id,
            request_event_id,
            &payload,
        )
        .expect("order decision draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:order-decision:{trade_order_id}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            seller_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_payment_record(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        agreement_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
    ) -> String {
        append_signed_payment_record_with_prev(
            paths,
            trade_order_id,
            event_key,
            request_event_id,
            agreement_event_id,
            agreement_event_id,
            listing_addr,
            buyer_pubkey,
            seller_pubkey,
            order_quantity,
        )
    }

    fn append_signed_payment_record_with_prev(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        prev_event_id: &str,
        agreement_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
    ) -> String {
        let economics = signed_order_request_economics(trade_order_id, order_quantity);
        let payload = RadrootsTradePaymentRecorded {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            previous_event_id: prev_event_id.to_owned(),
            agreement_event_id: agreement_event_id.to_owned(),
            quote_id: economics.quote_id.clone(),
            quote_version: economics.quote_version,
            economics_digest: radroots_trade_order_economics_digest(&economics)
                .expect("payment economics digest"),
            amount: economics.total.amount,
            currency: economics.total.currency,
            method: RadrootsTradePaymentMethod::ManualTransfer,
            reference: Some(format!("memo-{event_key}")),
            paid_at: Some(1_774_000_050),
        };
        let parts =
            active_trade_payment_recorded_event_build(request_event_id, prev_event_id, &payload)
                .expect("payment recorded draft should build");
        let record_id = format!("app:signed_event:payment:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            buyer_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_settlement_decision_record(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        agreement_event_id: &str,
        payment_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
        decision: RadrootsTradeSettlementDecision,
    ) -> String {
        let economics = signed_order_request_economics(trade_order_id, order_quantity);
        let payload = RadrootsTradeSettlementDecisionEvent {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            previous_event_id: payment_event_id.to_owned(),
            agreement_event_id: agreement_event_id.to_owned(),
            payment_event_id: payment_event_id.to_owned(),
            quote_id: economics.quote_id.clone(),
            quote_version: economics.quote_version,
            economics_digest: radroots_trade_order_economics_digest(&economics)
                .expect("k3436 economics digest"),
            amount: economics.total.amount,
            currency: economics.total.currency,
            decision,
            reason: (decision == RadrootsTradeSettlementDecision::Rejected)
                .then(|| "reference mismatch".to_owned()),
        };
        let parts = active_trade_settlement_decision_event_build(
            request_event_id,
            payment_event_id,
            &payload,
        )
        .expect("k3436 draft should build");
        let record_id = format!("app:signed_event:k3436:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            seller_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn signed_payment_recorded_relay_event(
        buyer: &RadrootsIdentity,
        trade_order_id: &str,
        request_event_id: &str,
        agreement_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
    ) -> radroots_nostr::prelude::RadrootsNostrEvent {
        let economics = signed_order_request_economics(trade_order_id, order_quantity);
        let payload = RadrootsTradePaymentRecorded {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            previous_event_id: agreement_event_id.to_owned(),
            agreement_event_id: agreement_event_id.to_owned(),
            quote_id: economics.quote_id.clone(),
            quote_version: economics.quote_version,
            economics_digest: radroots_trade_order_economics_digest(&economics)
                .expect("relay payment economics digest"),
            amount: economics.total.amount,
            currency: economics.total.currency,
            method: RadrootsTradePaymentMethod::ManualTransfer,
            reference: Some("relay-memo-1".to_owned()),
            paid_at: Some(1_774_000_050),
        };
        let parts = active_trade_payment_recorded_event_build(
            request_event_id,
            agreement_event_id,
            &payload,
        )
        .expect("relay payment draft should build");

        radroots_nostr_build_event(parts.kind, parts.content, parts.tags)
            .expect("relay payment builder")
            .sign_with_keys(buyer.keys())
            .expect("relay payment should sign")
    }

    fn signed_settlement_decision_relay_event(
        seller: &RadrootsIdentity,
        trade_order_id: &str,
        request_event_id: &str,
        agreement_event_id: &str,
        payment_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        order_quantity: u32,
        decision: RadrootsTradeSettlementDecision,
    ) -> radroots_nostr::prelude::RadrootsNostrEvent {
        let economics = signed_order_request_economics(trade_order_id, order_quantity);
        let payload = RadrootsTradeSettlementDecisionEvent {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            previous_event_id: payment_event_id.to_owned(),
            agreement_event_id: agreement_event_id.to_owned(),
            payment_event_id: payment_event_id.to_owned(),
            quote_id: economics.quote_id.clone(),
            quote_version: economics.quote_version,
            economics_digest: radroots_trade_order_economics_digest(&economics)
                .expect("relay k3436 economics digest"),
            amount: economics.total.amount,
            currency: economics.total.currency,
            decision,
            reason: (decision == RadrootsTradeSettlementDecision::Rejected)
                .then(|| "reference mismatch".to_owned()),
        };
        let parts = active_trade_settlement_decision_event_build(
            request_event_id,
            payment_event_id,
            &payload,
        )
        .expect("relay k3436 draft should build");

        radroots_nostr_build_event(parts.kind, parts.content, parts.tags)
            .expect("relay k3436 builder")
            .sign_with_keys(seller.keys())
            .expect("relay k3436 should sign")
    }

    fn selected_account_signing_identity(runtime: &DesktopAppRuntime) -> RadrootsIdentity {
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let account_id =
            RadrootsIdentityId::parse(account_id.as_str()).expect("selected account id");
        runtime
            .lock_state()
            .accounts_manager
            .as_ref()
            .expect("accounts manager")
            .get_signing_identity(&account_id)
            .expect("signer lookup should succeed")
            .expect("selected account should have local signer")
    }

    fn append_signed_order_fulfillment_record(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        request_event_id: &str,
        decision_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> String {
        append_signed_order_fulfillment_record_with_status(
            paths,
            trade_order_id,
            request_event_id,
            decision_event_id,
            listing_addr,
            buyer_pubkey,
            seller_pubkey,
            RadrootsActiveTradeFulfillmentState::ReadyForPickup,
        )
    }

    fn append_signed_order_fulfillment_record_with_status(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        request_event_id: &str,
        decision_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        status: RadrootsActiveTradeFulfillmentState,
    ) -> String {
        append_signed_order_fulfillment_record_with_status_and_key(
            paths,
            trade_order_id,
            trade_order_id,
            request_event_id,
            decision_event_id,
            listing_addr,
            buyer_pubkey,
            seller_pubkey,
            status,
        )
    }

    fn append_signed_order_fulfillment_record_with_status_and_key(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        prev_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        status: RadrootsActiveTradeFulfillmentState,
    ) -> String {
        let payload = RadrootsTradeFulfillmentUpdated {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            status,
        };
        let parts = radroots_sdk::trade::build_fulfillment_update_draft(
            request_event_id,
            prev_event_id,
            &payload,
        )
        .expect("fulfillment update draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:fulfillment:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            seller_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_order_cancellation_record_with_prev(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        prev_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> String {
        let payload = RadrootsTradeOrderCancelled {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            reason: "buyer cancelled order".to_owned(),
        };
        let parts = radroots_sdk::trade::build_order_cancellation_draft(
            request_event_id,
            prev_event_id,
            &payload,
        )
        .expect("order cancellation draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:cancellation:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            buyer_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_order_receipt_record_with_prev(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        prev_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        received: bool,
    ) -> String {
        let payload = RadrootsTradeBuyerReceipt {
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            received,
            issue: (!received).then(|| "items need review".to_owned()),
            received_at: 1_774_000_030,
        };
        let parts = radroots_sdk::trade::build_buyer_receipt_draft(
            request_event_id,
            prev_event_id,
            &payload,
        )
        .expect("buyer receipt draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:receipt:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            buyer_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_order_revision_proposal_record_with_prev(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        prev_event_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> String {
        let payload = RadrootsTradeOrderRevisionProposed {
            revision_id: format!("revision-{event_key}"),
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            prev_event_id: prev_event_id.to_owned(),
            items: revision_test_order_items(),
            economics: revision_test_order_economics(),
            reason: "harvest count updated".to_owned(),
        };
        let parts = radroots_sdk::trade::build_order_revision_proposal_draft(
            request_event_id,
            prev_event_id,
            &payload,
        )
        .expect("order revision proposal draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:revision-proposal:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            seller_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn append_signed_order_revision_decision_record_with_prev(
        paths: &AppDesktopRuntimePaths,
        trade_order_id: &str,
        event_key: &str,
        request_event_id: &str,
        proposal_event_id: &str,
        revision_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        decision: RadrootsTradeOrderRevisionDecision,
    ) -> String {
        let payload = RadrootsTradeOrderRevisionDecisionEvent {
            revision_id: revision_id.to_owned(),
            order_id: trade_order_id.to_owned(),
            listing_addr: listing_addr.to_owned(),
            buyer_pubkey: buyer_pubkey.to_owned(),
            seller_pubkey: seller_pubkey.to_owned(),
            root_event_id: request_event_id.to_owned(),
            prev_event_id: proposal_event_id.to_owned(),
            decision,
        };
        let parts = radroots_sdk::trade::build_order_revision_decision_draft(
            request_event_id,
            proposal_event_id,
            &payload,
        )
        .expect("order revision decision draft should build")
        .into_wire_parts();
        let record_id = format!("app:signed_event:revision-decision:{event_key}");
        let event_id = format!("event-{record_id}");
        append_trade_signed_event_record(
            paths,
            record_id.as_str(),
            event_id.as_str(),
            i64::from(parts.kind),
            buyer_pubkey,
            listing_addr,
            json!(parts.tags),
            parts.content,
        );
        event_id
    }

    fn revision_test_order_items() -> Vec<RadrootsTradeOrderItem> {
        vec![RadrootsTradeOrderItem {
            bin_id: "seller-order-primary-bin".to_owned(),
            bin_count: 3,
        }]
    }

    fn revision_test_order_economics() -> RadrootsTradeOrderEconomics {
        RadrootsTradeOrderEconomics {
            quote_id: "quote-revision-test".to_owned(),
            quote_version: 2,
            pricing_basis: RadrootsTradePricingBasis::ListingEvent,
            currency: RadrootsCoreCurrency::USD,
            items: vec![RadrootsTradeOrderEconomicItem {
                bin_id: "seller-order-primary-bin".to_owned(),
                bin_count: 3,
                quantity_amount: RadrootsCoreDecimal::from(1u32),
                quantity_unit: RadrootsCoreUnit::Each,
                unit_price_amount: RadrootsCoreDecimal::from(8u32),
                unit_price_currency: RadrootsCoreCurrency::USD,
                line_subtotal: RadrootsCoreMoney::from_minor_units_u32(
                    2400,
                    RadrootsCoreCurrency::USD,
                ),
            }],
            discounts: Vec::new(),
            adjustments: Vec::new(),
            subtotal: RadrootsCoreMoney::from_minor_units_u32(2400, RadrootsCoreCurrency::USD),
            discount_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
            adjustment_total: RadrootsCoreMoney::zero(RadrootsCoreCurrency::USD),
            total: RadrootsCoreMoney::from_minor_units_u32(2400, RadrootsCoreCurrency::USD),
        }
    }

    fn assert_order_lifecycle_evidence_invalid(error: AppSqliteError) {
        assert!(
            matches!(
                error,
                AppSqliteError::InvalidProjection {
                    reason: "order lifecycle evidence is invalid"
                }
            ),
            "{error:?}"
        );
    }

    fn append_trade_signed_event_record(
        paths: &AppDesktopRuntimePaths,
        record_id: &str,
        event_id: &str,
        event_kind: i64,
        event_pubkey: &str,
        listing_addr: &str,
        event_tags_json: serde_json::Value,
        event_content: String,
    ) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let relay_delivery_json = RelayDeliveryEvidence::acknowledged(
            ["wss://relay.example"],
            ["wss://relay.example"],
            ["wss://relay.example"],
            Vec::new(),
        )
        .expect("acknowledged relay delivery evidence")
        .to_json_value()
        .expect("acknowledged relay delivery json");
        store
            .append_record(&LocalEventRecordInput {
                record_id: record_id.to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Test,
                created_at_ms: 1_774_000_020_000,
                inserted_at_ms: 1_774_000_020_001,
                owner_account_id: None,
                owner_pubkey: Some(event_pubkey.to_owned()),
                farm_id: None,
                listing_addr: Some(listing_addr.to_owned()),
                local_work_json: None,
                event_id: Some(event_id.to_owned()),
                event_kind: Some(event_kind),
                event_pubkey: Some(event_pubkey.to_owned()),
                event_created_at: Some(1_774_000_020),
                event_tags_json: Some(event_tags_json.clone()),
                event_content: Some(event_content.clone()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": event_id,
                    "kind": event_kind,
                    "pubkey": event_pubkey,
                    "tags": event_tags_json,
                    "content": event_content
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(relay_delivery_json),
            })
            .expect("append signed trade event");
    }

    fn mark_shared_seller_order_request_evidence_pending(paths: &AppDesktopRuntimePaths) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        executor
            .exec(
                "UPDATE local_event_record
                 SET status = 'pending_publish',
                     outbox_status = 'pending',
                     relay_set_fingerprint = NULL,
                     relay_delivery_json = NULL
                 WHERE record_id = 'app:signed_event:order-request:seller-order-decision-1'",
                "[]",
            )
            .expect("mark shared order request evidence pending");
    }

    fn append_unrelated_signed_event_records(paths: &AppDesktopRuntimePaths, count: usize) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate shared local events");
        let pubkey = "2222222222222222222222222222222222222222222222222222222222222222";

        for index in 0..count {
            let record_id = format!("app:signed_event:unrelated:{index}");
            let event_id = format!("event-{record_id}");
            store
                .append_record(&LocalEventRecordInput {
                    record_id,
                    family: LocalRecordFamily::SignedEvent,
                    status: LocalRecordStatus::Published,
                    source_runtime: SourceRuntime::Test,
                    created_at_ms: 1_774_000_100_000 + i64::try_from(index).unwrap_or_default(),
                    inserted_at_ms: 1_774_000_100_001 + i64::try_from(index).unwrap_or_default(),
                    owner_account_id: None,
                    owner_pubkey: Some(pubkey.to_owned()),
                    farm_id: None,
                    listing_addr: None,
                    local_work_json: None,
                    event_id: Some(event_id.clone()),
                    event_kind: Some(1),
                    event_pubkey: Some(pubkey.to_owned()),
                    event_created_at: Some(
                        1_774_000_100 + i64::try_from(index).unwrap_or_default(),
                    ),
                    event_tags_json: Some(json!([])),
                    event_content: Some("{}".to_owned()),
                    event_sig: Some("signature".to_owned()),
                    raw_event_json: Some(json!({
                        "id": event_id,
                        "kind": 1,
                        "pubkey": pubkey,
                        "content": "{}"
                    })),
                    outbox_status: PublishOutboxStatus::Acknowledged,
                    relay_set_fingerprint: Some("relay-set".to_owned()),
                    relay_delivery_json: Some(json!({
                        "state": "acknowledged",
                        "acknowledged_relays": ["wss://relay.example"]
                    })),
                })
                .expect("append unrelated signed event");
        }
    }

    fn deterministic_cli_listing_product_id(
        owner_pubkey: Option<&str>,
        listing_key: &str,
    ) -> ProductId {
        let seed = format!(
            "radroots-cli-listing:{}:{}",
            owner_pubkey.unwrap_or("unknown-owner"),
            listing_key.trim()
        );

        ProductId::from(uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_URL,
            seed.as_bytes(),
        ))
    }

    fn assert_detail_open_imports_shared_local_events_before_lookup(
        label: &str,
        section: PersonalSection,
    ) {
        let (runtime, paths) = bootstrapped_runtime(label);
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        assert_eq!(
            runtime
                .summary()
                .personal_projection
                .browse
                .listings
                .rows
                .len(),
            0
        );

        let listing_key = "DDDDDDDDDDDDDDDDDDDDDD";
        append_cli_signed_buyer_listing_record_with(
            &paths,
            "detail-open-pending-listing",
            listing_key,
            "Buyer Visible Eggs",
            1100,
        );
        let product_id =
            deterministic_cli_listing_product_id(Some("buyer-visible-seller-pubkey"), listing_key);

        assert!(
            runtime
                .open_personal_product_detail(section, product_id)
                .expect("buyer detail should import before lookup")
        );
        let summary = runtime.summary();
        let detail = match section {
            PersonalSection::Browse => summary.personal_projection.browse.detail,
            PersonalSection::Search => summary.personal_projection.search.detail,
            _ => None,
        }
        .expect("buyer detail should open from imported shared local events");

        assert_eq!(detail.listing.product_id, product_id);
        assert_eq!(detail.listing.title, "Buyer Visible Eggs");

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    fn local_work_record(
        record_id: &str,
        account_id: &str,
        farm_key: &str,
        listing_addr: Option<String>,
        payload: serde_json::Value,
    ) -> LocalEventRecordInput {
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::Cli,
            created_at_ms: 1000,
            inserted_at_ms: 1001,
            owner_account_id: Some(account_id.to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some(farm_key.to_owned()),
            listing_addr,
            local_work_json: Some(payload),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status: PublishOutboxStatus::None,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        }
    }

    fn published_operation_receipt_fixture(
        source_account_id: String,
        source_local_event_id: Option<String>,
        event_id: &str,
    ) -> AppPublishedOperationReceipt {
        let event_pubkey = "1111111111111111111111111111111111111111111111111111111111111111";
        AppPublishedOperationReceipt {
            operation_key: "farm:upsert".to_owned(),
            source_account_id,
            source_local_event_id,
            listing_addr: None,
            event_id: event_id.to_owned(),
            event_kind: 30340,
            event_pubkey: event_pubkey.to_owned(),
            event_created_at: 1_774_000_000,
            event_tags_json: json!([["d", "farm-key"]]),
            event_content: "{}".to_owned(),
            event_sig: "signature".to_owned(),
            raw_event_json: json!({
                "id": event_id,
                "kind": 30340,
                "pubkey": event_pubkey,
                "content": "{}"
            }),
            relay_set_fingerprint: "relay-set".to_owned(),
            relay_delivery_json: json!({
                "state": "acknowledged",
                "acknowledged_relays": ["ws://127.0.0.1:1234/"]
            }),
        }
    }

    fn published_receipt_event(
        receipt: &AppPublishedOperationReceipt,
    ) -> radroots_sdk::RadrootsNostrEvent {
        radroots_sdk::RadrootsNostrEvent {
            id: receipt.event_id.clone(),
            author: receipt.event_pubkey.clone(),
            created_at: receipt.event_created_at,
            kind: receipt.event_kind,
            tags: serde_json::from_value(receipt.event_tags_json.clone())
                .expect("receipt event tags should decode"),
            content: receipt.event_content.clone(),
            sig: receipt.event_sig.clone(),
        }
    }

    fn shared_local_event_records(paths: &AppDesktopRuntimePaths) -> Vec<LocalEventRecord> {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store
            .list_records_after_seq(0, 100)
            .expect("shared local records should list")
    }

    fn shared_seller_order_decision_event(
        paths: &AppDesktopRuntimePaths,
        seller_pubkey: &str,
    ) -> radroots_sdk::RadrootsNostrEvent {
        let record = shared_local_event_records(paths)
            .into_iter()
            .find(|record| {
                record.family == LocalRecordFamily::SignedEvent
                    && record.event_kind == Some(3423)
                    && record.event_pubkey.as_deref() == Some(seller_pubkey)
            })
            .expect("shared seller order decision record should exist");
        signed_event_from_local_record(&record)
            .expect("shared seller order decision record should decode")
            .expect("shared seller order decision record should contain signed event")
    }

    fn shared_order_events_by_kind(
        paths: &AppDesktopRuntimePaths,
        kind: i64,
        pubkey: &str,
    ) -> Vec<radroots_sdk::RadrootsNostrEvent> {
        shared_local_event_records(paths)
            .into_iter()
            .filter(|record| {
                record.family == LocalRecordFamily::SignedEvent
                    && record.event_kind == Some(kind)
                    && record.event_pubkey.as_deref() == Some(pubkey)
            })
            .filter_map(|record| {
                signed_event_from_local_record(&record)
                    .expect("shared signed event record should decode")
            })
            .collect()
    }

    fn event_has_tag(event: &radroots_sdk::RadrootsNostrEvent, key: &str, value: &str) -> bool {
        event.tags.iter().any(|tag| {
            tag.first().map(String::as_str) == Some(key)
                && tag.get(1).map(String::as_str) == Some(value)
        })
    }

    fn event_has_nonempty_value_tag(event: &radroots_sdk::RadrootsNostrEvent, key: &str) -> bool {
        event.tags.iter().any(|tag| {
            tag.first().map(String::as_str) == Some(key)
                && tag.get(1).map(|value| !value.is_empty()).unwrap_or(false)
        })
    }

    fn persisted_order_status(runtime: &DesktopAppRuntime, order_id: OrderId) -> String {
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .query_row(
                "select status from orders where id = ?1 limit 1",
                [order_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .expect("order status should load")
    }

    fn set_persisted_order_status(runtime: &DesktopAppRuntime, order_id: OrderId, status: &str) {
        let order_id = order_id.to_string();
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute(
                "update orders set status = ?1 where id = ?2",
                [status, order_id.as_str()],
            )
            .expect("order status should update");
    }

    fn pending_order_sync_payloads(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        order_id: OrderId,
    ) -> Vec<String> {
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id)
            .expect("pending sync operations should load")
            .into_iter()
            .filter(|pending| {
                pending.operation.operation == SyncOperationKind::Upsert
                    && matches!(pending.operation.aggregate, SyncAggregateRef::Order(id) if id == order_id)
            })
            .map(|pending| pending.operation.payload_json)
            .collect()
    }

    fn pending_order_request_publish_payloads(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        order_id: OrderId,
    ) -> Vec<AppOrderRequestPublishPayload> {
        pending_order_sync_payloads(runtime, account_id, order_id)
            .into_iter()
            .map(|payload_json| {
                match serde_json::from_str::<AppPublishPayload>(payload_json.as_str())
                    .expect("pending order payload should be typed app publish work")
                {
                    AppPublishPayload::OrderRequest(payload) => payload,
                    payload => panic!("expected order request publish payload, got {payload:?}"),
                }
            })
            .collect()
    }

    fn assert_single_order_request_publish_payload(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        order_id: OrderId,
        farm_id: FarmId,
        status: &str,
    ) -> AppOrderRequestPublishPayload {
        let pending_payloads =
            pending_order_request_publish_payloads(runtime, account_id, order_id);
        assert_eq!(pending_payloads.len(), 1);
        let payload = pending_payloads
            .into_iter()
            .next()
            .expect("single order request publish payload");
        assert_eq!(payload.context.account_id, account_id);
        assert_eq!(payload.context.source, "place_personal_order");
        assert_eq!(payload.order_id, order_id);
        assert_eq!(payload.farm_id, farm_id);
        assert_eq!(payload.status.as_deref(), Some(status));
        payload
    }

    fn buyer_order_local_work_record_ids(paths: &AppDesktopRuntimePaths) -> Vec<String> {
        shared_local_event_records(paths)
            .into_iter()
            .filter(|record| {
                record.source_runtime == SourceRuntime::App
                    && record
                        .local_work_json
                        .as_ref()
                        .and_then(|payload| payload["record_kind"].as_str())
                        == Some(BUYER_ORDER_REQUEST_LOCAL_WORK_RECORD_KIND)
            })
            .map(|record| record.record_id)
            .collect()
    }

    fn blocked_buyer_order_runtime(
        label: &str,
    ) -> (
        DesktopAppRuntime,
        AppDesktopRuntimePaths,
        String,
        OrderId,
        FarmId,
    ) {
        let (runtime, paths) = bootstrapped_runtime(label);
        let _ = install_recorded_sync_transport(
            &runtime,
            RecordedAppSyncTransport::fail(AppSyncTransportError::unavailable(
                "test sync unavailable",
            )),
        );
        assert!(
            runtime
                .generate_local_account(Some("Buyer".to_owned()))
                .expect("account should generate")
        );
        let buyer_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        assert!(
            runtime
                .select_active_surface(ActiveSurface::Personal)
                .expect("surface should switch into marketplace")
        );
        let listing_key = "DDDDDDDDDDDDDDDDDDDDDD";
        append_cli_signed_buyer_listing_record_with(
            &paths,
            "buyer-order-append-failure-listing",
            listing_key,
            "Buyer Visible Eggs",
            1100,
        );
        let product_id =
            deterministic_cli_listing_product_id(Some("buyer-visible-seller-pubkey"), listing_key);
        assert!(
            runtime
                .open_personal_product_detail(PersonalSection::Browse, product_id)
                .expect("buyer detail should import before lookup")
        );
        assert!(
            runtime
                .add_personal_product_to_cart(PersonalSection::Browse, false)
                .expect("buyer product should add to cart")
        );
        assert!(
            runtime
                .save_personal_order_review_draft(BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.com".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                })
                .expect("buyer order review draft should save")
        );
        block_shared_local_events_database(&paths);

        let error = runtime
            .place_personal_order()
            .expect_err("blocked local events should fail order completion");

        assert!(matches!(error, AppSqliteError::LocalEventsSql { .. }));
        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Orders)
        );
        assert!(
            summary
                .personal_projection
                .orders
                .has_recoverable_coordination
        );
        assert!(summary.personal_projection.cart.cart.lines.is_empty());
        assert!(
            !summary
                .personal_projection
                .cart
                .order_review
                .can_place_order
        );
        assert_eq!(summary.personal_projection.orders.list.rows.len(), 1);
        let visible_order_id = summary.personal_projection.orders.list.rows[0].order_id;
        let order_detail = summary
            .personal_projection
            .orders
            .detail
            .as_ref()
            .expect("buyer order detail should remain visible after coordination failure");
        assert_eq!(order_detail.order_id, visible_order_id);
        let order_farm_id = order_detail.farm_id;
        {
            let state = runtime.lock_state_mut();
            let buyer_context = state.state_store.identity_projection().buyer_context();
            let sqlite_store = state.sqlite_store.as_ref().expect("sqlite store");
            let buyer_orders = sqlite_store
                .load_buyer_orders(&buyer_context)
                .expect("buyer order should persist after coordination failure");
            assert_eq!(buyer_orders.rows.len(), 1);
            let order_id = buyer_orders.rows[0].order_id;
            assert_eq!(order_id, visible_order_id);
            let coordination = sqlite_store
                .load_buyer_order_coordination_record(&buyer_context, order_id)
                .expect("buyer order coordination should load")
                .expect("buyer order coordination should exist");
            assert_eq!(coordination.state, BuyerOrderCoordinationState::Failed);
            assert_eq!(coordination.attempt_count, 1);
            assert!(coordination.record_id.is_some());
            assert!(coordination.payload_json.is_some());
            assert!(coordination.last_error_message.is_some());
        }
        assert!(
            pending_order_sync_payloads(&runtime, buyer_account_id.as_str(), visible_order_id)
                .is_empty()
        );

        (
            runtime,
            paths,
            buyer_account_id,
            visible_order_id,
            order_farm_id,
        )
    }

    fn block_shared_local_events_database(paths: &AppDesktopRuntimePaths) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).expect("shared local events directory should create");
        }
        if database_path.is_file() {
            fs::remove_file(&database_path).expect("shared local events file should remove");
        } else if database_path.is_dir() {
            fs::remove_dir_all(&database_path)
                .expect("shared local events directory should remove");
        }
        fs::create_dir(&database_path).expect("blocking directory should create");
    }

    fn unblock_shared_local_events_database(paths: &AppDesktopRuntimePaths) {
        let database_path = paths
            .shared_local_events_database_path()
            .expect("shared local events path");
        if database_path.is_dir() {
            fs::remove_dir_all(&database_path).expect("blocking directory should remove");
        }
    }

    fn fixture_pending_session() -> RadrootsAppRemoteSignerPendingSession {
        let signer_identity = RadrootsIdentity::from_secret_key_str(
            "1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("signer identity");
        let client_identity = RadrootsIdentity::from_secret_key_str(
            "3333333333333333333333333333333333333333333333333333333333333333",
        )
        .expect("client identity");

        RadrootsAppRemoteSignerPendingSession {
            record: RadrootsAppRemoteSignerSessionRecord::pending(
                client_identity.to_public(),
                signer_identity.to_public(),
                vec!["ws://127.0.0.1:8080".to_owned()],
            ),
            client_secret_key_hex: client_identity.secret_key_hex(),
        }
    }

    fn save_surface_activation(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        active_surface: ActiveSurface,
        farmer_active: bool,
    ) {
        let activation = AccountSurfaceActivationProjection::new(
            account_id,
            SelectedSurfaceProjection::new(active_surface),
            if farmer_active {
                FarmerActivationProjection::active(FarmId::new())
            } else {
                FarmerActivationProjection::inactive()
            },
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_surface_activation(&activation)
            .expect("surface activation should save");
    }

    fn save_farmer_surface_activation(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        active_surface: ActiveSurface,
    ) -> FarmId {
        let farm_id = FarmId::new();
        let activation = AccountSurfaceActivationProjection::new(
            account_id,
            SelectedSurfaceProjection::new(active_surface),
            FarmerActivationProjection::active(farm_id),
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_surface_activation(&activation)
            .expect("surface activation should save");
        farm_id
    }

    fn seed_product(
        runtime: &DesktopAppRuntime,
        farm_id: FarmId,
        title: &str,
        subtitle: &str,
        status: &str,
        stock_count: Option<u32>,
        updated_at: &str,
    ) -> radroots_studio_app_view::ProductId {
        let product_id = radroots_studio_app_view::ProductId::new();
        let stock_count = stock_count
            .map(|value| value.to_string())
            .unwrap_or_else(|| "null".to_owned());
        let sql = format!(
            "insert into products (
                id,
                farm_id,
                title,
                subtitle,
                status,
                unit_label,
                price_minor_units,
                price_currency,
                stock_count,
                availability_window_id,
                updated_at
            ) values (
                '{product_id}',
                '{farm_id}',
                '{title}',
                '{subtitle}',
                '{status}',
                'box',
                600,
                'USD',
                {stock_count},
                null,
                '{updated_at}'
            )",
            product_id = product_id,
            farm_id = farm_id,
            title = title,
            subtitle = subtitle,
            status = status,
            stock_count = stock_count,
            updated_at = updated_at,
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("product should seed");

        product_id
    }

    fn seed_buyer_marketplace_support(
        runtime: &DesktopAppRuntime,
        account_id: &str,
        farm_id: FarmId,
        farm_display_name: &str,
        fulfillment_label: &str,
    ) -> FulfillmentWindowId {
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let sql = format!(
            "insert into pickup_locations (
                id,
                farm_id,
                label,
                address_line,
                directions,
                is_default,
                created_at,
                updated_at
             ) values (
                '{pickup_location_id}',
                '{farm_id}',
                'North barn',
                '14 County Road',
                null,
                1,
                '2026-04-20T08:00:00Z',
                '2026-04-20T08:00:00Z'
             );
             insert into fulfillment_windows (
                id,
                farm_id,
                starts_at,
                ends_at,
                capacity_limit,
                created_at,
                updated_at,
                pickup_location_id,
                label,
                order_cutoff_at
             ) values (
                '{fulfillment_window_id}',
                '{farm_id}',
                '2099-04-18T16:00:00Z',
                '2099-04-18T18:00:00Z',
                null,
                '2099-04-18T16:00:00Z',
                '2099-04-18T16:00:00Z',
                '{pickup_location_id}',
                '{fulfillment_label}',
                '2099-04-17T18:00:00Z'
             );
             insert into account_farm_setups (
                account_id,
                farm_name,
                location_or_service_area,
                pickup_enabled,
                delivery_enabled,
                shipping_enabled,
                saved_farm_id,
                saved_farm_display_name,
                saved_farm_readiness,
                updated_at
             ) values (
                '{account_id}',
                '{farm_display_name}',
                'County Road',
                1,
                0,
                0,
                '{farm_id}',
                '{farm_display_name}',
                'ready',
                '2026-04-20T08:00:00Z'
             )
             on conflict(account_id) do update set
                farm_name = excluded.farm_name,
                location_or_service_area = excluded.location_or_service_area,
                pickup_enabled = excluded.pickup_enabled,
                delivery_enabled = excluded.delivery_enabled,
                shipping_enabled = excluded.shipping_enabled,
                saved_farm_id = excluded.saved_farm_id,
                saved_farm_display_name = excluded.saved_farm_display_name,
                saved_farm_readiness = excluded.saved_farm_readiness,
                updated_at = excluded.updated_at;"
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("buyer marketplace support should seed");

        fulfillment_window_id
    }

    fn provision_ready_farmer_account(runtime: &DesktopAppRuntime) -> (String, FarmId) {
        assert!(
            runtime
                .generate_local_account(Some("Farmer".to_owned()))
                .expect("account should generate")
        );
        let account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("selected account")
            .account
            .account_id
            .clone();
        let farm_id =
            save_farmer_surface_activation(runtime, account_id.as_str(), ActiveSurface::Farmer);
        let farm_setup_projection = FarmSetupProjection::from_saved_farm(FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: FarmReadiness::Ready,
        });
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_summary(
                farm_setup_projection
                    .saved_farm
                    .as_ref()
                    .expect("saved farm should exist"),
            )
            .expect("farm summary should save");
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .save_farm_setup(account_id.as_str(), &farm_setup_projection)
            .expect("farm setup should save");
        assert!(
            runtime
                .select_local_account(account_id.as_str())
                .expect("account should select")
        );

        (account_id, farm_id)
    }

    fn seed_order_workspace(
        runtime: &DesktopAppRuntime,
        farm_id: FarmId,
    ) -> (FulfillmentWindowId, OrderId) {
        let pickup_location_id = PickupLocationId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let order_id = OrderId::new();
        let sql = format!(
            "insert into pickup_locations (
                id,
                farm_id,
                label,
                address_line,
                directions,
                is_default,
                created_at,
                updated_at
             ) values (
                '{pickup_location_id}',
                '{farm_id}',
                'North barn',
                '14 County Road',
                null,
                1,
                '2026-04-17T08:00:00Z',
                '2026-04-17T08:00:00Z'
             );
             insert into fulfillment_windows (
                id,
                farm_id,
                starts_at,
                ends_at,
                capacity_limit,
                created_at,
                updated_at,
                pickup_location_id,
                label,
                order_cutoff_at
             ) values (
                '{fulfillment_window_id}',
                '{farm_id}',
                '2099-04-18T16:00:00Z',
                '2099-04-18T18:00:00Z',
                null,
                '2099-04-18T16:00:00Z',
                '2099-04-18T16:00:00Z',
                '{pickup_location_id}',
                'Friday pickup',
                '2099-04-17T18:00:00Z'
             );
             insert into orders (
                id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                updated_at
             ) values (
                '{order_id}',
                '{farm_id}',
                '{fulfillment_window_id}',
                'R-100',
                'Casey',
                'needs_action',
                '2026-04-17T10:00:00Z'
             );
             insert into order_lines (
                id,
                order_id,
                title,
                quantity_value,
                quantity_unit_label,
                quantity_display,
                sort_index
             ) values (
                'line-1',
                '{order_id}',
                'Salad mix',
                2,
                'bags',
                '2 bags',
                0
             )",
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("orders workspace should seed");

        (fulfillment_window_id, order_id)
    }

    fn seed_second_order_workspace(
        runtime: &DesktopAppRuntime,
        farm_id: FarmId,
        source_fulfillment_window_id: FulfillmentWindowId,
    ) -> (FulfillmentWindowId, OrderId) {
        let fulfillment_window_id = FulfillmentWindowId::new();
        let order_id = OrderId::new();
        let sql = format!(
            "insert into fulfillment_windows (
                id,
                farm_id,
                starts_at,
                ends_at,
                capacity_limit,
                created_at,
                updated_at,
                pickup_location_id,
                label,
                order_cutoff_at
             )
             select
                '{fulfillment_window_id}',
                farm_id,
                '2099-04-19T16:00:00Z',
                '2099-04-19T18:00:00Z',
                capacity_limit,
                '2099-04-19T16:00:00Z',
                '2099-04-19T16:00:00Z',
                pickup_location_id,
                'Saturday pickup',
                '2099-04-18T18:00:00Z'
             from fulfillment_windows
             where id = '{source_fulfillment_window_id}' and farm_id = '{farm_id}';
             insert into orders (
                id,
                farm_id,
                fulfillment_window_id,
                order_number,
                customer_display_name,
                status,
                updated_at
             ) values (
                '{order_id}',
                '{farm_id}',
                '{fulfillment_window_id}',
                'R-101',
                'Robin',
                'scheduled',
                '2026-04-17T11:00:00Z'
             )"
        );
        runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .connection()
            .execute_batch(&sql)
            .expect("second orders workspace should seed");

        (fulfillment_window_id, order_id)
    }

    fn cleanup_paths(paths: &AppSharedAccountsPaths) {
        let Some(base) = paths.data_root.ancestors().nth(3).map(PathBuf::from) else {
            return;
        };
        let _ = fs::remove_dir_all(base);
    }
}
