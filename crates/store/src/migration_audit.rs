use std::collections::{BTreeMap, BTreeSet};

use radroots_studio_app_sync::{AppPublishPayload, SyncOperationKind};
use radroots_events::kinds::{
    KIND_FARM, KIND_LISTING, KIND_LISTING_DRAFT, KIND_ORDER_CANCELLATION, KIND_ORDER_DECISION,
    KIND_ORDER_FULFILLMENT_UPDATE, KIND_ORDER_PAYMENT_RECORD, KIND_ORDER_RECEIPT,
    KIND_ORDER_REQUEST, KIND_ORDER_REVISION_DECISION, KIND_ORDER_REVISION_PROPOSAL,
    KIND_ORDER_SETTLEMENT_DECISION, KIND_TRADE_VALIDATION_RECEIPT,
};
use radroots_local_events::{
    LocalEventRecord, LocalEventsStore, LocalRecordFamily, LocalRecordStatus, PublishOutboxStatus,
};
use radroots_sql_core::SqlExecutor;
use rusqlite::params;
use serde_json::Value;

use crate::{
    AppSdkMigrationReceipt, AppSdkMigrationReceiptSourceKind, AppSdkMigrationState, AppSqliteError,
    AppSqliteStore,
};

pub const APP_SDK_MIGRATION_AUDIT_DEFAULT_BATCH_SIZE: u32 = 500;
pub const APP_SDK_MIGRATION_AUDIT_MAX_BATCH_SIZE: u32 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditRequest {
    pub batch_size: u32,
}

impl Default for AppSdkMigrationAuditRequest {
    fn default() -> Self {
        Self {
            batch_size: APP_SDK_MIGRATION_AUDIT_DEFAULT_BATCH_SIZE,
        }
    }
}

impl AppSdkMigrationAuditRequest {
    pub fn normalized_batch_size(self) -> u32 {
        if self.batch_size == 0 {
            APP_SDK_MIGRATION_AUDIT_DEFAULT_BATCH_SIZE
        } else {
            self.batch_size.min(APP_SDK_MIGRATION_AUDIT_MAX_BATCH_SIZE)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditReport {
    pub local_outbox: AppSdkMigrationAuditSourceReport,
    pub shared_local_events: AppSdkMigrationAuditSourceReport,
    pub issues: Vec<AppSdkMigrationAuditIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditSourceReport {
    pub source: AppSdkMigrationAuditSource,
    pub batch_size: u32,
    pub batch_count: u64,
    pub scanned_records: u64,
    pub kind_counts: Vec<AppSdkMigrationAuditCount>,
    pub status_counts: Vec<AppSdkMigrationAuditCount>,
    pub classification_counts: Vec<AppSdkMigrationAuditCount>,
    pub duplicate_candidates: Vec<AppSdkMigrationAuditDuplicateCandidate>,
    pub issues: Vec<AppSdkMigrationAuditIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditCount {
    pub key: String,
    pub count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditDuplicateCandidate {
    pub identity_kind: String,
    pub identity_key: String,
    pub record_count: u64,
    pub record_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkMigrationAuditIssue {
    pub source: AppSdkMigrationAuditSource,
    pub code: String,
    pub record_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkMigrationAuditSource {
    LocalOutbox,
    SharedLocalEvents,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkMigrationAuditClassification {
    PublishableCandidate,
    AlreadyRepresentedCandidate,
    RepresentedRecord,
    SkippedRecord,
    FailedRecord,
    LocalWorkDeferred,
    ManualReviewRequired,
    PaymentDeferred,
    SettlementDeferred,
    ValidationReceiptDeferred,
    Unsupported,
    Unknown,
}

impl AppSdkMigrationAuditClassification {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::PublishableCandidate => "publishable_candidate",
            Self::AlreadyRepresentedCandidate => "already_represented_candidate",
            Self::RepresentedRecord => "represented_record",
            Self::SkippedRecord => "skipped_record",
            Self::FailedRecord => "failed_record",
            Self::LocalWorkDeferred => "local_work_deferred",
            Self::ManualReviewRequired => "manual_review_required",
            Self::PaymentDeferred => "payment_deferred",
            Self::SettlementDeferred => "settlement_deferred",
            Self::ValidationReceiptDeferred => "validation_receipt_deferred",
            Self::Unsupported => "unsupported",
            Self::Unknown => "unknown",
        }
    }
}

impl AppSqliteStore {
    pub fn audit_sdk_migration<E>(
        &self,
        shared_local_events: &LocalEventsStore<E>,
        request: AppSdkMigrationAuditRequest,
    ) -> Result<AppSdkMigrationAuditReport, AppSqliteError>
    where
        E: SqlExecutor,
    {
        let local_outbox = self.audit_sdk_migration_local_outbox(request)?;
        let shared_local_events =
            self.audit_sdk_migration_shared_local_events(shared_local_events, request)?;
        let issues = local_outbox
            .issues
            .iter()
            .chain(shared_local_events.issues.iter())
            .cloned()
            .collect();

        Ok(AppSdkMigrationAuditReport {
            local_outbox,
            shared_local_events,
            issues,
        })
    }

    pub fn audit_sdk_migration_local_outbox(
        &self,
        request: AppSdkMigrationAuditRequest,
    ) -> Result<AppSdkMigrationAuditSourceReport, AppSqliteError> {
        let batch_size = request.normalized_batch_size();
        let mut report = AppSdkMigrationAuditSourceBuilder::new(
            AppSdkMigrationAuditSource::LocalOutbox,
            batch_size,
        );
        let mut last_rowid = 0_i64;

        loop {
            let rows = self.load_local_outbox_audit_batch(last_rowid, batch_size)?;
            if rows.is_empty() {
                break;
            }
            report.batch_count += 1;
            for row in &rows {
                last_rowid = row.rowid;
                let receipt = self.sdk_migration_receipt_repository().load_receipt(
                    AppSdkMigrationReceiptSourceKind::LocalOutbox,
                    row.id.as_str(),
                )?;
                audit_local_outbox_row(row, receipt.as_ref(), &mut report);
            }
            if rows.len() < batch_size as usize {
                break;
            }
        }

        Ok(report.finish())
    }

    pub fn audit_sdk_migration_shared_local_events<E>(
        &self,
        store: &LocalEventsStore<E>,
        request: AppSdkMigrationAuditRequest,
    ) -> Result<AppSdkMigrationAuditSourceReport, AppSqliteError>
    where
        E: SqlExecutor,
    {
        audit_sdk_migration_shared_local_events_with_receipts(store, request, |record_id| {
            self.sdk_migration_receipt_repository().load_receipt(
                AppSdkMigrationReceiptSourceKind::SharedLocalEvent,
                record_id,
            )
        })
    }
}

fn audit_sdk_migration_shared_local_events_with_receipts<E>(
    store: &LocalEventsStore<E>,
    request: AppSdkMigrationAuditRequest,
    mut load_receipt: impl FnMut(&str) -> Result<Option<AppSdkMigrationReceipt>, AppSqliteError>,
) -> Result<AppSdkMigrationAuditSourceReport, AppSqliteError>
where
    E: SqlExecutor,
{
    let batch_size = request.normalized_batch_size();
    let mut report = AppSdkMigrationAuditSourceBuilder::new(
        AppSdkMigrationAuditSource::SharedLocalEvents,
        batch_size,
    );
    let mut after_change_seq = 0_i64;

    loop {
        let records = store
            .list_records_changed_after(after_change_seq, batch_size)
            .map_err(|source| AppSqliteError::LocalEvents {
                operation: "audit shared local event records",
                source,
            })?;
        if records.is_empty() {
            break;
        }
        report.batch_count += 1;
        for record in &records {
            after_change_seq = record.change_seq;
            let receipt = load_receipt(record.record_id.as_str())?;
            audit_shared_local_event_record(record, receipt.as_ref(), &mut report);
        }
        if records.len() < batch_size as usize {
            break;
        }
    }

    Ok(report.finish())
}

impl AppSqliteStore {
    fn load_local_outbox_audit_batch(
        &self,
        after_rowid: i64,
        limit: u32,
    ) -> Result<Vec<LocalOutboxAuditRow>, AppSqliteError> {
        let mut statement = self
            .connection()
            .prepare(
                "SELECT
                    rowid,
                    id,
                    account_id,
                    operation_key,
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    state
                 FROM local_outbox
                 WHERE rowid > ?1
                 ORDER BY rowid ASC
                 LIMIT ?2",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare SDK migration local outbox audit query",
                source,
            })?;
        let rows = statement
            .query_map(params![after_rowid, i64::from(limit)], |row| {
                Ok(LocalOutboxAuditRow {
                    rowid: row.get(0)?,
                    id: row.get(1)?,
                    account_id: row.get(2)?,
                    operation_key: row.get(3)?,
                    aggregate_kind: row.get(4)?,
                    aggregate_id: row.get(5)?,
                    operation_kind: row.get(6)?,
                    payload_json: row.get(7)?,
                    state: row.get(8)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query SDK migration local outbox audit rows",
                source,
            })?;

        rows.map(|row| {
            row.map_err(|source| AppSqliteError::Query {
                operation: "read SDK migration local outbox audit row",
                source,
            })
        })
        .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalOutboxAuditRow {
    rowid: i64,
    id: String,
    account_id: String,
    operation_key: String,
    aggregate_kind: String,
    aggregate_id: String,
    operation_kind: String,
    payload_json: String,
    state: String,
}

struct AppSdkMigrationAuditSourceBuilder {
    source: AppSdkMigrationAuditSource,
    batch_size: u32,
    batch_count: u64,
    scanned_records: u64,
    kind_counts: BTreeMap<String, u64>,
    status_counts: BTreeMap<String, u64>,
    classification_counts: BTreeMap<String, u64>,
    duplicate_records: BTreeMap<DuplicateIdentity, BTreeSet<String>>,
    issues: Vec<AppSdkMigrationAuditIssue>,
}

impl AppSdkMigrationAuditSourceBuilder {
    fn new(source: AppSdkMigrationAuditSource, batch_size: u32) -> Self {
        Self {
            source,
            batch_size,
            batch_count: 0,
            scanned_records: 0,
            kind_counts: BTreeMap::new(),
            status_counts: BTreeMap::new(),
            classification_counts: BTreeMap::new(),
            duplicate_records: BTreeMap::new(),
            issues: Vec::new(),
        }
    }

    fn record(
        &mut self,
        record_id: &str,
        kind: String,
        status: String,
        classification: AppSdkMigrationAuditClassification,
        duplicate_identities: Vec<DuplicateIdentity>,
    ) {
        self.scanned_records += 1;
        increment_count(&mut self.kind_counts, kind);
        increment_count(&mut self.status_counts, status);
        increment_count(
            &mut self.classification_counts,
            classification.storage_key().to_owned(),
        );
        for identity in duplicate_identities {
            self.duplicate_records
                .entry(identity)
                .or_default()
                .insert(record_id.to_owned());
        }
    }

    fn issue(&mut self, code: &str, record_id: Option<&str>, message: impl Into<String>) {
        self.issues.push(AppSdkMigrationAuditIssue {
            source: self.source,
            code: code.to_owned(),
            record_id: record_id.map(ToOwned::to_owned),
            message: message.into(),
        });
    }

    fn finish(self) -> AppSdkMigrationAuditSourceReport {
        AppSdkMigrationAuditSourceReport {
            source: self.source,
            batch_size: self.batch_size,
            batch_count: self.batch_count,
            scanned_records: self.scanned_records,
            kind_counts: counts_from_map(self.kind_counts),
            status_counts: counts_from_map(self.status_counts),
            classification_counts: counts_from_map(self.classification_counts),
            duplicate_candidates: duplicate_candidates_from_map(self.duplicate_records),
            issues: self.issues,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DuplicateIdentity {
    kind: String,
    key: String,
}

fn audit_local_outbox_row(
    row: &LocalOutboxAuditRow,
    receipt: Option<&AppSdkMigrationReceipt>,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) {
    let payload = serde_json::from_str::<AppPublishPayload>(row.payload_json.as_str());
    let (kind, source_classification) = match payload {
        Ok(payload) => {
            if row.operation_kind == SyncOperationKind::Delete.storage_key() {
                report.issue(
                    "unsupported_local_outbox_operation",
                    Some(row.id.as_str()),
                    format!(
                        "local outbox delete operation `{}` is not a SDK publish migration candidate",
                        row.operation_key
                    ),
                );
                (
                    payload.work_kind().storage_key().to_owned(),
                    AppSdkMigrationAuditClassification::Unsupported,
                )
            } else {
                (
                    payload.work_kind().storage_key().to_owned(),
                    classify_local_outbox_state(row, report),
                )
            }
        }
        Err(source) => {
            report.issue(
                "unknown_local_outbox_payload",
                Some(row.id.as_str()),
                format!(
                    "local outbox payload for operation `{}` could not be decoded: {source}",
                    row.operation_key
                ),
            );
            (
                format!("{}:{}", row.aggregate_kind, row.operation_kind),
                AppSdkMigrationAuditClassification::Unknown,
            )
        }
    };
    let classification =
        classify_receipt_overlay(row.id.as_str(), source_classification, receipt, report);
    let identities = vec![
        DuplicateIdentity {
            kind: "operation".to_owned(),
            key: format!("{}:{}", row.account_id, row.operation_key),
        },
        DuplicateIdentity {
            kind: "aggregate".to_owned(),
            key: format!(
                "{}:{}:{}:{}",
                row.account_id, row.aggregate_kind, row.aggregate_id, row.operation_kind
            ),
        },
    ];
    report.record(
        row.id.as_str(),
        kind,
        row.state.clone(),
        classification,
        identities,
    );
}

fn classify_local_outbox_state(
    row: &LocalOutboxAuditRow,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) -> AppSdkMigrationAuditClassification {
    match row.state.as_str() {
        "pending" | "in_progress" | "retryable" => {
            AppSdkMigrationAuditClassification::PublishableCandidate
        }
        "succeeded" => AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate,
        "failed" | "blocked" => {
            report.issue(
                "manual_review_local_outbox_state",
                Some(row.id.as_str()),
                format!(
                    "local outbox operation `{}` is in `{}` state and requires migration review",
                    row.operation_key, row.state
                ),
            );
            AppSdkMigrationAuditClassification::ManualReviewRequired
        }
        _ => {
            report.issue(
                "unknown_local_outbox_state",
                Some(row.id.as_str()),
                format!(
                    "local outbox operation `{}` has unknown state `{}`",
                    row.operation_key, row.state
                ),
            );
            AppSdkMigrationAuditClassification::Unknown
        }
    }
}

fn audit_shared_local_event_record(
    record: &LocalEventRecord,
    receipt: Option<&AppSdkMigrationReceipt>,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) {
    let kind = shared_local_event_kind(record);
    let source_classification = shared_local_event_classification(record, report);
    let classification = classify_receipt_overlay(
        record.record_id.as_str(),
        source_classification,
        receipt,
        report,
    );
    report.record(
        record.record_id.as_str(),
        kind,
        format!(
            "{}:{}",
            record.status.as_str(),
            record.outbox_status.as_str()
        ),
        classification,
        shared_local_event_duplicate_identities(record),
    );
}

fn classify_receipt_overlay(
    record_id: &str,
    source_classification: AppSdkMigrationAuditClassification,
    receipt: Option<&AppSdkMigrationReceipt>,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) -> AppSdkMigrationAuditClassification {
    let Some(receipt) = receipt else {
        return source_classification;
    };
    if !receipt_allowed_for_source_classification(source_classification) {
        report.issue(
            "sdk_migration_receipt_for_non_migratable_source",
            Some(record_id),
            format!(
                "SDK migration receipt `{}` for operation `{}` cannot override source classification `{}`",
                receipt.id,
                receipt.sdk_operation_kind,
                source_classification.storage_key()
            ),
        );
        return source_classification;
    }

    match receipt.migration_state {
        AppSdkMigrationState::Pending | AppSdkMigrationState::Prepared => source_classification,
        AppSdkMigrationState::Enqueued | AppSdkMigrationState::Pushed => {
            AppSdkMigrationAuditClassification::RepresentedRecord
        }
        AppSdkMigrationState::Skipped => AppSdkMigrationAuditClassification::SkippedRecord,
        AppSdkMigrationState::Failed => {
            report.issue(
                "sdk_migration_receipt_failed",
                Some(record_id),
                format!(
                    "SDK migration receipt `{}` for operation `{}` is failed",
                    receipt.id, receipt.sdk_operation_kind
                ),
            );
            AppSdkMigrationAuditClassification::FailedRecord
        }
        AppSdkMigrationState::Blocked | AppSdkMigrationState::ManualReview => {
            report.issue(
                "sdk_migration_receipt_manual_review",
                Some(record_id),
                format!(
                    "SDK migration receipt `{}` for operation `{}` requires manual review",
                    receipt.id, receipt.sdk_operation_kind
                ),
            );
            AppSdkMigrationAuditClassification::ManualReviewRequired
        }
        AppSdkMigrationState::Unsupported => {
            report.issue(
                "sdk_migration_receipt_unsupported",
                Some(record_id),
                format!(
                    "SDK migration receipt `{}` for operation `{}` is unsupported",
                    receipt.id, receipt.sdk_operation_kind
                ),
            );
            AppSdkMigrationAuditClassification::Unsupported
        }
        AppSdkMigrationState::Unknown => {
            report.issue(
                "sdk_migration_receipt_unknown",
                Some(record_id),
                format!(
                    "SDK migration receipt `{}` for operation `{}` is unknown",
                    receipt.id, receipt.sdk_operation_kind
                ),
            );
            AppSdkMigrationAuditClassification::Unknown
        }
    }
}

fn receipt_allowed_for_source_classification(
    classification: AppSdkMigrationAuditClassification,
) -> bool {
    matches!(
        classification,
        AppSdkMigrationAuditClassification::PublishableCandidate
            | AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate
    )
}

fn shared_local_event_kind(record: &LocalEventRecord) -> String {
    match record.family {
        LocalRecordFamily::LocalWork => record
            .local_work_json
            .as_ref()
            .and_then(local_work_record_kind)
            .map(|kind| format!("local_work:{kind}"))
            .unwrap_or_else(|| "local_work:unknown".to_owned()),
        LocalRecordFamily::SignedEvent => record
            .event_kind
            .map(shared_signed_event_kind)
            .unwrap_or_else(|| "signed_event:unknown".to_owned()),
    }
}

fn shared_local_event_classification(
    record: &LocalEventRecord,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) -> AppSdkMigrationAuditClassification {
    match record.family {
        LocalRecordFamily::LocalWork => classify_shared_local_work(record, report),
        LocalRecordFamily::SignedEvent => classify_shared_signed_event(record, report),
    }
}

fn classify_shared_local_work(
    record: &LocalEventRecord,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) -> AppSdkMigrationAuditClassification {
    match record
        .local_work_json
        .as_ref()
        .and_then(local_work_record_kind)
    {
        Some("farm_config_v1" | "listing_draft_v1") => classify_shared_local_work_status(record),
        Some(record_kind) => {
            report.issue(
                "unsupported_shared_local_work_kind",
                Some(record.record_id.as_str()),
                format!("shared local work kind `{record_kind}` is not a SDK migration candidate"),
            );
            AppSdkMigrationAuditClassification::Unsupported
        }
        None => {
            report.issue(
                "unknown_shared_local_work_kind",
                Some(record.record_id.as_str()),
                "shared local work record does not expose a record_kind",
            );
            AppSdkMigrationAuditClassification::Unknown
        }
    }
}

fn classify_shared_local_work_status(
    record: &LocalEventRecord,
) -> AppSdkMigrationAuditClassification {
    if matches!(record.outbox_status, PublishOutboxStatus::Acknowledged)
        || matches!(record.status, LocalRecordStatus::Published)
    {
        AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate
    } else if matches!(record.outbox_status, PublishOutboxStatus::Failed)
        || matches!(
            record.status,
            LocalRecordStatus::Failed | LocalRecordStatus::Conflict
        )
    {
        AppSdkMigrationAuditClassification::ManualReviewRequired
    } else if matches!(record.status, LocalRecordStatus::PendingPublish) {
        AppSdkMigrationAuditClassification::PublishableCandidate
    } else {
        AppSdkMigrationAuditClassification::LocalWorkDeferred
    }
}

fn classify_shared_signed_event(
    record: &LocalEventRecord,
    report: &mut AppSdkMigrationAuditSourceBuilder,
) -> AppSdkMigrationAuditClassification {
    match record.event_kind {
        Some(kind) if kind == KIND_ORDER_PAYMENT_RECORD as i64 => {
            AppSdkMigrationAuditClassification::PaymentDeferred
        }
        Some(kind) if kind == KIND_ORDER_SETTLEMENT_DECISION as i64 => {
            AppSdkMigrationAuditClassification::SettlementDeferred
        }
        Some(kind) if kind == KIND_TRADE_VALIDATION_RECEIPT as i64 => {
            AppSdkMigrationAuditClassification::ValidationReceiptDeferred
        }
        Some(kind) if supported_signed_event_kind(kind) => {
            if signed_event_is_already_represented(record.status, record.outbox_status) {
                AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate
            } else {
                AppSdkMigrationAuditClassification::PublishableCandidate
            }
        }
        Some(kind) => {
            report.issue(
                "unsupported_shared_signed_event_kind",
                Some(record.record_id.as_str()),
                format!("shared signed event kind `{kind}` is not a SDK migration candidate"),
            );
            AppSdkMigrationAuditClassification::Unsupported
        }
        None => {
            report.issue(
                "unknown_shared_signed_event_kind",
                Some(record.record_id.as_str()),
                "shared signed event record does not expose an event_kind",
            );
            AppSdkMigrationAuditClassification::Unknown
        }
    }
}

fn signed_event_is_already_represented(
    status: LocalRecordStatus,
    outbox_status: PublishOutboxStatus,
) -> bool {
    matches!(status, LocalRecordStatus::Published)
        || matches!(outbox_status, PublishOutboxStatus::Acknowledged)
}

fn shared_local_event_duplicate_identities(record: &LocalEventRecord) -> Vec<DuplicateIdentity> {
    let mut identities = Vec::new();
    if let (Some(event_kind), Some(event_id)) = (
        record.event_kind,
        non_empty_value(record.event_id.as_deref()),
    ) {
        identities.push(DuplicateIdentity {
            kind: "event".to_owned(),
            key: format!("{event_kind}:{event_id}"),
        });
    }
    if let Some(key) = shared_local_event_aggregate_key(record) {
        identities.push(DuplicateIdentity {
            kind: "aggregate".to_owned(),
            key,
        });
    }
    identities
}

fn shared_local_event_aggregate_key(record: &LocalEventRecord) -> Option<String> {
    match record.family {
        LocalRecordFamily::LocalWork => {
            let record_kind = record
                .local_work_json
                .as_ref()
                .and_then(local_work_record_kind)?;
            non_empty_value(record.farm_id.as_deref())
                .map(|farm_id| format!("local_work:{record_kind}:farm:{farm_id}"))
                .or_else(|| {
                    non_empty_value(record.listing_addr.as_deref()).map(|listing_addr| {
                        format!("local_work:{record_kind}:listing:{listing_addr}")
                    })
                })
        }
        LocalRecordFamily::SignedEvent => {
            let event_kind = record.event_kind?;
            non_empty_value(record.listing_addr.as_deref())
                .map(|listing_addr| format!("signed_event:{event_kind}:listing:{listing_addr}"))
                .or_else(|| {
                    non_empty_value(record.farm_id.as_deref())
                        .map(|farm_id| format!("signed_event:{event_kind}:farm:{farm_id}"))
                })
        }
    }
}

fn local_work_record_kind(payload: &Value) -> Option<&str> {
    payload
        .get("record_kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn supported_signed_event_kind(kind: i64) -> bool {
    matches!(
        kind,
        value if value == KIND_FARM as i64
            || value == KIND_LISTING as i64
            || value == KIND_LISTING_DRAFT as i64
            || value == KIND_ORDER_REQUEST as i64
            || value == KIND_ORDER_DECISION as i64
            || value == KIND_ORDER_REVISION_PROPOSAL as i64
            || value == KIND_ORDER_REVISION_DECISION as i64
            || value == KIND_ORDER_CANCELLATION as i64
            || value == KIND_ORDER_FULFILLMENT_UPDATE as i64
            || value == KIND_ORDER_RECEIPT as i64
    )
}

fn shared_signed_event_kind(kind: i64) -> String {
    let name = match kind {
        value if value == KIND_FARM as i64 => "farm",
        value if value == KIND_LISTING as i64 => "listing",
        value if value == KIND_LISTING_DRAFT as i64 => "listing_draft",
        value if value == KIND_ORDER_REQUEST as i64 => "order_request",
        value if value == KIND_ORDER_DECISION as i64 => "order_decision",
        value if value == KIND_ORDER_REVISION_PROPOSAL as i64 => "order_revision_proposal",
        value if value == KIND_ORDER_REVISION_DECISION as i64 => "order_revision_decision",
        value if value == KIND_ORDER_CANCELLATION as i64 => "order_cancellation",
        value if value == KIND_ORDER_FULFILLMENT_UPDATE as i64 => "order_fulfillment",
        value if value == KIND_ORDER_RECEIPT as i64 => "order_receipt",
        value if value == KIND_ORDER_PAYMENT_RECORD as i64 => "order_payment",
        value if value == KIND_ORDER_SETTLEMENT_DECISION as i64 => "order_settlement",
        value if value == KIND_TRADE_VALIDATION_RECEIPT as i64 => "trade_validation_receipt",
        _ => "unsupported",
    };
    format!("signed_event:{name}:{kind}")
}

fn non_empty_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn increment_count(counts: &mut BTreeMap<String, u64>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn counts_from_map(counts: BTreeMap<String, u64>) -> Vec<AppSdkMigrationAuditCount> {
    counts
        .into_iter()
        .map(|(key, count)| AppSdkMigrationAuditCount { key, count })
        .collect()
}

fn duplicate_candidates_from_map(
    duplicate_records: BTreeMap<DuplicateIdentity, BTreeSet<String>>,
) -> Vec<AppSdkMigrationAuditDuplicateCandidate> {
    duplicate_records
        .into_iter()
        .filter_map(|(identity, records)| {
            if records.len() < 2 {
                return None;
            }
            Some(AppSdkMigrationAuditDuplicateCandidate {
                identity_kind: identity.kind,
                identity_key: identity.key,
                record_count: records.len() as u64,
                record_ids: records.into_iter().collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_sync::{
        AppFarmProfilePublishPayload, AppPublishContext, AppPublishPayload, PendingSyncOperation,
    };
    use radroots_studio_app_view::{FarmId, FarmReadiness};
    use radroots_events::kinds::{
        KIND_LISTING, KIND_ORDER_PAYMENT_RECORD, KIND_ORDER_SETTLEMENT_DECISION,
        KIND_TRADE_VALIDATION_RECEIPT,
    };
    use radroots_local_events::{
        LocalEventRecord, LocalEventRecordInput, LocalEventsStore, LocalRecordFamily,
        LocalRecordStatus, PublishOutboxStatus, SourceRuntime,
    };
    use radroots_sql_core::SqliteExecutor;
    use rusqlite::params;
    use serde_json::json;

    use crate::{
        AppSdkMigrationAuditClassification, AppSdkMigrationAuditRequest,
        AppSdkMigrationReceiptInput, AppSdkMigrationReceiptSourceKind, AppSdkMigrationState,
        AppSqliteStore, DatabaseTarget,
    };

    fn local_events_store() -> LocalEventsStore<SqliteExecutor> {
        let executor = SqliteExecutor::open_memory().expect("open local events memory db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate local events store");
        store
    }

    fn count_named(counts: &[crate::AppSdkMigrationAuditCount], key: &str) -> u64 {
        counts
            .iter()
            .find(|count| count.key == key)
            .map(|count| count.count)
            .unwrap_or_default()
    }

    #[test]
    fn local_outbox_audit_reads_batches_without_mutating_rows() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();
        let farm_id = FarmId::new();
        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
                context: AppPublishContext::new("acct_a", "farm_setup"),
                farm_id,
                display_name: "Green Loop Farm".to_owned(),
                readiness: Some(FarmReadiness::Ready),
            }),
            "2026-06-18T12:00:00Z",
        )
        .expect("build publish operation");
        store
            .sync_repository()
            .enqueue_pending_operation("acct_a", &operation)
            .expect("enqueue operation");
        store
            .connection()
            .execute(
                "INSERT INTO local_outbox (
                    id,
                    account_id,
                    operation_key,
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count,
                    state,
                    last_error_message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
                params![
                    "succeeded-duplicate",
                    "acct_a",
                    operation.operation_key,
                    operation.aggregate.aggregate_kind(),
                    operation.aggregate.aggregate_id(),
                    operation.operation.storage_key(),
                    operation.payload_json,
                    "2026-06-18T11:00:00Z",
                    "2026-06-18T11:00:00Z",
                    0_i64,
                    "succeeded",
                ],
            )
            .expect("insert succeeded duplicate");
        let before_count = local_outbox_row_count(&store);

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 1 },
            )
            .expect("audit should run");

        assert_eq!(local_outbox_row_count(&store), before_count);
        assert_eq!(report.local_outbox.batch_size, 1);
        assert_eq!(report.local_outbox.batch_count, 2);
        assert_eq!(report.local_outbox.scanned_records, 2);
        assert_eq!(
            count_named(&report.local_outbox.kind_counts, "farm_profile"),
            2
        );
        assert_eq!(
            count_named(&report.local_outbox.status_counts, "pending"),
            1
        );
        assert_eq!(
            count_named(&report.local_outbox.status_counts, "succeeded"),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::PublishableCandidate.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate.storage_key()
            ),
            1
        );
        assert!(
            report
                .local_outbox
                .duplicate_candidates
                .iter()
                .any(|candidate| candidate.identity_kind == "operation"
                    && candidate.record_count == 2)
        );
        assert_eq!(report.shared_local_events.scanned_records, 0);
    }

    #[test]
    fn local_outbox_audit_classifies_status_matrix() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();
        let operation = farm_profile_operation("acct_seed", "status_matrix");

        for (index, state) in [
            "pending",
            "in_progress",
            "retryable",
            "failed",
            "blocked",
            "succeeded",
        ]
        .iter()
        .enumerate()
        {
            insert_local_outbox_audit_row(
                &store,
                &format!("local-outbox-{state}"),
                &format!("acct_{index}"),
                state,
                &operation,
            );
        }

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 2 },
            )
            .expect("audit should run");

        assert_eq!(report.local_outbox.scanned_records, 6);
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::PublishableCandidate.storage_key()
            ),
            3
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::ManualReviewRequired.storage_key()
            ),
            2
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate.storage_key()
            ),
            1
        );
        assert_eq!(
            report
                .local_outbox
                .issues
                .iter()
                .filter(|issue| issue.code == "manual_review_local_outbox_state")
                .count(),
            2
        );
    }

    #[test]
    fn local_outbox_audit_uses_migration_receipts_for_migratable_records() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();
        let operation = farm_profile_operation("acct_seed", "receipt_matrix");

        for (id, state) in [
            ("represented-source", AppSdkMigrationState::Enqueued),
            ("skipped-source", AppSdkMigrationState::Skipped),
            ("failed-source", AppSdkMigrationState::Failed),
        ] {
            insert_local_outbox_audit_row(&store, id, id, "pending", &operation);
            record_local_outbox_receipt(&store, id, state);
        }

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 10 },
            )
            .expect("audit should run");

        assert_eq!(report.local_outbox.scanned_records, 3);
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::RepresentedRecord.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::SkippedRecord.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::FailedRecord.storage_key()
            ),
            1
        );
        assert!(
            report
                .local_outbox
                .issues
                .iter()
                .any(|issue| issue.code == "sdk_migration_receipt_failed")
        );
    }

    #[test]
    fn local_outbox_audit_does_not_let_receipts_hide_non_migratable_rows() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();
        let operation = farm_profile_operation("acct_seed", "non_migratable");

        insert_local_outbox_audit_row(&store, "failed-source", "acct_failed", "failed", &operation);
        record_local_outbox_receipt(&store, "failed-source", AppSdkMigrationState::Enqueued);
        insert_local_outbox_audit_row(
            &store,
            "unsupported-source",
            "acct_unsupported",
            "pending",
            &PendingSyncOperation {
                operation: radroots_studio_app_sync::SyncOperationKind::Delete,
                ..operation.clone()
            },
        );
        record_local_outbox_receipt(&store, "unsupported-source", AppSdkMigrationState::Enqueued);
        store
            .connection()
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .expect("disable sqlite checks for defensive unknown state row");
        insert_local_outbox_audit_row(
            &store,
            "unknown-source",
            "acct_unknown",
            "mystery",
            &operation,
        );
        store
            .connection()
            .execute_batch("PRAGMA ignore_check_constraints = OFF")
            .expect("restore sqlite checks");
        record_local_outbox_receipt(&store, "unknown-source", AppSdkMigrationState::Enqueued);

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 10 },
            )
            .expect("audit should run");

        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::RepresentedRecord.storage_key()
            ),
            0
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::ManualReviewRequired.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::Unsupported.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.local_outbox.classification_counts,
                AppSdkMigrationAuditClassification::Unknown.storage_key()
            ),
            1
        );
        assert_eq!(
            report
                .local_outbox
                .issues
                .iter()
                .filter(|issue| issue.code == "sdk_migration_receipt_for_non_migratable_source")
                .count(),
            3
        );
    }

    #[test]
    fn shared_local_events_audit_defers_payment_and_settlement() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();
        shared_events
            .append_record(&signed_event_record(
                "listing-a",
                "duplicate-listing-event",
                KIND_LISTING as i64,
            ))
            .expect("append listing a");
        shared_events
            .append_record(&signed_event_record(
                "listing-b",
                "duplicate-listing-event",
                KIND_LISTING as i64,
            ))
            .expect("append listing b");
        shared_events
            .append_record(&signed_event_record(
                "payment",
                "payment-event",
                KIND_ORDER_PAYMENT_RECORD as i64,
            ))
            .expect("append payment");
        shared_events
            .append_record(&signed_event_record(
                "settlement",
                "settlement-event",
                KIND_ORDER_SETTLEMENT_DECISION as i64,
            ))
            .expect("append settlement");
        shared_events
            .append_record(&signed_event_record(
                "validation-receipt",
                "validation-receipt-event",
                KIND_TRADE_VALIDATION_RECEIPT as i64,
            ))
            .expect("append validation receipt");
        let before_records = shared_events
            .list_records_changed_after(0, 10)
            .expect("list records before audit")
            .len();

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 1 },
            )
            .expect("audit should run");

        assert_eq!(
            shared_events
                .list_records_changed_after(0, 10)
                .expect("list records after audit")
                .len(),
            before_records
        );
        assert_eq!(report.shared_local_events.batch_count, 5);
        assert_eq!(report.shared_local_events.scanned_records, 5);
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate.storage_key()
            ),
            2
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::PaymentDeferred.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::SettlementDeferred.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::ValidationReceiptDeferred.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::PublishableCandidate.storage_key()
            ),
            0
        );
        assert!(
            report
                .shared_local_events
                .duplicate_candidates
                .iter()
                .any(|candidate| candidate.identity_kind == "event" && candidate.record_count == 2)
        );
    }

    #[test]
    fn shared_local_work_audit_classifies_status_matrix() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app store");
        let shared_events = local_events_store();

        for (record_id, record_kind, status, outbox_status) in [
            (
                "local-draft",
                "farm_config_v1",
                LocalRecordStatus::LocalDraft,
                PublishOutboxStatus::None,
            ),
            (
                "local-saved",
                "listing_draft_v1",
                LocalRecordStatus::LocalSaved,
                PublishOutboxStatus::None,
            ),
            (
                "pending-publish",
                "listing_draft_v1",
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::None,
            ),
            (
                "published",
                "farm_config_v1",
                LocalRecordStatus::Published,
                PublishOutboxStatus::None,
            ),
            (
                "failed",
                "farm_config_v1",
                LocalRecordStatus::Failed,
                PublishOutboxStatus::None,
            ),
            (
                "conflict",
                "listing_draft_v1",
                LocalRecordStatus::Conflict,
                PublishOutboxStatus::None,
            ),
        ] {
            shared_events
                .append_record(&local_work_record(
                    record_id,
                    record_kind,
                    status,
                    outbox_status,
                ))
                .expect("append local work record");
        }

        let report = store
            .audit_sdk_migration(
                &shared_events,
                AppSdkMigrationAuditRequest { batch_size: 3 },
            )
            .expect("audit should run");

        assert_eq!(report.shared_local_events.scanned_records, 6);
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::LocalWorkDeferred.storage_key()
            ),
            2
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::PublishableCandidate.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate.storage_key()
            ),
            1
        );
        assert_eq!(
            count_named(
                &report.shared_local_events.classification_counts,
                AppSdkMigrationAuditClassification::ManualReviewRequired.storage_key()
            ),
            2
        );
    }

    #[test]
    fn shared_local_work_status_classifier_handles_defensive_outbox_states() {
        assert_eq!(
            super::classify_shared_local_work_status(&local_work_model_record(
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::Acknowledged,
            )),
            AppSdkMigrationAuditClassification::AlreadyRepresentedCandidate
        );
        assert_eq!(
            super::classify_shared_local_work_status(&local_work_model_record(
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::Failed,
            )),
            AppSdkMigrationAuditClassification::ManualReviewRequired
        );
    }

    fn local_outbox_row_count(store: &AppSqliteStore) -> i64 {
        store
            .connection()
            .query_row("SELECT count(*) FROM local_outbox", [], |row| row.get(0))
            .expect("count local outbox rows")
    }

    fn farm_profile_operation(account_id: &str, source: &str) -> PendingSyncOperation {
        PendingSyncOperation::from_publish_payload(
            AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
                context: AppPublishContext::new(account_id, source),
                farm_id: FarmId::new(),
                display_name: "Green Loop Farm".to_owned(),
                readiness: Some(FarmReadiness::Ready),
            }),
            "2026-06-18T12:00:00Z",
        )
        .expect("build publish operation")
    }

    fn insert_local_outbox_audit_row(
        store: &AppSqliteStore,
        id: &str,
        account_id: &str,
        state: &str,
        operation: &PendingSyncOperation,
    ) {
        store
            .connection()
            .execute(
                "INSERT INTO local_outbox (
                    id,
                    account_id,
                    operation_key,
                    aggregate_kind,
                    aggregate_id,
                    operation_kind,
                    payload_json,
                    created_at,
                    available_at,
                    attempt_count,
                    state,
                    last_error_message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
                params![
                    id,
                    account_id,
                    operation.operation_key.as_str(),
                    operation.aggregate.aggregate_kind(),
                    operation.aggregate.aggregate_id(),
                    operation.operation.storage_key(),
                    operation.payload_json.as_str(),
                    operation.created_at.as_str(),
                    operation.available_at.as_str(),
                    i64::from(operation.attempt_count),
                    state,
                ],
            )
            .expect("insert local outbox audit row");
    }

    fn record_local_outbox_receipt(
        store: &AppSqliteStore,
        source_record_id: &str,
        migration_state: AppSdkMigrationState,
    ) {
        store
            .sdk_migration_receipt_repository()
            .record_receipt(&AppSdkMigrationReceiptInput {
                source_kind: AppSdkMigrationReceiptSourceKind::LocalOutbox,
                source_record_id: source_record_id.to_owned(),
                sdk_operation_kind: "farm.publish".to_owned(),
                sdk_outbox_event_ids: vec![format!("sdk-outbox-{source_record_id}")],
                expected_event_id: Some(format!("event-{source_record_id}")),
                actor_pubkey: Some("actor-pubkey".to_owned()),
                idempotency_digest_prefix: Some("digest-prefix".to_owned()),
                migration_state,
                recorded_at: "2026-06-18T12:00:00Z".to_owned(),
                detail_json: json!({"source": source_record_id}),
            })
            .expect("record local outbox receipt");
    }

    fn local_work_record(
        record_id: &str,
        record_kind: &str,
        status: LocalRecordStatus,
        outbox_status: PublishOutboxStatus,
    ) -> LocalEventRecordInput {
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::LocalWork,
            status,
            source_runtime: SourceRuntime::App,
            created_at_ms: 1000,
            inserted_at_ms: 1001,
            owner_account_id: Some("acct_a".to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some("farm-key".to_owned()),
            listing_addr: Some("30402:seller-pubkey:listing-key".to_owned()),
            local_work_json: Some(json!({"record_kind": record_kind})),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        }
    }

    fn local_work_model_record(
        status: LocalRecordStatus,
        outbox_status: PublishOutboxStatus,
    ) -> LocalEventRecord {
        LocalEventRecord {
            seq: 1,
            change_seq: 1,
            record_id: "defensive-local-work".to_owned(),
            family: LocalRecordFamily::LocalWork,
            status,
            source_runtime: SourceRuntime::App,
            created_at_ms: 1000,
            inserted_at_ms: 1001,
            updated_at_ms: 1002,
            owner_account_id: Some("acct_a".to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some("farm-key".to_owned()),
            listing_addr: Some("30402:seller-pubkey:listing-key".to_owned()),
            local_work_json: Some(json!({"record_kind": "listing_draft_v1"})),
            event_id: None,
            event_kind: None,
            event_pubkey: None,
            event_created_at: None,
            event_tags_json: None,
            event_content: None,
            event_sig: None,
            raw_event_json: None,
            outbox_status,
            relay_set_fingerprint: None,
            relay_delivery_json: None,
        }
    }

    fn signed_event_record(
        record_id: &str,
        event_id: &str,
        event_kind: i64,
    ) -> LocalEventRecordInput {
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::SignedEvent,
            status: LocalRecordStatus::Published,
            source_runtime: SourceRuntime::App,
            created_at_ms: 1000,
            inserted_at_ms: 1001,
            owner_account_id: Some("acct_a".to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some("farm-key".to_owned()),
            listing_addr: Some("30402:seller-pubkey:listing-key".to_owned()),
            local_work_json: None,
            event_id: Some(event_id.to_owned()),
            event_kind: Some(event_kind),
            event_pubkey: Some("seller-pubkey".to_owned()),
            event_created_at: Some(1000),
            event_tags_json: Some(json!([["d", "listing-key"]])),
            event_content: Some("{}".to_owned()),
            event_sig: Some("signature".to_owned()),
            raw_event_json: Some(json!({
                "id": event_id,
                "kind": event_kind,
                "pubkey": "seller-pubkey"
            })),
            outbox_status: PublishOutboxStatus::Acknowledged,
            relay_set_fingerprint: Some("relay-set".to_owned()),
            relay_delivery_json: Some(json!({
                "state": "acknowledged",
                "acknowledged_relays": ["wss://relay.example"]
            })),
        }
    }
}
