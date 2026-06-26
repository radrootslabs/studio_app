use std::{fs, path::Path};

use radroots_events::{
    RadrootsNostrEvent,
    ids::{RadrootsEventId, RadrootsOrderId, RadrootsPublicKey},
    kinds::{
        KIND_FARM as RADROOTS_KIND_FARM, KIND_LISTING as RADROOTS_KIND_LISTING,
        KIND_LISTING_DRAFT as RADROOTS_KIND_LISTING_DRAFT,
        KIND_ORDER_CANCELLATION as RADROOTS_KIND_ORDER_CANCELLATION,
        KIND_ORDER_DECISION as RADROOTS_KIND_ORDER_DECISION,
        KIND_ORDER_REQUEST as RADROOTS_KIND_ORDER_REQUEST,
        KIND_ORDER_REVISION_DECISION as RADROOTS_KIND_ORDER_REVISION_DECISION,
        KIND_ORDER_REVISION_PROPOSAL as RADROOTS_KIND_ORDER_REVISION_PROPOSAL,
        KIND_TRADE_VALIDATION_RECEIPT,
    },
    order::{
        RadrootsOrderEconomics, RadrootsOrderItem, RadrootsOrderRequest,
        RadrootsOrderRevisionOutcome,
    },
};
use radroots_events_codec::order::{
    order_cancellation_from_event, order_decision_from_event, order_event_context_from_tags,
    order_request_from_event, order_revision_decision_from_event,
    order_revision_proposal_from_event,
};
use radroots_local_events::{
    LocalEventRecord, LocalEventsStore, LocalRecordFamily, LocalRecordStatus, PublishOutboxStatus,
    RelayDeliveryEvidence, RelayDeliveryState, SourceRuntime,
};
use radroots_sql_core::{SqlExecutor, SqliteExecutor};
use radroots_studio_app_view::{
    FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection, FarmSummary,
    FulfillmentWindowId, OrderId, OrderStatus, PickupLocationId, ProductId, ProductStatus,
    TradeAgreementStatus, TradeInventoryStatus, TradeProvenanceProjection, TradeRevisionStatus,
    TradeValidationReceiptProofSystem, TradeValidationReceiptResult, TradeValidationReceiptType,
    TradeWorkflowProjection, TradeWorkflowSource, order_status_from_active_order_projection,
};
use radroots_trade::order::{
    RadrootsOrderCancellationRecord, RadrootsOrderDecisionRecord, RadrootsOrderProjection,
    RadrootsOrderReductionInputs, RadrootsOrderRequestRecord, RadrootsOrderRevisionDecisionRecord,
    RadrootsOrderRevisionProposalRecord, reduce_order_events,
};
use radroots_trade::validation_receipt::{
    RadrootsTradeValidationReceipt, RadrootsValidationReceiptTags, validation_receipt_from_event,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use uuid::Uuid;

use crate::repo::farm_setup::AppFarmSetupRepository;
use crate::{AppSqliteError, AppSqliteStore};

const LOCAL_EVENTS_BATCH_LIMIT: u32 = 500;
const APP_LOCAL_INTEROP_CURSOR_ID: &str = "radroots_studio_app_sqlite_projection_v1";
const KIND_FARM: i64 = RADROOTS_KIND_FARM as i64;
const KIND_LISTING: i64 = RADROOTS_KIND_LISTING as i64;
const KIND_LISTING_DRAFT: i64 = RADROOTS_KIND_LISTING_DRAFT as i64;
const KIND_ORDER_REQUEST: i64 = RADROOTS_KIND_ORDER_REQUEST as i64;
const KIND_ORDER_DECISION: i64 = RADROOTS_KIND_ORDER_DECISION as i64;
const KIND_ORDER_REVISION: i64 = RADROOTS_KIND_ORDER_REVISION_PROPOSAL as i64;
const KIND_ORDER_REVISION_DECISION: i64 = RADROOTS_KIND_ORDER_REVISION_DECISION as i64;
const KIND_ORDER_CANCEL: i64 = RADROOTS_KIND_ORDER_CANCELLATION as i64;
const KIND_VALIDATION_RECEIPT: i64 = KIND_TRADE_VALIDATION_RECEIPT as i64;
const ACTIVE_ORDER_EVENT_KINDS: [i64; 5] = [
    KIND_ORDER_REQUEST,
    KIND_ORDER_DECISION,
    KIND_ORDER_REVISION,
    KIND_ORDER_REVISION_DECISION,
    KIND_ORDER_CANCEL,
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppLocalInteropImportReport {
    pub scanned_records: u32,
    pub imported_records: u32,
    pub skipped_records: u32,
    pub self_observed_records: u32,
    pub last_change_seq: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredLocalInteropRecord {
    pub record_id: String,
    pub local_seq: i64,
    pub record_family: String,
    pub local_status: String,
    pub source_runtime: String,
    pub owner_account_id: Option<String>,
    pub owner_pubkey: Option<String>,
    pub farm_key: Option<String>,
    pub listing_addr: Option<String>,
    pub projected_kind: String,
    pub projected_id: Option<String>,
    pub event_id: Option<String>,
    pub event_kind: Option<i64>,
    pub outbox_status: String,
    pub relay_delivery_json: Option<String>,
}

pub struct AppLocalInteropRepository<'a> {
    connection: &'a Connection,
}

impl<'a> AppLocalInteropRepository<'a> {
    pub const fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }

    pub fn import_from_path(
        &self,
        shared_database_path: &Path,
    ) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        if let Some(parent) = shared_database_path.parent() {
            fs::create_dir_all(parent).map_err(|source| AppSqliteError::CreateParentDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let executor = SqliteExecutor::open(shared_database_path).map_err(|source| {
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
        self.import_from_store(&store)
    }

    pub fn import_from_store<E>(
        &self,
        store: &LocalEventsStore<E>,
    ) -> Result<AppLocalInteropImportReport, AppSqliteError>
    where
        E: SqlExecutor,
    {
        let mut report = AppLocalInteropImportReport::default();
        let mut after_change_seq = self.last_imported_change_seq()?;
        loop {
            let records = store
                .list_records_changed_after(after_change_seq, LOCAL_EVENTS_BATCH_LIMIT)
                .map_err(|source| AppSqliteError::LocalEvents {
                    operation: "list changed shared local event records",
                    source,
                })?;
            let batch_len = records.len();
            for record in records {
                after_change_seq = record.change_seq;
                report.scanned_records += 1;
                report.last_change_seq = Some(record.change_seq);
                match self.import_record(&record)? {
                    ImportOutcome::Imported => report.imported_records += 1,
                    ImportOutcome::Skipped => report.skipped_records += 1,
                }
            }
            if batch_len < LOCAL_EVENTS_BATCH_LIMIT as usize {
                break;
            }
        }
        if let Some(last_change_seq) = report.last_change_seq {
            self.advance_import_cursor(last_change_seq)?;
        }
        Ok(report)
    }

    pub fn import_records(
        &self,
        records: &[LocalEventRecord],
    ) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        let mut report = AppLocalInteropImportReport::default();
        for record in records {
            report.scanned_records += 1;
            report.last_change_seq = Some(record.change_seq);
            match self.import_record(record)? {
                ImportOutcome::Imported => report.imported_records += 1,
                ImportOutcome::Skipped => report.skipped_records += 1,
            }
        }
        Ok(report)
    }

    pub fn load_records(&self) -> Result<Vec<StoredLocalInteropRecord>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    record_id,
                    local_seq,
                    record_family,
                    local_status,
                    source_runtime,
                    owner_account_id,
                    owner_pubkey,
                    farm_key,
                    listing_addr,
                    projected_kind,
                    projected_id,
                    event_id,
                    event_kind,
                    outbox_status,
                    relay_delivery_json
                 FROM local_interop_imports
                 ORDER BY local_seq ASC, record_id ASC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare local interop import query",
                source,
            })?;
        let rows = statement
            .query_map([], |row| {
                Ok(StoredLocalInteropRecord {
                    record_id: row.get(0)?,
                    local_seq: row.get(1)?,
                    record_family: row.get(2)?,
                    local_status: row.get(3)?,
                    source_runtime: row.get(4)?,
                    owner_account_id: row.get(5)?,
                    owner_pubkey: row.get(6)?,
                    farm_key: row.get(7)?,
                    listing_addr: row.get(8)?,
                    projected_kind: row.get(9)?,
                    projected_id: row.get(10)?,
                    event_id: row.get(11)?,
                    event_kind: row.get(12)?,
                    outbox_status: row.get(13)?,
                    relay_delivery_json: row.get(14)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query local interop imports",
                source,
            })?;
        rows.map(|row| {
            row.map_err(|source| AppSqliteError::Query {
                operation: "read local interop import row",
                source,
            })
        })
        .collect()
    }

    pub fn load_signed_events_by_kind(
        &self,
        event_kind: i64,
    ) -> Result<Vec<RadrootsNostrEvent>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    event_id,
                    event_kind,
                    local_status,
                    outbox_status,
                    relay_delivery_json,
                    event_pubkey,
                    event_created_at,
                    event_tags_json,
                    event_content,
                    event_sig
                 FROM local_interop_imports
                 WHERE record_family = 'signed_event'
                    AND local_status = 'published'
                    AND event_kind = ?1
                 ORDER BY local_seq ASC, record_id ASC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare local interop signed event evidence query",
                source,
            })?;
        let rows = statement
            .query_map(params![event_kind], |row| {
                Ok(StoredLocalInteropSignedEventEvidence {
                    event_id: row.get(0)?,
                    event_kind: row.get(1)?,
                    local_status: row.get(2)?,
                    outbox_status: row.get(3)?,
                    relay_delivery_json: row.get(4)?,
                    event_pubkey: row.get(5)?,
                    event_created_at: row.get(6)?,
                    event_tags_json: row.get(7)?,
                    event_content: row.get(8)?,
                    event_sig: row.get(9)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query local interop signed event evidence",
                source,
            })?;
        let mut events = Vec::new();
        for row in rows {
            let evidence = row.map_err(|source| AppSqliteError::Query {
                operation: "read local interop signed event evidence row",
                source,
            })?;
            if !signed_event_local_interop_evidence_is_usable(&evidence) {
                continue;
            }
            if let Some(event) = signed_event_from_local_interop_evidence(&evidence)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    fn load_signed_event_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<RadrootsNostrEvent>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT
                    event_id,
                    event_kind,
                    local_status,
                    outbox_status,
                    relay_delivery_json,
                    event_pubkey,
                    event_created_at,
                    event_tags_json,
                    event_content,
                    event_sig
                 FROM local_interop_imports
                 WHERE record_family = 'signed_event'
                    AND event_id = ?1
                 ORDER BY local_seq DESC, record_id DESC",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare local interop signed event id evidence query",
                source,
            })?;
        let rows = statement
            .query_map(params![event_id], |row| {
                Ok(StoredLocalInteropSignedEventEvidence {
                    event_id: row.get(0)?,
                    event_kind: row.get(1)?,
                    local_status: row.get(2)?,
                    outbox_status: row.get(3)?,
                    relay_delivery_json: row.get(4)?,
                    event_pubkey: row.get(5)?,
                    event_created_at: row.get(6)?,
                    event_tags_json: row.get(7)?,
                    event_content: row.get(8)?,
                    event_sig: row.get(9)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query local interop signed event id evidence",
                source,
            })?;

        for row in rows {
            let evidence = row.map_err(|source| AppSqliteError::Query {
                operation: "read local interop signed event id evidence row",
                source,
            })?;
            if !signed_event_local_interop_evidence_is_usable(&evidence) {
                continue;
            }
            if let Some(event) = signed_event_from_local_interop_evidence(&evidence)? {
                return Ok(Some(event));
            }
        }

        Ok(None)
    }

    fn last_imported_change_seq(&self) -> Result<i64, AppSqliteError> {
        match self.connection.query_row(
            "SELECT last_change_seq
             FROM local_interop_projection_cursor
             WHERE consumer_id = ?1
             LIMIT 1",
            [APP_LOCAL_INTEROP_CURSOR_ID],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(last_change_seq) => Ok(last_change_seq),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(source) => Err(AppSqliteError::Query {
                operation: "read app local interop projection cursor",
                source,
            }),
        }
    }

    fn advance_import_cursor(&self, last_change_seq: i64) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO local_interop_projection_cursor (
                    consumer_id,
                    last_change_seq,
                    updated_at
                 ) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(consumer_id) DO UPDATE SET
                    last_change_seq = max(
                        local_interop_projection_cursor.last_change_seq,
                        excluded.last_change_seq
                    ),
                    updated_at = excluded.updated_at",
                params![APP_LOCAL_INTEROP_CURSOR_ID, last_change_seq],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "advance app local interop projection cursor",
                source,
            })?;
        Ok(())
    }

    fn import_record(&self, record: &LocalEventRecord) -> Result<ImportOutcome, AppSqliteError> {
        self.begin_import_record_savepoint()?;
        match self.import_record_inner(record) {
            Ok(outcome) => {
                self.release_import_record_savepoint()?;
                Ok(outcome)
            }
            Err(error) => {
                let _ = self.rollback_import_record_savepoint();
                let _ = self.release_import_record_savepoint();
                Err(error)
            }
        }
    }

    fn import_record_inner(
        &self,
        record: &LocalEventRecord,
    ) -> Result<ImportOutcome, AppSqliteError> {
        let superseded_listing_ids = match self.duplicate_signed_event_action(record)? {
            DuplicateSignedEventAction::Import => Vec::new(),
            DuplicateSignedEventAction::ReplaceExisting(event_id) => self
                .delete_duplicate_signed_event_imports(
                    event_id.as_str(),
                    record.record_id.as_str(),
                )?,
            DuplicateSignedEventAction::Skip => return Ok(ImportOutcome::Skipped),
        };
        let projection = match record.family {
            LocalRecordFamily::LocalWork => self.import_local_work(record)?,
            LocalRecordFamily::SignedEvent => self.import_signed_event(record)?,
        };
        match projection {
            Some(projection) => {
                let projected_kind = projection.kind;
                let projected_id = projection.projected_id;
                self.record_import(record, projected_kind, projected_id.clone())?;
                if projected_kind == "listing" {
                    if let Some(projected_id) = projected_id.as_deref() {
                        self.finish_duplicate_listing_replacement(
                            &superseded_listing_ids,
                            projected_id,
                        )?;
                    }
                }
                Ok(ImportOutcome::Imported)
            }
            None => {
                self.record_import(record, "unsupported", None)?;
                Ok(ImportOutcome::Skipped)
            }
        }
    }

    fn begin_import_record_savepoint(&self) -> Result<(), AppSqliteError> {
        self.connection
            .execute_batch("SAVEPOINT app_local_interop_import_record")
            .map_err(|source| AppSqliteError::Query {
                operation: "begin local interop import record transaction",
                source,
            })
    }

    fn rollback_import_record_savepoint(&self) -> Result<(), AppSqliteError> {
        self.connection
            .execute_batch("ROLLBACK TO app_local_interop_import_record")
            .map_err(|source| AppSqliteError::Query {
                operation: "rollback local interop import record transaction",
                source,
            })
    }

    fn release_import_record_savepoint(&self) -> Result<(), AppSqliteError> {
        self.connection
            .execute_batch("RELEASE app_local_interop_import_record")
            .map_err(|source| AppSqliteError::Query {
                operation: "release local interop import record transaction",
                source,
            })
    }

    fn duplicate_signed_event_action(
        &self,
        record: &LocalEventRecord,
    ) -> Result<DuplicateSignedEventAction, AppSqliteError> {
        if record.family != LocalRecordFamily::SignedEvent {
            return Ok(DuplicateSignedEventAction::Import);
        }
        let Some(event_id) = record
            .event_id
            .as_deref()
            .map(str::trim)
            .filter(|event_id| !event_id.is_empty())
        else {
            return Ok(DuplicateSignedEventAction::Import);
        };
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_runtime, owner_account_id, local_status, outbox_status
                 FROM local_interop_imports
                 WHERE event_id = ?1
                    AND record_id <> ?2
                    AND record_family = 'signed_event'",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare duplicate local interop signed event query",
                source,
            })?;
        let rows = statement
            .query_map(params![event_id, record.record_id.as_str()], |row| {
                Ok(StoredSignedEventDuplicate {
                    source_runtime: row.get(0)?,
                    owner_account_id: row.get(1)?,
                    local_status: row.get(2)?,
                    outbox_status: row.get(3)?,
                })
            })
            .map_err(|source| AppSqliteError::Query {
                operation: "query duplicate local interop signed events",
                source,
            })?;
        let mut existing_precedence = None;
        for row in rows {
            let duplicate = row.map_err(|source| AppSqliteError::Query {
                operation: "read duplicate local interop signed event",
                source,
            })?;
            existing_precedence = Some(existing_precedence.unwrap_or(0).max(
                signed_event_evidence_precedence(
                    duplicate.source_runtime.as_str(),
                    duplicate.owner_account_id.as_deref(),
                    duplicate.local_status.as_str(),
                    duplicate.outbox_status.as_str(),
                ),
            ));
        }
        let Some(existing_precedence) = existing_precedence else {
            return Ok(DuplicateSignedEventAction::Import);
        };
        let incoming_precedence = signed_event_evidence_precedence(
            record.source_runtime.as_str(),
            record.owner_account_id.as_deref(),
            record.status.as_str(),
            record.outbox_status.as_str(),
        );
        if incoming_precedence > existing_precedence {
            Ok(DuplicateSignedEventAction::ReplaceExisting(
                event_id.to_owned(),
            ))
        } else {
            Ok(DuplicateSignedEventAction::Skip)
        }
    }

    fn delete_duplicate_signed_event_imports(
        &self,
        event_id: &str,
        record_id: &str,
    ) -> Result<Vec<String>, AppSqliteError> {
        let superseded_listing_ids =
            self.superseded_duplicate_listing_projection_ids(event_id, record_id)?;
        self.connection
            .execute(
                "DELETE FROM local_interop_imports
                 WHERE event_id = ?1
                    AND record_id <> ?2
                    AND record_family = 'signed_event'",
                params![event_id, record_id],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "delete superseded duplicate local interop signed event",
                source,
            })?;
        Ok(superseded_listing_ids)
    }

    fn finish_duplicate_listing_replacement(
        &self,
        superseded_listing_ids: &[String],
        canonical_listing_product_id: &str,
    ) -> Result<(), AppSqliteError> {
        self.migrate_duplicate_buyer_cart_lines(
            superseded_listing_ids,
            canonical_listing_product_id,
        )?;
        self.delete_unreferenced_listing_products(superseded_listing_ids)?;
        Ok(())
    }

    fn superseded_duplicate_listing_projection_ids(
        &self,
        event_id: &str,
        record_id: &str,
    ) -> Result<Vec<String>, AppSqliteError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT projected_id
                 FROM local_interop_imports
                 WHERE event_id = ?1
                    AND record_id <> ?2
                    AND record_family = 'signed_event'
                    AND projected_kind = 'listing'
                    AND projected_id IS NOT NULL",
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "prepare superseded duplicate listing projection query",
                source,
            })?;
        let rows = statement
            .query_map(params![event_id, record_id], |row| row.get::<_, String>(0))
            .map_err(|source| AppSqliteError::Query {
                operation: "query superseded duplicate listing projections",
                source,
            })?;
        rows.map(|row| {
            row.map_err(|source| AppSqliteError::Query {
                operation: "read superseded duplicate listing projection",
                source,
            })
        })
        .collect()
    }

    fn delete_unreferenced_listing_products(
        &self,
        product_ids: &[String],
    ) -> Result<(), AppSqliteError> {
        for product_id in product_ids {
            self.connection
                .execute(
                    "DELETE FROM products
                     WHERE id = ?1
                        AND NOT EXISTS (
                            SELECT 1
                            FROM local_interop_imports
                            WHERE projected_kind = 'listing'
                               AND projected_id = ?1
                        )",
                    params![product_id],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "delete unreferenced superseded listing product",
                    source,
                })?;
        }
        Ok(())
    }

    fn migrate_duplicate_buyer_cart_lines(
        &self,
        product_ids: &[String],
        canonical_product_id: &str,
    ) -> Result<(), AppSqliteError> {
        for product_id in product_ids {
            if product_id == canonical_product_id {
                continue;
            }
            self.connection
                .execute(
                    "INSERT INTO buyer_cart_lines (
                        buyer_context_key,
                        product_id,
                        quantity,
                        listing_bin_id,
                        quantity_unit_label,
                        unit_price_minor_units,
                        price_currency,
                        farm_key,
                        listing_addr,
                        listing_event_id,
                        seller_pubkey,
                        listing_relays_json,
                        updated_at
                     )
                     SELECT
                        buyer_context_key,
                        ?2,
                        quantity,
                        listing_bin_id,
                        quantity_unit_label,
                        unit_price_minor_units,
                        price_currency,
                        farm_key,
                        listing_addr,
                        listing_event_id,
                        seller_pubkey,
                        listing_relays_json,
                        strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                     FROM buyer_cart_lines
                     WHERE product_id = ?1
                     ON CONFLICT(buyer_context_key, product_id) DO UPDATE SET
                        quantity = buyer_cart_lines.quantity + excluded.quantity,
                        listing_bin_id = coalesce(nullif(buyer_cart_lines.listing_bin_id, ''), excluded.listing_bin_id),
                        quantity_unit_label = coalesce(nullif(buyer_cart_lines.quantity_unit_label, ''), excluded.quantity_unit_label),
                        unit_price_minor_units = coalesce(buyer_cart_lines.unit_price_minor_units, excluded.unit_price_minor_units),
                        price_currency = coalesce(nullif(buyer_cart_lines.price_currency, ''), excluded.price_currency),
                        farm_key = coalesce(nullif(buyer_cart_lines.farm_key, ''), excluded.farm_key),
                        listing_addr = coalesce(nullif(buyer_cart_lines.listing_addr, ''), excluded.listing_addr),
                        listing_event_id = coalesce(nullif(buyer_cart_lines.listing_event_id, ''), excluded.listing_event_id),
                        seller_pubkey = coalesce(nullif(buyer_cart_lines.seller_pubkey, ''), excluded.seller_pubkey),
                        listing_relays_json = coalesce(nullif(buyer_cart_lines.listing_relays_json, ''), excluded.listing_relays_json),
                        updated_at = excluded.updated_at",
                    params![product_id, canonical_product_id],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "migrate duplicate listing buyer cart lines",
                    source,
                })?;
            self.connection
                .execute(
                    "DELETE FROM buyer_cart_lines
                     WHERE product_id = ?1",
                    params![product_id],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "delete migrated duplicate listing buyer cart lines",
                    source,
                })?;
        }
        Ok(())
    }

    fn import_local_work(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        let Some(payload) = record.local_work_json.as_ref() else {
            return Ok(None);
        };
        match string_at(payload, &["record_kind"]).as_deref() {
            Some("farm_config_v1") => self.import_farm_config(record, payload),
            Some("listing_draft_v1") => self.import_listing_draft(record, payload),
            _ => Ok(None),
        }
    }

    fn import_signed_event(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        match record.event_kind {
            Some(KIND_FARM) => self.import_signed_farm(record),
            Some(KIND_LISTING | KIND_LISTING_DRAFT) => self.import_signed_listing(record),
            Some(KIND_VALIDATION_RECEIPT) => self.import_signed_validation_receipt(record),
            Some(kind) if active_order_event_kind(kind) => self.import_signed_active_order(record),
            _ => Ok(Some(ProjectionRecord {
                kind: "signed_event",
                projected_id: record.event_id.clone(),
            })),
        }
    }

    fn import_farm_config(
        &self,
        record: &LocalEventRecord,
        payload: &Value,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        let Some(document) = payload.get("document") else {
            return Ok(None);
        };
        let Some(farm_key) = record
            .farm_id
            .clone()
            .or_else(|| string_at(document, &["selection", "farm_d_tag"]))
            .or_else(|| string_at(document, &["farm", "d_tag"]))
        else {
            return Ok(None);
        };
        let owner_pubkey = record.owner_pubkey.clone();
        let Some(farm_id) = projected_farm_id(
            record.source_runtime,
            owner_pubkey.as_deref(),
            farm_key.as_str(),
        ) else {
            return Ok(None);
        };
        let display_name = string_at(document, &["profile", "display_name"])
            .or_else(|| string_at(document, &["profile", "name"]))
            .or_else(|| string_at(document, &["farm", "name"]))
            .unwrap_or_else(|| "Local farm".to_owned());
        let location = string_at(document, &["farm", "location", "primary"])
            .or_else(|| string_at(document, &["listing_defaults", "location", "primary"]))
            .unwrap_or_default();
        let methods = string_at(document, &["listing_defaults", "delivery_method"])
            .and_then(|method| farm_order_method(method.as_str()))
            .into_iter()
            .collect::<Vec<_>>();
        let saved_farm = FarmSummary {
            farm_id,
            display_name: display_name.clone(),
            readiness: FarmReadiness::Incomplete,
        };
        self.upsert_local_work_farm_summary(&saved_farm)?;
        let owner_account_id = record
            .owner_account_id
            .clone()
            .or_else(|| string_at(document, &["selection", "account"]));
        if let Some(owner_account_id) = owner_account_id.as_deref() {
            let projection = FarmSetupProjection::new(
                FarmSetupDraft::new(display_name, location, methods),
                Some(saved_farm),
            );
            AppFarmSetupRepository::new(self.connection)
                .save_farm_setup(owner_account_id, &projection)?;
        }
        Ok(Some(ProjectionRecord {
            kind: "farm",
            projected_id: Some(farm_id.to_string()),
        }))
    }

    fn import_listing_draft(
        &self,
        record: &LocalEventRecord,
        payload: &Value,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        let Some(document) = payload.get("document") else {
            return Ok(None);
        };
        let Some(listing_key) =
            string_at(document, &["listing", "d_tag"]).or_else(|| listing_id(record))
        else {
            return Ok(None);
        };
        let owner_pubkey = record
            .owner_pubkey
            .clone()
            .or_else(|| string_at(document, &["seller_actor", "pubkey"]));
        let farm_key = record
            .farm_id
            .clone()
            .or_else(|| string_at(document, &["listing", "farm_d_tag"]));
        let Some(farm_key) = farm_key else {
            return Ok(None);
        };
        let Some(farm_id) = projected_farm_id(
            record.source_runtime,
            owner_pubkey.as_deref(),
            farm_key.as_str(),
        ) else {
            return Ok(None);
        };
        self.ensure_farm_exists(farm_id)?;
        let Some(product_id) = projected_product_id(
            record.source_runtime,
            owner_pubkey.as_deref(),
            listing_key.as_str(),
        ) else {
            return Ok(None);
        };
        let title = string_at(document, &["product", "title"])
            .or_else(|| string_at(document, &["product", "key"]))
            .unwrap_or_else(|| "Local product".to_owned());
        let subtitle = string_at(document, &["product", "summary"]).unwrap_or_default();
        let unit_label = string_at(document, &["primary_bin", "quantity_unit"])
            .or_else(|| string_at(document, &["primary_bin", "price_per_unit"]))
            .unwrap_or_default();
        let listing_bin_id = string_at(document, &["primary_bin", "bin_id"]);
        let price_minor_units = string_at(document, &["primary_bin", "price_amount"])
            .and_then(|price| parse_decimal_minor_units(price.as_str()));
        let price_currency = string_at(document, &["primary_bin", "price_currency"])
            .unwrap_or_else(|| "USD".to_owned());
        let stock_count = string_at(document, &["inventory", "available"])
            .and_then(|quantity| parse_u32_quantity(quantity.as_str()));
        self.upsert_product(ProductProjection {
            product_id,
            farm_id,
            title,
            subtitle,
            status: ProductStatus::Draft,
            unit_label,
            price_minor_units,
            price_currency,
            stock_count,
            availability_window_id: None,
            listing_bin_id,
        })?;
        Ok(Some(ProjectionRecord {
            kind: "listing",
            projected_id: Some(product_id.to_string()),
        }))
    }

    fn import_signed_farm(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        let Some(content) = record.event_content.as_deref() else {
            return Ok(None);
        };
        let content = parse_json_value(content)?;
        let tags = record.event_tags_json.as_ref();
        let Some(farm_key) = tag_index_value(tags, "d", 1)
            .or_else(|| string_at(&content, &["d_tag"]))
            .or_else(|| record.farm_id.clone())
        else {
            return Ok(None);
        };
        let owner_pubkey = record
            .event_pubkey
            .as_deref()
            .or(record.owner_pubkey.as_deref());
        let Some(farm_id) =
            projected_farm_id(record.source_runtime, owner_pubkey, farm_key.as_str())
        else {
            return Ok(None);
        };
        let display_name =
            string_at(&content, &["name"]).unwrap_or_else(|| "Local farm".to_owned());
        let readiness = match signed_farm_readiness(&content, tags) {
            Some(readiness) => readiness,
            None => self
                .load_farm_readiness(farm_id)?
                .unwrap_or(FarmReadiness::Incomplete),
        };
        self.upsert_farm_summary(&FarmSummary {
            farm_id,
            display_name,
            readiness,
        })?;
        Ok(Some(ProjectionRecord {
            kind: "farm",
            projected_id: Some(farm_id.to_string()),
        }))
    }

    fn import_signed_listing(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        let content = record
            .event_content
            .as_deref()
            .and_then(parse_json_value_opt);
        let tags = record.event_tags_json.as_ref();
        let listing_key = content
            .as_ref()
            .and_then(|content| string_at(content, &["d_tag"]))
            .or_else(|| tag_index_value(tags, "d", 1))
            .or_else(|| listing_id(record));
        let Some(listing_key) = listing_key else {
            return Ok(None);
        };
        let farm_key = content
            .as_ref()
            .and_then(|content| string_at(content, &["farm", "d_tag"]))
            .or_else(|| tag_index_value(tags, "a", 1).and_then(|addr| address_d_tag(&addr)))
            .or_else(|| record.farm_id.clone());
        let Some(farm_key) = farm_key else {
            return Ok(None);
        };
        let signed_farm_pubkey = content
            .as_ref()
            .and_then(|content| string_at(content, &["farm", "pubkey"]))
            .or_else(|| tag_index_value(tags, "a", 1).and_then(|addr| address_pubkey(&addr)));
        let farm_pubkey = signed_farm_pubkey
            .as_deref()
            .or(record.event_pubkey.as_deref())
            .or(record.owner_pubkey.as_deref());
        let listing_pubkey = record
            .event_pubkey
            .as_deref()
            .or(signed_farm_pubkey.as_deref())
            .or(record.owner_pubkey.as_deref());
        let app_shaped_network_listing = record.source_runtime == SourceRuntime::Network
            && parse_app_d_tag_uuid(farm_key.as_str()).is_some()
            && parse_app_d_tag_uuid(listing_key.as_str()).is_some();
        let mut existing_projection = if app_shaped_network_listing {
            None
        } else {
            self.existing_listing_projection(record.listing_addr.as_deref())?
        };
        if existing_projection.is_none() {
            existing_projection = self.existing_app_origin_listing_projection(
                record,
                farm_key.as_str(),
                listing_key.as_str(),
                listing_pubkey,
                tags,
            )?;
        }
        let (farm_id, product_id) = if let Some(existing_projection) = existing_projection {
            (existing_projection.farm_id, existing_projection.product_id)
        } else {
            let Some(farm_id) =
                projected_farm_id(record.source_runtime, farm_pubkey, farm_key.as_str())
            else {
                return Ok(None);
            };
            let Some(product_id) =
                projected_product_id(record.source_runtime, listing_pubkey, listing_key.as_str())
            else {
                return Ok(None);
            };
            (farm_id, product_id)
        };
        let projection_record = ProjectionRecord {
            kind: "listing",
            projected_id: Some(product_id.to_string()),
        };
        if !self.signed_listing_is_current(record, listing_key.as_str())? {
            return Ok(Some(projection_record));
        }
        self.ensure_farm_exists(farm_id)?;
        let title = content
            .as_ref()
            .and_then(|content| string_at(content, &["product", "title"]))
            .or_else(|| tag_index_value(tags, "title", 1))
            .or_else(|| {
                content
                    .as_ref()
                    .and_then(|content| string_at(content, &["product", "key"]))
            })
            .or_else(|| tag_index_value(tags, "key", 1))
            .unwrap_or_else(|| "Local product".to_owned());
        let subtitle = content
            .as_ref()
            .and_then(|content| string_at(content, &["product", "summary"]))
            .or_else(|| tag_index_value(tags, "summary", 1))
            .unwrap_or_default();
        let bin = content.as_ref().and_then(primary_bin);
        let listing_bin_id = bin
            .and_then(|value| string_at(value, &["bin_id"]))
            .or_else(|| tag_index_value(tags, "radroots:bin", 1));
        let unit_label = bin
            .and_then(|value| {
                string_at(value, &["quantity", "unit"])
                    .or_else(|| string_at(value, &["display_unit"]))
                    .or_else(|| string_at(value, &["display_price_unit"]))
            })
            .or_else(|| tag_index_value(tags, "radroots:bin", 3))
            .unwrap_or_default();
        let price_minor_units = bin
            .and_then(|value| {
                string_at(value, &["price_per_canonical_unit", "amount", "amount"])
                    .or_else(|| string_at(value, &["display_price", "amount"]))
                    .and_then(|price| parse_decimal_minor_units(price.as_str()))
            })
            .or_else(|| {
                tag_index_value(tags, "radroots:price", 2)
                    .or_else(|| tag_index_value(tags, "price", 1))
                    .and_then(|price| parse_decimal_minor_units(price.as_str()))
            });
        let price_currency = bin
            .and_then(|value| {
                string_at(value, &["price_per_canonical_unit", "amount", "currency"])
                    .or_else(|| string_at(value, &["display_price", "currency"]))
            })
            .or_else(|| tag_index_value(tags, "radroots:price", 3))
            .or_else(|| tag_index_value(tags, "price", 2))
            .unwrap_or_else(|| "USD".to_owned());
        let stock_count = content
            .as_ref()
            .and_then(|content| string_at(content, &["inventory_available"]))
            .or_else(|| tag_index_value(tags, "inventory", 1))
            .and_then(|quantity| parse_u32_quantity(quantity.as_str()));
        let Some(status) = signed_listing_product_status(record, content.as_ref(), tags) else {
            return Ok(None);
        };
        let fulfillment_method = signed_listing_fulfillment_method(content.as_ref(), tags);
        let availability_window_id = if status == ProductStatus::Published {
            match fulfillment_method {
                Some(method) => self.ensure_signed_listing_availability_window(
                    farm_id,
                    listing_key.as_str(),
                    content.as_ref(),
                    tags,
                    method,
                )?,
                None => None,
            }
        } else {
            None
        };
        if availability_window_id.is_some()
            && let Some(method) = fulfillment_method
        {
            self.mark_farm_buyer_visible(farm_id, record, method)?;
        }
        self.upsert_product(ProductProjection {
            product_id,
            farm_id,
            title,
            subtitle,
            status,
            unit_label,
            price_minor_units,
            price_currency,
            stock_count,
            availability_window_id,
            listing_bin_id,
        })?;
        Ok(Some(projection_record))
    }

    fn import_signed_active_order(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        if !signed_event_record_is_usable(record) {
            return Ok(Some(signed_event_projection(record)));
        }
        let Some(event) = signed_event_from_record(record)? else {
            return Ok(Some(signed_event_projection(record)));
        };
        let Some(current_evidence) = active_order_evidence_from_event(&event) else {
            return Ok(Some(signed_event_projection(record)));
        };
        self.project_active_order(record, current_evidence)?;
        Ok(Some(signed_event_projection(record)))
    }

    fn import_signed_validation_receipt(
        &self,
        record: &LocalEventRecord,
    ) -> Result<Option<ProjectionRecord>, AppSqliteError> {
        if !signed_event_record_is_usable(record) {
            return Ok(Some(signed_event_projection(record)));
        }
        let Some(event) = signed_event_from_record(record)? else {
            return Ok(Some(signed_event_projection(record)));
        };
        let Ok(verified) = validation_receipt_from_event(&event) else {
            return Ok(Some(signed_event_projection(record)));
        };
        self.upsert_validation_receipt_projection(&event, &verified.receipt, &verified.tags)?;
        Ok(Some(ProjectionRecord {
            kind: "validation_receipt",
            projected_id: Some(event.id),
        }))
    }

    fn project_active_order(
        &self,
        record: &LocalEventRecord,
        current_evidence: ActiveOrderEvidence,
    ) -> Result<(), AppSqliteError> {
        if let ActiveOrderEvidence::Request(request) = &current_evidence {
            let order_id = self.upsert_order_request(record, &request.payload)?;
            self.attach_validation_receipts_for_request(
                request.event_id.as_str(),
                request.payload.order_id.as_str(),
                order_id,
            )?;
        }
        let mut evidence = self.load_active_order_evidence(current_evidence.order_id())?;
        evidence.push(current_evidence);
        dedupe_active_order_evidence(&mut evidence);
        let Some((raw_order_id, buyer_pubkey)) = evidence
            .first()
            .map(ActiveOrderEvidence::order_projection_identity)
        else {
            return Ok(());
        };
        let raw_order_id = raw_order_id.to_owned();
        let buyer_pubkey = buyer_pubkey.to_owned();
        let order_id = projected_order_id(raw_order_id.as_str(), buyer_pubkey.as_str());
        let buckets = ActiveOrderEvidenceBuckets::from_evidence(evidence);
        let requests = buckets.requests.clone();
        let revision_proposals = buckets.revision_proposals.clone();
        let revision_decisions = buckets.revision_decisions.clone();
        let reducer_order_id =
            raw_order_id
                .parse::<RadrootsOrderId>()
                .map_err(|_| AppSqliteError::DecodeId {
                    field: "order_id",
                    value: raw_order_id.clone(),
                })?;
        let projection = reduce_order_events(
            &reducer_order_id,
            RadrootsOrderReductionInputs {
                requests: buckets.requests,
                decisions: buckets.decisions,
                revision_proposals: buckets.revision_proposals,
                revision_decisions: buckets.revision_decisions,
                cancellations: buckets.cancellations,
            },
        );
        let request_payload = projection.request_event_id.as_deref().and_then(|event_id| {
            requests
                .iter()
                .find(|request| request.event_id == event_id)
                .map(|request| &request.payload)
        });
        let revision =
            active_order_revision_status(&projection, &revision_proposals, &revision_decisions);
        let agreement_source = request_payload.map(|request| {
            active_order_agreement_source(
                request,
                &projection,
                &revision_proposals,
                &revision_decisions,
            )
        });
        self.apply_active_order_projection(
            order_id,
            &projection,
            revision,
            agreement_source.as_ref(),
        )
    }

    fn upsert_order_request(
        &self,
        record: &LocalEventRecord,
        payload: &RadrootsOrderRequest,
    ) -> Result<OrderId, AppSqliteError> {
        let existing_listing =
            self.existing_listing_projection(Some(payload.listing_addr.as_str()))?;
        let farm_id = if let Some(existing_listing) = existing_listing.as_ref() {
            existing_listing.farm_id
        } else {
            deterministic_farm_id(
                Some(payload.seller_pubkey.as_str()),
                payload.listing_addr.as_str(),
            )
        };
        self.ensure_farm_exists(farm_id)?;
        let order_id = projected_order_id(payload.order_id.as_str(), payload.buyer_pubkey.as_str());
        let order_number = existing_order_number(self.connection, order_id)?
            .unwrap_or_else(|| deterministic_order_number(payload.order_id.as_str()));
        self.connection
            .execute(
                "INSERT INTO orders (
                    id,
                    farm_id,
                    fulfillment_window_id,
                    order_number,
                    customer_display_name,
                    status,
                    updated_at,
                    buyer_context_key,
                    buyer_email,
                    buyer_phone,
                    buyer_order_note
                 ) VALUES (?1, ?2, null, ?3, ?4, 'needs_action', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?5, '', '', '')
                 ON CONFLICT(id) DO UPDATE SET
                    farm_id = excluded.farm_id,
                    order_number = excluded.order_number,
                    customer_display_name = excluded.customer_display_name,
                    status = excluded.status,
                    buyer_context_key = coalesce(orders.buyer_context_key, excluded.buyer_context_key),
                    updated_at = excluded.updated_at",
                params![
                    order_id.to_string(),
                    farm_id.to_string(),
                    order_number.as_str(),
                    order_customer_display_name(payload.buyer_pubkey.as_str()),
                    order_buyer_context_key(record, payload.buyer_pubkey.as_str()),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop order request",
                source,
            })?;
        self.replace_order_request_lines(order_id, payload, existing_listing.as_ref(), record)?;
        Ok(order_id)
    }

    fn apply_active_order_projection(
        &self,
        order_id: OrderId,
        projection: &RadrootsOrderProjection,
        revision: TradeRevisionStatus,
        agreement_source: Option<&ActiveOrderAgreementSource>,
    ) -> Result<(), AppSqliteError> {
        let workflow = TradeWorkflowProjection::from_active_order_projection(
            order_id,
            projection,
            revision,
            TradeProvenanceProjection::from_primary_source(TradeWorkflowSource::LocalEvents),
        );
        let Some(status) = order_status_from_active_order_projection(projection) else {
            return Ok(());
        };
        self.connection
            .execute(
                "UPDATE orders
                 SET status = ?2,
                     workflow_revision = ?3,
                     workflow_agreement = ?4,
                     workflow_inventory = ?5,
                     workflow_provenance_source = ?6,
                     workflow_provenance_last_event_id = ?7,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1",
                params![
                    workflow.order_id.to_string(),
                    status.storage_key(),
                    workflow.revision.storage_key(),
                    workflow.agreement.storage_key(),
                    workflow.inventory.storage_key(),
                    workflow.provenance.primary_source.storage_key(),
                    workflow.provenance.last_event_id.as_deref()
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "apply local interop active order projection",
                source,
            })?;
        if projection.economics.is_some()
            && let Some(agreement_source) = agreement_source
        {
            self.replace_active_order_agreement_lines(workflow.order_id, agreement_source)?;
        }
        self.apply_validation_receipt_workflow_projection(workflow.order_id)?;
        Ok(())
    }

    fn upsert_validation_receipt_projection(
        &self,
        event: &RadrootsNostrEvent,
        receipt: &RadrootsTradeValidationReceipt,
        tags: &RadrootsValidationReceiptTags,
    ) -> Result<(), AppSqliteError> {
        let order_id = match self.validation_receipt_order_attachment(tags)? {
            ValidationReceiptOrderAttachment::Pending => None,
            ValidationReceiptOrderAttachment::Attached(order_id) => Some(order_id),
            ValidationReceiptOrderAttachment::Rejected => return Ok(()),
        };
        let result = TradeValidationReceiptResult::from_validation_receipt_result(receipt.result);
        let receipt_type =
            TradeValidationReceiptType::from_validation_receipt_type(receipt.receipt_type);
        let proof_system = TradeValidationReceiptProofSystem::from_validation_receipt_proof_system(
            receipt.proof.system,
        );

        self.connection
            .execute(
                "INSERT INTO order_validation_receipts (
                    event_id,
                    order_id,
                    raw_order_id,
                    root_event_id,
                    listing_event_id,
                    target_event_id,
                    receipt_type,
                    result,
                    proof_system,
                    event_set_root,
                    reducer_output_root,
                    public_values_hash,
                    event_created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(event_id) DO UPDATE SET
                    order_id = excluded.order_id,
                    raw_order_id = excluded.raw_order_id,
                    root_event_id = excluded.root_event_id,
                    listing_event_id = excluded.listing_event_id,
                    target_event_id = excluded.target_event_id,
                    receipt_type = excluded.receipt_type,
                    result = excluded.result,
                    proof_system = excluded.proof_system,
                    event_set_root = excluded.event_set_root,
                    reducer_output_root = excluded.reducer_output_root,
                    public_values_hash = excluded.public_values_hash,
                    event_created_at = excluded.event_created_at",
                params![
                    event.id.as_str(),
                    order_id.map(|order_id| order_id.to_string()),
                    tags.order_id.as_str(),
                    tags.root_event_id.as_str(),
                    tags.listing_event_id.as_str(),
                    tags.target_event_id.as_str(),
                    receipt_type.storage_key(),
                    result.storage_key(),
                    proof_system.storage_key(),
                    tags.event_set_root.as_str(),
                    tags.reducer_output_root.as_str(),
                    tags.public_values_hash.as_str(),
                    i64::from(event.created_at),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop validation receipt projection",
                source,
            })?;
        if let Some(order_id) = order_id {
            self.apply_validation_receipt_workflow_projection(order_id)?;
        }
        Ok(())
    }

    fn validation_receipt_order_attachment(
        &self,
        tags: &RadrootsValidationReceiptTags,
    ) -> Result<ValidationReceiptOrderAttachment, AppSqliteError> {
        let Some(root_event) = self.load_signed_event_by_event_id(tags.root_event_id.as_str())?
        else {
            return Ok(ValidationReceiptOrderAttachment::Pending);
        };
        let Ok(envelope) = order_request_from_event(&root_event) else {
            return Ok(ValidationReceiptOrderAttachment::Rejected);
        };
        if envelope.payload.order_id != tags.order_id {
            return Ok(ValidationReceiptOrderAttachment::Rejected);
        }

        Ok(ValidationReceiptOrderAttachment::Attached(
            projected_order_id(
                envelope.payload.order_id.as_str(),
                envelope.payload.buyer_pubkey.as_str(),
            ),
        ))
    }

    fn attach_validation_receipts_for_request(
        &self,
        root_event_id: &str,
        raw_order_id: &str,
        order_id: OrderId,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "UPDATE order_validation_receipts
                 SET order_id = ?3
                 WHERE root_event_id = ?1
                    AND raw_order_id = ?2
                    AND order_id IS NULL",
                params![root_event_id, raw_order_id, order_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "attach local interop validation receipts to request",
                source,
            })?;
        self.apply_validation_receipt_workflow_projection(order_id)?;
        Ok(())
    }

    fn apply_validation_receipt_workflow_projection(
        &self,
        order_id: OrderId,
    ) -> Result<(), AppSqliteError> {
        let current: Option<(String, String, Option<String>)> = self
            .connection
            .query_row(
                "SELECT status, workflow_agreement, workflow_provenance_last_event_id
                 FROM orders
                 WHERE id = ?1",
                params![order_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load order workflow before validation receipt projection",
                source,
            })?;
        let Some((status, agreement, Some(target_event_id))) = current else {
            return Ok(());
        };
        if !matches!(
            agreement.as_str(),
            value if value == TradeAgreementStatus::AgreedPendingRhi.storage_key()
                || value == TradeAgreementStatus::Committed.storage_key()
                || value == TradeAgreementStatus::Invalid.storage_key()
        ) {
            return Ok(());
        }
        if !matches!(
            status.as_str(),
            value if value == OrderStatus::NeedsAction.storage_key()
                || value == OrderStatus::Scheduled.storage_key()
                || value == OrderStatus::NeedsReview.storage_key()
        ) {
            return Ok(());
        }
        let latest_receipt: Option<String> = self
            .connection
            .query_row(
                "SELECT result
                 FROM order_validation_receipts
                 WHERE order_id = ?1
                    AND receipt_type = 'trade_transition'
                    AND target_event_id = ?2
                 ORDER BY event_created_at DESC, event_id DESC
                 LIMIT 1",
                params![order_id.to_string(), target_event_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load latest validation receipt workflow projection",
                source,
            })?;
        let Some(result) = latest_receipt else {
            return Ok(());
        };
        let (status, agreement, inventory) = match result.as_str() {
            "valid" => (
                OrderStatus::Scheduled.storage_key(),
                TradeAgreementStatus::Committed.storage_key(),
                TradeInventoryStatus::Reserved.storage_key(),
            ),
            "needs_review" => (
                OrderStatus::NeedsReview.storage_key(),
                TradeAgreementStatus::Invalid.storage_key(),
                TradeInventoryStatus::NeedsReview.storage_key(),
            ),
            _ => {
                return Err(AppSqliteError::DecodeEnum {
                    field: "order_validation_receipts.result",
                    value: result,
                });
            }
        };
        self.connection
            .execute(
                "UPDATE orders
                 SET status = ?2,
                     workflow_agreement = ?3,
                     workflow_inventory = ?4,
                     workflow_provenance_source = ?5,
                     workflow_provenance_last_event_id = ?6,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1",
                params![
                    order_id.to_string(),
                    status,
                    agreement,
                    inventory,
                    TradeWorkflowSource::LocalEvents.storage_key(),
                    target_event_id,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "apply validation receipt workflow projection",
                source,
            })?;
        Ok(())
    }

    fn load_active_order_evidence(
        &self,
        order_id: &str,
    ) -> Result<Vec<ActiveOrderEvidence>, AppSqliteError> {
        let mut evidence = Vec::new();
        for kind in ACTIVE_ORDER_EVENT_KINDS {
            for event in self.load_signed_events_by_kind(kind)? {
                let Some(record) = active_order_evidence_from_event(&event) else {
                    continue;
                };
                if record.order_id() == order_id {
                    evidence.push(record);
                }
            }
        }
        Ok(evidence)
    }

    fn replace_order_request_lines(
        &self,
        order_id: OrderId,
        payload: &RadrootsOrderRequest,
        existing_listing: Option<&ExistingListingProjection>,
        record: &LocalEventRecord,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "DELETE FROM order_lines WHERE order_id = ?1",
                params![order_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "replace local interop order lines",
                source,
            })?;
        for (index, item) in payload.items.iter().enumerate() {
            let economics_item = payload
                .economics
                .items
                .iter()
                .find(|candidate| candidate.bin_id == item.bin_id);
            let unit_label = economics_item
                .map(|item| item.quantity_unit.to_string())
                .or_else(|| existing_listing.map(|listing| listing.unit_label.clone()))
                .unwrap_or_else(|| "item".to_owned());
            let unit_price_minor_units = economics_item.and_then(|item| {
                parse_decimal_minor_units(item.unit_price_amount.to_string().as_str())
            });
            let price_currency = economics_item
                .map(|item| item.unit_price_currency.to_string())
                .unwrap_or_else(|| payload.economics.currency.to_string());
            let title = existing_listing
                .map(|listing| listing.title.clone())
                .unwrap_or_else(|| item.bin_id.to_string());
            self.connection
                .execute(
                    "INSERT INTO order_lines (
                        id,
                        order_id,
                        title,
                        quantity_value,
                        quantity_unit_label,
                        quantity_display,
                        listing_bin_id,
                        unit_price_minor_units,
                        price_currency,
                        farm_key,
                        listing_addr,
                        listing_event_id,
                        listing_relays_json,
                        seller_pubkey,
                        sort_index
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, null, ?13, ?14)",
                    params![
                        format!(
                            "{}:{}",
                            order_id,
                            order_line_product_id(payload, existing_listing, item)
                        ),
                        order_id.to_string(),
                        title.as_str(),
                        i64::from(item.bin_count),
                        unit_label.as_str(),
                        format_quantity_display(item.bin_count, unit_label.as_str()),
                        item.bin_id.as_str(),
                        unit_price_minor_units,
                        price_currency.as_str(),
                        existing_listing.and_then(|listing| listing.farm_key.as_deref()),
                        payload.listing_addr.as_str(),
                        listing_event_id_from_order_record(record).as_deref(),
                        payload.seller_pubkey.as_str(),
                        index as i64,
                    ],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "insert local interop order line",
                    source,
                })?;
        }
        Ok(())
    }

    fn replace_active_order_agreement_lines(
        &self,
        order_id: OrderId,
        source: &ActiveOrderAgreementSource,
    ) -> Result<(), AppSqliteError> {
        let existing_listing =
            self.existing_listing_projection(Some(source.listing_addr.as_str()))?;
        let metadata = self.existing_order_line_metadata(order_id)?;
        self.connection
            .execute(
                "DELETE FROM order_lines WHERE order_id = ?1",
                params![order_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "replace local interop active order agreement lines",
                source,
            })?;
        for (index, item) in source.items.iter().enumerate() {
            let economics_item = source
                .economics
                .items
                .iter()
                .find(|candidate| candidate.bin_id == item.bin_id);
            let unit_label = economics_item
                .map(|item| item.quantity_unit.to_string())
                .or_else(|| {
                    existing_listing
                        .as_ref()
                        .map(|listing| listing.unit_label.clone())
                })
                .unwrap_or_else(|| "item".to_owned());
            let unit_price_minor_units = economics_item.and_then(|item| {
                parse_decimal_minor_units(item.unit_price_amount.to_string().as_str())
            });
            let price_currency = economics_item
                .map(|item| item.unit_price_currency.to_string())
                .unwrap_or_else(|| source.economics.currency.to_string());
            let title = existing_listing
                .as_ref()
                .filter(|listing| {
                    listing
                        .listing_bin_id
                        .as_deref()
                        .is_none_or(|listing_bin_id| listing_bin_id == item.bin_id)
                })
                .map(|listing| listing.title.clone())
                .unwrap_or_else(|| item.bin_id.to_string());
            self.connection
                .execute(
                    "INSERT INTO order_lines (
                        id,
                        order_id,
                        title,
                        quantity_value,
                        quantity_unit_label,
                        quantity_display,
                        listing_bin_id,
                        unit_price_minor_units,
                        price_currency,
                        farm_key,
                        listing_addr,
                        listing_event_id,
                        listing_relays_json,
                        seller_pubkey,
                        sort_index
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        format!(
                            "{}:{}",
                            order_id,
                            order_agreement_line_product_id(
                                source.listing_addr.as_str(),
                                source.seller_pubkey.as_str(),
                                existing_listing.as_ref(),
                                item,
                            )
                        ),
                        order_id.to_string(),
                        title.as_str(),
                        i64::from(item.bin_count),
                        unit_label.as_str(),
                        format_quantity_display(item.bin_count, unit_label.as_str()),
                        item.bin_id.as_str(),
                        unit_price_minor_units,
                        price_currency.as_str(),
                        existing_listing
                            .as_ref()
                            .and_then(|listing| listing.farm_key.as_deref()),
                        source.listing_addr.as_str(),
                        metadata
                            .as_ref()
                            .and_then(|metadata| metadata.listing_event_id.as_deref()),
                        metadata
                            .as_ref()
                            .and_then(|metadata| metadata.listing_relays_json.as_deref()),
                        source.seller_pubkey.as_str(),
                        index as i64,
                    ],
                )
                .map_err(|source| AppSqliteError::Query {
                    operation: "insert local interop active order agreement line",
                    source,
                })?;
        }
        Ok(())
    }

    fn existing_order_line_metadata(
        &self,
        order_id: OrderId,
    ) -> Result<Option<ExistingOrderLineMetadata>, AppSqliteError> {
        self.connection
            .query_row(
                "SELECT listing_event_id, listing_relays_json
                 FROM order_lines
                 WHERE order_id = ?1
                 ORDER BY sort_index ASC, id ASC
                 LIMIT 1",
                params![order_id.to_string()],
                |row| {
                    Ok(ExistingOrderLineMetadata {
                        listing_event_id: row.get::<_, Option<String>>(0)?,
                        listing_relays_json: row.get::<_, Option<String>>(1)?,
                    })
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load existing local interop order line metadata",
                source,
            })
    }

    fn upsert_farm_summary(&self, farm: &FarmSummary) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO farms (id, display_name, readiness, created_at, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    display_name = excluded.display_name,
                    readiness = excluded.readiness,
                    updated_at = excluded.updated_at",
                params![
                    farm.farm_id.to_string(),
                    farm.display_name.as_str(),
                    farm_readiness_storage_key(farm.readiness),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop farm summary",
                source,
            })?;
        Ok(())
    }

    fn upsert_local_work_farm_summary(&self, farm: &FarmSummary) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO farms (id, display_name, readiness, created_at, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    display_name = excluded.display_name,
                    readiness = CASE
                        WHEN farms.readiness = 'ready' AND excluded.readiness = 'incomplete'
                        THEN farms.readiness
                        ELSE excluded.readiness
                    END,
                    updated_at = excluded.updated_at",
                params![
                    farm.farm_id.to_string(),
                    farm.display_name.as_str(),
                    farm_readiness_storage_key(farm.readiness),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop local work farm summary",
                source,
            })?;
        Ok(())
    }

    fn mark_farm_buyer_visible(
        &self,
        farm_id: FarmId,
        record: &LocalEventRecord,
        method: FarmOrderMethod,
    ) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "UPDATE farms
                 SET readiness = 'ready',
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?1",
                [farm_id.to_string()],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "mark local interop farm buyer visible",
                source,
            })?;
        let Some(account_id) = record
            .owner_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(());
        };
        let display_name = self
            .load_farm_display_name(farm_id)?
            .unwrap_or_else(|| "Local farm".to_owned());
        self.connection
            .execute(
                "INSERT INTO account_farm_setups (
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
                 ) VALUES (?1, ?2, '', ?3, ?4, ?5, ?6, ?2, 'ready', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(account_id) DO UPDATE SET
                    farm_name = CASE
                        WHEN trim(account_farm_setups.farm_name) = '' THEN excluded.farm_name
                        ELSE account_farm_setups.farm_name
                    END,
                    pickup_enabled = max(account_farm_setups.pickup_enabled, excluded.pickup_enabled),
                    delivery_enabled = max(account_farm_setups.delivery_enabled, excluded.delivery_enabled),
                    shipping_enabled = max(account_farm_setups.shipping_enabled, excluded.shipping_enabled),
                    saved_farm_id = excluded.saved_farm_id,
                    saved_farm_display_name = excluded.saved_farm_display_name,
                    saved_farm_readiness = excluded.saved_farm_readiness,
                    updated_at = excluded.updated_at",
                params![
                    account_id,
                    display_name.as_str(),
                    i64::from(method == FarmOrderMethod::Pickup),
                    i64::from(method == FarmOrderMethod::Delivery),
                    i64::from(method == FarmOrderMethod::Shipping),
                    farm_id.to_string(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop buyer fulfillment method",
                source,
            })?;
        Ok(())
    }

    fn ensure_farm_exists(&self, farm_id: FarmId) -> Result<(), AppSqliteError> {
        let exists = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM farms WHERE id = ?1)",
                [farm_id.to_string()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "check local interop farm existence",
                source,
            })?;
        if !exists {
            self.upsert_farm_summary(&FarmSummary {
                farm_id,
                display_name: "Local farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            })?;
        }
        Ok(())
    }

    fn load_farm_display_name(&self, farm_id: FarmId) -> Result<Option<String>, AppSqliteError> {
        self.connection
            .query_row(
                "SELECT display_name FROM farms WHERE id = ?1 LIMIT 1",
                [farm_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load local interop farm display name",
                source,
            })
    }

    fn load_farm_readiness(
        &self,
        farm_id: FarmId,
    ) -> Result<Option<FarmReadiness>, AppSqliteError> {
        self.connection
            .query_row(
                "SELECT readiness FROM farms WHERE id = ?1 LIMIT 1",
                [farm_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load local interop farm readiness",
                source,
            })?
            .map(|readiness| farm_readiness_from_storage_key(readiness.as_str()))
            .transpose()
    }

    fn ensure_signed_listing_availability_window(
        &self,
        farm_id: FarmId,
        listing_key: &str,
        content: Option<&Value>,
        tags: Option<&Value>,
        method: FarmOrderMethod,
    ) -> Result<Option<FulfillmentWindowId>, AppSqliteError> {
        let Some(window) = signed_listing_availability_window(content, tags) else {
            return Ok(None);
        };
        let starts_at =
            self.unix_epoch_to_utc_timestamp(window.start, "format listing availability start")?;
        let ends_at =
            self.unix_epoch_to_utc_timestamp(window.end, "format listing availability end")?;
        if ends_at <= starts_at {
            return Ok(None);
        }
        let pickup_location_id = if method == FarmOrderMethod::Pickup {
            let Some(location_primary) = signed_listing_location_primary(content, tags) else {
                return Ok(None);
            };
            Some(self.upsert_signed_listing_pickup_location(farm_id, location_primary.as_str())?)
        } else {
            None
        };
        let farm_id_string = farm_id.to_string();
        let fulfillment_window_id = FulfillmentWindowId::from(deterministic_uuid(
            "radroots-app-local-interop-fulfillment-window",
            Some(farm_id_string.as_str()),
            listing_key,
        ));
        self.connection
            .execute(
                "INSERT INTO fulfillment_windows (
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
                 ) VALUES (?1, ?2, ?3, ?4, null, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?5, '', ?3)
                 ON CONFLICT(id) DO UPDATE SET
                    farm_id = excluded.farm_id,
                    starts_at = excluded.starts_at,
                    ends_at = excluded.ends_at,
                    pickup_location_id = excluded.pickup_location_id,
                    order_cutoff_at = excluded.order_cutoff_at,
                    updated_at = excluded.updated_at",
                params![
                    fulfillment_window_id.to_string(),
                    farm_id_string.as_str(),
                    starts_at.as_str(),
                    ends_at.as_str(),
                    pickup_location_id.map(|id| id.to_string()),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop listing fulfillment window",
                source,
            })?;
        Ok(Some(fulfillment_window_id))
    }

    fn upsert_signed_listing_pickup_location(
        &self,
        farm_id: FarmId,
        location_primary: &str,
    ) -> Result<PickupLocationId, AppSqliteError> {
        let farm_id_string = farm_id.to_string();
        let pickup_location_id = PickupLocationId::from(deterministic_uuid(
            "radroots-app-local-interop-pickup-location",
            Some(farm_id_string.as_str()),
            location_primary,
        ));
        self.connection
            .execute(
                "INSERT INTO pickup_locations (
                    id,
                    farm_id,
                    label,
                    address_line,
                    directions,
                    is_default,
                    created_at,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?3, null, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    farm_id = excluded.farm_id,
                    label = excluded.label,
                    address_line = excluded.address_line,
                    updated_at = excluded.updated_at",
                params![
                    pickup_location_id.to_string(),
                    farm_id_string.as_str(),
                    location_primary,
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop listing pickup location",
                source,
            })?;
        Ok(pickup_location_id)
    }

    fn unix_epoch_to_utc_timestamp(
        &self,
        seconds: u64,
        operation: &'static str,
    ) -> Result<String, AppSqliteError> {
        let seconds = i64::try_from(seconds).map_err(|_| AppSqliteError::InvalidProjection {
            reason: "listing availability timestamp is out of range",
        })?;
        let timestamp = self
            .connection
            .query_row(
                "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', ?1, 'unixepoch')",
                [seconds],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|source| AppSqliteError::Query { operation, source })?;
        timestamp.ok_or(AppSqliteError::InvalidProjection {
            reason: "listing availability timestamp is invalid",
        })
    }

    fn upsert_product(&self, projection: ProductProjection) -> Result<(), AppSqliteError> {
        self.connection
            .execute(
                "INSERT INTO products (
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
                    listing_bin_id,
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    farm_id = excluded.farm_id,
                    title = excluded.title,
                    subtitle = excluded.subtitle,
                    status = CASE
                        WHEN excluded.status = 'draft'
                            AND products.status IN ('published', 'paused', 'archived')
                        THEN products.status
                        ELSE excluded.status
                    END,
                    unit_label = excluded.unit_label,
                    price_minor_units = excluded.price_minor_units,
                    price_currency = excluded.price_currency,
                    stock_count = excluded.stock_count,
                    availability_window_id = CASE
                        WHEN excluded.status = 'draft'
                            AND products.status IN ('published', 'paused', 'archived')
                        THEN products.availability_window_id
                        ELSE excluded.availability_window_id
                    END,
                    listing_bin_id = coalesce(excluded.listing_bin_id, products.listing_bin_id),
                    updated_at = excluded.updated_at",
                params![
                    projection.product_id.to_string(),
                    projection.farm_id.to_string(),
                    projection.title.as_str(),
                    projection.subtitle.as_str(),
                    projection.status.storage_key(),
                    projection.unit_label.as_str(),
                    projection.price_minor_units,
                    projection.price_currency.as_str(),
                    projection.stock_count,
                    projection.availability_window_id.map(|id| id.to_string()),
                    projection.listing_bin_id.as_deref(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop product",
                source,
            })?;
        Ok(())
    }

    fn existing_listing_projection(
        &self,
        listing_addr: Option<&str>,
    ) -> Result<Option<ExistingListingProjection>, AppSqliteError> {
        let Some(listing_addr) = listing_addr
            .map(str::trim)
            .filter(|listing_addr| !listing_addr.is_empty())
        else {
            return Ok(None);
        };
        let Some((product_id, farm_id, title, unit_label, listing_bin_id, farm_key)) = self
            .connection
            .query_row(
                "SELECT
                    products.id,
                    products.farm_id,
                    products.title,
                    products.unit_label,
                    products.listing_bin_id,
                    local_interop_imports.farm_key
                 FROM local_interop_imports
                 JOIN products ON products.id = local_interop_imports.projected_id
                 WHERE local_interop_imports.projected_kind = 'listing'
                    AND local_interop_imports.projected_id IS NOT NULL
                    AND local_interop_imports.listing_addr = ?1
                 ORDER BY local_interop_imports.local_seq DESC
                 LIMIT 1",
                [listing_addr],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| AppSqliteError::Query {
                operation: "load existing local interop listing projection",
                source,
            })?
        else {
            return Ok(None);
        };
        Ok(Some(ExistingListingProjection {
            product_id: product_id
                .parse()
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "existing listing projection product id must parse",
                })?,
            farm_id: farm_id
                .parse()
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "existing listing projection farm id must parse",
                })?,
            title,
            unit_label,
            listing_bin_id,
            farm_key,
        }))
    }

    fn existing_app_origin_listing_projection(
        &self,
        record: &LocalEventRecord,
        farm_key: &str,
        listing_key: &str,
        listing_pubkey: Option<&str>,
        tags: Option<&Value>,
    ) -> Result<Option<ExistingListingProjection>, AppSqliteError> {
        if record.source_runtime != SourceRuntime::Network {
            return Ok(None);
        }
        let Some(farm_id) = parse_app_d_tag_uuid(farm_key).map(FarmId::from) else {
            return Ok(None);
        };
        let Some(product_id) = parse_app_d_tag_uuid(listing_key).map(ProductId::from) else {
            return Ok(None);
        };
        let Some(listing_addr) = record
            .listing_addr
            .as_deref()
            .map(str::trim)
            .filter(|listing_addr| !listing_addr.is_empty())
        else {
            return Ok(None);
        };
        let Some(listing_addr_parts) = listing_address_parts(listing_addr) else {
            return Ok(None);
        };
        let Some(event_pubkey) = record
            .event_pubkey
            .as_deref()
            .map(str::trim)
            .filter(|event_pubkey| !event_pubkey.is_empty())
        else {
            return Ok(None);
        };
        if listing_addr_parts.kind != KIND_LISTING
            || listing_addr_parts.pubkey != event_pubkey
            || listing_addr_parts.d_tag != listing_key
            || listing_pubkey.map(str::trim) != Some(event_pubkey)
            || !signed_farm_address_matches(tags, farm_key, event_pubkey)
        {
            return Ok(None);
        }
        let Some((product_id, farm_id, title, unit_label, listing_bin_id, evidence_farm_key)) =
            self.connection
                .query_row(
                    "SELECT
                    products.id,
                    products.farm_id,
                    products.title,
                    products.unit_label,
                    products.listing_bin_id,
                    local_interop_imports.farm_key
                 FROM local_interop_imports
                 JOIN products ON products.id = local_interop_imports.projected_id
                 WHERE local_interop_imports.projected_kind = 'listing'
                    AND local_interop_imports.projected_id = ?1
                    AND local_interop_imports.source_runtime = 'app'
                    AND local_interop_imports.farm_key = ?2
                    AND local_interop_imports.listing_addr = ?3
                    AND local_interop_imports.owner_pubkey = ?4
                    AND products.id = ?1
                    AND products.farm_id = ?5
                 LIMIT 1",
                    params![
                        product_id.to_string(),
                        farm_key,
                        listing_addr,
                        event_pubkey,
                        farm_id.to_string(),
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|source| AppSqliteError::Query {
                    operation: "load existing app-origin listing projection",
                    source,
                })?
        else {
            return Ok(None);
        };
        Ok(Some(ExistingListingProjection {
            product_id: product_id
                .parse()
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "existing app-origin listing projection product id must parse",
                })?,
            farm_id: farm_id
                .parse()
                .map_err(|_| AppSqliteError::InvalidProjection {
                    reason: "existing app-origin listing projection farm id must parse",
                })?,
            title,
            unit_label,
            listing_bin_id,
            farm_key: evidence_farm_key,
        }))
    }

    fn signed_listing_is_current(
        &self,
        record: &LocalEventRecord,
        listing_key: &str,
    ) -> Result<bool, AppSqliteError> {
        if !signed_listing_has_public_evidence(record) {
            return Ok(true);
        }
        let Some(incoming_key) = listing_currentness_key(
            record.event_created_at,
            record.event_id.as_deref(),
            signed_event_evidence_precedence(
                record.source_runtime.as_str(),
                record.owner_account_id.as_deref(),
                record.status.as_str(),
                record.outbox_status.as_str(),
            ),
        ) else {
            return Ok(true);
        };
        let Some(identity) = ListingCurrentnessIdentity::from_record(record, listing_key) else {
            return Ok(true);
        };
        let Some(current_key) = self.current_listing_key(&identity)? else {
            return Ok(true);
        };
        Ok(incoming_key >= current_key)
    }

    fn current_listing_key(
        &self,
        identity: &ListingCurrentnessIdentity,
    ) -> Result<Option<ListingCurrentnessKey>, AppSqliteError> {
        let mut keys = Vec::new();
        match identity {
            ListingCurrentnessIdentity::ListingAddress(listing_addr) => {
                let mut statement = self
                    .connection
                    .prepare(
                        "SELECT
                            event_id,
                            event_created_at,
                            source_runtime,
                            owner_account_id,
                            local_status,
                            outbox_status,
                            relay_delivery_json
                         FROM local_interop_imports
                         WHERE record_family = 'signed_event'
                            AND projected_kind = 'listing'
                            AND listing_addr = ?1",
                    )
                    .map_err(|source| AppSqliteError::Query {
                        operation: "prepare current listing-address evidence query",
                        source,
                    })?;
                let rows = statement
                    .query_map(params![listing_addr.as_str()], listing_currentness_row)
                    .map_err(|source| AppSqliteError::Query {
                        operation: "query current listing-address evidence",
                        source,
                    })?;
                for row in rows {
                    let evidence = row.map_err(|source| AppSqliteError::Query {
                        operation: "read current listing-address evidence",
                        source,
                    })?;
                    if let Some(key) = evidence.into_currentness_key() {
                        keys.push(key);
                    }
                }
            }
            ListingCurrentnessIdentity::KindPubkeyDTag {
                event_kind,
                event_pubkey,
                listing_key,
            } => {
                let mut statement = self
                    .connection
                    .prepare(
                        "SELECT
                            event_id,
                            event_created_at,
                            source_runtime,
                            owner_account_id,
                            local_status,
                            outbox_status,
                            relay_delivery_json,
                            event_tags_json,
                            event_content,
                            listing_addr
                         FROM local_interop_imports
                         WHERE record_family = 'signed_event'
                            AND projected_kind = 'listing'
                            AND event_kind = ?1
                            AND event_pubkey = ?2",
                    )
                    .map_err(|source| AppSqliteError::Query {
                        operation: "prepare current listing identity evidence query",
                        source,
                    })?;
                let rows = statement
                    .query_map(
                        params![event_kind, event_pubkey.as_str()],
                        listing_currentness_identity_row,
                    )
                    .map_err(|source| AppSqliteError::Query {
                        operation: "query current listing identity evidence",
                        source,
                    })?;
                for row in rows {
                    let evidence = row.map_err(|source| AppSqliteError::Query {
                        operation: "read current listing identity evidence",
                        source,
                    })?;
                    if evidence.listing_key().as_deref() == Some(listing_key.as_str())
                        && let Some(key) = evidence.currentness.into_currentness_key()
                    {
                        keys.push(key);
                    }
                }
            }
        }
        Ok(keys.into_iter().max())
    }

    fn record_import(
        &self,
        record: &LocalEventRecord,
        projected_kind: &str,
        projected_id: Option<String>,
    ) -> Result<(), AppSqliteError> {
        let event_tags_json = record
            .event_tags_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "local interop event tags json must encode",
            })?;
        let raw_event_json = record
            .raw_event_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "local interop raw event json must encode",
            })?;
        let relay_delivery_json = record
            .relay_delivery_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppSqliteError::InvalidProjection {
                reason: "local interop relay delivery json must encode",
            })?;
        self.connection
            .execute(
                "INSERT INTO local_interop_imports (
                    record_id,
                    local_seq,
                    record_family,
                    local_status,
                    source_runtime,
                    owner_account_id,
                    owner_pubkey,
                    farm_key,
                    listing_addr,
                    projected_kind,
                    projected_id,
                    event_id,
                    event_kind,
                    event_pubkey,
                    event_created_at,
                    event_tags_json,
                    event_content,
                    event_sig,
                    raw_event_json,
                    outbox_status,
                    relay_delivery_json,
                    imported_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(record_id) DO UPDATE SET
                    local_seq = excluded.local_seq,
                    record_family = excluded.record_family,
                    local_status = excluded.local_status,
                    source_runtime = excluded.source_runtime,
                    owner_account_id = excluded.owner_account_id,
                    owner_pubkey = excluded.owner_pubkey,
                    farm_key = excluded.farm_key,
                    listing_addr = excluded.listing_addr,
                    projected_kind = excluded.projected_kind,
                    projected_id = excluded.projected_id,
                    event_id = excluded.event_id,
                    event_kind = excluded.event_kind,
                    event_pubkey = excluded.event_pubkey,
                    event_created_at = excluded.event_created_at,
                    event_tags_json = excluded.event_tags_json,
                    event_content = excluded.event_content,
                    event_sig = excluded.event_sig,
                    raw_event_json = excluded.raw_event_json,
                    outbox_status = excluded.outbox_status,
                    relay_delivery_json = excluded.relay_delivery_json,
                    imported_at = excluded.imported_at",
                params![
                    record.record_id.as_str(),
                    record.seq,
                    record.family.as_str(),
                    record.status.as_str(),
                    record.source_runtime.as_str(),
                    record.owner_account_id.as_deref(),
                    record.owner_pubkey.as_deref(),
                    record.farm_id.as_deref(),
                    record.listing_addr.as_deref(),
                    projected_kind,
                    projected_id.as_deref(),
                    record.event_id.as_deref(),
                    record.event_kind,
                    record.event_pubkey.as_deref(),
                    record.event_created_at,
                    event_tags_json.as_deref(),
                    record.event_content.as_deref(),
                    record.event_sig.as_deref(),
                    raw_event_json.as_deref(),
                    record.outbox_status.as_str(),
                    relay_delivery_json.as_deref(),
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "record local interop import",
                source,
            })?;
        Ok(())
    }
}

impl AppSqliteStore {
    pub fn local_interop_repository(&self) -> AppLocalInteropRepository<'_> {
        AppLocalInteropRepository::new(&self.connection)
    }

    pub fn import_shared_local_events_from_path(
        &self,
        shared_database_path: &Path,
    ) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        self.local_interop_repository()
            .import_from_path(shared_database_path)
    }

    pub fn import_shared_local_events_from_store<E>(
        &self,
        store: &LocalEventsStore<E>,
    ) -> Result<AppLocalInteropImportReport, AppSqliteError>
    where
        E: SqlExecutor,
    {
        self.local_interop_repository().import_from_store(store)
    }

    pub fn import_local_event_records(
        &self,
        records: &[LocalEventRecord],
    ) -> Result<AppLocalInteropImportReport, AppSqliteError> {
        self.local_interop_repository().import_records(records)
    }

    pub fn load_local_interop_records(
        &self,
    ) -> Result<Vec<StoredLocalInteropRecord>, AppSqliteError> {
        self.local_interop_repository().load_records()
    }

    pub fn load_local_interop_signed_events_by_kind(
        &self,
        event_kind: i64,
    ) -> Result<Vec<RadrootsNostrEvent>, AppSqliteError> {
        self.local_interop_repository()
            .load_signed_events_by_kind(event_kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImportOutcome {
    Imported,
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DuplicateSignedEventAction {
    Import,
    ReplaceExisting(String),
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidationReceiptOrderAttachment {
    Pending,
    Attached(OrderId),
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectionRecord {
    kind: &'static str,
    projected_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredSignedEventDuplicate {
    source_runtime: String,
    owner_account_id: Option<String>,
    local_status: String,
    outbox_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredLocalInteropSignedEventEvidence {
    event_id: Option<String>,
    event_kind: Option<i64>,
    local_status: String,
    outbox_status: String,
    relay_delivery_json: Option<String>,
    event_pubkey: Option<String>,
    event_created_at: Option<i64>,
    event_tags_json: Option<String>,
    event_content: Option<String>,
    event_sig: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredListingCurrentnessEvidence {
    event_id: Option<String>,
    event_created_at: Option<i64>,
    source_runtime: String,
    owner_account_id: Option<String>,
    local_status: String,
    outbox_status: String,
    relay_delivery_json: Option<String>,
}

impl StoredListingCurrentnessEvidence {
    fn into_currentness_key(self) -> Option<ListingCurrentnessKey> {
        if !signed_event_import_has_public_evidence(
            self.local_status.as_str(),
            self.outbox_status.as_str(),
            self.relay_delivery_json.as_deref(),
        ) {
            return None;
        }
        listing_currentness_key(
            self.event_created_at,
            self.event_id.as_deref(),
            signed_event_evidence_precedence(
                self.source_runtime.as_str(),
                self.owner_account_id.as_deref(),
                self.local_status.as_str(),
                self.outbox_status.as_str(),
            ),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredListingCurrentnessIdentityEvidence {
    currentness: StoredListingCurrentnessEvidence,
    event_tags_json: Option<String>,
    event_content: Option<String>,
    listing_addr: Option<String>,
}

impl StoredListingCurrentnessIdentityEvidence {
    fn listing_key(&self) -> Option<String> {
        self.event_content
            .as_deref()
            .and_then(parse_json_value_opt)
            .and_then(|content| string_at(&content, &["d_tag"]))
            .or_else(|| {
                self.event_tags_json
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                    .and_then(|tags| tag_index_value(Some(&tags), "d", 1))
            })
            .or_else(|| self.listing_addr.as_deref().and_then(address_d_tag))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ListingCurrentnessIdentity {
    ListingAddress(String),
    KindPubkeyDTag {
        event_kind: i64,
        event_pubkey: String,
        listing_key: String,
    },
}

impl ListingCurrentnessIdentity {
    fn from_record(record: &LocalEventRecord, listing_key: &str) -> Option<Self> {
        if let Some(listing_addr) = record
            .listing_addr
            .as_deref()
            .map(str::trim)
            .filter(|listing_addr| !listing_addr.is_empty())
        {
            return Some(Self::ListingAddress(listing_addr.to_owned()));
        }
        let event_kind = record.event_kind?;
        let event_pubkey = record
            .event_pubkey
            .as_deref()
            .map(str::trim)
            .filter(|event_pubkey| !event_pubkey.is_empty())?;
        Some(Self::KindPubkeyDTag {
            event_kind,
            event_pubkey: event_pubkey.to_owned(),
            listing_key: listing_key.to_owned(),
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ListingCurrentnessKey {
    event_created_at: i64,
    evidence_precedence: u8,
    event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProductProjection {
    product_id: ProductId,
    farm_id: FarmId,
    title: String,
    subtitle: String,
    status: ProductStatus,
    unit_label: String,
    price_minor_units: Option<u32>,
    price_currency: String,
    stock_count: Option<u32>,
    availability_window_id: Option<FulfillmentWindowId>,
    listing_bin_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExistingListingProjection {
    product_id: ProductId,
    farm_id: FarmId,
    title: String,
    unit_label: String,
    listing_bin_id: Option<String>,
    farm_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveOrderAgreementSource {
    listing_addr: String,
    seller_pubkey: String,
    items: Vec<RadrootsOrderItem>,
    economics: RadrootsOrderEconomics,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExistingOrderLineMetadata {
    listing_event_id: Option<String>,
    listing_relays_json: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActiveOrderEvidence {
    Request(RadrootsOrderRequestRecord),
    Decision(RadrootsOrderDecisionRecord),
    RevisionProposal(RadrootsOrderRevisionProposalRecord),
    RevisionDecision(RadrootsOrderRevisionDecisionRecord),
    Cancellation(RadrootsOrderCancellationRecord),
}

impl ActiveOrderEvidence {
    fn event_id(&self) -> &str {
        match self {
            Self::Request(record) => record.event_id.as_str(),
            Self::Decision(record) => record.event_id.as_str(),
            Self::RevisionProposal(record) => record.event_id.as_str(),
            Self::RevisionDecision(record) => record.event_id.as_str(),
            Self::Cancellation(record) => record.event_id.as_str(),
        }
    }

    fn order_id(&self) -> &str {
        match self {
            Self::Request(record) => record.payload.order_id.as_str(),
            Self::Decision(record) => record.payload.order_id.as_str(),
            Self::RevisionProposal(record) => record.payload.order_id.as_str(),
            Self::RevisionDecision(record) => record.payload.order_id.as_str(),
            Self::Cancellation(record) => record.payload.order_id.as_str(),
        }
    }

    fn order_projection_identity(&self) -> (&str, &str) {
        match self {
            Self::Request(record) => (
                record.payload.order_id.as_str(),
                record.payload.buyer_pubkey.as_str(),
            ),
            Self::Decision(record) => (
                record.payload.order_id.as_str(),
                record.payload.buyer_pubkey.as_str(),
            ),
            Self::RevisionProposal(record) => (
                record.payload.order_id.as_str(),
                record.payload.buyer_pubkey.as_str(),
            ),
            Self::RevisionDecision(record) => (
                record.payload.order_id.as_str(),
                record.payload.buyer_pubkey.as_str(),
            ),
            Self::Cancellation(record) => (
                record.payload.order_id.as_str(),
                record.payload.buyer_pubkey.as_str(),
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ActiveOrderEvidenceBuckets {
    requests: Vec<RadrootsOrderRequestRecord>,
    decisions: Vec<RadrootsOrderDecisionRecord>,
    revision_proposals: Vec<RadrootsOrderRevisionProposalRecord>,
    revision_decisions: Vec<RadrootsOrderRevisionDecisionRecord>,
    cancellations: Vec<RadrootsOrderCancellationRecord>,
}

impl ActiveOrderEvidenceBuckets {
    fn from_evidence(evidence: Vec<ActiveOrderEvidence>) -> Self {
        let mut buckets = Self::default();
        for record in evidence {
            match record {
                ActiveOrderEvidence::Request(record) => buckets.requests.push(record),
                ActiveOrderEvidence::Decision(record) => buckets.decisions.push(record),
                ActiveOrderEvidence::RevisionProposal(record) => {
                    buckets.revision_proposals.push(record);
                }
                ActiveOrderEvidence::RevisionDecision(record) => {
                    buckets.revision_decisions.push(record);
                }
                ActiveOrderEvidence::Cancellation(record) => buckets.cancellations.push(record),
            }
        }
        buckets
    }
}

fn listing_currentness_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredListingCurrentnessEvidence> {
    Ok(StoredListingCurrentnessEvidence {
        event_id: row.get(0)?,
        event_created_at: row.get(1)?,
        source_runtime: row.get(2)?,
        owner_account_id: row.get(3)?,
        local_status: row.get(4)?,
        outbox_status: row.get(5)?,
        relay_delivery_json: row.get(6)?,
    })
}

fn listing_currentness_identity_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<StoredListingCurrentnessIdentityEvidence> {
    Ok(StoredListingCurrentnessIdentityEvidence {
        currentness: StoredListingCurrentnessEvidence {
            event_id: row.get(0)?,
            event_created_at: row.get(1)?,
            source_runtime: row.get(2)?,
            owner_account_id: row.get(3)?,
            local_status: row.get(4)?,
            outbox_status: row.get(5)?,
            relay_delivery_json: row.get(6)?,
        },
        event_tags_json: row.get(7)?,
        event_content: row.get(8)?,
        listing_addr: row.get(9)?,
    })
}

fn listing_currentness_key(
    event_created_at: Option<i64>,
    event_id: Option<&str>,
    evidence_precedence: u8,
) -> Option<ListingCurrentnessKey> {
    Some(ListingCurrentnessKey {
        event_created_at: event_created_at?,
        evidence_precedence,
        event_id: event_id
            .map(str::trim)
            .filter(|event_id| !event_id.is_empty())?
            .to_owned(),
    })
}

fn signed_event_evidence_precedence(
    source_runtime: &str,
    owner_account_id: Option<&str>,
    local_status: &str,
    outbox_status: &str,
) -> u8 {
    let mut precedence = 0;
    if local_status == LocalRecordStatus::Published.as_str() {
        precedence += 1;
    }
    if outbox_status == PublishOutboxStatus::Acknowledged.as_str() {
        precedence += 2;
    }
    if owner_account_id
        .map(str::trim)
        .is_some_and(|owner_account_id| !owner_account_id.is_empty())
    {
        precedence += 4;
    }
    if source_runtime == SourceRuntime::App.as_str() {
        precedence += 8;
    }
    precedence
}

fn deterministic_farm_id(owner_pubkey: Option<&str>, farm_key: &str) -> FarmId {
    FarmId::from(deterministic_uuid(
        "radroots-cli-farm",
        owner_pubkey,
        farm_key,
    ))
}

fn deterministic_product_id(owner_pubkey: Option<&str>, listing_key: &str) -> ProductId {
    ProductId::from(deterministic_uuid(
        "radroots-cli-listing",
        owner_pubkey,
        listing_key,
    ))
}

fn projected_farm_id(
    source_runtime: SourceRuntime,
    owner_pubkey: Option<&str>,
    farm_key: &str,
) -> Option<FarmId> {
    match source_runtime {
        SourceRuntime::App => parse_app_d_tag_uuid(farm_key).map(FarmId::from),
        _ => Some(deterministic_farm_id(owner_pubkey, farm_key)),
    }
}

fn projected_product_id(
    source_runtime: SourceRuntime,
    owner_pubkey: Option<&str>,
    listing_key: &str,
) -> Option<ProductId> {
    match source_runtime {
        SourceRuntime::App => parse_app_d_tag_uuid(listing_key).map(ProductId::from),
        _ => Some(deterministic_product_id(owner_pubkey, listing_key)),
    }
}

fn deterministic_uuid(scope: &str, owner_pubkey: Option<&str>, key: &str) -> Uuid {
    let seed = format!(
        "{scope}:{}:{}",
        owner_pubkey.unwrap_or("unknown-owner"),
        key.trim()
    );
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes())
}

fn parse_app_d_tag_uuid(value: &str) -> Option<Uuid> {
    let mut decoded = Vec::with_capacity(16);
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in value.trim().bytes() {
        let digit = base64_url_digit(byte)?;
        buffer = (buffer << 6) | u32::from(digit);
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            decoded.push(((buffer >> bits) & 0xff) as u8);
            buffer &= (1u32 << bits) - 1;
        }
    }
    if bits > 0 && buffer != 0 {
        return None;
    }
    if decoded.len() == 16 {
        Uuid::from_slice(decoded.as_slice()).ok()
    } else {
        None
    }
}

fn active_order_event_kind(kind: i64) -> bool {
    ACTIVE_ORDER_EVENT_KINDS.contains(&kind)
}

fn active_event_id(event: &RadrootsNostrEvent) -> Option<RadrootsEventId> {
    event.id.parse().ok()
}

fn active_author_pubkey(event: &RadrootsNostrEvent) -> Option<RadrootsPublicKey> {
    event.author.parse().ok()
}

fn active_order_evidence_from_event(event: &RadrootsNostrEvent) -> Option<ActiveOrderEvidence> {
    match i64::from(event.kind) {
        KIND_ORDER_REQUEST => {
            let envelope = order_request_from_event(event).ok()?;
            Some(ActiveOrderEvidence::Request(RadrootsOrderRequestRecord {
                event_id: active_event_id(event)?,
                author_pubkey: active_author_pubkey(event)?,
                payload: envelope.payload,
            }))
        }
        KIND_ORDER_DECISION => {
            let envelope = order_decision_from_event(event).ok()?;
            let context = order_event_context_from_tags(envelope.message_type, &event.tags).ok()?;
            Some(ActiveOrderEvidence::Decision(RadrootsOrderDecisionRecord {
                event_id: active_event_id(event)?,
                author_pubkey: active_author_pubkey(event)?,
                counterparty_pubkey: context.counterparty_pubkey,
                root_event_id: context.root_event_id?,
                prev_event_id: context.prev_event_id?,
                payload: envelope.payload,
            }))
        }
        KIND_ORDER_REVISION => {
            let envelope = order_revision_proposal_from_event(event).ok()?;
            let context = order_event_context_from_tags(envelope.message_type, &event.tags).ok()?;
            Some(ActiveOrderEvidence::RevisionProposal(
                RadrootsOrderRevisionProposalRecord {
                    event_id: active_event_id(event)?,
                    author_pubkey: active_author_pubkey(event)?,
                    counterparty_pubkey: context.counterparty_pubkey,
                    root_event_id: context.root_event_id?,
                    prev_event_id: context.prev_event_id?,
                    payload: envelope.payload,
                },
            ))
        }
        KIND_ORDER_REVISION_DECISION => {
            let envelope = order_revision_decision_from_event(event).ok()?;
            let context = order_event_context_from_tags(envelope.message_type, &event.tags).ok()?;
            Some(ActiveOrderEvidence::RevisionDecision(
                RadrootsOrderRevisionDecisionRecord {
                    event_id: active_event_id(event)?,
                    author_pubkey: active_author_pubkey(event)?,
                    counterparty_pubkey: context.counterparty_pubkey,
                    root_event_id: context.root_event_id?,
                    prev_event_id: context.prev_event_id?,
                    payload: envelope.payload,
                },
            ))
        }
        KIND_ORDER_CANCEL => {
            let envelope = order_cancellation_from_event(event).ok()?;
            let context = order_event_context_from_tags(envelope.message_type, &event.tags).ok()?;
            Some(ActiveOrderEvidence::Cancellation(
                RadrootsOrderCancellationRecord {
                    event_id: active_event_id(event)?,
                    author_pubkey: active_author_pubkey(event)?,
                    counterparty_pubkey: context.counterparty_pubkey,
                    root_event_id: context.root_event_id?,
                    prev_event_id: context.prev_event_id?,
                    payload: envelope.payload,
                },
            ))
        }
        _ => None,
    }
}

fn dedupe_active_order_evidence(evidence: &mut Vec<ActiveOrderEvidence>) {
    evidence.sort_by(|left, right| left.event_id().cmp(right.event_id()));
    evidence.dedup_by(|left, right| left.event_id() == right.event_id());
}

fn signed_event_projection(record: &LocalEventRecord) -> ProjectionRecord {
    ProjectionRecord {
        kind: "signed_event",
        projected_id: record.event_id.clone(),
    }
}

fn signed_event_from_record(
    record: &LocalEventRecord,
) -> Result<Option<RadrootsNostrEvent>, AppSqliteError> {
    let Some(id) = record
        .event_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(author) = record
        .event_pubkey
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(kind) = record.event_kind.and_then(|kind| u32::try_from(kind).ok()) else {
        return Ok(None);
    };
    let Some(created_at) = record
        .event_created_at
        .and_then(|created_at| u32::try_from(created_at).ok())
    else {
        return Ok(None);
    };
    let Some(sig) = record
        .event_sig
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(tags) = record.event_tags_json.as_ref().and_then(tags_from_json) else {
        return Ok(None);
    };
    Ok(Some(RadrootsNostrEvent {
        id: id.to_owned(),
        author: author.to_owned(),
        created_at,
        kind,
        tags,
        content: record.event_content.clone().unwrap_or_default(),
        sig: sig.to_owned(),
    }))
}

fn signed_event_record_is_usable(record: &LocalEventRecord) -> bool {
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
    matches!(
        relay_delivery.state,
        RelayDeliveryState::Acknowledged | RelayDeliveryState::Observed
    )
}

fn signed_event_local_interop_evidence_is_usable(
    evidence: &StoredLocalInteropSignedEventEvidence,
) -> bool {
    if evidence.local_status != LocalRecordStatus::Published.as_str()
        || matches!(evidence.outbox_status.as_str(), "pending" | "failed")
    {
        return false;
    }
    let Some(relay_delivery_json) = evidence.relay_delivery_json.as_deref() else {
        return false;
    };
    let Ok(relay_delivery_value) = serde_json::from_str::<Value>(relay_delivery_json) else {
        return false;
    };
    let Ok(relay_delivery) = RelayDeliveryEvidence::from_json_value(&relay_delivery_value) else {
        return false;
    };
    matches!(
        relay_delivery.state,
        RelayDeliveryState::Acknowledged | RelayDeliveryState::Observed
    )
}

fn signed_event_from_local_interop_evidence(
    evidence: &StoredLocalInteropSignedEventEvidence,
) -> Result<Option<RadrootsNostrEvent>, AppSqliteError> {
    let Some(id) = evidence
        .event_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(author) = evidence
        .event_pubkey
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(kind) = evidence
        .event_kind
        .and_then(|kind| u32::try_from(kind).ok())
    else {
        return Ok(None);
    };
    let Some(created_at) = evidence
        .event_created_at
        .and_then(|created_at| u32::try_from(created_at).ok())
    else {
        return Ok(None);
    };
    let Some(sig) = evidence
        .event_sig
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(tags_json) = evidence.event_tags_json.as_deref() else {
        return Ok(None);
    };
    let Ok(tags_value) = serde_json::from_str::<Value>(tags_json) else {
        return Ok(None);
    };
    let Some(tags) = tags_from_json(&tags_value) else {
        return Ok(None);
    };
    Ok(Some(RadrootsNostrEvent {
        id: id.to_owned(),
        author: author.to_owned(),
        created_at,
        kind,
        tags,
        content: evidence.event_content.clone().unwrap_or_default(),
        sig: sig.to_owned(),
    }))
}

fn tags_from_json(value: &Value) -> Option<Vec<Vec<String>>> {
    value.as_array().map(|tags| {
        tags.iter()
            .filter_map(|tag| {
                tag.as_array().map(|values| {
                    values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
    })
}

pub fn projected_order_id_from_trade_request(order_id: &str, buyer_pubkey: &str) -> OrderId {
    order_id.parse().unwrap_or_else(|_| {
        OrderId::from(deterministic_uuid(
            "radroots-cli-order",
            Some(buyer_pubkey),
            order_id,
        ))
    })
}

fn projected_order_id(order_id: &str, buyer_pubkey: &str) -> OrderId {
    projected_order_id_from_trade_request(order_id, buyer_pubkey)
}

fn active_order_revision_status(
    projection: &RadrootsOrderProjection,
    revision_proposals: &[RadrootsOrderRevisionProposalRecord],
    revision_decisions: &[RadrootsOrderRevisionDecisionRecord],
) -> TradeRevisionStatus {
    let Some(mut parent_event_id) = projection
        .decision_event_id
        .clone()
        .or_else(|| projection.request_event_id.clone())
    else {
        return TradeRevisionStatus::None;
    };
    let mut status = TradeRevisionStatus::None;
    loop {
        let matching_proposals = revision_proposals
            .iter()
            .filter(|proposal| proposal.prev_event_id == parent_event_id)
            .collect::<Vec<_>>();
        let proposal = match matching_proposals.as_slice() {
            [] => return status,
            [proposal] => *proposal,
            _ => return TradeRevisionStatus::None,
        };
        let matching_decisions = revision_decisions
            .iter()
            .filter(|decision| decision.prev_event_id == proposal.event_id)
            .collect::<Vec<_>>();
        let decision = match matching_decisions.as_slice() {
            [] => return TradeRevisionStatus::ChangeProposed,
            [decision] => *decision,
            _ => return TradeRevisionStatus::None,
        };
        if decision.payload.revision_id != proposal.payload.revision_id {
            return TradeRevisionStatus::None;
        }
        status = match &decision.payload.decision {
            RadrootsOrderRevisionOutcome::Accepted => TradeRevisionStatus::Updated,
            RadrootsOrderRevisionOutcome::Declined { .. } => TradeRevisionStatus::KeptAsPlaced,
        };
        parent_event_id.clone_from(&decision.event_id);
    }
}

fn active_order_agreement_source(
    request: &RadrootsOrderRequest,
    projection: &RadrootsOrderProjection,
    revision_proposals: &[RadrootsOrderRevisionProposalRecord],
    revision_decisions: &[RadrootsOrderRevisionDecisionRecord],
) -> ActiveOrderAgreementSource {
    if let Some(agreement_event_id) = projection.agreement_event_id.as_deref()
        && projection.decision_event_id.as_deref() != Some(agreement_event_id)
        && let Some(revision_decision) = revision_decisions.iter().find(|decision| {
            decision.event_id == agreement_event_id
                && matches!(
                    &decision.payload.decision,
                    RadrootsOrderRevisionOutcome::Accepted
                )
        })
        && let Some(revision_proposal) = revision_proposals.iter().find(|proposal| {
            proposal.event_id == revision_decision.prev_event_id
                && proposal.payload.revision_id == revision_decision.payload.revision_id
        })
    {
        return ActiveOrderAgreementSource {
            listing_addr: revision_proposal.payload.listing_addr.to_string(),
            seller_pubkey: revision_proposal.payload.seller_pubkey.to_string(),
            items: revision_proposal.payload.items.clone(),
            economics: revision_proposal.payload.economics.clone(),
        };
    }
    ActiveOrderAgreementSource {
        listing_addr: request.listing_addr.to_string(),
        seller_pubkey: request.seller_pubkey.to_string(),
        items: request.items.clone(),
        economics: request.economics.clone(),
    }
}

fn order_line_product_id(
    payload: &RadrootsOrderRequest,
    existing_listing: Option<&ExistingListingProjection>,
    item: &radroots_events::order::RadrootsOrderItem,
) -> ProductId {
    order_agreement_line_product_id(
        payload.listing_addr.as_str(),
        payload.seller_pubkey.as_str(),
        existing_listing,
        item,
    )
}

fn order_agreement_line_product_id(
    listing_addr: &str,
    seller_pubkey: &str,
    existing_listing: Option<&ExistingListingProjection>,
    item: &RadrootsOrderItem,
) -> ProductId {
    if let Some(existing_listing) = existing_listing
        && existing_listing
            .listing_bin_id
            .as_deref()
            .is_none_or(|listing_bin_id| listing_bin_id == item.bin_id)
    {
        return existing_listing.product_id;
    }
    let product_key = format!("{listing_addr}:{}", item.bin_id);
    deterministic_product_id(Some(seller_pubkey), product_key.as_str())
}

fn deterministic_order_number(order_id: &str) -> String {
    let trimmed = order_id.trim();
    let suffix = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>();
    if suffix.is_empty() {
        "R-RELAY".to_owned()
    } else {
        format!("R-{suffix}")
    }
}

fn existing_order_number(
    connection: &Connection,
    order_id: OrderId,
) -> Result<Option<String>, AppSqliteError> {
    connection
        .query_row(
            "SELECT order_number FROM orders WHERE id = ?1 LIMIT 1",
            params![order_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|source| AppSqliteError::Query {
            operation: "load existing local interop order number",
            source,
        })
}

fn order_customer_display_name(buyer_pubkey: &str) -> String {
    let prefix = buyer_pubkey.trim().chars().take(12).collect::<String>();
    if prefix.is_empty() {
        "Relay buyer".to_owned()
    } else {
        format!("Relay buyer {prefix}")
    }
}

fn order_buyer_context_key(record: &LocalEventRecord, buyer_pubkey: &str) -> String {
    if record.source_runtime == SourceRuntime::App
        && record
            .event_pubkey
            .as_deref()
            .map(str::trim)
            .is_some_and(|event_pubkey| event_pubkey == buyer_pubkey.trim())
        && let Some(owner_account_id) = record
            .owner_account_id
            .as_deref()
            .map(str::trim)
            .filter(|owner_account_id| !owner_account_id.is_empty())
    {
        return format!("account:{owner_account_id}");
    }
    format!("nostr:{}", buyer_pubkey.trim())
}

fn format_quantity_display(quantity: u32, unit_label: &str) -> String {
    let unit_label = unit_label.trim();
    if unit_label.is_empty() {
        quantity.to_string()
    } else {
        format!("{quantity} {unit_label}")
    }
}

fn listing_event_id_from_order_record(record: &LocalEventRecord) -> Option<String> {
    record
        .event_tags_json
        .as_ref()
        .and_then(|tags| tag_index_value(Some(tags), "listing_event", 1))
}

fn base64_url_digit(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    match cursor {
        Value::String(value) => {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn listing_id(record: &LocalEventRecord) -> Option<String> {
    record
        .listing_addr
        .as_deref()
        .and_then(|addr| addr.rsplit(':').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn farm_order_method(value: &str) -> Option<FarmOrderMethod> {
    match value.trim() {
        "pickup" => Some(FarmOrderMethod::Pickup),
        "delivery" | "local_delivery" => Some(FarmOrderMethod::Delivery),
        "shipping" => Some(FarmOrderMethod::Shipping),
        _ => None,
    }
}

fn parse_decimal_minor_units(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return None;
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let whole_units = whole.parse::<u32>().ok()?;
    let cents = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u32>().ok()? * 10,
        _ => fraction.get(0..2)?.parse::<u32>().ok()?,
    };
    whole_units.checked_mul(100)?.checked_add(cents)
}

fn parse_u32_quantity(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return None;
    }
    let whole = value.split_once('.').map_or(value, |(whole, _)| whole);
    whole.parse::<u32>().ok()
}

fn parse_u64_quantity(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return None;
    }
    let whole = value.split_once('.').map_or(value, |(whole, _)| whole);
    whole.parse::<u64>().ok()
}

fn signed_listing_product_status(
    record: &LocalEventRecord,
    content: Option<&Value>,
    tags: Option<&Value>,
) -> Option<ProductStatus> {
    if !signed_listing_has_public_evidence(record) {
        return Some(ProductStatus::Draft);
    }
    match signed_listing_lifecycle(content, tags)? {
        SignedListingLifecycle::Active | SignedListingLifecycle::Window => {
            Some(ProductStatus::Published)
        }
        SignedListingLifecycle::Archived => Some(ProductStatus::Archived),
        SignedListingLifecycle::Sold => Some(ProductStatus::Paused),
    }
}

fn signed_listing_has_public_evidence(record: &LocalEventRecord) -> bool {
    if record.status != LocalRecordStatus::Published {
        return false;
    }
    if record.outbox_status == PublishOutboxStatus::Acknowledged {
        return true;
    }
    record
        .relay_delivery_json
        .as_ref()
        .and_then(|delivery| RelayDeliveryEvidence::from_json_value(delivery).ok())
        .is_some_and(|delivery| delivery.state == RelayDeliveryState::Observed)
}

fn signed_event_import_has_public_evidence(
    local_status: &str,
    outbox_status: &str,
    relay_delivery_json: Option<&str>,
) -> bool {
    if local_status != LocalRecordStatus::Published.as_str() {
        return false;
    }
    if outbox_status == PublishOutboxStatus::Acknowledged.as_str() {
        return true;
    }
    relay_delivery_json
        .and_then(|delivery| serde_json::from_str::<Value>(delivery).ok())
        .and_then(|delivery| RelayDeliveryEvidence::from_json_value(&delivery).ok())
        .is_some_and(|delivery| delivery.state == RelayDeliveryState::Observed)
}

fn signed_farm_readiness(content: &Value, tags: Option<&Value>) -> Option<FarmReadiness> {
    string_at(content, &["readiness"])
        .or_else(|| {
            content
                .get("tags")?
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .find_map(readiness_tag_value)
        })
        .or_else(|| {
            tags?.as_array()?.iter().find_map(|tag| {
                let values = tag.as_array()?;
                (values.first()?.as_str()? == "t")
                    .then(|| values.get(1).and_then(Value::as_str))
                    .flatten()
                    .and_then(readiness_tag_value)
            })
        })
        .and_then(|value| match value.as_str() {
            "ready" => Some(FarmReadiness::Ready),
            "incomplete" => Some(FarmReadiness::Incomplete),
            _ => None,
        })
}

fn readiness_tag_value(value: &str) -> Option<String> {
    value
        .strip_prefix("radroots:readiness:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn signed_listing_fulfillment_method(
    content: Option<&Value>,
    tags: Option<&Value>,
) -> Option<FarmOrderMethod> {
    content.and_then(delivery_method_from_content).or_else(|| {
        tag_index_value(tags, "delivery", 1).and_then(|method| farm_order_method(&method))
    })
}

fn delivery_method_from_content(content: &Value) -> Option<FarmOrderMethod> {
    string_at(content, &["delivery_method", "kind"])
        .or_else(|| string_at(content, &["delivery", "method"]))
        .or_else(|| string_at(content, &["delivery_method"]))
        .and_then(|method| farm_order_method(method.as_str()))
}

fn signed_listing_availability_window(
    content: Option<&Value>,
    tags: Option<&Value>,
) -> Option<ListingAvailabilityWindow> {
    let start = content
        .and_then(|content| string_at(content, &["availability", "amount", "start"]))
        .or_else(|| content.and_then(|content| string_at(content, &["availability", "start"])))
        .or_else(|| tag_index_value(tags, "radroots:availability_start", 1))
        .and_then(|value| parse_u64_quantity(value.as_str()));
    let end = content
        .and_then(|content| string_at(content, &["availability", "amount", "end"]))
        .or_else(|| content.and_then(|content| string_at(content, &["availability", "end"])))
        .or_else(|| tag_index_value(tags, "expires_at", 1))
        .and_then(|value| parse_u64_quantity(value.as_str()));

    match (start, end) {
        (Some(start), Some(end)) if end > start => Some(ListingAvailabilityWindow { start, end }),
        _ => None,
    }
}

fn signed_listing_location_primary(
    content: Option<&Value>,
    tags: Option<&Value>,
) -> Option<String> {
    content
        .and_then(|content| string_at(content, &["location", "primary"]))
        .or_else(|| tag_index_value(tags, "location", 1))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ListingAvailabilityWindow {
    start: u64,
    end: u64,
}

fn signed_listing_lifecycle(
    content: Option<&Value>,
    tags: Option<&Value>,
) -> Option<SignedListingLifecycle> {
    content
        .and_then(lifecycle_from_content)
        .or_else(|| lifecycle_from_tags(tags))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignedListingLifecycle {
    Active,
    Window,
    Archived,
    Sold,
}

fn lifecycle_from_content(content: &Value) -> Option<SignedListingLifecycle> {
    string_at(content, &["status"])
        .or_else(|| string_at(content, &["availability", "status"]))
        .or_else(|| string_at(content, &["availability", "amount", "status"]))
        .or_else(|| string_at(content, &["availability", "amount", "kind"]))
        .or_else(|| string_at(content, &["availability", "amount", "value"]))
        .and_then(|status| parse_listing_lifecycle(status.as_str()))
        .or_else(|| {
            matches!(
                string_at(content, &["availability", "kind"]).as_deref(),
                Some("window")
            )
            .then_some(SignedListingLifecycle::Window)
        })
}

fn lifecycle_from_tags(tags: Option<&Value>) -> Option<SignedListingLifecycle> {
    tag_index_value(tags, "status", 1)
        .and_then(|status| parse_listing_lifecycle(status.as_str()))
        .or_else(|| {
            tag_index_value(tags, "radroots:availability_start", 1)
                .or_else(|| tag_index_value(tags, "expires_at", 1))
                .map(|_| SignedListingLifecycle::Window)
        })
}

fn parse_listing_lifecycle(value: &str) -> Option<SignedListingLifecycle> {
    match value.trim().to_ascii_lowercase().as_str() {
        "active" | "available" | "published" => Some(SignedListingLifecycle::Active),
        "window" => Some(SignedListingLifecycle::Window),
        "archived" => Some(SignedListingLifecycle::Archived),
        "sold" => Some(SignedListingLifecycle::Sold),
        _ => None,
    }
}

fn primary_bin(content: &Value) -> Option<&Value> {
    let bins = content.get("bins")?.as_array()?;
    let primary_bin_id = string_at(content, &["primary_bin_id"]);
    primary_bin_id
        .as_deref()
        .and_then(|primary_bin_id| {
            bins.iter()
                .find(|bin| string_at(bin, &["bin_id"]).as_deref() == Some(primary_bin_id))
        })
        .or_else(|| bins.first())
}

fn parse_json_value(raw: &str) -> Result<Value, AppSqliteError> {
    serde_json::from_str(raw).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "shared local signed event content must be json",
    })
}

fn parse_json_value_opt(raw: &str) -> Option<Value> {
    serde_json::from_str(raw).ok()
}

fn tag_index_value(tags: Option<&Value>, tag_name: &str, index: usize) -> Option<String> {
    tags?.as_array()?.iter().find_map(|tag| {
        let values = tag.as_array()?;
        (values.first()?.as_str()? == tag_name)
            .then(|| values.get(index).and_then(Value::as_str))
            .flatten()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn signed_farm_address_matches(tags: Option<&Value>, farm_key: &str, seller_pubkey: &str) -> bool {
    let Some(address) = tag_index_value(tags, "a", 1) else {
        return false;
    };
    address_d_tag(address.as_str()).as_deref() == Some(farm_key)
        && address_pubkey(address.as_str()).as_deref() == Some(seller_pubkey)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListingAddressParts<'a> {
    kind: i64,
    pubkey: &'a str,
    d_tag: &'a str,
}

fn listing_address_parts(address: &str) -> Option<ListingAddressParts<'_>> {
    let mut parts = address.trim().split(':');
    let kind = parts.next()?.parse::<i64>().ok()?;
    let pubkey = parts.next()?.trim();
    let d_tag = parts.next()?.trim();
    if parts.next().is_some() || pubkey.is_empty() || d_tag.is_empty() {
        return None;
    }
    Some(ListingAddressParts {
        kind,
        pubkey,
        d_tag,
    })
}

fn address_d_tag(address: &str) -> Option<String> {
    address
        .rsplit(':')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn address_pubkey(address: &str) -> Option<String> {
    let mut parts = address.split(':');
    let _kind = parts.next()?;
    parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn farm_readiness_storage_key(readiness: FarmReadiness) -> &'static str {
    match readiness {
        FarmReadiness::Incomplete => "incomplete",
        FarmReadiness::Ready => "ready",
    }
}

fn farm_readiness_from_storage_key(readiness: &str) -> Result<FarmReadiness, AppSqliteError> {
    match readiness {
        "incomplete" => Ok(FarmReadiness::Incomplete),
        "ready" => Ok(FarmReadiness::Ready),
        _ => Err(AppSqliteError::InvalidProjection {
            reason: "farm readiness storage key is invalid",
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use radroots_core::{
        RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreUnit,
    };
    use radroots_events::{
        RadrootsNostrEvent, RadrootsNostrEventPtr,
        ids::{
            RadrootsEventId, RadrootsInventoryBinId, RadrootsListingAddress, RadrootsOrderId,
            RadrootsOrderQuoteId, RadrootsOrderRevisionId, RadrootsPublicKey,
        },
        order::{
            RadrootsOrderCancellation, RadrootsOrderDecision, RadrootsOrderDecisionOutcome,
            RadrootsOrderEconomicItem, RadrootsOrderEconomicLine, RadrootsOrderEconomics,
            RadrootsOrderInventoryCommitment, RadrootsOrderItem, RadrootsOrderPricingBasis,
            RadrootsOrderRequest, RadrootsOrderRevisionDecision, RadrootsOrderRevisionOutcome,
            RadrootsOrderRevisionProposal,
        },
    };
    use radroots_events_codec::{
        order::{
            order_cancellation_event_build, order_decision_event_build, order_request_event_build,
            order_revision_decision_event_build, order_revision_proposal_event_build,
        },
        wire::WireEventParts,
    };
    use radroots_local_events::{
        LocalEventRecordInput, LocalEventRecordUpdate, LocalEventsStore, LocalRecordFamily,
        LocalRecordStatus, PublishOutboxStatus, RelayDeliveryEvidence, SourceRuntime,
    };
    use radroots_sql_core::SqliteExecutor;
    use radroots_studio_app_view::{
        BuyerContext, BuyerOrderStatus, FarmId, FarmOrderMethod, OrderId, OrderStatus,
        OrdersFilter, OrdersScreenQueryState, ProductAvailabilityState, ProductId,
        TradeAgreementStatus, TradeInventoryStatus, TradeRevisionStatus,
        TradeValidationReceiptProofSystem, TradeValidationReceiptResult,
        TradeValidationReceiptType, TradeWorkflowSource,
    };
    use radroots_trade::validation_receipt::{
        RadrootsTradeValidationReceipt, RadrootsValidationReceiptProof,
        RadrootsValidationReceiptProofSystem, RadrootsValidationReceiptResult,
        RadrootsValidationReceiptStatement, RadrootsValidationReceiptType,
        VALIDATION_RECEIPT_DOMAIN, VALIDATION_RECEIPT_VERSION, validation_receipt_event_build,
    };
    use rusqlite::params;
    use serde_json::json;
    use uuid::Uuid;

    use super::{
        KIND_FARM, KIND_LISTING, KIND_ORDER_REQUEST, KIND_VALIDATION_RECEIPT,
        deterministic_farm_id, deterministic_product_id, projected_farm_id, projected_order_id,
        projected_product_id,
    };
    use crate::{AppSqliteStore, BuyerRepeatDemandApplyOutcome, DatabaseTarget};

    fn local_events_store() -> LocalEventsStore<SqliteExecutor> {
        let executor = SqliteExecutor::open_memory().expect("open local events memory db");
        let store = LocalEventsStore::new(executor);
        store.migrate_up().expect("migrate local events store");
        store
    }

    fn local_work_record(
        record_id: &str,
        farm_key: &str,
        payload: serde_json::Value,
    ) -> LocalEventRecordInput {
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::Cli,
            created_at_ms: 1000,
            inserted_at_ms: 1001,
            owner_account_id: Some("seller-account".to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some(farm_key.to_owned()),
            listing_addr: None,
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

    fn signed_farm_record(
        record_id: &str,
        event_id: &str,
        source_runtime: SourceRuntime,
        owner_pubkey: &str,
        farm_key: &str,
        readiness: &str,
        display_name: &str,
    ) -> LocalEventRecordInput {
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::SignedEvent,
            status: LocalRecordStatus::Published,
            source_runtime,
            created_at_ms: 1100,
            inserted_at_ms: 1101,
            owner_account_id: Some("seller-account".to_owned()),
            owner_pubkey: Some(owner_pubkey.to_owned()),
            farm_id: Some(farm_key.to_owned()),
            listing_addr: None,
            local_work_json: None,
            event_id: Some(event_id.to_owned()),
            event_kind: Some(KIND_FARM),
            event_pubkey: Some(owner_pubkey.to_owned()),
            event_created_at: Some(1100),
            event_tags_json: Some(json!([
                ["d", farm_key],
                ["t", format!("radroots:readiness:{readiness}")]
            ])),
            event_content: Some(
                json!({
                    "d_tag": farm_key,
                    "name": display_name,
                    "tags": [format!("radroots:readiness:{readiness}")]
                })
                .to_string(),
            ),
            event_sig: Some("signature".to_owned()),
            raw_event_json: Some(json!({
                "id": event_id,
                "kind": KIND_FARM,
                "pubkey": owner_pubkey,
            })),
            outbox_status: PublishOutboxStatus::Acknowledged,
            relay_set_fingerprint: Some("relay-set".to_owned()),
            relay_delivery_json: Some(json!({
                "state": "acknowledged",
                "target_relays": ["ws://127.0.0.1:1234"],
                "connected_relays": ["ws://127.0.0.1:1234"],
                "acknowledged_relays": ["ws://127.0.0.1:1234"]
            })),
        }
    }

    fn signed_listing_record(
        record_id: &str,
        farm_key: &str,
        listing_key: &str,
        status_tag: &str,
    ) -> LocalEventRecordInput {
        signed_listing_record_with_publish_state(
            record_id,
            farm_key,
            listing_key,
            status_tag,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        )
    }

    fn signed_listing_record_with_publish_state(
        record_id: &str,
        farm_key: &str,
        listing_key: &str,
        status_tag: &str,
        record_status: LocalRecordStatus,
        outbox_status: PublishOutboxStatus,
    ) -> LocalEventRecordInput {
        let relay_delivery_json = match outbox_status {
            PublishOutboxStatus::Acknowledged => Some(json!({
                "state": "acknowledged",
                "acknowledged_relays": ["ws://127.0.0.1:1234/"]
            })),
            PublishOutboxStatus::Failed => Some(json!({
                "state": "failed",
                "failed_relays": ["ws://127.0.0.1:1234/"]
            })),
            PublishOutboxStatus::Pending | PublishOutboxStatus::None => None,
        };
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::SignedEvent,
            status: record_status,
            source_runtime: SourceRuntime::Cli,
            created_at_ms: 1100,
            inserted_at_ms: 1101,
            owner_account_id: Some("seller-account".to_owned()),
            owner_pubkey: Some("seller-pubkey".to_owned()),
            farm_id: Some(farm_key.to_owned()),
            listing_addr: Some(format!("30402:seller-pubkey:{listing_key}")),
            local_work_json: None,
            event_id: Some(format!("event-{record_id}")),
            event_kind: Some(KIND_LISTING),
            event_pubkey: Some("seller-pubkey".to_owned()),
            event_created_at: Some(1100),
            event_tags_json: Some(json!([
                ["d", listing_key],
                ["a", format!("30340:seller-pubkey:{farm_key}")],
                ["key", "eggs"],
                ["title", "Relay Eggs"],
                ["summary", "Published eggs"],
                ["radroots:bin", "bin-1", "1", "each"],
                ["radroots:price", "bin-1", "8", "USD", "1", "each"],
                ["inventory", "9"],
                ["status", status_tag]
            ])),
            event_content: Some("# Relay Eggs\n\nPublished eggs".to_owned()),
            event_sig: Some("signature".to_owned()),
            raw_event_json: Some(json!({
                "id": format!("event-{record_id}"),
                "kind": KIND_LISTING,
                "pubkey": "seller-pubkey",
                "content": "# Relay Eggs\n\nPublished eggs"
            })),
            outbox_status,
            relay_set_fingerprint: Some("relay-set".to_owned()),
            relay_delivery_json,
        }
    }

    fn signed_market_listing_record(
        record_id: &str,
        owner_pubkey: &str,
        farm_key: &str,
        listing_key: &str,
        title: &str,
        inventory_available: &str,
        status_tag: &str,
        delivery_method: &str,
        location_primary: &str,
        availability_start: u64,
        availability_end: u64,
        record_status: LocalRecordStatus,
        outbox_status: PublishOutboxStatus,
    ) -> LocalEventRecordInput {
        let relay_delivery_json = match outbox_status {
            PublishOutboxStatus::Acknowledged => Some(json!({
                "state": "acknowledged",
                "acknowledged_relays": ["ws://127.0.0.1:1234/"]
            })),
            PublishOutboxStatus::Failed => Some(json!({
                "state": "failed",
                "failed_relays": ["ws://127.0.0.1:1234/"]
            })),
            PublishOutboxStatus::Pending | PublishOutboxStatus::None => None,
        };
        let content = json!({
            "d_tag": listing_key,
            "status": status_tag,
            "farm": {
                "pubkey": owner_pubkey,
                "d_tag": farm_key,
            },
            "product": {
                "key": listing_key,
                "title": title,
                "summary": "Published local listing",
            },
            "availability": {
                "kind": "window",
                "amount": {
                    "start": availability_start,
                    "end": availability_end,
                },
            },
            "delivery_method": {
                "kind": delivery_method,
            },
            "location": {
                "primary": location_primary,
            },
        });

        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::SignedEvent,
            status: record_status,
            source_runtime: SourceRuntime::Cli,
            created_at_ms: 1100,
            inserted_at_ms: 1101,
            owner_account_id: Some("seller-account".to_owned()),
            owner_pubkey: Some(owner_pubkey.to_owned()),
            farm_id: Some(farm_key.to_owned()),
            listing_addr: Some(format!("30402:{owner_pubkey}:{listing_key}")),
            local_work_json: None,
            event_id: Some(format!("event-{record_id}")),
            event_kind: Some(KIND_LISTING),
            event_pubkey: Some(owner_pubkey.to_owned()),
            event_created_at: Some(1100),
            event_tags_json: Some(json!([
                ["d", listing_key],
                ["a", format!("30340:{owner_pubkey}:{farm_key}")],
                ["key", listing_key],
                ["title", title],
                ["summary", "Published local listing"],
                ["radroots:bin", "bin-1", "1", "each"],
                ["radroots:price", "bin-1", "8", "USD", "1", "each"],
                ["inventory", inventory_available],
                ["status", status_tag],
                [
                    "radroots:availability_start",
                    availability_start.to_string()
                ],
                ["expires_at", availability_end.to_string()],
                ["delivery", delivery_method],
                ["location", location_primary],
            ])),
            event_content: Some(content.to_string()),
            event_sig: Some("signature".to_owned()),
            raw_event_json: Some(json!({
                "id": format!("event-{record_id}"),
                "kind": KIND_LISTING,
                "pubkey": owner_pubkey,
                "content": content.to_string(),
            })),
            outbox_status,
            relay_set_fingerprint: Some("relay-set".to_owned()),
            relay_delivery_json,
        }
    }

    fn set_listing_event_version(
        record: &mut LocalEventRecordInput,
        event_id: &str,
        created_at: i64,
        title: &str,
        inventory_available: &str,
    ) {
        record.event_id = Some(event_id.to_owned());
        record.event_created_at = Some(created_at);
        record.created_at_ms = created_at * 1_000;
        record.inserted_at_ms = created_at * 1_000 + 1;
        if let Some(content) = record.event_content.as_deref() {
            let mut content: serde_json::Value =
                serde_json::from_str(content).expect("listing content should parse");
            content["product"]["title"] = json!(title);
            content["inventory_available"] = json!(inventory_available);
            record.event_content = Some(content.to_string());
        }
        if let Some(serde_json::Value::Array(tags)) = record.event_tags_json.as_mut() {
            for tag in tags {
                let Some(values) = tag.as_array_mut() else {
                    continue;
                };
                match values.first().and_then(serde_json::Value::as_str) {
                    Some("title") => {
                        values[1] = json!(title);
                    }
                    Some("inventory") => {
                        values[1] = json!(inventory_available);
                    }
                    _ => {}
                }
            }
        }
        record.raw_event_json = Some(json!({
            "id": event_id,
            "kind": record.event_kind,
            "pubkey": record.event_pubkey,
            "content": record.event_content,
        }));
    }

    fn buyer_listing_titles(app_store: &AppSqliteStore) -> Vec<String> {
        app_store
            .load_buyer_listings("", &BTreeSet::new())
            .expect("buyer listings should load")
            .rows
            .into_iter()
            .map(|row| row.title)
            .collect()
    }

    fn app_d_tag_from_uuid(uuid: Uuid) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let bytes = uuid.as_bytes();
        let mut output = String::with_capacity((bytes.len() * 4).div_ceil(3));
        let mut chunks = bytes.chunks_exact(3);
        for chunk in &mut chunks {
            output.push(ALPHABET[(chunk[0] >> 2) as usize] as char);
            output.push(
                ALPHABET[(((chunk[0] & 0b0000_0011) << 4) | (chunk[1] >> 4)) as usize] as char,
            );
            output.push(
                ALPHABET[(((chunk[1] & 0b0000_1111) << 2) | (chunk[2] >> 6)) as usize] as char,
            );
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
            _ => unreachable!(),
        }
        output
    }

    #[test]
    fn app_shaped_keys_use_uuid_projection_only_for_app_runtime() {
        let owner_pubkey = "projection-owner-pubkey";
        let farm_uuid = Uuid::from_u128(0x11111111111141118111111111111111);
        let product_uuid = Uuid::from_u128(0x22222222222242228222222222222222);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);

        assert_eq!(
            projected_farm_id(SourceRuntime::App, Some(owner_pubkey), farm_key.as_str()),
            Some(FarmId::from(farm_uuid))
        );
        assert_eq!(
            projected_product_id(SourceRuntime::App, Some(owner_pubkey), listing_key.as_str()),
            Some(ProductId::from(product_uuid))
        );
        assert_eq!(
            projected_farm_id(
                SourceRuntime::Network,
                Some(owner_pubkey),
                farm_key.as_str()
            ),
            Some(deterministic_farm_id(Some(owner_pubkey), farm_key.as_str()))
        );
        assert_eq!(
            projected_product_id(
                SourceRuntime::Network,
                Some(owner_pubkey),
                listing_key.as_str()
            ),
            Some(deterministic_product_id(
                Some(owner_pubkey),
                listing_key.as_str()
            ))
        );
    }

    fn app_local_work_record(
        record_id: &str,
        farm_key: &str,
        payload: serde_json::Value,
    ) -> LocalEventRecordInput {
        let mut record = local_work_record(record_id, farm_key, payload);
        record.source_runtime = SourceRuntime::App;
        record.owner_pubkey = Some("app-seller-pubkey".to_owned());
        record
    }

    fn seed_app_projection(app_store: &AppSqliteStore, farm_id: Uuid, product_id: Uuid) {
        app_store
            .connection()
            .execute(
                "INSERT INTO farms (id, display_name, readiness, created_at, updated_at)
                 VALUES (?1, 'Origin Farm', 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                params![farm_id.to_string()],
            )
            .expect("seed origin farm");
        app_store
            .connection()
            .execute(
                "INSERT INTO products (
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
                 ) VALUES (
                    ?1,
                    ?2,
                    'Origin Eggs',
                    'Seeded product',
                    'draft',
                    'each',
                    400,
                    'USD',
                    3,
                    NULL,
                    '2026-01-01T00:00:00Z'
                 )",
                params![product_id.to_string(), farm_id.to_string()],
            )
            .expect("seed origin product");
    }

    fn decimal(raw: &str) -> RadrootsCoreDecimal {
        raw.parse().expect("valid decimal")
    }

    fn usd(raw: &str) -> RadrootsCoreMoney {
        RadrootsCoreMoney::new(decimal(raw), RadrootsCoreCurrency::USD)
    }

    fn test_pubkey(seed: &str) -> String {
        let left = Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes());
        let right_seed = format!("{seed}:right");
        let right = Uuid::new_v5(&Uuid::NAMESPACE_URL, right_seed.as_bytes());
        format!("{}{}", left.simple(), right.simple())
    }

    fn test_event_id_seed(seed: &str) -> String {
        test_pubkey(seed)
    }

    fn typed_order_id(raw: &str) -> RadrootsOrderId {
        raw.parse().expect("valid order id")
    }

    fn typed_revision_id(raw: &str) -> RadrootsOrderRevisionId {
        raw.parse().expect("valid revision id")
    }

    fn typed_quote_id(raw: &str) -> RadrootsOrderQuoteId {
        raw.parse().expect("valid quote id")
    }

    fn typed_bin_id(raw: &str) -> RadrootsInventoryBinId {
        raw.parse().expect("valid bin id")
    }

    fn typed_event_id(raw: &str) -> RadrootsEventId {
        raw.parse().expect("valid event id")
    }

    fn typed_pubkey(raw: &str) -> RadrootsPublicKey {
        raw.parse().expect("valid pubkey")
    }

    fn typed_listing_addr(raw: &str) -> RadrootsListingAddress {
        raw.parse().expect("valid listing address")
    }

    fn listing_event_ptr(event_id: &str) -> RadrootsNostrEventPtr {
        RadrootsNostrEventPtr {
            id: test_event_id_seed(event_id),
            relays: Some("ws://127.0.0.1:1234/".to_owned()),
        }
    }

    fn order_request_payload(
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> RadrootsOrderRequest {
        RadrootsOrderRequest {
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            items: vec![RadrootsOrderItem {
                bin_id: typed_bin_id("bin-1"),
                bin_count: 2,
            }],
            economics: RadrootsOrderEconomics {
                quote_id: typed_quote_id(format!("quote-{order_id}").as_str()),
                quote_version: 1,
                pricing_basis: RadrootsOrderPricingBasis::ListingEvent,
                currency: RadrootsCoreCurrency::USD,
                items: vec![RadrootsOrderEconomicItem {
                    bin_id: typed_bin_id("bin-1"),
                    bin_count: 2,
                    quantity_amount: decimal("1"),
                    quantity_unit: RadrootsCoreUnit::Each,
                    unit_price_amount: decimal("8"),
                    unit_price_currency: RadrootsCoreCurrency::USD,
                    line_subtotal: usd("16"),
                }],
                discounts: Vec::<RadrootsOrderEconomicLine>::new(),
                adjustments: Vec::<RadrootsOrderEconomicLine>::new(),
                subtotal: usd("16"),
                discount_total: usd("0"),
                adjustment_total: usd("0"),
                total: usd("16"),
            },
        }
    }

    fn accepted_order_decision_payload(
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> RadrootsOrderDecision {
        RadrootsOrderDecision {
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            decision: RadrootsOrderDecisionOutcome::Accepted {
                inventory_commitments: vec![RadrootsOrderInventoryCommitment {
                    bin_id: typed_bin_id("bin-1"),
                    bin_count: 2,
                }],
            },
        }
    }

    fn declined_order_decision_payload(
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> RadrootsOrderDecision {
        RadrootsOrderDecision {
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            decision: RadrootsOrderDecisionOutcome::Declined {
                reason: "not available for this pickup".to_owned(),
            },
        }
    }

    fn revision_proposal_payload(
        revision_id: &str,
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        root_event_id: &str,
        prev_event_id: &str,
    ) -> RadrootsOrderRevisionProposal {
        let mut request =
            order_request_payload(order_id, listing_addr, buyer_pubkey, seller_pubkey);
        request.items[0].bin_count = 3;
        request.economics.quote_id =
            typed_quote_id(format!("quote-{order_id}-{revision_id}").as_str());
        request.economics.quote_version = 2;
        request.economics.items[0].bin_count = 3;
        request.economics.items[0].line_subtotal = usd("24");
        request.economics.subtotal = usd("24");
        request.economics.total = usd("24");
        RadrootsOrderRevisionProposal {
            revision_id: typed_revision_id(revision_id),
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            root_event_id: typed_event_id(root_event_id),
            prev_event_id: typed_event_id(prev_event_id),
            items: request.items,
            economics: request.economics,
            reason: "seller confirmed updated pickup details".to_owned(),
        }
    }

    fn revision_decision_payload(
        revision_id: &str,
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
        root_event_id: &str,
        prev_event_id: &str,
        decision: RadrootsOrderRevisionOutcome,
    ) -> RadrootsOrderRevisionDecision {
        RadrootsOrderRevisionDecision {
            revision_id: typed_revision_id(revision_id),
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            root_event_id: typed_event_id(root_event_id),
            prev_event_id: typed_event_id(prev_event_id),
            decision,
        }
    }

    fn order_cancel_payload(
        order_id: &str,
        listing_addr: &str,
        buyer_pubkey: &str,
        seller_pubkey: &str,
    ) -> RadrootsOrderCancellation {
        RadrootsOrderCancellation {
            order_id: typed_order_id(order_id),
            listing_addr: typed_listing_addr(listing_addr),
            buyer_pubkey: typed_pubkey(buyer_pubkey),
            seller_pubkey: typed_pubkey(seller_pubkey),
            reason: "buyer changed pickup plan".to_owned(),
        }
    }

    struct ValidationReceiptOrderFixture {
        app_store: AppSqliteStore,
        events: LocalEventsStore<SqliteExecutor>,
        buyer_context: BuyerContext,
        seller_farm_id: FarmId,
        order_id: OrderId,
        order_id_raw: String,
        listing_addr: String,
        buyer_pubkey: String,
        seller_pubkey: String,
        listing_event_id: String,
        request_event_id: String,
        decision_event_id: String,
    }

    fn validation_receipt_order_fixture(label: &str) -> ValidationReceiptOrderFixture {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "DDDDDDDDDDDDDDDDDDDDDD";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAw";
        let seller_pubkey = test_pubkey(format!("{label}-seller").as_str());
        let buyer_pubkey = test_pubkey(format!("{label}-buyer").as_str());
        let order_id_raw = format!("{label}-order");
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let listing_event_id = hex_event_id(21);
        let request_event_id = hex_event_id(22);
        let decision_event_id = hex_event_id(23);
        events
            .append_record(&signed_market_listing_record(
                format!("{label}-listing-record").as_str(),
                seller_pubkey.as_str(),
                farm_key,
                listing_key,
                "Validation Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");

        let request_payload = order_request_payload(
            order_id_raw.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
        );
        let request_parts =
            order_request_event_build(&listing_event_ptr(&listing_event_id), &request_payload)
                .expect("build validation order request");
        let request_event = event_from_parts_at(
            request_event_id.as_str(),
            buyer_pubkey.as_str(),
            request_parts,
            1_777_665_601,
        );
        events
            .append_record(&signed_order_event_record(
                format!("app:signed_event:{label}:request").as_str(),
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_validation"),
            ))
            .expect("append validation order request");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import validation order request");

        let decision_payload = accepted_order_decision_payload(
            order_id_raw.as_str(),
            listing_addr.as_str(),
            buyer_pubkey.as_str(),
            seller_pubkey.as_str(),
        );
        let decision_parts = order_decision_event_build(
            &typed_event_id(request_event_id.as_str()),
            &typed_event_id(request_event_id.as_str()),
            &decision_payload,
        )
        .expect("build validation order decision");
        let decision_event = event_from_parts_at(
            decision_event_id.as_str(),
            seller_pubkey.as_str(),
            decision_parts,
            1_777_665_602,
        );
        events
            .append_record(&signed_order_event_record(
                format!("cli:signed_event:{label}:decision").as_str(),
                &decision_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append validation order decision");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import validation order decision");

        ValidationReceiptOrderFixture {
            app_store,
            events,
            buyer_context: BuyerContext::account("acct_validation"),
            seller_farm_id: deterministic_farm_id(Some(seller_pubkey.as_str()), farm_key),
            order_id: projected_order_id(order_id_raw.as_str(), buyer_pubkey.as_str()),
            order_id_raw,
            listing_addr,
            buyer_pubkey,
            seller_pubkey,
            listing_event_id,
            request_event_id,
            decision_event_id,
        }
    }

    fn event_from_parts(event_id: &str, author: &str, parts: WireEventParts) -> RadrootsNostrEvent {
        let event_id = event_id
            .parse::<RadrootsEventId>()
            .map(|event_id| event_id.to_string())
            .unwrap_or_else(|_| test_event_id_seed(event_id));
        RadrootsNostrEvent {
            sig: format!("sig-{event_id}"),
            id: event_id,
            author: author.to_owned(),
            created_at: 1_777_665_600,
            kind: parts.kind,
            tags: parts.tags,
            content: parts.content,
        }
    }

    fn event_from_parts_at(
        event_id: &str,
        author: &str,
        parts: WireEventParts,
        created_at: u32,
    ) -> RadrootsNostrEvent {
        let mut event = event_from_parts(event_id, author, parts);
        event.created_at = created_at;
        event
    }

    fn hex_event_id(seed: u8) -> String {
        format!("{seed:064x}")
    }

    fn hash32(seed: u8) -> String {
        format!("0x{seed:064x}")
    }

    fn validation_error_bitmap(result: RadrootsValidationReceiptResult) -> String {
        match result {
            RadrootsValidationReceiptResult::Valid => format!("0x{:032x}", 0),
            RadrootsValidationReceiptResult::Invalid => format!("0x{:032x}", 1),
        }
    }

    fn validation_receipt_payload(
        listing_event_id: &str,
        root_event_id: &str,
        target_event_id: &str,
        result: RadrootsValidationReceiptResult,
    ) -> RadrootsTradeValidationReceipt {
        RadrootsTradeValidationReceipt {
            changed_records_root: hash32(41),
            domain: VALIDATION_RECEIPT_DOMAIN.to_owned(),
            error_bitmap: validation_error_bitmap(result),
            event_set_root: hash32(42),
            new_state_root: hash32(43),
            previous_state_root: hash32(44),
            proof: RadrootsValidationReceiptProof {
                inline_proof_base64: None,
                mode: None,
                program_hash: None,
                proof_reference: None,
                system: RadrootsValidationReceiptProofSystem::None,
                verifying_key_hash: None,
            },
            public_values_hash: hash32(45),
            receipt_type: RadrootsValidationReceiptType::TradeTransition,
            result,
            statement: RadrootsValidationReceiptStatement {
                listing_event_id: listing_event_id.to_owned(),
                root_event_id: root_event_id.to_owned(),
                target_event_id: target_event_id.to_owned(),
                statement_type: RadrootsValidationReceiptType::TradeTransition,
            },
            version: VALIDATION_RECEIPT_VERSION,
        }
    }

    fn validation_receipt_event(
        event_id: &str,
        author: &str,
        order_id: &str,
        listing_event_id: &str,
        root_event_id: &str,
        target_event_id: &str,
        result: RadrootsValidationReceiptResult,
        created_at: u32,
    ) -> RadrootsNostrEvent {
        let receipt =
            validation_receipt_payload(listing_event_id, root_event_id, target_event_id, result);
        let parts =
            validation_receipt_event_build(order_id, &receipt).expect("validation receipt parts");
        event_from_parts_at(event_id, author, parts, created_at)
    }

    fn signed_order_event_record(
        record_id: &str,
        event: &RadrootsNostrEvent,
        listing_addr: &str,
        source_runtime: SourceRuntime,
        owner_account_id: Option<&str>,
    ) -> LocalEventRecordInput {
        let relay_delivery_json = RelayDeliveryEvidence::acknowledged(
            ["ws://127.0.0.1:1234"],
            ["ws://127.0.0.1:1234"],
            ["ws://127.0.0.1:1234"],
            Vec::new(),
        )
        .expect("acknowledged relay evidence")
        .to_json_value()
        .expect("acknowledged relay evidence json");
        LocalEventRecordInput {
            record_id: record_id.to_owned(),
            family: LocalRecordFamily::SignedEvent,
            status: LocalRecordStatus::Published,
            source_runtime,
            created_at_ms: i64::from(event.created_at) * 1_000,
            inserted_at_ms: i64::from(event.created_at) * 1_000 + 1,
            owner_account_id: owner_account_id.map(str::to_owned),
            owner_pubkey: Some(event.author.clone()),
            farm_id: None,
            listing_addr: Some(listing_addr.to_owned()),
            local_work_json: None,
            event_id: Some(event.id.clone()),
            event_kind: Some(i64::from(event.kind)),
            event_pubkey: Some(event.author.clone()),
            event_created_at: Some(i64::from(event.created_at)),
            event_tags_json: Some(json!(event.tags)),
            event_content: Some(event.content.clone()),
            event_sig: Some(event.sig.clone()),
            raw_event_json: Some(json!({
                "id": event.id,
                "kind": event.kind,
                "pubkey": event.author,
                "created_at": event.created_at,
                "tags": event.tags,
                "content": event.content,
                "sig": event.sig,
            })),
            outbox_status: PublishOutboxStatus::Acknowledged,
            relay_set_fingerprint: Some("relay-set".to_owned()),
            relay_delivery_json: Some(relay_delivery_json),
        }
    }

    #[test]
    fn imports_signed_order_request_into_seller_order_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAg";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "relay-order-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "order-visible-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Order Visible Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let parts = order_request_event_build(&listing_event_ptr("listing-event-1"), &payload)
            .expect("build order request event");
        let event = event_from_parts("order-request-event-1", buyer_pubkey, parts);
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:order-request:1",
                &event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append order request");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed order request");
        let farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key);
        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let orders = app_store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load seller orders");
        let detail = app_store
            .load_order_detail(farm_id, order_id)
            .expect("load order detail")
            .expect("order detail");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let signed_evidence = app_store
            .load_local_interop_signed_events_by_kind(KIND_ORDER_REQUEST)
            .expect("load signed event evidence");
        let buyer_context_key: String = app_store
            .connection()
            .query_row(
                "SELECT buyer_context_key FROM orders WHERE id = ?1",
                [order_id.to_string()],
                |row| row.get(0),
            )
            .expect("load buyer context key");

        assert_eq!(report.imported_records, 1);
        assert!(
            imported
                .iter()
                .any(|record| record.projected_kind == "signed_event"
                    && record.event_kind == Some(KIND_ORDER_REQUEST)
                    && record.event_id.as_deref() == Some(event.id.as_str()))
        );
        assert_eq!(signed_evidence, vec![event.clone()]);
        assert_eq!(orders.rows.len(), 1);
        assert_eq!(orders.rows[0].order_id, order_id);
        assert_eq!(orders.rows[0].status, OrderStatus::NeedsAction);
        assert_eq!(
            orders.rows[0].customer_display_name,
            format!("Relay buyer {}", &buyer_pubkey[..12])
        );
        assert_eq!(detail.items.len(), 1);
        assert_eq!(detail.items[0].title, "Order Visible Eggs");
        assert_eq!(detail.items[0].quantity_display, "2 each");
        assert_eq!(buyer_context_key, format!("nostr:{buyer_pubkey}"));
    }

    #[test]
    fn local_interop_order_request_evidence_requires_usable_delivery_state() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let buyer_pubkey = test_pubkey("buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let listing_addr = format!("30402:{seller_pubkey}:AAAAAAAAAAAAAAAAAAAAAg");
        let relay_url = "ws://127.0.0.1:1234";
        let build_event = |event_id: &str, order_id_raw: &str| {
            let payload = order_request_payload(
                order_id_raw,
                listing_addr.as_str(),
                buyer_pubkey,
                seller_pubkey,
            );
            let parts = order_request_event_build(&listing_event_ptr("listing-event-1"), &payload)
                .expect("build order request event");
            event_from_parts(event_id, buyer_pubkey, parts)
        };
        let acknowledged_event = build_event("order-request-evidence-ack", "usable-ack");
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:order-request:evidence-ack",
                &acknowledged_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append acknowledged order request evidence");

        let observed_event = build_event("order-request-evidence-observed", "usable-observed");
        let mut observed_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-observed",
            &observed_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        observed_record.outbox_status = PublishOutboxStatus::None;
        observed_record.relay_delivery_json = Some(
            RelayDeliveryEvidence::observed([relay_url], [relay_url], [relay_url], Vec::new())
                .expect("observed relay evidence")
                .to_json_value()
                .expect("observed relay evidence json"),
        );
        events
            .append_record(&observed_record)
            .expect("append observed order request evidence");

        let pending_event = build_event("order-request-evidence-pending", "pending");
        let mut pending_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-pending",
            &pending_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        pending_record.status = LocalRecordStatus::PendingPublish;
        pending_record.outbox_status = PublishOutboxStatus::Pending;
        pending_record.relay_delivery_json = Some(
            RelayDeliveryEvidence::pending([relay_url])
                .expect("pending relay evidence")
                .to_json_value()
                .expect("pending relay evidence json"),
        );
        events
            .append_record(&pending_record)
            .expect("append pending order request evidence");

        let failed_event = build_event("order-request-evidence-failed", "failed");
        let mut failed_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-failed",
            &failed_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        failed_record.outbox_status = PublishOutboxStatus::Failed;
        failed_record.relay_delivery_json = Some(json!({
            "state": "failed",
            "target_relays": [relay_url],
            "connected_relays": [relay_url],
            "acknowledged_relays": [],
            "failed_relays": [{"relay_url": relay_url, "error": "relay rejected event"}]
        }));
        events
            .append_record(&failed_record)
            .expect("append failed order request evidence");

        let local_only_event = build_event("order-request-evidence-local-only", "local-only");
        let mut local_only_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-local-only",
            &local_only_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        local_only_record.outbox_status = PublishOutboxStatus::None;
        local_only_record.relay_set_fingerprint = None;
        local_only_record.relay_delivery_json = None;
        events
            .append_record(&local_only_record)
            .expect("append local-only order request evidence");

        let malformed_delivery_event = build_event(
            "order-request-evidence-malformed-delivery",
            "malformed-delivery",
        );
        let mut malformed_delivery_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-malformed-delivery",
            &malformed_delivery_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        malformed_delivery_record.relay_delivery_json = Some(json!({
            "state": "acknowledged"
        }));
        events
            .append_record(&malformed_delivery_record)
            .expect("append malformed delivery order request evidence");

        let malformed_event =
            build_event("order-request-evidence-malformed-event", "malformed-event");
        let mut malformed_record = signed_order_event_record(
            "cli:signed_event:order-request:evidence-malformed-event",
            &malformed_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        malformed_record.event_tags_json = Some(json!({"invalid": "tags"}));
        events
            .append_record(&malformed_record)
            .expect("append malformed order request evidence");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed evidence records");
        let signed_evidence = app_store
            .load_local_interop_signed_events_by_kind(KIND_ORDER_REQUEST)
            .expect("load filtered signed event evidence");

        assert_eq!(signed_evidence, vec![acknowledged_event, observed_event]);
    }

    #[test]
    fn app_origin_signed_order_request_and_decision_project_to_buyer_orders() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "CCCCCCCCCCCCCCCCCCCCCC";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAg";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("app-buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "app-relay-order-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "buyer-order-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Buyer Order Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr("buyer-order-listing-event"),
            &request_payload,
        )
        .expect("build order request event");
        let request_event =
            event_from_parts("buyer-order-request-event", buyer_pubkey, request_parts);
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:order-request:buyer",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_buyer"),
            ))
            .expect("append app order request");

        let request_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app order request");
        let buyer_context = BuyerContext::account("acct_buyer");
        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key);
        let buyer_orders = app_store
            .load_buyer_orders(&buyer_context)
            .expect("load buyer orders after request");

        assert_eq!(request_report.imported_records, 1);
        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(buyer_orders.rows[0].order_id, order_id);
        assert_eq!(buyer_orders.rows[0].status, BuyerOrderStatus::Placed);

        let decision_payload = accepted_order_decision_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let decision_parts = order_decision_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &decision_payload,
        )
        .expect("build order decision event");
        let decision_event =
            event_from_parts("buyer-order-decision-event", seller_pubkey, decision_parts);
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:order-decision:buyer",
                &decision_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append order decision");

        let decision_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import order decision");
        let buyer_orders = app_store
            .load_buyer_orders(&buyer_context)
            .expect("load buyer orders after decision");
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load buyer order detail")
            .expect("buyer order detail");
        let seller_orders = app_store
            .load_orders_list(
                farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load seller orders after decision");

        assert_eq!(decision_report.imported_records, 1);
        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(buyer_orders.rows[0].status, BuyerOrderStatus::Placed);
        assert_eq!(buyer_detail.status, BuyerOrderStatus::Placed);
        assert_eq!(seller_orders.rows[0].status, OrderStatus::NeedsAction);
        assert_eq!(buyer_detail.workflow, buyer_orders.rows[0].workflow);
        assert_eq!(
            seller_orders.rows[0].workflow,
            buyer_orders.rows[0].workflow
        );
        assert_eq!(
            buyer_orders.rows[0].workflow.agreement,
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(
            buyer_orders.rows[0].workflow.inventory,
            TradeInventoryStatus::Reserved
        );
        assert_eq!(
            buyer_orders.rows[0].workflow.provenance.primary_source,
            TradeWorkflowSource::LocalEvents
        );
        assert_eq!(
            buyer_orders.rows[0]
                .workflow
                .provenance
                .last_event_id
                .as_deref(),
            Some(decision_event.id.as_str())
        );
        assert_eq!(
            buyer_orders.rows[0].workflow.economics.total_minor_units,
            Some(1600)
        );
    }

    #[test]
    fn app_origin_signed_order_request_and_decline_project_to_buyer_orders() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "CCCCCCCCCCCCCCCCCCCCCC";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAg";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("app-buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "app-relay-order-declined-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "buyer-order-decline-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Buyer Order Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr("buyer-order-decline-listing-event"),
            &request_payload,
        )
        .expect("build order request event");
        let request_event = event_from_parts(
            "buyer-order-decline-request-event",
            buyer_pubkey,
            request_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:order-request:buyer-declined",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_buyer"),
            ))
            .expect("append app order request");

        let request_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app order request");
        let buyer_context = BuyerContext::account("acct_buyer");
        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let buyer_orders = app_store
            .load_buyer_orders(&buyer_context)
            .expect("load buyer orders after request");

        assert_eq!(request_report.imported_records, 1);
        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(buyer_orders.rows[0].order_id, order_id);
        assert_eq!(buyer_orders.rows[0].status, BuyerOrderStatus::Placed);

        let decision_payload = declined_order_decision_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let decision_parts = order_decision_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &decision_payload,
        )
        .expect("build declined order decision event");
        let decision_event = event_from_parts(
            "buyer-order-decline-decision-event",
            seller_pubkey,
            decision_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:order-decision:buyer-declined",
                &decision_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append declined order decision");

        let decision_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import declined order decision");
        let buyer_orders = app_store
            .load_buyer_orders(&buyer_context)
            .expect("load buyer orders after declined decision");
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load buyer order detail")
            .expect("buyer order detail");
        let seller_orders = app_store
            .load_orders_list(
                deterministic_farm_id(Some(seller_pubkey), farm_key),
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load seller orders after declined decision");

        assert_eq!(decision_report.imported_records, 1);
        assert_eq!(buyer_orders.rows.len(), 1);
        assert_eq!(buyer_orders.rows[0].status, BuyerOrderStatus::Declined);
        assert_eq!(buyer_detail.status, BuyerOrderStatus::Declined);
        assert_eq!(seller_orders.rows[0].status, OrderStatus::Declined);
        assert_eq!(seller_orders.summary.needs_action_orders, 0);
        assert_eq!(seller_orders.summary.scheduled_orders, 0);
        assert_eq!(seller_orders.summary.packed_orders, 0);
        assert!(seller_orders.rows[0].primary_action.is_none());
    }

    #[test]
    fn active_order_decision_projects_agreement_state_through_cli_reducer() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "DDDDDDDDDDDDDDDDDDDDDD";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAw";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("app-buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "active-lifecycle-order-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "active-lifecycle-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Lifecycle Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");

        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr("active-lifecycle-listing-event"),
            &request_payload,
        )
        .expect("build lifecycle order request");
        let request_event = event_from_parts(
            "active-lifecycle-request-event",
            buyer_pubkey,
            request_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:active-lifecycle:request",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_lifecycle"),
            ))
            .expect("append lifecycle order request");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import lifecycle order request");

        let seller_farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key);
        let decision_payload = accepted_order_decision_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let decision_parts = order_decision_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &decision_payload,
        )
        .expect("build lifecycle order decision");
        let decision_event = event_from_parts(
            "active-lifecycle-decision-event",
            seller_pubkey,
            decision_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:active-lifecycle:decision",
                &decision_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append lifecycle order decision");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import lifecycle order decision");
        let seller_orders = app_store
            .load_orders_list(
                seller_farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load lifecycle seller orders after decision");
        assert_eq!(seller_orders.rows[0].status, OrderStatus::NeedsAction);
        assert_eq!(
            seller_orders.rows[0].workflow.agreement,
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(
            seller_orders.rows[0].workflow.inventory,
            TradeInventoryStatus::Reserved
        );
        assert_eq!(seller_orders.rows[0].primary_action, None);
    }

    #[test]
    fn valid_validation_receipt_commits_buyer_and_seller_order_details() {
        let fixture = validation_receipt_order_fixture("validation-receipt-valid-commit");
        let valid_event = validation_receipt_event(
            hex_event_id(29).as_str(),
            fixture.seller_pubkey.as_str(),
            fixture.order_id_raw.as_str(),
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_603,
        );
        fixture
            .events
            .append_record(&signed_order_event_record(
                "cli:signed_event:validation-receipt:valid-commit",
                &valid_event,
                fixture.listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append valid validation receipt");
        fixture
            .app_store
            .import_shared_local_events_from_store(&fixture.events)
            .expect("import valid validation receipt");

        let buyer_detail = fixture
            .app_store
            .load_buyer_order_detail(&fixture.buyer_context, fixture.order_id)
            .expect("load buyer committed validation receipt detail")
            .expect("buyer committed validation receipt detail");
        let seller_detail = fixture
            .app_store
            .load_order_detail(fixture.seller_farm_id, fixture.order_id)
            .expect("load seller committed validation receipt detail")
            .expect("seller committed validation receipt detail");

        assert_eq!(buyer_detail.status, BuyerOrderStatus::Scheduled);
        assert_eq!(seller_detail.status, OrderStatus::Scheduled);
        assert_eq!(
            buyer_detail.workflow.agreement,
            TradeAgreementStatus::Committed
        );
        assert_eq!(
            seller_detail.workflow.agreement,
            TradeAgreementStatus::Committed
        );
        assert_eq!(
            buyer_detail.workflow.inventory,
            TradeInventoryStatus::Reserved
        );
        assert_eq!(
            buyer_detail.workflow.provenance.last_event_id.as_deref(),
            Some(fixture.decision_event_id.as_str())
        );
        assert_eq!(buyer_detail.validation_receipts.len(), 1);
        assert_eq!(
            buyer_detail.validation_receipts[0].result,
            TradeValidationReceiptResult::Valid
        );
    }

    #[test]
    fn validation_receipts_drive_buyer_and_seller_order_details() {
        let fixture = validation_receipt_order_fixture("validation-receipt-driven");
        let valid_event = validation_receipt_event(
            hex_event_id(31).as_str(),
            fixture.seller_pubkey.as_str(),
            fixture.order_id_raw.as_str(),
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_603,
        );
        let invalid_event = validation_receipt_event(
            hex_event_id(32).as_str(),
            fixture.seller_pubkey.as_str(),
            fixture.order_id_raw.as_str(),
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Invalid,
            1_777_665_604,
        );
        let duplicate_valid_event = valid_event.clone();
        fixture
            .events
            .append_record(&signed_order_event_record(
                "cli:signed_event:validation-receipt:valid",
                &valid_event,
                fixture.listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append valid validation receipt");
        fixture
            .events
            .append_record(&signed_order_event_record(
                "cli:signed_event:validation-receipt:invalid",
                &invalid_event,
                fixture.listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append invalid validation receipt");
        fixture
            .events
            .append_record(&signed_order_event_record(
                "cli:signed_event:validation-receipt:valid-duplicate",
                &duplicate_valid_event,
                fixture.listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append duplicate validation receipt");
        fixture
            .app_store
            .import_shared_local_events_from_store(&fixture.events)
            .expect("import validation receipts");

        let buyer_detail = fixture
            .app_store
            .load_buyer_order_detail(&fixture.buyer_context, fixture.order_id)
            .expect("load buyer validation receipt detail")
            .expect("buyer validation receipt detail");
        let seller_detail = fixture
            .app_store
            .load_order_detail(fixture.seller_farm_id, fixture.order_id)
            .expect("load seller validation receipt detail")
            .expect("seller validation receipt detail");
        let imports = fixture
            .app_store
            .load_local_interop_records()
            .expect("load validation receipt imports");

        assert_eq!(buyer_detail.status, BuyerOrderStatus::NeedsReview);
        assert_eq!(seller_detail.status, OrderStatus::NeedsReview);
        assert_eq!(
            buyer_detail.workflow.agreement,
            TradeAgreementStatus::Invalid
        );
        assert_eq!(
            seller_detail.workflow.agreement,
            TradeAgreementStatus::Invalid
        );
        assert_eq!(
            buyer_detail.workflow.inventory,
            TradeInventoryStatus::NeedsReview
        );
        assert_eq!(
            buyer_detail.workflow.provenance.last_event_id.as_deref(),
            Some(fixture.decision_event_id.as_str())
        );
        assert_eq!(seller_detail.primary_action, None);
        assert_eq!(buyer_detail.validation_receipts.len(), 2);
        assert_eq!(
            buyer_detail.validation_receipts,
            seller_detail.validation_receipts
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].event_id,
            invalid_event.id
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].result,
            TradeValidationReceiptResult::NeedsReview
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].receipt_type,
            TradeValidationReceiptType::TradeTransition
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].proof_system,
            TradeValidationReceiptProofSystem::None
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].target_event_id,
            fixture.decision_event_id
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].event_set_root,
            hash32(42)
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].reducer_output_root,
            hash32(43)
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].public_values_hash,
            hash32(45)
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].recorded_at,
            1_777_665_604
        );
        assert_eq!(buyer_detail.validation_receipts[1].event_id, valid_event.id);
        assert_eq!(
            buyer_detail.validation_receipts[1].result,
            TradeValidationReceiptResult::Valid
        );
        assert!(
            imports
                .iter()
                .any(|record| record.projected_kind == "validation_receipt"
                    && record.event_kind == Some(KIND_VALIDATION_RECEIPT)
                    && record.event_id.as_deref() == Some(invalid_event.id.as_str()))
        );
    }

    #[test]
    fn validation_receipt_import_before_order_request_attaches_when_request_arrives() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "DDDDDDDDDDDDDDDDDDDDDD";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAw";
        let seller_pubkey = test_pubkey("validation-out-of-order-seller");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("validation-out-of-order-buyer");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "validation-out-of-order";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        let listing_event_id = hex_event_id(51);
        let request_event_id = hex_event_id(52);
        let target_event_id = hex_event_id(53);
        events
            .append_record(&signed_market_listing_record(
                "validation-out-of-order-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Validation Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append out-of-order listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import out-of-order listing");

        let receipt_event = validation_receipt_event(
            hex_event_id(54).as_str(),
            seller_pubkey,
            order_id_raw,
            listing_event_id.as_str(),
            request_event_id.as_str(),
            target_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_603,
        );
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:validation-receipt:before-request",
                &receipt_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append receipt before request");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import receipt before request");

        let pending_count: i64 = app_store
            .connection()
            .query_row(
                "SELECT count(*) FROM order_validation_receipts WHERE order_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("count pending validation receipts");
        assert_eq!(pending_count, 1);

        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr(listing_event_id.as_str()),
            &request_payload,
        )
        .expect("build request after validation receipt");
        let request_event = event_from_parts_at(
            request_event_id.as_str(),
            buyer_pubkey,
            request_parts,
            1_777_665_604,
        );
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:validation-receipt:request-after",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_validation_out_of_order"),
            ))
            .expect("append request after receipt");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import request after receipt");

        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let buyer_context = BuyerContext::account("acct_validation_out_of_order");
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load attached buyer validation receipt detail")
            .expect("attached buyer validation receipt detail");
        let seller_detail = app_store
            .load_order_detail(
                deterministic_farm_id(Some(seller_pubkey), farm_key),
                order_id,
            )
            .expect("load attached seller validation receipt detail")
            .expect("attached seller validation receipt detail");

        assert_eq!(buyer_detail.validation_receipts.len(), 1);
        assert_eq!(
            buyer_detail.validation_receipts,
            seller_detail.validation_receipts
        );
        assert_eq!(
            buyer_detail.validation_receipts[0].event_id,
            receipt_event.id
        );
        assert_eq!(buyer_detail.status, BuyerOrderStatus::Placed);
        assert_eq!(seller_detail.status, OrderStatus::NeedsAction);
    }

    #[test]
    fn validation_receipt_invalid_candidates_do_not_surface_as_order_evidence() {
        let fixture = validation_receipt_order_fixture("validation-receipt-invalid-candidates");
        let mut mismatched_tag_event = validation_receipt_event(
            hex_event_id(61).as_str(),
            fixture.seller_pubkey.as_str(),
            fixture.order_id_raw.as_str(),
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_603,
        );
        if let Some(tag) = mismatched_tag_event
            .tags
            .iter_mut()
            .find(|tag| tag.first().map(String::as_str) == Some("event_set_root"))
        {
            tag[1] = hash32(99);
        }
        let wrong_order_event = validation_receipt_event(
            hex_event_id(62).as_str(),
            fixture.seller_pubkey.as_str(),
            "wrong-order-id",
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_604,
        );
        let mut buyer_kind_candidate = validation_receipt_event(
            hex_event_id(63).as_str(),
            fixture.buyer_pubkey.as_str(),
            fixture.order_id_raw.as_str(),
            fixture.listing_event_id.as_str(),
            fixture.request_event_id.as_str(),
            fixture.decision_event_id.as_str(),
            RadrootsValidationReceiptResult::Valid,
            1_777_665_605,
        );
        buyer_kind_candidate.kind = KIND_ORDER_REQUEST as u32;

        for (record_id, event) in [
            (
                "cli:signed_event:validation-receipt:mismatched-tag",
                &mismatched_tag_event,
            ),
            (
                "cli:signed_event:validation-receipt:wrong-order",
                &wrong_order_event,
            ),
            (
                "cli:signed_event:validation-receipt:buyer-kind",
                &buyer_kind_candidate,
            ),
        ] {
            fixture
                .events
                .append_record(&signed_order_event_record(
                    record_id,
                    event,
                    fixture.listing_addr.as_str(),
                    SourceRuntime::Cli,
                    None,
                ))
                .expect("append invalid validation receipt candidate");
        }
        fixture
            .app_store
            .import_shared_local_events_from_store(&fixture.events)
            .expect("import invalid validation receipt candidates");

        let buyer_detail = fixture
            .app_store
            .load_buyer_order_detail(&fixture.buyer_context, fixture.order_id)
            .expect("load buyer detail after invalid validation receipt candidates")
            .expect("buyer detail after invalid validation receipt candidates");
        let seller_detail = fixture
            .app_store
            .load_order_detail(fixture.seller_farm_id, fixture.order_id)
            .expect("load seller detail after invalid validation receipt candidates")
            .expect("seller detail after invalid validation receipt candidates");

        assert!(buyer_detail.validation_receipts.is_empty());
        assert!(seller_detail.validation_receipts.is_empty());
        assert_eq!(buyer_detail.status, BuyerOrderStatus::Placed);
        assert_eq!(seller_detail.status, OrderStatus::NeedsAction);
        assert_eq!(
            buyer_detail.workflow.agreement,
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(seller_detail.primary_action, None);
    }

    #[test]
    fn active_order_revision_projects_through_cli_reducer_state() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "EEEEEEEEEEEEEEEEEEEEEE";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAw";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("app-buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "active-revision-order-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "active-revision-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Revision Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append revision listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import revision listing");

        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr("active-revision-listing-event"),
            &request_payload,
        )
        .expect("build revision order request");
        let request_event =
            event_from_parts("active-revision-request-event", buyer_pubkey, request_parts);
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:active-revision:request",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_revision"),
            ))
            .expect("append revision order request");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import revision order request");

        let proposal_payload = revision_proposal_payload(
            "revision-1",
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
            request_event.id.as_str(),
            request_event.id.as_str(),
        );
        let proposal_parts = order_revision_proposal_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &proposal_payload,
        )
        .expect("build revision proposal");
        let proposal_event = event_from_parts(
            "active-revision-proposal-event",
            seller_pubkey,
            proposal_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "cli:signed_event:active-revision:proposal",
                &proposal_event,
                listing_addr.as_str(),
                SourceRuntime::Cli,
                None,
            ))
            .expect("append revision proposal");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import revision proposal");

        let seller_farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key);
        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let buyer_context = BuyerContext::account("acct_revision");
        let seller_orders = app_store
            .load_orders_list(
                seller_farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load revision seller orders after proposal");
        assert_eq!(seller_orders.rows[0].status, OrderStatus::NeedsAction);
        assert_eq!(
            seller_orders.rows[0].workflow.revision,
            TradeRevisionStatus::ChangeProposed
        );
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load revision buyer detail after proposal")
            .expect("revision buyer detail after proposal");
        assert_eq!(
            buyer_detail.workflow.revision,
            TradeRevisionStatus::ChangeProposed
        );
        assert_eq!(buyer_detail.economics.total_minor_units, Some(1600));

        let revision_decision_payload = revision_decision_payload(
            "revision-1",
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
            request_event.id.as_str(),
            proposal_event.id.as_str(),
            RadrootsOrderRevisionOutcome::Accepted,
        );
        let revision_decision_parts = order_revision_decision_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(proposal_event.id.as_str()),
            &revision_decision_payload,
        )
        .expect("build revision decision");
        let revision_decision_event = event_from_parts(
            "active-revision-decision-response-event",
            buyer_pubkey,
            revision_decision_parts,
        );
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:active-revision:decision-response",
                &revision_decision_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_revision"),
            ))
            .expect("append revision decision");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import revision decision");
        let seller_orders = app_store
            .load_orders_list(
                seller_farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load revision seller orders after decision");
        assert_eq!(seller_orders.rows[0].status, OrderStatus::NeedsAction);
        assert_eq!(
            seller_orders.rows[0].workflow.agreement,
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(
            seller_orders.rows[0].workflow.revision,
            TradeRevisionStatus::Updated
        );
        let seller_detail = app_store
            .load_order_detail(seller_farm_id, order_id)
            .expect("load revision seller detail after decision")
            .expect("revision seller detail after decision");
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load revision buyer detail after decision")
            .expect("revision buyer detail after decision");
        assert_eq!(
            seller_detail.workflow.revision,
            TradeRevisionStatus::Updated
        );
        assert_eq!(seller_detail.economics.total_minor_units, Some(2400));
        assert_eq!(buyer_detail.workflow.revision, TradeRevisionStatus::Updated);
        assert_eq!(buyer_detail.economics.total_minor_units, Some(2400));
    }

    #[test]
    fn active_order_pre_agreement_cancellation_projects_through_cli_reducer_state() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "EEEEEEEEEEEEEEEEEEEEEE";
        let listing_key = "AAAAAAAAAAAAAAAAAAAAAx";
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("app-buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let order_id_raw = "active-cancel-order-1";
        let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
        events
            .append_record(&signed_market_listing_record(
                "active-cancel-listing",
                seller_pubkey,
                farm_key,
                listing_key,
                "Cancellation Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append cancellation listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import cancellation listing");

        let request_payload = order_request_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts = order_request_event_build(
            &listing_event_ptr("active-cancel-listing-event"),
            &request_payload,
        )
        .expect("build cancellation order request");
        let request_event =
            event_from_parts("active-cancel-request-event", buyer_pubkey, request_parts);
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:active-cancel:request",
                &request_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_cancel"),
            ))
            .expect("append cancellation order request");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import cancellation order request");

        let cancel_payload = order_cancel_payload(
            order_id_raw,
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let cancel_parts = order_cancellation_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &cancel_payload,
        )
        .expect("build cancellation");
        let cancel_event = event_from_parts("active-cancel-event", buyer_pubkey, cancel_parts);
        events
            .append_record(&signed_order_event_record(
                "app:signed_event:active-cancel:cancel",
                &cancel_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_cancel"),
            ))
            .expect("append cancellation");
        let cancel_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import cancellation");

        let seller_farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key);
        let order_id = projected_order_id(order_id_raw, buyer_pubkey);
        let buyer_context = BuyerContext::account("acct_cancel");
        let buyer_detail = app_store
            .load_buyer_order_detail(&buyer_context, order_id)
            .expect("load cancellation buyer detail")
            .expect("cancellation buyer detail");
        let seller_orders = app_store
            .load_orders_list(
                seller_farm_id,
                &OrdersScreenQueryState {
                    filter: OrdersFilter::All,
                    fulfillment_window_id: None,
                },
            )
            .expect("load cancellation seller orders");

        assert_eq!(cancel_report.imported_records, 1);
        assert_eq!(buyer_detail.status, BuyerOrderStatus::Declined);
        assert_eq!(
            buyer_detail.workflow.agreement,
            TradeAgreementStatus::Cancelled
        );
        assert_eq!(seller_orders.rows[0].status, OrderStatus::Declined);
        assert_eq!(
            seller_orders.rows[0].workflow.agreement,
            TradeAgreementStatus::Cancelled
        );
        assert_eq!(seller_orders.rows[0].primary_action, None);
    }

    #[test]
    fn conflicting_order_decisions_project_to_needs_action_deterministically() {
        let run_case = |accepted_first: bool| {
            let app_store =
                AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
            let events = local_events_store();
            let farm_key = "FFFFFFFFFFFFFFFFFFFFFF";
            let listing_key = "AAAAAAAAAAAAAAAAAAAAAw";
            let seller_pubkey = test_pubkey("seller-pubkey");
            let seller_pubkey = seller_pubkey.as_str();
            let buyer_pubkey = test_pubkey("app-buyer-pubkey");
            let buyer_pubkey = buyer_pubkey.as_str();
            let order_id_raw = if accepted_first {
                "active-conflict-order-accepted-first"
            } else {
                "active-conflict-order-declined-first"
            };
            let listing_addr = format!("30402:{seller_pubkey}:{listing_key}");
            events
                .append_record(&signed_market_listing_record(
                    "active-conflict-listing",
                    seller_pubkey,
                    farm_key,
                    listing_key,
                    "Conflict Eggs",
                    "9",
                    "active",
                    "pickup",
                    "North barn pickup",
                    4_102_444_800,
                    4_102_531_200,
                    LocalRecordStatus::Published,
                    PublishOutboxStatus::Acknowledged,
                ))
                .expect("append conflict listing");
            let request_payload = order_request_payload(
                order_id_raw,
                listing_addr.as_str(),
                buyer_pubkey,
                seller_pubkey,
            );
            let request_parts = order_request_event_build(
                &listing_event_ptr("active-conflict-listing-event"),
                &request_payload,
            )
            .expect("build conflict request");
            let request_event =
                event_from_parts("active-conflict-request-event", buyer_pubkey, request_parts);
            events
                .append_record(&signed_order_event_record(
                    "app:signed_event:active-conflict:request",
                    &request_event,
                    listing_addr.as_str(),
                    SourceRuntime::App,
                    Some("acct_conflict"),
                ))
                .expect("append conflict request");
            let accepted_payload = accepted_order_decision_payload(
                order_id_raw,
                listing_addr.as_str(),
                buyer_pubkey,
                seller_pubkey,
            );
            let accepted_parts = order_decision_event_build(
                &typed_event_id(request_event.id.as_str()),
                &typed_event_id(request_event.id.as_str()),
                &accepted_payload,
            )
            .expect("build accepted conflict decision");
            let accepted_event = event_from_parts(
                "active-conflict-accepted-event",
                seller_pubkey,
                accepted_parts,
            );
            let declined_payload = declined_order_decision_payload(
                order_id_raw,
                listing_addr.as_str(),
                buyer_pubkey,
                seller_pubkey,
            );
            let declined_parts = order_decision_event_build(
                &typed_event_id(request_event.id.as_str()),
                &typed_event_id(request_event.id.as_str()),
                &declined_payload,
            )
            .expect("build declined conflict decision");
            let declined_event = event_from_parts(
                "active-conflict-declined-event",
                seller_pubkey,
                declined_parts,
            );
            let ordered_events = if accepted_first {
                [accepted_event, declined_event]
            } else {
                [declined_event, accepted_event]
            };
            for (index, event) in ordered_events.iter().enumerate() {
                events
                    .append_record(&signed_order_event_record(
                        &format!("cli:signed_event:active-conflict:decision:{index}"),
                        event,
                        listing_addr.as_str(),
                        SourceRuntime::Cli,
                        None,
                    ))
                    .expect("append conflict decision");
            }

            app_store
                .import_shared_local_events_from_store(&events)
                .expect("import conflicting decisions");
            let order_id = projected_order_id(order_id_raw, buyer_pubkey);
            let detail = app_store
                .load_order_detail(
                    deterministic_farm_id(Some(seller_pubkey), farm_key),
                    order_id,
                )
                .expect("load conflict order detail")
                .expect("conflict order detail");
            detail.status
        };

        assert_eq!(run_case(true), OrderStatus::NeedsAction);
        assert_eq!(run_case(false), OrderStatus::NeedsAction);
    }

    #[test]
    fn malformed_order_event_remains_signed_event_evidence_without_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:order-request:malformed".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1100,
                inserted_at_ms: 1101,
                owner_account_id: None,
                owner_pubkey: Some("buyer-pubkey".to_owned()),
                farm_id: None,
                listing_addr: Some("30402:seller-pubkey:listing-key".to_owned()),
                local_work_json: None,
                event_id: Some("malformed-order-event".to_owned()),
                event_kind: Some(KIND_ORDER_REQUEST),
                event_pubkey: Some("buyer-pubkey".to_owned()),
                event_created_at: Some(1100),
                event_tags_json: Some(json!([["d", "bad-order"]])),
                event_content: Some("not-json".to_owned()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "malformed-order-event",
                    "kind": KIND_ORDER_REQUEST,
                    "pubkey": "buyer-pubkey",
                    "content": "not-json"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append malformed order event");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import malformed order event");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let order_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .expect("load order count");

        assert_eq!(report.imported_records, 1);
        assert_eq!(report.skipped_records, 0);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].projected_kind, "signed_event");
        assert_eq!(
            imported[0].event_id.as_deref(),
            Some("malformed-order-event")
        );
        assert_eq!(order_count, 0);
    }

    #[test]
    fn imports_cli_local_work_into_app_farm_and_product_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        events
            .append_record(&local_work_record(
                "cli:local_work:farm",
                farm_key,
                json!({
                    "record_kind": "farm_config_v1",
                    "document": {
                        "selection": {
                            "account": "seller-account",
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
        let mut listing = local_work_record(
            "cli:local_work:listing",
            farm_key,
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
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
        );
        listing.listing_addr = Some(format!("30402:seller-pubkey:{listing_key}"));
        events
            .append_record(&listing)
            .expect("append listing local work");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import shared local events");
        let second_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import shared local events again");

        assert_eq!(report.scanned_records, 2);
        assert_eq!(report.imported_records, 2);
        assert!(report.last_change_seq.is_some());
        assert_eq!(second_report.scanned_records, 0);
        assert_eq!(second_report.imported_records, 0);
        assert_eq!(second_report.skipped_records, 0);
        assert_eq!(second_report.self_observed_records, 0);
        assert!(
            events
                .get_cursor("radroots_studio_app_sqlite_projection_v1")
                .expect("read shared cursor")
                .is_none()
        );
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        assert_eq!(imported.len(), 2);
        assert!(
            imported
                .iter()
                .all(|record| record.local_status == "local_saved")
        );
        let farm_setup = app_store
            .load_farm_setup("seller-account")
            .expect("load farm setup");
        let saved_farm = farm_setup.saved_farm.expect("saved farm");
        assert_eq!(saved_farm.display_name, "Green Farm");
        assert_eq!(farm_setup.draft.farm_name, "Green Farm");
        let products = app_store
            .load_products(
                saved_farm.farm_id,
                "",
                Default::default(),
                Default::default(),
            )
            .expect("load products");
        assert_eq!(products.rows.len(), 1);
        assert_eq!(products.rows[0].title, "Eggs");
        assert_eq!(products.rows[0].subtitle.as_deref(), Some("Fresh eggs"));
        assert_eq!(
            products.rows[0]
                .price
                .as_ref()
                .expect("price")
                .amount_minor_units,
            600
        );
        assert_eq!(products.rows[0].stock.quantity, Some(10));
        assert_eq!(
            products.rows[0].status,
            radroots_studio_app_view::ProductStatus::Draft
        );
    }

    #[test]
    fn fresh_app_store_replays_existing_shared_records_after_another_app_imported_them() {
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        events
            .append_record(&local_work_record(
                "cli:local_work:farm",
                farm_key,
                json!({
                    "record_kind": "farm_config_v1",
                    "document": {
                        "selection": {
                            "account": "seller-account",
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
                        }
                    }
                }),
            ))
            .expect("append farm local work");
        let first_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open first app sqlite store");
        let first_report = first_store
            .import_shared_local_events_from_store(&events)
            .expect("first app imports shared local events");
        let second_same_store_report = first_store
            .import_shared_local_events_from_store(&events)
            .expect("first app imports unchanged shared local events");
        let second_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open second app sqlite store");
        let fresh_store_report = second_store
            .import_shared_local_events_from_store(&events)
            .expect("fresh app imports shared local events");

        assert_eq!(first_report.scanned_records, 1);
        assert_eq!(first_report.imported_records, 1);
        assert_eq!(second_same_store_report.scanned_records, 0);
        assert_eq!(second_same_store_report.imported_records, 0);
        assert_eq!(fresh_store_report.scanned_records, 1);
        assert_eq!(fresh_store_report.imported_records, 1);
        assert!(
            events
                .get_cursor("radroots_studio_app_sqlite_projection_v1")
                .expect("read shared cursor")
                .is_none()
        );
    }

    #[test]
    fn imports_signed_listing_tags_into_existing_local_product_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        events
            .append_record(&local_work_record(
                "cli:local_work:farm",
                farm_key,
                json!({
                    "record_kind": "farm_config_v1",
                    "document": {
                        "selection": {
                            "account": "seller-account",
                            "farm_d_tag": farm_key
                        },
                        "profile": {
                            "name": "Green Farm"
                        },
                        "farm": {
                            "d_tag": farm_key,
                            "name": "Green Farm",
                            "location": {
                                "primary": "farmstand"
                            }
                        }
                    }
                }),
            ))
            .expect("append farm local work");
        let mut listing = local_work_record(
            "cli:local_work:listing",
            farm_key,
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
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
        );
        listing.listing_addr = Some(format!("30402:seller-pubkey:{listing_key}"));
        events
            .append_record(&listing)
            .expect("append listing local work");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import local work records");
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:listing:event-1".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1100,
                inserted_at_ms: 1101,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some("seller-pubkey".to_owned()),
                farm_id: Some(farm_key.to_owned()),
                listing_addr: Some(format!("30402:seller-pubkey:{listing_key}")),
                local_work_json: None,
                event_id: Some("event-1".to_owned()),
                event_kind: Some(KIND_LISTING),
                event_pubkey: Some("seller-pubkey".to_owned()),
                event_created_at: Some(1100),
                event_tags_json: Some(json!([
                    ["d", listing_key],
                    ["a", format!("30340:seller-pubkey:{farm_key}")],
                    ["key", "eggs"],
                    ["title", "Relay Eggs"],
                    ["summary", "Published eggs"],
                    ["radroots:bin", "bin-1", "1", "each"],
                    ["radroots:price", "bin-1", "8", "USD", "1", "each"],
                    ["inventory", "9"],
                    ["status", "active"]
                ])),
                event_content: Some("# Relay Eggs\n\nPublished eggs".to_owned()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "event-1",
                    "kind": KIND_LISTING,
                    "pubkey": "seller-pubkey",
                    "content": "# Relay Eggs\n\nPublished eggs"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append signed listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let listing_records = imported
            .iter()
            .filter(|record| record.projected_kind == "listing")
            .collect::<Vec<_>>();
        assert_eq!(listing_records.len(), 2);
        assert_eq!(
            listing_records[0].projected_id,
            listing_records[1].projected_id
        );
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let product: (String, String, Option<i64>, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT title, status, price_minor_units, stock_count FROM products",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load product");
        assert_eq!(product_count, 1);
        assert_eq!(product.0, "Relay Eggs");
        assert_eq!(product.1, "published");
        assert_eq!(product.2, Some(800));
        assert_eq!(product.3, Some(9));
    }

    #[test]
    fn cli_origin_signed_window_listing_projects_into_buyer_browse_and_search() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        events
            .append_record(&signed_market_listing_record(
                "buyer-visible-cli",
                "seller-pubkey",
                farm_key,
                listing_key,
                "Buyer Visible Eggs",
                "9",
                "active",
                "pickup",
                "North barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let browse = app_store
            .load_buyer_listings("", &BTreeSet::new())
            .expect("buyer browse should load");
        let search = app_store
            .load_buyer_listings("eggs", &BTreeSet::from([FarmOrderMethod::Pickup]))
            .expect("buyer search should load");
        let detail = app_store
            .load_buyer_product_detail(search.rows[0].product_id)
            .expect("buyer detail should load")
            .expect("buyer detail should exist");

        assert_eq!(report.imported_records, 1);
        assert_eq!(browse.rows.len(), 1);
        assert_eq!(search.rows.len(), 1);
        assert_eq!(search.rows[0].title, "Buyer Visible Eggs");
        assert_eq!(
            search.rows[0].availability.state,
            ProductAvailabilityState::Scheduled
        );
        assert_eq!(search.rows[0].stock.quantity, Some(9));
        assert_eq!(
            search.rows[0].fulfillment_methods,
            BTreeSet::from([FarmOrderMethod::Pickup])
        );
        assert_eq!(detail.listing.title, "Buyer Visible Eggs");
    }

    #[test]
    fn app_origin_signed_window_listing_converges_into_buyer_visibility() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x55555555555545558555555555555555);
        let product_uuid = Uuid::from_u128(0x66666666666646668666666666666666);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let listing_addr = format!("30402:app-seller-pubkey:{listing_key}");
        let app_farm_record = app_local_work_record(
            "app:local_work:farm:buyer-visible",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "App Farm",
                        "location": {
                            "primary": "app farmstand"
                        }
                    }
                }
            }),
        );
        let mut app_listing_record = app_local_work_record(
            "app:local_work:listing:buyer-visible",
            farm_key.as_str(),
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
                        "pubkey": "app-seller-pubkey"
                    },
                    "product": {
                        "key": listing_key,
                        "title": "App Draft Eggs",
                        "summary": "Fresh app-origin eggs"
                    },
                    "primary_bin": {
                        "quantity_unit": "each",
                        "price_amount": "7",
                        "price_currency": "USD"
                    },
                    "inventory": {
                        "available": "12"
                    }
                }
            }),
        );
        app_listing_record.listing_addr = Some(listing_addr);
        events
            .append_record(&app_farm_record)
            .expect("append app farm local work");
        events
            .append_record(&app_listing_record)
            .expect("append app listing local work");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app local records");
        events
            .append_record(&signed_market_listing_record(
                "buyer-visible-app-origin",
                "app-seller-pubkey",
                farm_key.as_str(),
                listing_key.as_str(),
                "Buyer Visible App Eggs",
                "11",
                "active",
                "pickup",
                "App farmstand pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append signed app-origin listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed app-origin listing");
        let buyer_listings = app_store
            .load_buyer_listings("app eggs", &BTreeSet::new())
            .expect("buyer listings should load");

        assert_eq!(buyer_listings.rows.len(), 1);
        assert_eq!(buyer_listings.rows[0].product_id.as_uuid(), product_uuid);
        assert_eq!(buyer_listings.rows[0].title, "Buyer Visible App Eggs");
        assert_eq!(buyer_listings.rows[0].stock.quantity, Some(11));
    }

    #[test]
    fn network_app_origin_listing_cannot_claim_app_product_without_app_owned_evidence() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x77777777777747778777777777777777);
        let product_uuid = Uuid::from_u128(0x88888888888848888888888888888888);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let listing_addr = format!("30402:app-seller-pubkey:{listing_key}");
        seed_app_projection(&app_store, farm_uuid, product_uuid);
        let mut network_listing = signed_market_listing_record(
            "network-app-origin",
            "app-seller-pubkey",
            farm_key.as_str(),
            listing_key.as_str(),
            "Relay App Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        network_listing.source_runtime = SourceRuntime::Network;
        network_listing.owner_account_id = None;
        events
            .append_record(&network_listing)
            .expect("append network app-origin listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import network app-origin listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let app_product: (String, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT title, stock_count FROM products WHERE id = ?1",
                [product_uuid.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load app product");
        let network_product_id =
            deterministic_product_id(Some("app-seller-pubkey"), listing_key.as_str());
        let network_product: (String, String, String, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT id, farm_id, title, stock_count FROM products WHERE id = ?1",
                [network_product_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load network product");
        let buyer_listings = app_store
            .load_buyer_listings("relay app", &BTreeSet::new())
            .expect("buyer listings should load");
        let listing_import = imported
            .iter()
            .find(|record| record.record_id == "network-app-origin")
            .expect("network app-origin listing import");

        assert_eq!(report.imported_records, 1);
        assert_eq!(product_count, 2);
        assert_eq!(app_product.0, "Origin Eggs");
        assert_eq!(app_product.1, Some(3));
        assert_ne!(network_product_id.as_uuid(), product_uuid);
        assert_ne!(network_product.1, farm_uuid.to_string());
        assert_eq!(network_product.2, "Relay App Eggs");
        assert_eq!(network_product.3, Some(11));
        assert_eq!(buyer_listings.rows.len(), 1);
        assert_eq!(
            buyer_listings.rows[0].product_id.as_uuid(),
            network_product_id.as_uuid()
        );
        assert_eq!(
            listing_import.source_runtime,
            SourceRuntime::Network.as_str()
        );
        assert_eq!(
            listing_import.listing_addr.as_deref(),
            Some(listing_addr.as_str())
        );
        assert_eq!(
            listing_import.projected_id.as_deref(),
            Some(network_product_id.to_string().as_str())
        );
    }

    #[test]
    fn network_app_origin_listing_reuses_app_product_with_matching_app_owned_evidence() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x79797979797949799797979797979797);
        let product_uuid = Uuid::from_u128(0x89898989898949898989898989898989);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let listing_addr = format!("30402:app-seller-pubkey:{listing_key}");
        let app_farm_record = app_local_work_record(
            "app:local_work:farm:network-claim-gate",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "App Farm"
                    }
                }
            }),
        );
        let mut app_listing_record = app_local_work_record(
            "app:local_work:listing:network-claim-gate",
            farm_key.as_str(),
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
                        "pubkey": "app-seller-pubkey"
                    },
                    "product": {
                        "key": listing_key,
                        "title": "App Draft Eggs",
                        "summary": "Fresh app-origin eggs"
                    },
                    "primary_bin": {
                        "quantity_unit": "each",
                        "price_amount": "7",
                        "price_currency": "USD"
                    },
                    "inventory": {
                        "available": "12"
                    }
                }
            }),
        );
        app_listing_record.listing_addr = Some(listing_addr.clone());
        events
            .append_record(&app_farm_record)
            .expect("append app farm local work");
        events
            .append_record(&app_listing_record)
            .expect("append app listing local work");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app local work");
        let mut network_listing = signed_market_listing_record(
            "network-app-origin-matching-evidence",
            "app-seller-pubkey",
            farm_key.as_str(),
            listing_key.as_str(),
            "Relay App Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        network_listing.source_runtime = SourceRuntime::Network;
        network_listing.owner_account_id = None;
        events
            .append_record(&network_listing)
            .expect("append network app-origin listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import network app-origin listing");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let product: (String, String, String, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT id, farm_id, title, stock_count FROM products",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load product");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let listing_import = imported
            .iter()
            .find(|record| record.record_id == "network-app-origin-matching-evidence")
            .expect("network app-origin listing import");

        assert_eq!(report.imported_records, 1);
        assert_eq!(product_count, 1);
        assert_eq!(product.0, product_uuid.to_string());
        assert_eq!(product.1, farm_uuid.to_string());
        assert_eq!(product.2, "Relay App Eggs");
        assert_eq!(product.3, Some(11));
        assert_eq!(
            listing_import.source_runtime,
            SourceRuntime::Network.as_str()
        );
        assert_eq!(
            listing_import.projected_id.as_deref(),
            Some(product_uuid.to_string().as_str())
        );
    }

    #[test]
    fn network_app_origin_listing_requires_matching_event_pubkey_for_app_product_reuse() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x7a7a7a7a7a7a4a7a9a7a7a7a7a7a7a7a);
        let product_uuid = Uuid::from_u128(0x8a8a8a8a8a8a4a8aaa8a8a8a8a8a8a8a);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let listing_addr = format!("30402:app-seller-pubkey:{listing_key}");
        let app_farm_record = app_local_work_record(
            "app:local_work:farm:network-foreign-claim",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "App Farm"
                    }
                }
            }),
        );
        let mut app_listing_record = app_local_work_record(
            "app:local_work:listing:network-foreign-claim",
            farm_key.as_str(),
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
                        "pubkey": "app-seller-pubkey"
                    },
                    "product": {
                        "key": listing_key,
                        "title": "App Draft Eggs",
                        "summary": "Fresh app-origin eggs"
                    },
                    "primary_bin": {
                        "quantity_unit": "each",
                        "price_amount": "7",
                        "price_currency": "USD"
                    },
                    "inventory": {
                        "available": "12"
                    }
                }
            }),
        );
        app_listing_record.listing_addr = Some(listing_addr.clone());
        events
            .append_record(&app_farm_record)
            .expect("append app farm local work");
        events
            .append_record(&app_listing_record)
            .expect("append app listing local work");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app local work");
        let mut network_listing = signed_market_listing_record(
            "network-app-origin-foreign-event-pubkey",
            "app-seller-pubkey",
            farm_key.as_str(),
            listing_key.as_str(),
            "Foreign Relay App Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        network_listing.source_runtime = SourceRuntime::Network;
        network_listing.owner_account_id = None;
        network_listing.event_pubkey = Some("foreign-seller-pubkey".to_owned());
        events
            .append_record(&network_listing)
            .expect("append foreign network app-origin listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import network app-origin listing");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let app_product: (String, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT title, stock_count FROM products WHERE id = ?1",
                [product_uuid.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load app product");
        let foreign_product_id =
            deterministic_product_id(Some("foreign-seller-pubkey"), listing_key.as_str());
        let foreign_product_count: i64 = app_store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id = ?1",
                [foreign_product_id.to_string()],
                |row| row.get(0),
            )
            .expect("foreign product count");

        assert_eq!(product_count, 2);
        assert_eq!(app_product.0, "App Draft Eggs");
        assert_eq!(app_product.1, Some(12));
        assert_eq!(foreign_product_count, 1);
    }

    #[test]
    fn app_signed_duplicate_replaces_network_listing_product_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x99999999999949999999999999999999);
        let product_uuid = Uuid::from_u128(0xaaaaaaaaaaaa4aaaaaaaaaaaaaaaaaaa);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let seller_pubkey = "app-seller-pubkey";
        let duplicate_event_id = "duplicate-app-origin-listing-event";
        let mut network_listing = signed_market_listing_record(
            "duplicate-network-app-origin",
            seller_pubkey,
            farm_key.as_str(),
            listing_key.as_str(),
            "Relay App Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        network_listing.source_runtime = SourceRuntime::Network;
        network_listing.owner_account_id = None;
        network_listing.record_id = "app:relay_event:duplicate-app-origin".to_owned();
        network_listing.event_id = Some(duplicate_event_id.to_owned());
        events
            .append_record(&network_listing)
            .expect("append network app-origin listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import network app-origin listing");
        let network_product_id =
            deterministic_product_id(Some(seller_pubkey), listing_key.as_str());
        let network_product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("network product count");
        assert_eq!(network_product_count, 1);
        assert_ne!(network_product_id.as_uuid(), product_uuid);
        let buyer_context = BuyerContext::account("acct_buyer");
        let network_listing = app_store
            .load_buyer_product_detail(network_product_id)
            .expect("network buyer detail should load")
            .expect("network listing should exist")
            .listing;
        app_store
            .replace_buyer_cart(
                &buyer_context,
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(network_listing.farm_id),
                    farm_display_name: Some(network_listing.farm_display_name.clone()),
                    lines: vec![radroots_studio_app_view::BuyerCartLineProjection {
                        product_id: network_listing.product_id,
                        farm_id: network_listing.farm_id,
                        farm_display_name: network_listing.farm_display_name.clone(),
                        title: network_listing.title.clone(),
                        quantity: 2,
                        unit_price: network_listing.price.clone(),
                        line_total_minor_units: 1600,
                        fulfillment_summary: network_listing
                            .next_fulfillment_window_label
                            .clone()
                            .expect("network listing fulfillment summary"),
                    }],
                    subtotal_minor_units: Some(1600),
                    currency_code: Some("USD".to_owned()),
                    replace_confirmation: None,
                },
            )
            .expect("buyer cart should save");
        app_store
            .save_buyer_order_review_draft(
                &buyer_context,
                &radroots_studio_app_view::BuyerOrderReviewDraft {
                    name: "Casey Buyer".to_owned(),
                    email: "casey@example.test".to_owned(),
                    phone: String::new(),
                    order_note: String::new(),
                },
            )
            .expect("order review draft should save");
        let order_id = app_store
            .place_buyer_order(&buyer_context)
            .expect("buyer order should place");
        app_store
            .replace_buyer_cart(
                &buyer_context,
                &radroots_studio_app_view::BuyerCartProjection {
                    farm_id: Some(network_listing.farm_id),
                    farm_display_name: Some(network_listing.farm_display_name.clone()),
                    lines: vec![radroots_studio_app_view::BuyerCartLineProjection {
                        product_id: network_listing.product_id,
                        farm_id: network_listing.farm_id,
                        farm_display_name: network_listing.farm_display_name.clone(),
                        title: network_listing.title.clone(),
                        quantity: 3,
                        unit_price: network_listing.price,
                        line_total_minor_units: 2400,
                        fulfillment_summary: network_listing
                            .next_fulfillment_window_label
                            .expect("network listing fulfillment summary"),
                    }],
                    subtotal_minor_units: Some(2400),
                    currency_code: Some("USD".to_owned()),
                    replace_confirmation: None,
                },
            )
            .expect("buyer cart should save again");

        seed_app_projection(&app_store, farm_uuid, product_uuid);
        let mut app_listing = signed_market_listing_record(
            "duplicate-app-signed-origin",
            seller_pubkey,
            farm_key.as_str(),
            listing_key.as_str(),
            "Relay App Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        app_listing.source_runtime = SourceRuntime::App;
        app_listing.record_id = "app:signed_event:duplicate-app-origin".to_owned();
        app_listing.event_id = Some(duplicate_event_id.to_owned());
        events
            .append_record(&app_listing)
            .expect("append app signed duplicate listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app signed duplicate listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let stale_product_count: i64 = app_store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id = ?1",
                [network_product_id.to_string()],
                |row| row.get(0),
            )
            .expect("stale product count");
        let listing_import = imported
            .iter()
            .find(|record| record.record_id == "app:signed_event:duplicate-app-origin")
            .expect("app signed duplicate listing import");
        let migrated_cart = app_store
            .load_buyer_cart(&buyer_context)
            .expect("buyer cart should load after duplicate convergence");
        let order_line_id: String = app_store
            .connection()
            .query_row(
                "SELECT id FROM order_lines WHERE order_id = ?1",
                [order_id.to_string()],
                |row| row.get(0),
            )
            .expect("order line id should load");

        assert_eq!(product_count, 1);
        assert_eq!(stale_product_count, 0);
        assert_eq!(migrated_cart.lines.len(), 1);
        assert_eq!(migrated_cart.lines[0].product_id.as_uuid(), product_uuid);
        assert_eq!(migrated_cart.lines[0].quantity, 3);
        assert!(order_line_id.contains(network_product_id.to_string().as_str()));
        assert_eq!(listing_import.source_runtime, SourceRuntime::App.as_str());
        assert_eq!(
            listing_import.projected_id.as_deref(),
            Some(product_uuid.to_string().as_str())
        );
        assert!(
            imported
                .iter()
                .all(|record| record.record_id != "app:relay_event:duplicate-app-origin")
        );
        app_store
            .clear_buyer_cart(&buyer_context)
            .expect("buyer cart should clear");
        assert_eq!(
            app_store
                .apply_buyer_repeat_demand_to_cart(&buyer_context, order_id, false)
                .expect("repeat demand should apply"),
            BuyerRepeatDemandApplyOutcome::Applied
        );
        let repeated_cart = app_store
            .load_buyer_cart(&buyer_context)
            .expect("buyer cart should load after repeat demand");
        assert_eq!(repeated_cart.lines.len(), 1);
        assert_eq!(repeated_cart.lines[0].product_id.as_uuid(), product_uuid);
        assert_eq!(repeated_cart.lines[0].quantity, 2);
    }

    #[test]
    fn failed_duplicate_listing_replacement_rolls_back_prior_visible_state() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x9b9b9b9b9b9b4b9bbb9b9b9b9b9b9b9b);
        let product_uuid = Uuid::from_u128(0xabababababab4abababababababababa);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let seller_pubkey = "app-seller-pubkey";
        let duplicate_event_id = "duplicate-app-origin-rollback-event";
        let mut network_listing = signed_market_listing_record(
            "rollback-network-app-origin",
            seller_pubkey,
            farm_key.as_str(),
            listing_key.as_str(),
            "Rollback Relay Eggs",
            "11",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        network_listing.source_runtime = SourceRuntime::Network;
        network_listing.owner_account_id = None;
        network_listing.record_id = "app:relay_event:rollback-app-origin".to_owned();
        network_listing.event_id = Some(duplicate_event_id.to_owned());
        events
            .append_record(&network_listing)
            .expect("append network app-origin listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import network app-origin listing");

        let network_product_id =
            deterministic_product_id(Some(seller_pubkey), listing_key.as_str());
        let network_farm_id = deterministic_farm_id(Some(seller_pubkey), farm_key.as_str());
        seed_app_projection(&app_store, farm_uuid, product_uuid);
        app_store
            .connection()
            .execute(
                "INSERT INTO buyer_carts (
                    buyer_context_key,
                    farm_id,
                     updated_at
                 ) VALUES ('account:acct_buyer', ?1, '2026-01-01T00:00:00Z')",
                [network_farm_id.to_string()],
            )
            .expect("insert buyer cart header");
        app_store
            .connection()
            .execute(
                "INSERT INTO buyer_cart_lines (
                    buyer_context_key,
                    product_id,
                    quantity,
                    updated_at
                 ) VALUES ('account:acct_buyer', ?1, 2, '2026-01-01T00:00:00Z')",
                [network_product_id.to_string()],
            )
            .expect("insert stale buyer cart line");
        app_store
            .connection()
            .execute_batch(
                format!(
                    "CREATE TEMP TRIGGER fail_duplicate_cart_delete
                     BEFORE DELETE ON buyer_cart_lines
                     WHEN old.product_id = '{}'
                     BEGIN
                        SELECT RAISE(ABORT, 'forced duplicate cart migration failure');
                     END;",
                    network_product_id
                )
                .as_str(),
            )
            .expect("create failure trigger");

        let mut app_listing = signed_market_listing_record(
            "rollback-app-signed-origin",
            seller_pubkey,
            farm_key.as_str(),
            listing_key.as_str(),
            "Rollback App Eggs",
            "9",
            "active",
            "pickup",
            "App farmstand pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        app_listing.source_runtime = SourceRuntime::App;
        app_listing.record_id = "app:signed_event:rollback-app-origin".to_owned();
        app_listing.event_id = Some(duplicate_event_id.to_owned());
        events
            .append_record(&app_listing)
            .expect("append app signed duplicate listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect_err("duplicate replacement should roll back on cart migration failure");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let stale_cart_quantity: i64 = app_store
            .connection()
            .query_row(
                "SELECT quantity FROM buyer_cart_lines WHERE product_id = ?1",
                [network_product_id.to_string()],
                |row| row.get(0),
            )
            .expect("stale cart quantity");
        let canonical_cart_count: i64 = app_store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM buyer_cart_lines WHERE product_id = ?1",
                [product_uuid.to_string()],
                |row| row.get(0),
            )
            .expect("canonical cart count");
        let network_product_title: String = app_store
            .connection()
            .query_row(
                "SELECT title FROM products WHERE id = ?1",
                [network_product_id.to_string()],
                |row| row.get(0),
            )
            .expect("network product title");

        assert_eq!(product_count, 2);
        assert_eq!(stale_cart_quantity, 2);
        assert_eq!(canonical_cart_count, 0);
        assert_eq!(network_product_title, "Rollback Relay Eggs");
        assert!(
            imported
                .iter()
                .any(|record| record.record_id == "app:relay_event:rollback-app-origin")
        );
        assert!(
            imported
                .iter()
                .all(|record| record.record_id != "app:signed_event:rollback-app-origin")
        );
    }

    #[test]
    fn buyer_visibility_rejects_incomplete_unpublished_stale_and_unsupported_records() {
        for record in [
            signed_market_listing_record(
                "pending-window",
                "seller-pubkey",
                "AAAAAAAAAAAAAAAAAAAAAA",
                "BBBBBBBBBBBBBBBBBBBBBB",
                "Pending Eggs",
                "8",
                "active",
                "pickup",
                "Pending barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::Pending,
            ),
            signed_market_listing_record(
                "sold-out-window",
                "seller-pubkey",
                "CCCCCCCCCCCCCCCCCCCCCC",
                "DDDDDDDDDDDDDDDDDDDDDD",
                "Sold Out Eggs",
                "0",
                "active",
                "pickup",
                "South barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ),
            signed_market_listing_record(
                "expired-window",
                "seller-pubkey",
                "EEEEEEEEEEEEEEEEEEEEEE",
                "FFFFFFFFFFFFFFFFFFFFFF",
                "Expired Eggs",
                "8",
                "active",
                "pickup",
                "East barn pickup",
                946_684_800,
                946_771_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ),
            signed_market_listing_record(
                "unsupported-fulfillment",
                "seller-pubkey",
                "GGGGGGGGGGGGGGGGGGGGGG",
                "HHHHHHHHHHHHHHHHHHHHHH",
                "Unsupported Eggs",
                "8",
                "active",
                "other",
                "Unknown exchange point",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ),
            signed_listing_record(
                "status-only",
                "IIIIIIIIIIIIIIIIIIIIII",
                "JJJJJJJJJJJJJJJJJJJJJJ",
                "active",
            ),
        ] {
            let app_store =
                AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
            let events = local_events_store();
            events.append_record(&record).expect("append record");

            app_store
                .import_shared_local_events_from_store(&events)
                .expect("import hidden listing record");

            assert!(buyer_listing_titles(&app_store).is_empty());
        }

        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "KKKKKKKKKKKKKKKKKKKKKK";
        let listing_key = "LLLLLLLLLLLLLLLLLLLLLL";
        events
            .append_record(&local_work_record(
                "local-only-listing",
                farm_key,
                json!({
                    "record_kind": "listing_draft_v1",
                    "document": {
                        "listing": {
                            "d_tag": listing_key,
                            "farm_d_tag": farm_key
                        },
                        "product": {
                            "title": "Local Only Eggs"
                        },
                        "primary_bin": {
                            "quantity_unit": "each",
                            "price_amount": "7",
                            "price_currency": "USD"
                        },
                        "inventory": {
                            "available": "7"
                        }
                    }
                }),
            ))
            .expect("append local-only listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import local-only listing");
        assert!(buyer_listing_titles(&app_store).is_empty());

        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        events
            .append_record(&signed_market_listing_record(
                "current-active-window",
                "seller-pubkey",
                farm_key,
                listing_key,
                "Current Eggs",
                "8",
                "active",
                "pickup",
                "West barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append active listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import active listing");
        assert_eq!(buyer_listing_titles(&app_store), vec!["Current Eggs"]);
        events
            .append_record(&signed_market_listing_record(
                "newer-archived-window",
                "seller-pubkey",
                farm_key,
                listing_key,
                "Archived Eggs",
                "8",
                "archived",
                "pickup",
                "West barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append archived listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import archived listing");
        assert!(buyer_listing_titles(&app_store).is_empty());
    }

    #[test]
    fn older_signed_listing_import_does_not_roll_back_current_product_state() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "CURRENTFARMAAAAAAAAAA";
        let listing_key = "CURRENTLISTINGBBBBBB";
        let mut newer = signed_market_listing_record(
            "listing-current-newer",
            "seller-pubkey",
            farm_key,
            listing_key,
            "New Eggs",
            "12",
            "active",
            "pickup",
            "North barn pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        set_listing_event_version(
            &mut newer,
            "event-listing-current-newer",
            2_000,
            "New Eggs",
            "12",
        );
        events.append_record(&newer).expect("append newer listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import newer listing");

        let mut older = signed_market_listing_record(
            "listing-current-older",
            "seller-pubkey",
            farm_key,
            listing_key,
            "Old Eggs",
            "3",
            "active",
            "pickup",
            "North barn pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        set_listing_event_version(
            &mut older,
            "event-listing-current-older",
            1_000,
            "Old Eggs",
            "3",
        );
        events.append_record(&older).expect("append older listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import older listing");
        let product: (String, Option<i64>) = app_store
            .connection()
            .query_row("SELECT title, stock_count FROM products", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("load product");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");

        assert_eq!(report.imported_records, 1);
        assert_eq!(product.0, "New Eggs");
        assert_eq!(product.1, Some(12));
        assert_eq!(
            imported
                .iter()
                .filter(|record| record.projected_kind == "listing")
                .count(),
            2
        );
    }

    #[test]
    fn equal_timestamp_signed_listing_currentness_uses_event_id_tie_breaker() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "TIEFARMAAAAAAAAAAAAAA";
        let listing_key = "TIELISTINGBBBBBBBBBB";
        let mut winning = signed_market_listing_record(
            "listing-tie-winning",
            "seller-pubkey",
            farm_key,
            listing_key,
            "Tie Winner Eggs",
            "10",
            "active",
            "pickup",
            "North barn pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        set_listing_event_version(
            &mut winning,
            "event-z-winning",
            3_000,
            "Tie Winner Eggs",
            "10",
        );
        events
            .append_record(&winning)
            .expect("append winning listing");
        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import winning listing");

        let mut losing = signed_market_listing_record(
            "listing-tie-losing",
            "seller-pubkey",
            farm_key,
            listing_key,
            "Tie Loser Eggs",
            "1",
            "active",
            "pickup",
            "North barn pickup",
            4_102_444_800,
            4_102_531_200,
            LocalRecordStatus::Published,
            PublishOutboxStatus::Acknowledged,
        );
        set_listing_event_version(&mut losing, "event-a-losing", 3_000, "Tie Loser Eggs", "1");
        events
            .append_record(&losing)
            .expect("append losing listing");

        app_store
            .import_shared_local_events_from_store(&events)
            .expect("import losing listing");
        let product: (String, Option<i64>) = app_store
            .connection()
            .query_row("SELECT title, stock_count FROM products", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("load product");

        assert_eq!(product.0, "Tie Winner Eggs");
        assert_eq!(product.1, Some(10));
    }

    #[test]
    fn signed_farm_import_prefers_event_identity_over_local_owner_metadata() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let signed_farm_key = "SIGNEDFARMAAAAAAAAAAAA";
        let expected_farm_id = deterministic_farm_id(Some("event-pubkey"), signed_farm_key);
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:farm:event-identity".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1100,
                inserted_at_ms: 1101,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some("stale-owner-pubkey".to_owned()),
                farm_id: Some("STALEFARMTAG".to_owned()),
                listing_addr: None,
                local_work_json: None,
                event_id: Some("event-farm-identity".to_owned()),
                event_kind: Some(KIND_FARM),
                event_pubkey: Some("event-pubkey".to_owned()),
                event_created_at: Some(1100),
                event_tags_json: Some(json!([["d", signed_farm_key]])),
                event_content: Some(
                    json!({
                        "d_tag": signed_farm_key,
                        "name": "Signed Farm"
                    })
                    .to_string(),
                ),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "event-farm-identity",
                    "kind": KIND_FARM,
                    "pubkey": "event-pubkey"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append signed farm");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed farm");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let stored_farm: (String, String) = app_store
            .connection()
            .query_row("SELECT id, display_name FROM farms", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("load farm");

        assert_eq!(report.imported_records, 1);
        assert_eq!(imported[0].projected_kind, "farm");
        assert_eq!(
            imported[0].projected_id.as_deref(),
            Some(expected_farm_id.to_string().as_str())
        );
        assert_eq!(stored_farm.0, expected_farm_id.to_string());
        assert_eq!(stored_farm.1, "Signed Farm");
    }

    #[test]
    fn cli_signed_listing_import_uses_cli_identity_for_app_shaped_keys() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let signed_farm_key =
            app_d_tag_from_uuid(Uuid::from_u128(0x77777777777747778777777777777777));
        let signed_listing_key =
            app_d_tag_from_uuid(Uuid::from_u128(0x88888888888848888888888888888888));
        let expected_farm_id =
            deterministic_farm_id(Some("farm-tag-pubkey"), signed_farm_key.as_str());
        let expected_product_id =
            deterministic_product_id(Some("listing-event-pubkey"), signed_listing_key.as_str());
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:listing:event-identity".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1100,
                inserted_at_ms: 1101,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some("stale-owner-pubkey".to_owned()),
                farm_id: Some("STALEFARMTAG".to_owned()),
                listing_addr: Some("30402:stale-owner-pubkey:STALELISTING".to_owned()),
                local_work_json: None,
                event_id: Some("event-listing-identity".to_owned()),
                event_kind: Some(KIND_LISTING),
                event_pubkey: Some("listing-event-pubkey".to_owned()),
                event_created_at: Some(1100),
                event_tags_json: Some(json!([
                    ["d", signed_listing_key],
                    ["a", format!("30340:farm-tag-pubkey:{signed_farm_key}")],
                    ["title", "Signed Event Eggs"],
                    ["summary", "Signed event summary"],
                    ["radroots:bin", "bin-1", "1", "each"],
                    ["radroots:price", "bin-1", "8", "USD", "1", "each"],
                    ["inventory", "9"],
                    ["status", "active"]
                ])),
                event_content: Some(
                    json!({
                        "product": {
                            "title": "Signed Event Eggs",
                            "summary": "Signed event summary"
                        }
                    })
                    .to_string(),
                ),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "event-listing-identity",
                    "kind": KIND_LISTING,
                    "pubkey": "listing-event-pubkey"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append signed listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let product: (String, String) = app_store
            .connection()
            .query_row("SELECT id, farm_id FROM products", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("load product");

        assert_eq!(report.imported_records, 1);
        assert_eq!(imported[0].projected_kind, "listing");
        assert_eq!(
            imported[0].projected_id.as_deref(),
            Some(expected_product_id.to_string().as_str())
        );
        assert_eq!(product.0, expected_product_id.to_string());
        assert_eq!(product.1, expected_farm_id.to_string());
    }

    #[test]
    fn direct_record_import_dedupes_signed_events_by_event_id() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "SIGNEDFARMAAAAAAAAAAAA";
        let listing_key = "SIGNEDLISTINGBBBBBBBB";
        let first = events
            .append_record(&signed_listing_record(
                "shared-record",
                farm_key,
                listing_key,
                "active",
            ))
            .expect("append shared signed listing");
        let mut duplicate = signed_listing_record("relay-record", farm_key, listing_key, "active");
        duplicate.event_id = first.event_id.clone();
        let duplicate = events
            .append_record(&duplicate)
            .expect("append relay signed listing");

        let report = app_store
            .import_local_event_records(&[first, duplicate])
            .expect("direct records should import");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");

        assert_eq!(report.scanned_records, 2);
        assert_eq!(report.imported_records, 1);
        assert_eq!(report.skipped_records, 1);
        assert_eq!(
            imported
                .iter()
                .filter(|record| record.projected_kind == "listing")
                .count(),
            1
        );
    }

    #[test]
    fn app_order_request_receipt_replaces_prior_relay_duplicate() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let listing_addr = format!("30402:{seller_pubkey}:app-order-listing");
        let payload = order_request_payload(
            "app-order-receipt-replaces-relay",
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let parts = order_request_event_build(&listing_event_ptr("listing-event"), &payload)
            .expect("build order request event");
        let event = event_from_parts("app-order-request-event", buyer_pubkey, parts);
        let mut relay_record = signed_order_event_record(
            "app:relay_event:order-request:duplicate",
            &event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        relay_record.outbox_status = PublishOutboxStatus::None;
        relay_record.relay_delivery_json = Some(json!({
            "state": "observed",
            "observed_relays": ["ws://127.0.0.1:1234/"]
        }));
        let relay_record = events
            .append_record(&relay_record)
            .expect("append relay order request");
        let app_record = events
            .append_record(&signed_order_event_record(
                "app:signed_event:order-request:duplicate",
                &event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_buyer"),
            ))
            .expect("append app order request receipt");

        let report = app_store
            .import_local_event_records(&[relay_record, app_record])
            .expect("import duplicate order request records");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let stored = imported
            .iter()
            .find(|record| record.event_id.as_deref() == Some(event.id.as_str()))
            .expect("app order request evidence");

        assert_eq!(report.imported_records, 2);
        assert_eq!(report.skipped_records, 0);
        assert_eq!(
            imported
                .iter()
                .filter(|record| record.event_id.as_deref() == Some(event.id.as_str()))
                .count(),
            1
        );
        assert_eq!(stored.record_id, "app:signed_event:order-request:duplicate");
        assert_eq!(stored.source_runtime, SourceRuntime::App.as_str());
        assert_eq!(stored.owner_account_id.as_deref(), Some("acct_buyer"));
        assert_eq!(
            stored.outbox_status,
            PublishOutboxStatus::Acknowledged.as_str()
        );
        assert_eq!(stored.listing_addr.as_deref(), Some(listing_addr.as_str()));
    }

    #[test]
    fn relay_order_decision_duplicate_does_not_downgrade_app_receipt() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let seller_pubkey = test_pubkey("seller-pubkey");
        let seller_pubkey = seller_pubkey.as_str();
        let buyer_pubkey = test_pubkey("buyer-pubkey");
        let buyer_pubkey = buyer_pubkey.as_str();
        let listing_addr = format!("30402:{seller_pubkey}:app-decision-listing");
        let request_payload = order_request_payload(
            "app-decision-receipt",
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let request_parts =
            order_request_event_build(&listing_event_ptr("listing-event"), &request_payload)
                .expect("build order request event");
        let request_event =
            event_from_parts("app-decision-request-event", buyer_pubkey, request_parts);
        let decision_payload = accepted_order_decision_payload(
            "app-decision-receipt",
            listing_addr.as_str(),
            buyer_pubkey,
            seller_pubkey,
        );
        let decision_parts = order_decision_event_build(
            &typed_event_id(request_event.id.as_str()),
            &typed_event_id(request_event.id.as_str()),
            &decision_payload,
        )
        .expect("build order decision event");
        let decision_event =
            event_from_parts("app-order-decision-event", seller_pubkey, decision_parts);
        let app_record = events
            .append_record(&signed_order_event_record(
                "app:signed_event:order-decision:duplicate",
                &decision_event,
                listing_addr.as_str(),
                SourceRuntime::App,
                Some("acct_seller"),
            ))
            .expect("append app order decision receipt");
        let mut relay_record = signed_order_event_record(
            "app:relay_event:order-decision:duplicate",
            &decision_event,
            listing_addr.as_str(),
            SourceRuntime::Cli,
            None,
        );
        relay_record.outbox_status = PublishOutboxStatus::None;
        relay_record.relay_delivery_json = Some(json!({
            "state": "observed",
            "observed_relays": ["ws://127.0.0.1:1234/"]
        }));
        let relay_record = events
            .append_record(&relay_record)
            .expect("append relay order decision");

        let report = app_store
            .import_local_event_records(&[app_record, relay_record])
            .expect("import duplicate order decision records");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let stored = imported
            .iter()
            .find(|record| record.event_id.as_deref() == Some(decision_event.id.as_str()))
            .expect("app order decision evidence");

        assert_eq!(report.imported_records, 1);
        assert_eq!(report.skipped_records, 1);
        assert_eq!(
            imported
                .iter()
                .filter(|record| record.event_id.as_deref() == Some(decision_event.id.as_str()))
                .count(),
            1
        );
        assert_eq!(
            stored.record_id,
            "app:signed_event:order-decision:duplicate"
        );
        assert_eq!(stored.source_runtime, SourceRuntime::App.as_str());
        assert_eq!(stored.owner_account_id.as_deref(), Some("acct_seller"));
        assert_eq!(
            stored.outbox_status,
            PublishOutboxStatus::Acknowledged.as_str()
        );
        assert_eq!(stored.listing_addr.as_deref(), Some(listing_addr.as_str()));
    }

    #[test]
    fn local_work_farm_import_preserves_duplicate_relay_signed_ready_farm() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let relay_events = local_events_store();
        let shared_events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x55555555555545558555555555555555);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let signed_event_id = "event-app-relay-ready-farm";
        let relay_record = relay_events
            .append_record(&signed_farm_record(
                "app:relay_event:farm-ready",
                signed_event_id,
                SourceRuntime::App,
                "app-seller-pubkey",
                farm_key.as_str(),
                "ready",
                "Relay Ready Farm",
            ))
            .expect("append relay farm");
        let direct_report = app_store
            .import_local_event_records(&[relay_record])
            .expect("direct relay import");
        let local_farm_record = app_local_work_record(
            "app:local_work:farm:ready-preserve",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "Draft Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "Draft Farm"
                    }
                }
            }),
        );
        shared_events
            .append_record(&local_farm_record)
            .expect("append local farm work");
        shared_events
            .append_record(&signed_farm_record(
                "app:signed_event:farm-ready",
                signed_event_id,
                SourceRuntime::App,
                "app-seller-pubkey",
                farm_key.as_str(),
                "ready",
                "Relay Ready Farm",
            ))
            .expect("append duplicate signed farm");

        let shared_report = app_store
            .import_shared_local_events_from_store(&shared_events)
            .expect("import shared local work after relay");
        let stored_farm: (String, String, String) = app_store
            .connection()
            .query_row("SELECT id, display_name, readiness FROM farms", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("load farm");

        assert_eq!(direct_report.imported_records, 1);
        assert_eq!(shared_report.imported_records, 1);
        assert_eq!(shared_report.skipped_records, 1);
        assert_eq!(stored_farm.0, farm_uuid.to_string());
        assert_eq!(stored_farm.1, "Draft Farm");
        assert_eq!(stored_farm.2, "ready");
    }

    #[test]
    fn signed_farm_without_readiness_preserves_listing_visible_farm() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "SIGNEDFARMAAAAAAAAAAAA";
        let listing_key = "SIGNEDLISTINGBBBBBBBB";
        let expected_farm_id = deterministic_farm_id(Some("seller-pubkey"), farm_key);
        events
            .append_record(&signed_market_listing_record(
                "visible-listing",
                "seller-pubkey",
                farm_key,
                listing_key,
                "Relay Ready Eggs",
                "8",
                "active",
                "pickup",
                "West barn pickup",
                4_102_444_800,
                4_102_531_200,
                LocalRecordStatus::Published,
                PublishOutboxStatus::Acknowledged,
            ))
            .expect("append visible listing");
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:farm:no-readiness".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1200,
                inserted_at_ms: 1201,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some("seller-pubkey".to_owned()),
                farm_id: Some(farm_key.to_owned()),
                listing_addr: None,
                local_work_json: None,
                event_id: Some("event-farm-no-readiness".to_owned()),
                event_kind: Some(KIND_FARM),
                event_pubkey: Some("seller-pubkey".to_owned()),
                event_created_at: Some(1200),
                event_tags_json: Some(json!([["d", farm_key]])),
                event_content: Some(
                    json!({
                        "d_tag": farm_key,
                        "name": "Relay Ready Farm"
                    })
                    .to_string(),
                ),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "event-farm-no-readiness",
                    "kind": KIND_FARM,
                    "pubkey": "seller-pubkey"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append farm without readiness");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import listing and farm");
        let stored_farm: (String, String, String) = app_store
            .connection()
            .query_row("SELECT id, display_name, readiness FROM farms", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("load farm");

        assert_eq!(report.imported_records, 2);
        assert_eq!(stored_farm.0, expected_farm_id.to_string());
        assert_eq!(stored_farm.1, "Relay Ready Farm");
        assert_eq!(stored_farm.2, "ready");
        assert_eq!(buyer_listing_titles(&app_store), vec!["Relay Ready Eggs"]);
    }

    #[test]
    fn maps_acknowledged_signed_listing_lifecycle_statuses() {
        for (status_tag, expected_product_status) in [
            ("active", "published"),
            ("window", "published"),
            ("archived", "archived"),
            ("sold", "paused"),
        ] {
            let app_store =
                AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
            let events = local_events_store();
            let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
            let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
            events
                .append_record(&signed_listing_record(
                    status_tag,
                    farm_key,
                    listing_key,
                    status_tag,
                ))
                .expect("append signed listing");

            let report = app_store
                .import_shared_local_events_from_store(&events)
                .expect("import signed listing");
            let product_status: String = app_store
                .connection()
                .query_row("SELECT status FROM products", [], |row| row.get(0))
                .expect("load product status");

            assert_eq!(report.imported_records, 1);
            assert_eq!(report.skipped_records, 0);
            assert_eq!(product_status, expected_product_status);
        }
    }

    #[test]
    fn maps_observed_signed_listing_as_published_without_outbox_acknowledgement() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        let mut record = signed_listing_record_with_publish_state(
            "observed-listing",
            farm_key,
            listing_key,
            "active",
            LocalRecordStatus::Published,
            PublishOutboxStatus::None,
        );
        record.relay_delivery_json = Some(json!({
            "state": "observed",
            "target_relays": ["ws://127.0.0.1:1234"],
            "connected_relays": ["ws://127.0.0.1:1234"],
            "acknowledged_relays": [],
            "observed_relays": ["ws://127.0.0.1:1234"],
            "failed_relays": []
        }));
        events
            .append_record(&record)
            .expect("append observed signed listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import observed signed listing");
        let product_status: String = app_store
            .connection()
            .query_row("SELECT status FROM products", [], |row| row.get(0))
            .expect("load product status");

        assert_eq!(report.imported_records, 1);
        assert_eq!(report.skipped_records, 0);
        assert_eq!(product_status, "published");
    }

    #[test]
    fn unknown_acknowledged_signed_listing_status_is_not_published() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        events
            .append_record(&signed_listing_record(
                "unknown-status",
                farm_key,
                listing_key,
                "unknown-status",
            ))
            .expect("append signed listing");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");

        assert_eq!(report.imported_records, 0);
        assert_eq!(report.skipped_records, 1);
        assert_eq!(imported[0].projected_kind, "unsupported");
        assert_eq!(product_count, 0);
    }

    #[test]
    fn pending_or_failed_signed_listing_records_do_not_downgrade_published_product() {
        for (record_status, outbox_status) in [
            (
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::Pending,
            ),
            (LocalRecordStatus::Failed, PublishOutboxStatus::Failed),
        ] {
            let app_store =
                AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
            let events = local_events_store();
            let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
            let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
            events
                .append_record(&signed_listing_record(
                    "confirmed",
                    farm_key,
                    listing_key,
                    "active",
                ))
                .expect("append confirmed signed listing");
            app_store
                .import_shared_local_events_from_store(&events)
                .expect("import confirmed signed listing");
            events
                .append_record(&signed_listing_record_with_publish_state(
                    record_status.as_str(),
                    farm_key,
                    listing_key,
                    "active",
                    record_status,
                    outbox_status,
                ))
                .expect("append unconfirmed signed listing");

            app_store
                .import_shared_local_events_from_store(&events)
                .expect("import unconfirmed signed listing");
            let product_status: String = app_store
                .connection()
                .query_row("SELECT status FROM products", [], |row| row.get(0))
                .expect("load product status");
            let imported = app_store
                .load_local_interop_records()
                .expect("load imported records");

            assert_eq!(product_status, "published");
            assert!(imported.iter().any(|record| {
                record.local_status == record_status.as_str()
                    && record.outbox_status == outbox_status.as_str()
            }));
        }
    }

    #[test]
    fn observes_outbox_updates_after_first_import_without_replaying_unchanged_rows() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_key = "AAAAAAAAAAAAAAAAAAAAAA";
        let listing_key = "BBBBBBBBBBBBBBBBBBBBBB";
        events
            .append_record(&signed_listing_record_with_publish_state(
                "pending-listing",
                farm_key,
                listing_key,
                "active",
                LocalRecordStatus::PendingPublish,
                PublishOutboxStatus::Pending,
            ))
            .expect("append pending signed listing");
        let first_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import pending listing");
        let unchanged_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import unchanged listing");

        assert_eq!(first_report.scanned_records, 1);
        assert_eq!(first_report.imported_records, 1);
        assert_eq!(unchanged_report.scanned_records, 0);

        events
            .update_outbox(&LocalEventRecordUpdate {
                record_id: "pending-listing".to_owned(),
                status: LocalRecordStatus::Published,
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
                updated_at_ms: 1200,
            })
            .expect("update listing outbox");
        let changed_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import updated listing");
        let product_status: String = app_store
            .connection()
            .query_row("SELECT status FROM products", [], |row| row.get(0))
            .expect("load product status");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");

        assert_eq!(changed_report.scanned_records, 1);
        assert_eq!(changed_report.imported_records, 1);
        assert_eq!(product_status, "published");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].local_status, "published");
        assert_eq!(imported[0].outbox_status, "acknowledged");
    }

    #[test]
    fn app_authored_shared_records_replay_into_fresh_store_without_origin_duplicates() {
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x11111111111111111111111111111111);
        let product_uuid = Uuid::from_u128(0x22222222222222222222222222222222);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let app_farm_record = app_local_work_record(
            "app:local_work:farm",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "App Farm",
                        "location": {
                            "primary": "app farmstand"
                        }
                    },
                    "listing_defaults": {
                        "delivery_method": "pickup",
                        "location": {
                            "primary": "app farmstand"
                        }
                    }
                }
            }),
        );
        let mut app_listing_record = app_local_work_record(
            "app:local_work:listing",
            farm_key.as_str(),
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
                        "pubkey": "app-seller-pubkey"
                    },
                    "product": {
                        "key": listing_key,
                        "title": "App Eggs",
                        "summary": "Fresh app-origin eggs"
                    },
                    "primary_bin": {
                        "quantity_unit": "each",
                        "price_amount": "7",
                        "price_currency": "USD"
                    },
                    "inventory": {
                        "available": "12"
                    }
                }
            }),
        );
        app_listing_record.listing_addr = Some(format!("30402:app-seller-pubkey:{listing_key}"));
        events
            .append_record(&app_farm_record)
            .expect("append app farm local work");
        events
            .append_record(&app_listing_record)
            .expect("append app listing local work");

        let origin_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open origin app sqlite store");
        seed_app_projection(&origin_store, farm_uuid, product_uuid);
        let origin_report = origin_store
            .import_shared_local_events_from_store(&events)
            .expect("import shared local events into origin store");
        let origin_second_report = origin_store
            .import_shared_local_events_from_store(&events)
            .expect("import unchanged shared local events into origin store");
        let origin_product_count: i64 = origin_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("origin product count");
        let origin_product: (String, String, String, Option<i64>, Option<i64>) = origin_store
            .connection()
            .query_row(
                "SELECT id, farm_id, title, price_minor_units, stock_count FROM products",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("load origin product");
        let origin_imports = origin_store
            .load_local_interop_records()
            .expect("load origin imported records");

        assert_eq!(origin_report.scanned_records, 2);
        assert_eq!(origin_report.imported_records, 2);
        assert_eq!(origin_report.skipped_records, 0);
        assert_eq!(origin_report.self_observed_records, 0);
        assert_eq!(origin_second_report.scanned_records, 0);
        assert_eq!(origin_product_count, 1);
        assert_eq!(origin_product.0, product_uuid.to_string());
        assert_eq!(origin_product.1, farm_uuid.to_string());
        assert_eq!(origin_product.2, "App Eggs");
        assert_eq!(origin_product.3, Some(700));
        assert_eq!(origin_product.4, Some(12));
        assert_eq!(origin_imports.len(), 2);

        let fresh_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open fresh app sqlite store");
        let fresh_report = fresh_store
            .import_shared_local_events_from_store(&events)
            .expect("import shared local events into fresh store");
        let fresh_product_count: i64 = fresh_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("fresh product count");
        let fresh_product: (String, String, String) = fresh_store
            .connection()
            .query_row("SELECT id, farm_id, title FROM products", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("load fresh product");
        let fresh_imports = fresh_store
            .load_local_interop_records()
            .expect("load fresh imported records");

        assert_eq!(fresh_report.scanned_records, 2);
        assert_eq!(fresh_report.imported_records, 2);
        assert_eq!(fresh_report.skipped_records, 0);
        assert_eq!(fresh_report.self_observed_records, 0);
        assert_eq!(fresh_product_count, 1);
        assert_eq!(fresh_product.0, product_uuid.to_string());
        assert_eq!(fresh_product.1, farm_uuid.to_string());
        assert_eq!(fresh_product.2, "App Eggs");
        assert_eq!(fresh_imports.len(), 2);
    }

    #[test]
    fn app_authored_records_with_non_uuid_tags_do_not_fallback_to_cli_identity() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let app_record = app_local_work_record(
            "app:local_work:farm:invalid-tag",
            "not-a-uuid-d-tag",
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": "not-a-uuid-d-tag"
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": "not-a-uuid-d-tag",
                        "name": "App Farm"
                    }
                }
            }),
        );
        events
            .append_record(&app_record)
            .expect("append app local work");

        let report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import shared local events");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let farm_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM farms", [], |row| row.get(0))
            .expect("farm count");

        assert_eq!(report.scanned_records, 1);
        assert_eq!(report.imported_records, 0);
        assert_eq!(report.skipped_records, 1);
        assert_eq!(report.self_observed_records, 0);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].projected_kind, "unsupported");
        assert_eq!(farm_count, 0);
    }

    #[test]
    fn signed_app_origin_listing_updates_existing_app_projection() {
        let app_store =
            AppSqliteStore::open(DatabaseTarget::InMemory).expect("open app sqlite store");
        let events = local_events_store();
        let farm_uuid = Uuid::from_u128(0x33333333333343338333333333333333);
        let product_uuid = Uuid::from_u128(0x44444444444444448444444444444444);
        let farm_key = app_d_tag_from_uuid(farm_uuid);
        let listing_key = app_d_tag_from_uuid(product_uuid);
        let listing_addr = format!("30402:app-seller-pubkey:{listing_key}");
        let app_farm_record = app_local_work_record(
            "app:local_work:farm:signed-convergence",
            farm_key.as_str(),
            json!({
                "record_kind": "farm_config_v1",
                "document": {
                    "selection": {
                        "account": "seller-account",
                        "farm_d_tag": farm_key
                    },
                    "profile": {
                        "display_name": "App Farm"
                    },
                    "farm": {
                        "d_tag": farm_key,
                        "name": "App Farm"
                    }
                }
            }),
        );
        let mut app_listing_record = app_local_work_record(
            "app:local_work:listing:signed-convergence",
            farm_key.as_str(),
            json!({
                "record_kind": "listing_draft_v1",
                "document": {
                    "listing": {
                        "d_tag": listing_key,
                        "farm_d_tag": farm_key
                    },
                    "seller_actor": {
                        "account_id": "seller-account",
                        "pubkey": "app-seller-pubkey"
                    },
                    "product": {
                        "key": listing_key,
                        "title": "App Draft Eggs",
                        "summary": "Fresh app-origin eggs"
                    },
                    "primary_bin": {
                        "quantity_unit": "each",
                        "price_amount": "7",
                        "price_currency": "USD"
                    },
                    "inventory": {
                        "available": "12"
                    }
                }
            }),
        );
        app_listing_record.listing_addr = Some(listing_addr.clone());
        events
            .append_record(&app_farm_record)
            .expect("append app farm local work");
        events
            .append_record(&app_listing_record)
            .expect("append app listing local work");

        let local_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import app local work");
        events
            .append_record(&LocalEventRecordInput {
                record_id: "cli:signed_event:listing:app-origin".to_owned(),
                family: LocalRecordFamily::SignedEvent,
                status: LocalRecordStatus::Published,
                source_runtime: SourceRuntime::Cli,
                created_at_ms: 1100,
                inserted_at_ms: 1101,
                owner_account_id: Some("seller-account".to_owned()),
                owner_pubkey: Some("app-seller-pubkey".to_owned()),
                farm_id: Some(farm_key.clone()),
                listing_addr: Some(listing_addr.clone()),
                local_work_json: None,
                event_id: Some("event-app-origin".to_owned()),
                event_kind: Some(KIND_LISTING),
                event_pubkey: Some("app-seller-pubkey".to_owned()),
                event_created_at: Some(1100),
                event_tags_json: Some(json!([
                    ["d", listing_key],
                    ["a", format!("30340:app-seller-pubkey:{farm_key}")],
                    ["title", "Relay App Eggs"],
                    ["summary", "Published app-origin eggs"],
                    ["radroots:bin", "bin-1", "1", "each"],
                    ["radroots:price", "bin-1", "8", "USD", "1", "each"],
                    ["inventory", "9"],
                    ["status", "active"]
                ])),
                event_content: Some("# Relay App Eggs\n\nPublished app-origin eggs".to_owned()),
                event_sig: Some("signature".to_owned()),
                raw_event_json: Some(json!({
                    "id": "event-app-origin",
                    "kind": KIND_LISTING,
                    "pubkey": "app-seller-pubkey",
                    "content": "# Relay App Eggs\n\nPublished app-origin eggs"
                })),
                outbox_status: PublishOutboxStatus::Acknowledged,
                relay_set_fingerprint: Some("relay-set".to_owned()),
                relay_delivery_json: Some(json!({
                    "state": "acknowledged",
                    "acknowledged_relays": ["ws://127.0.0.1:1234/"]
                })),
            })
            .expect("append signed app-origin listing");
        let signed_report = app_store
            .import_shared_local_events_from_store(&events)
            .expect("import signed app-origin listing");
        let imported = app_store
            .load_local_interop_records()
            .expect("load imported records");
        let listing_records = imported
            .iter()
            .filter(|record| record.projected_kind == "listing")
            .collect::<Vec<_>>();
        let product_count: i64 = app_store
            .connection()
            .query_row("SELECT COUNT(*) FROM products", [], |row| row.get(0))
            .expect("product count");
        let product: (String, String, String, Option<i64>, Option<i64>) = app_store
            .connection()
            .query_row(
                "SELECT id, farm_id, status, price_minor_units, stock_count FROM products",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("load product");

        assert_eq!(local_report.imported_records, 2);
        assert_eq!(signed_report.scanned_records, 1);
        assert_eq!(signed_report.imported_records, 1);
        assert_eq!(listing_records.len(), 2);
        assert_eq!(
            listing_records[0].projected_id,
            listing_records[1].projected_id
        );
        assert_eq!(product_count, 1);
        assert_eq!(product.0, product_uuid.to_string());
        assert_eq!(product.1, farm_uuid.to_string());
        assert_eq!(product.2, "published");
        assert_eq!(product.3, Some(800));
        assert_eq!(product.4, Some(9));
    }
}
