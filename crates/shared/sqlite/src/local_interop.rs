use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use radroots_studio_app_models::{
    FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection, FarmSummary,
    ProductId, ProductStatus,
};
use radroots_local_events::{
    LocalEventRecord, LocalEventsStore, LocalRecordFamily, LocalRecordStatus, PublishOutboxStatus,
    SourceRuntime,
};
use radroots_sql_core::{SqlExecutor, SqliteExecutor};
use rusqlite::{Connection, params};
use serde_json::Value;
use uuid::Uuid;

use crate::farm_setup::AppFarmSetupRepository;
use crate::{AppSqliteError, AppSqliteStore};

const LOCAL_EVENTS_BATCH_LIMIT: u32 = 500;
const APP_LOCAL_EVENTS_CONSUMER_ID: &str = "radroots_studio_app_sqlite_projection_v1";
const KIND_FARM: i64 = 30340;
const KIND_LISTING: i64 = 30402;
const KIND_LISTING_DRAFT: i64 = 30403;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppLocalInteropImportReport {
    pub scanned_records: u32,
    pub imported_records: u32,
    pub skipped_records: u32,
    pub last_seq: Option<i64>,
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
        let mut after_seq = 0i64;
        loop {
            let records = store
                .list_records_after(after_seq, LOCAL_EVENTS_BATCH_LIMIT)
                .map_err(|source| AppSqliteError::LocalEvents {
                    operation: "list shared local event records",
                    source,
                })?;
            let batch_len = records.len();
            for record in records {
                after_seq = record.seq;
                report.scanned_records += 1;
                report.last_seq = Some(record.seq);
                match self.import_record(&record)? {
                    ImportOutcome::Imported => report.imported_records += 1,
                    ImportOutcome::Skipped => report.skipped_records += 1,
                }
            }
            if batch_len < LOCAL_EVENTS_BATCH_LIMIT as usize {
                break;
            }
        }
        if let Some(last_seq) = report.last_seq {
            store
                .advance_cursor(APP_LOCAL_EVENTS_CONSUMER_ID, last_seq, current_time_ms()?)
                .map_err(|source| AppSqliteError::LocalEvents {
                    operation: "advance shared local event cursor",
                    source,
                })?;
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

    fn import_record(&self, record: &LocalEventRecord) -> Result<ImportOutcome, AppSqliteError> {
        if record.source_runtime == SourceRuntime::App {
            self.record_import(record, "unsupported", None)?;
            return Ok(ImportOutcome::Skipped);
        }
        let projection = match record.family {
            LocalRecordFamily::LocalWork => self.import_local_work(record)?,
            LocalRecordFamily::SignedEvent => self.import_signed_event(record)?,
        };
        match projection {
            Some(projection) => {
                self.record_import(record, projection.kind, projection.projected_id)?;
                Ok(ImportOutcome::Imported)
            }
            None => {
                self.record_import(record, "unsupported", None)?;
                Ok(ImportOutcome::Skipped)
            }
        }
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
        let farm_id = deterministic_farm_id(owner_pubkey.as_deref(), farm_key.as_str());
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
        self.upsert_farm_summary(&saved_farm)?;
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
        let farm_id = deterministic_farm_id(owner_pubkey.as_deref(), farm_key.as_str());
        self.ensure_farm_exists(farm_id)?;
        let product_id = deterministic_product_id(owner_pubkey.as_deref(), listing_key.as_str());
        let title = string_at(document, &["product", "title"])
            .or_else(|| string_at(document, &["product", "key"]))
            .unwrap_or_else(|| "Local product".to_owned());
        let subtitle = string_at(document, &["product", "summary"]).unwrap_or_default();
        let unit_label = string_at(document, &["primary_bin", "quantity_unit"])
            .or_else(|| string_at(document, &["primary_bin", "price_per_unit"]))
            .unwrap_or_default();
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
            status: product_status_for_record(record),
            unit_label,
            price_minor_units,
            price_currency,
            stock_count,
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
        let Some(farm_key) = record
            .farm_id
            .clone()
            .or_else(|| string_at(&content, &["d_tag"]))
        else {
            return Ok(None);
        };
        let owner_pubkey = record
            .owner_pubkey
            .as_deref()
            .or(record.event_pubkey.as_deref());
        let farm_id = deterministic_farm_id(owner_pubkey, farm_key.as_str());
        let display_name =
            string_at(&content, &["name"]).unwrap_or_else(|| "Local farm".to_owned());
        self.upsert_farm_summary(&FarmSummary {
            farm_id,
            display_name,
            readiness: FarmReadiness::Incomplete,
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
        let farm_key = record
            .farm_id
            .clone()
            .or_else(|| {
                content
                    .as_ref()
                    .and_then(|content| string_at(content, &["farm", "d_tag"]))
            })
            .or_else(|| tag_index_value(tags, "a", 1).and_then(|addr| address_d_tag(&addr)));
        let Some(farm_key) = farm_key else {
            return Ok(None);
        };
        let signed_farm_pubkey = content
            .as_ref()
            .and_then(|content| string_at(content, &["farm", "pubkey"]))
            .or_else(|| tag_index_value(tags, "a", 1).and_then(|addr| address_pubkey(&addr)));
        let owner_pubkey = record
            .owner_pubkey
            .as_deref()
            .or(record.event_pubkey.as_deref())
            .or(signed_farm_pubkey.as_deref());
        let farm_id = deterministic_farm_id(owner_pubkey, farm_key.as_str());
        self.ensure_farm_exists(farm_id)?;
        let product_id = deterministic_product_id(owner_pubkey, listing_key.as_str());
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
        self.upsert_product(ProductProjection {
            product_id,
            farm_id,
            title,
            subtitle,
            status: product_status_for_record(record),
            unit_label,
            price_minor_units,
            price_currency,
            stock_count,
        })?;
        Ok(Some(ProjectionRecord {
            kind: "listing",
            projected_id: Some(product_id.to_string()),
        }))
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
                    updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    farm_id = excluded.farm_id,
                    title = excluded.title,
                    subtitle = excluded.subtitle,
                    status = excluded.status,
                    unit_label = excluded.unit_label,
                    price_minor_units = excluded.price_minor_units,
                    price_currency = excluded.price_currency,
                    stock_count = excluded.stock_count,
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
                ],
            )
            .map_err(|source| AppSqliteError::Query {
                operation: "upsert local interop product",
                source,
            })?;
        Ok(())
    }

    fn record_import(
        &self,
        record: &LocalEventRecord,
        projected_kind: &str,
        projected_id: Option<String>,
    ) -> Result<(), AppSqliteError> {
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
                    outbox_status,
                    relay_delivery_json,
                    imported_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
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

    pub fn load_local_interop_records(
        &self,
    ) -> Result<Vec<StoredLocalInteropRecord>, AppSqliteError> {
        self.local_interop_repository().load_records()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImportOutcome {
    Imported,
    Skipped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectionRecord {
    kind: &'static str,
    projected_id: Option<String>,
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
}

fn current_time_ms() -> Result<i64, AppSqliteError> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        AppSqliteError::InvalidProjection {
            reason: "current local interop timestamp must be after unix epoch",
        }
    })?;
    i64::try_from(duration.as_millis()).map_err(|_| AppSqliteError::InvalidProjection {
        reason: "current local interop timestamp must fit i64 milliseconds",
    })
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

fn deterministic_uuid(scope: &str, owner_pubkey: Option<&str>, key: &str) -> Uuid {
    let seed = format!(
        "{scope}:{}:{}",
        owner_pubkey.unwrap_or("unknown-owner"),
        key.trim()
    );
    Uuid::new_v5(&Uuid::NAMESPACE_URL, seed.as_bytes())
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

fn product_status_for_record(record: &LocalEventRecord) -> ProductStatus {
    if record.status == LocalRecordStatus::Published
        && record.outbox_status == PublishOutboxStatus::Acknowledged
    {
        ProductStatus::Published
    } else {
        ProductStatus::Draft
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

#[cfg(test)]
mod tests {
    use radroots_local_events::{
        LocalEventRecordInput, LocalEventsStore, LocalRecordFamily, LocalRecordStatus,
        PublishOutboxStatus, SourceRuntime,
    };
    use radroots_sql_core::SqliteExecutor;
    use serde_json::json;

    use super::KIND_LISTING;
    use crate::{AppSqliteStore, DatabaseTarget};

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
        assert_eq!(second_report.imported_records, 2);
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
}
