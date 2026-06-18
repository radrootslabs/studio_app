#![forbid(unsafe_code)]

mod error;
mod interop;
mod migration_audit;
mod migrations;
mod repo;
mod sync;

use std::{collections::BTreeSet, fs, path::PathBuf, time::Duration};

use radroots_studio_app_sync::{
    AppRelayIngestScopeFreshness, PendingSyncOperation, SyncCheckpointStatus, SyncConflict,
    SyncConflictResolutionStatus,
};
use radroots_studio_app_view::{
    AccountSurfaceActivationProjection, AppActivityContext, AppActivityEvent, AppActivityKind,
    BuyerCartProjection, BuyerContext, BuyerListingsProjection, BuyerOrderDetailProjection,
    BuyerOrderReviewDraft, BuyerOrderReviewProjection, BuyerOrdersProjection,
    BuyerProductDetailProjection, FarmId, FarmOrderMethod, FarmRulesProjection,
    FarmSetupProjection, FarmSummary, FulfillmentWindowId, OrderDetailProjection, OrderId,
    OrderRecoveryProjection, OrdersListProjection, OrdersScreenQueryState, PackDayOutputSource,
    PackDayProjection, PackDayScreenQueryState, ProductEditorDraft, ProductId,
    ProductPublishBlocker, ProductsFilter, ProductsListProjection, ProductsSort, RecoveryKind,
    RecoveryQueueProjection, ReminderFeedProjection, ReminderLogEntryProjection,
    ReminderLogProjection, TodayAgendaProjection,
};
use rusqlite::Connection;

pub use error::AppSqliteError;
pub use interop::{
    AppLocalInteropImportReport, AppLocalInteropRepository, StoredLocalInteropRecord,
    projected_order_id_from_trade_request,
};
pub use migration_audit::{
    APP_SDK_MIGRATION_AUDIT_DEFAULT_BATCH_SIZE, APP_SDK_MIGRATION_AUDIT_MAX_BATCH_SIZE,
    AppSdkMigrationAuditClassification, AppSdkMigrationAuditCount,
    AppSdkMigrationAuditDuplicateCandidate, AppSdkMigrationAuditIssue, AppSdkMigrationAuditReport,
    AppSdkMigrationAuditRequest, AppSdkMigrationAuditSource, AppSdkMigrationAuditSourceReport,
};
pub use migrations::latest_schema_version;
pub use repo::{
    APP_ACTIVITY_CONTEXT_LIMIT, APP_ACTIVITY_RETENTION_LIMIT, AppActivationRepository,
    AppActivityRepository, AppBuyerRepository, AppFarmRulesRepository, AppFarmSetupRepository,
    AppOrdersRepository, AppProductsRepository, AppRemindersRepository, AppTodayAgendaRepository,
    BuyerOrderCoordinationRecord, BuyerOrderCoordinationState, BuyerOrderLocalEventExport,
    BuyerOrderLocalEventLine, BuyerRepeatDemandApplyOutcome, SelectedBuyerOrderScope,
    SellerOrderDecisionExport, SellerOrderDecisionLineExport, TODAY_AGENDA_LIST_LIMIT,
    TODAY_AGENDA_LOW_STOCK_THRESHOLD, derive_farm_rules_readiness,
};
pub use sync::{
    AppSyncRepository, StoredPendingSyncOperation, StoredRelayIngestCursor, StoredSyncConflict,
};

const SQLITE_BUSY_TIMEOUT_MS: u64 = 5_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseTarget {
    InMemory,
    Path(PathBuf),
}

pub struct AppSqliteStore {
    connection: Connection,
}

impl AppSqliteStore {
    pub fn open(target: DatabaseTarget) -> Result<Self, AppSqliteError> {
        let mut connection = open_connection(&target)?;
        bootstrap_connection(&mut connection, &target)?;

        Ok(Self { connection })
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn schema_version(&self) -> Result<u32, AppSqliteError> {
        schema_version(&self.connection)
    }

    pub fn today_agenda_repository(&self) -> AppTodayAgendaRepository<'_> {
        AppTodayAgendaRepository::new(&self.connection)
    }

    pub fn activity_repository(&self) -> AppActivityRepository<'_> {
        AppActivityRepository::new(&self.connection)
    }

    pub fn activation_repository(&self) -> AppActivationRepository<'_> {
        AppActivationRepository::new(&self.connection)
    }

    pub fn farm_setup_repository(&self) -> AppFarmSetupRepository<'_> {
        AppFarmSetupRepository::new(&self.connection)
    }

    pub fn farm_rules_repository(&self) -> AppFarmRulesRepository<'_> {
        AppFarmRulesRepository::new(&self.connection)
    }

    pub fn buyer_repository(&self) -> AppBuyerRepository<'_> {
        AppBuyerRepository::new(&self.connection)
    }

    pub fn products_repository(&self) -> AppProductsRepository<'_> {
        AppProductsRepository::new(&self.connection)
    }

    pub fn orders_repository(&self) -> AppOrdersRepository<'_> {
        AppOrdersRepository::new(&self.connection)
    }

    pub fn sync_repository(&self) -> AppSyncRepository<'_> {
        AppSyncRepository::new(&self.connection)
    }

    pub fn reminders_repository(&self) -> AppRemindersRepository<'_> {
        AppRemindersRepository::new(&self.connection)
    }

    pub fn load_today_agenda(
        &self,
        farm_id: Option<FarmId>,
    ) -> Result<TodayAgendaProjection, AppSqliteError> {
        self.today_agenda_repository().load(farm_id)
    }

    pub fn save_farm_summary(&self, farm: &FarmSummary) -> Result<(), AppSqliteError> {
        self.today_agenda_repository().save_farm_summary(farm)
    }

    pub fn record_activity_event(&self, kind: &AppActivityKind) -> Result<(), AppSqliteError> {
        self.activity_repository().record(kind)
    }

    pub fn load_recent_activity_events(
        &self,
        limit: usize,
    ) -> Result<Vec<AppActivityEvent>, AppSqliteError> {
        self.activity_repository().load_recent(limit)
    }

    pub fn load_activity_context(
        &self,
        limit: usize,
    ) -> Result<AppActivityContext, AppSqliteError> {
        self.activity_repository().load_context(limit)
    }

    pub fn load_surface_activation(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountSurfaceActivationProjection>, AppSqliteError> {
        self.activation_repository()
            .load_surface_activation(account_id)
    }

    pub fn save_surface_activation(
        &self,
        projection: &AccountSurfaceActivationProjection,
    ) -> Result<(), AppSqliteError> {
        self.activation_repository()
            .save_surface_activation(projection)
    }

    pub fn clear_surface_activation(&self, account_id: &str) -> Result<(), AppSqliteError> {
        self.activation_repository()
            .clear_surface_activation(account_id)
    }

    pub fn load_farm_setup(&self, account_id: &str) -> Result<FarmSetupProjection, AppSqliteError> {
        self.farm_setup_repository().load_farm_setup(account_id)
    }

    pub fn save_farm_setup(
        &self,
        account_id: &str,
        projection: &FarmSetupProjection,
    ) -> Result<(), AppSqliteError> {
        self.farm_setup_repository()
            .save_farm_setup(account_id, projection)
    }

    pub fn clear_farm_setup(&self, account_id: &str) -> Result<(), AppSqliteError> {
        self.farm_setup_repository().clear_farm_setup(account_id)
    }

    pub fn load_farm_rules(&self, farm_id: FarmId) -> Result<FarmRulesProjection, AppSqliteError> {
        self.farm_rules_repository().load_farm_rules(farm_id)
    }

    pub fn save_farm_rules(&self, projection: &FarmRulesProjection) -> Result<(), AppSqliteError> {
        self.farm_rules_repository().save_farm_rules(projection)
    }

    pub fn load_products(
        &self,
        farm_id: FarmId,
        search_query: &str,
        filter: ProductsFilter,
        sort: ProductsSort,
    ) -> Result<ProductsListProjection, AppSqliteError> {
        self.products_repository()
            .load_products(farm_id, search_query, filter, sort)
    }

    pub fn load_product_editor_draft(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ProductEditorDraft>, AppSqliteError> {
        self.products_repository()
            .load_product_editor_draft(product_id)
    }

    pub fn create_product_draft(&self, farm_id: FarmId) -> Result<ProductId, AppSqliteError> {
        self.products_repository().create_product_draft(farm_id)
    }

    pub fn load_orders_list(
        &self,
        farm_id: FarmId,
        query: &OrdersScreenQueryState,
    ) -> Result<OrdersListProjection, AppSqliteError> {
        self.orders_repository().load_orders_list(farm_id, query)
    }

    pub fn load_order_detail(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<Option<OrderDetailProjection>, AppSqliteError> {
        self.orders_repository()
            .load_order_detail(farm_id, order_id)
    }

    pub fn load_seller_order_decision_export(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<Option<SellerOrderDecisionExport>, AppSqliteError> {
        self.orders_repository()
            .load_seller_order_decision_export(farm_id, order_id)
    }

    pub fn load_pack_day(
        &self,
        farm_id: FarmId,
        query: &PackDayScreenQueryState,
    ) -> Result<PackDayProjection, AppSqliteError> {
        self.orders_repository().load_pack_day(farm_id, query)
    }

    pub fn load_pack_day_output_source(
        &self,
        farm_id: FarmId,
        fulfillment_window_id: FulfillmentWindowId,
    ) -> Result<Option<PackDayOutputSource>, AppSqliteError> {
        self.orders_repository()
            .load_pack_day_output_source(farm_id, fulfillment_window_id)
    }

    pub fn load_reminder_schedule(
        &self,
        account_id: &str,
        farm_id: FarmId,
    ) -> Result<ReminderFeedProjection, AppSqliteError> {
        self.reminders_repository()
            .load_reminder_schedule(account_id, farm_id)
    }

    pub fn replace_reminder_schedule(
        &self,
        account_id: &str,
        farm_id: FarmId,
        projection: &ReminderFeedProjection,
    ) -> Result<(), AppSqliteError> {
        self.reminders_repository()
            .replace_reminder_schedule(account_id, farm_id, projection)
    }

    pub fn apply_reminder_schedule_update(
        &self,
        account_id: &str,
        farm_id: FarmId,
        projection: &ReminderFeedProjection,
        log_entries: &[ReminderLogEntryProjection],
    ) -> Result<(), AppSqliteError> {
        self.reminders_repository().apply_reminder_schedule_update(
            account_id,
            farm_id,
            projection,
            log_entries,
        )
    }

    pub fn record_reminder_log_entry(
        &self,
        account_id: &str,
        farm_id: FarmId,
        entry: &ReminderLogEntryProjection,
    ) -> Result<String, AppSqliteError> {
        self.reminders_repository()
            .record_reminder_log_entry(account_id, farm_id, entry)
    }

    pub fn load_reminder_log(
        &self,
        account_id: &str,
        farm_id: FarmId,
        limit: usize,
    ) -> Result<ReminderLogProjection, AppSqliteError> {
        self.reminders_repository()
            .load_reminder_log(account_id, farm_id, limit)
    }

    pub fn load_recovery_queue(
        &self,
        account_id: &str,
        farm_id: FarmId,
    ) -> Result<RecoveryQueueProjection, AppSqliteError> {
        self.reminders_repository()
            .load_recovery_queue(account_id, farm_id)
    }

    pub fn load_recovery_record(
        &self,
        account_id: &str,
        order_id: OrderId,
        kind: RecoveryKind,
    ) -> Result<Option<OrderRecoveryProjection>, AppSqliteError> {
        self.reminders_repository()
            .load_recovery_record(account_id, order_id, kind)
    }

    pub fn save_recovery_record(
        &self,
        account_id: &str,
        farm_id: FarmId,
        record: &OrderRecoveryProjection,
    ) -> Result<(), AppSqliteError> {
        self.reminders_repository()
            .save_recovery_record(account_id, farm_id, record)
    }

    pub fn save_product_editor_draft(
        &self,
        product_id: ProductId,
        draft: &ProductEditorDraft,
    ) -> Result<bool, AppSqliteError> {
        self.products_repository()
            .save_product_editor_draft(product_id, draft)
    }

    pub fn update_product_stock(
        &self,
        product_id: ProductId,
        stock_quantity: u32,
    ) -> Result<bool, AppSqliteError> {
        self.products_repository()
            .update_product_stock(product_id, stock_quantity)
    }

    pub fn evaluate_product_publish_blockers(
        &self,
        product_id: ProductId,
    ) -> Result<Option<Vec<ProductPublishBlocker>>, AppSqliteError> {
        self.products_repository()
            .evaluate_product_publish_blockers(product_id)
    }

    pub fn load_buyer_listings(
        &self,
        search_query: &str,
        fulfillment_methods: &BTreeSet<FarmOrderMethod>,
    ) -> Result<BuyerListingsProjection, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_listings(search_query, fulfillment_methods)
    }

    pub fn load_buyer_product_detail(
        &self,
        product_id: ProductId,
    ) -> Result<Option<BuyerProductDetailProjection>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_product_detail(product_id)
    }

    pub fn load_buyer_cart(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerCartProjection, AppSqliteError> {
        self.buyer_repository().load_buyer_cart(context)
    }

    pub fn replace_buyer_cart(
        &self,
        context: &BuyerContext,
        cart: &BuyerCartProjection,
    ) -> Result<(), AppSqliteError> {
        self.buyer_repository().replace_buyer_cart(context, cart)
    }

    pub fn clear_buyer_cart(&self, context: &BuyerContext) -> Result<(), AppSqliteError> {
        self.buyer_repository().clear_buyer_cart(context)
    }

    pub fn load_buyer_order_review(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerOrderReviewProjection, AppSqliteError> {
        self.buyer_repository().load_buyer_order_review(context)
    }

    pub fn save_buyer_order_review_draft(
        &self,
        context: &BuyerContext,
        draft: &BuyerOrderReviewDraft,
    ) -> Result<(), AppSqliteError> {
        self.buyer_repository()
            .save_buyer_order_review_draft(context, draft)
    }

    pub fn place_buyer_order(&self, context: &BuyerContext) -> Result<OrderId, AppSqliteError> {
        self.buyer_repository().place_buyer_order(context)
    }

    pub fn load_buyer_orders(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerOrdersProjection, AppSqliteError> {
        self.buyer_repository().load_buyer_orders(context)
    }

    pub fn load_buyer_orders_for_scope(
        &self,
        scope: &SelectedBuyerOrderScope,
    ) -> Result<BuyerOrdersProjection, AppSqliteError> {
        self.buyer_repository().load_buyer_orders_for_scope(scope)
    }

    pub fn load_buyer_order_detail(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderDetailProjection>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_order_detail(context, order_id)
    }

    pub fn load_buyer_order_detail_for_scope(
        &self,
        scope: &SelectedBuyerOrderScope,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderDetailProjection>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_order_detail_for_scope(scope, order_id)
    }

    pub fn load_buyer_order_local_event_export(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderLocalEventExport>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_order_local_event_export(context, order_id)
    }

    pub fn load_buyer_order_coordination_record(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderCoordinationRecord>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_order_coordination_record(context, order_id)
    }

    pub fn load_recoverable_buyer_order_coordination_records(
        &self,
        context: &BuyerContext,
    ) -> Result<Vec<BuyerOrderCoordinationRecord>, AppSqliteError> {
        self.buyer_repository()
            .load_recoverable_buyer_order_coordination_records(context)
    }

    pub fn buyer_order_coordination_is_synced(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.buyer_repository()
            .buyer_order_coordination_is_synced(context, order_id)
    }

    pub fn prepare_buyer_order_coordination_attempt(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        record_id: &str,
        payload_json: &str,
    ) -> Result<bool, AppSqliteError> {
        self.buyer_repository()
            .prepare_buyer_order_coordination_attempt(context, order_id, record_id, payload_json)
    }

    pub fn mark_buyer_order_coordination_synced(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.buyer_repository()
            .mark_buyer_order_coordination_synced(context, order_id)
    }

    pub fn mark_buyer_order_coordination_failed(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        error_message: &str,
    ) -> Result<bool, AppSqliteError> {
        self.buyer_repository()
            .mark_buyer_order_coordination_failed(context, order_id, error_message)
    }

    pub fn apply_buyer_repeat_demand_to_cart(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
        replace_existing: bool,
    ) -> Result<BuyerRepeatDemandApplyOutcome, AppSqliteError> {
        self.buyer_repository().apply_buyer_repeat_demand_to_cart(
            context,
            order_id,
            replace_existing,
        )
    }

    pub fn apply_buyer_repeat_demand_from_scope_to_cart(
        &self,
        source_scope: &SelectedBuyerOrderScope,
        cart_context: &BuyerContext,
        order_id: OrderId,
        replace_existing: bool,
    ) -> Result<BuyerRepeatDemandApplyOutcome, AppSqliteError> {
        self.buyer_repository()
            .apply_buyer_repeat_demand_from_scope_to_cart(
                source_scope,
                cart_context,
                order_id,
                replace_existing,
            )
    }

    pub fn enqueue_pending_sync_operation(
        &self,
        account_id: &str,
        operation: &PendingSyncOperation,
    ) -> Result<String, AppSqliteError> {
        self.sync_repository()
            .enqueue_pending_operation(account_id, operation)
    }

    pub fn load_pending_sync_operations(
        &self,
        account_id: &str,
    ) -> Result<Vec<StoredPendingSyncOperation>, AppSqliteError> {
        self.sync_repository().load_pending_operations(account_id)
    }

    pub fn update_pending_sync_operation_retry(
        &self,
        account_id: &str,
        operation_id: &str,
        available_at: &str,
        attempt_count: u32,
        last_error_message: Option<&str>,
    ) -> Result<bool, AppSqliteError> {
        self.sync_repository().update_pending_operation_retry(
            account_id,
            operation_id,
            available_at,
            attempt_count,
            last_error_message,
        )
    }

    pub fn dequeue_pending_sync_operation(
        &self,
        account_id: &str,
        operation_id: &str,
    ) -> Result<bool, AppSqliteError> {
        self.sync_repository()
            .dequeue_pending_operation(account_id, operation_id)
    }

    pub fn load_sync_checkpoint(
        &self,
        account_id: &str,
    ) -> Result<SyncCheckpointStatus, AppSqliteError> {
        self.sync_repository().load_checkpoint(account_id)
    }

    pub fn save_sync_checkpoint(
        &self,
        account_id: &str,
        checkpoint: &SyncCheckpointStatus,
    ) -> Result<(), AppSqliteError> {
        self.sync_repository()
            .save_checkpoint(account_id, checkpoint)
    }

    pub fn load_relay_ingest_cursors(
        &self,
        scope_key: &str,
        relay_urls: &[String],
    ) -> Result<Vec<StoredRelayIngestCursor>, AppSqliteError> {
        self.sync_repository()
            .load_relay_ingest_cursors(scope_key, relay_urls)
    }

    pub fn load_relay_ingest_freshness(
        &self,
        scope_key: &str,
        relay_urls: &[String],
        now_unix_seconds: i64,
        stale_after_seconds: i64,
    ) -> Result<AppRelayIngestScopeFreshness, AppSqliteError> {
        self.sync_repository().load_relay_ingest_freshness(
            scope_key,
            relay_urls,
            now_unix_seconds,
            stale_after_seconds,
        )
    }

    pub fn record_relay_ingest_success(
        &self,
        scope_key: &str,
        relay_url: &str,
        cursor_since_unix_seconds: i64,
        last_event_created_at_unix_seconds: Option<i64>,
        started_at: &str,
        started_unix_seconds: i64,
        completed_at: &str,
        completed_unix_seconds: i64,
    ) -> Result<(), AppSqliteError> {
        self.sync_repository().record_relay_ingest_success(
            scope_key,
            relay_url,
            cursor_since_unix_seconds,
            last_event_created_at_unix_seconds,
            started_at,
            started_unix_seconds,
            completed_at,
            completed_unix_seconds,
        )
    }

    pub fn record_relay_ingest_failure(
        &self,
        scope_key: &str,
        relay_url: &str,
        started_at: &str,
        started_unix_seconds: i64,
        completed_at: &str,
        completed_unix_seconds: i64,
        error_message: &str,
    ) -> Result<(), AppSqliteError> {
        self.sync_repository().record_relay_ingest_failure(
            scope_key,
            relay_url,
            started_at,
            started_unix_seconds,
            completed_at,
            completed_unix_seconds,
            error_message,
        )
    }

    pub fn record_sync_conflict(
        &self,
        account_id: &str,
        conflict: &SyncConflict,
    ) -> Result<String, AppSqliteError> {
        self.sync_repository().record_conflict(account_id, conflict)
    }

    pub fn replace_sync_conflicts(
        &self,
        account_id: &str,
        conflicts: &[SyncConflict],
    ) -> Result<(), AppSqliteError> {
        self.sync_repository()
            .replace_conflicts(account_id, conflicts)
    }

    pub fn load_sync_conflicts(
        &self,
        account_id: &str,
    ) -> Result<Vec<StoredSyncConflict>, AppSqliteError> {
        self.sync_repository().load_conflicts(account_id)
    }

    pub fn resolve_sync_conflict(
        &self,
        account_id: &str,
        conflict_id: &str,
        resolution: SyncConflictResolutionStatus,
        resolved_at: &str,
    ) -> Result<bool, AppSqliteError> {
        self.sync_repository()
            .resolve_conflict(account_id, conflict_id, resolution, resolved_at)
    }
}

fn open_connection(target: &DatabaseTarget) -> Result<Connection, AppSqliteError> {
    match target {
        DatabaseTarget::InMemory => {
            Connection::open_in_memory().map_err(|source| AppSqliteError::OpenInMemory { source })
        }
        DatabaseTarget::Path(path) => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|source| {
                        AppSqliteError::CreateParentDirectory {
                            path: parent.to_path_buf(),
                            source,
                        }
                    })?;
                }
            }

            Connection::open(path).map_err(|source| AppSqliteError::OpenPath {
                path: path.clone(),
                source,
            })
        }
    }
}

fn bootstrap_connection(
    connection: &mut Connection,
    target: &DatabaseTarget,
) -> Result<(), AppSqliteError> {
    connection
        .busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
        .map_err(|source| AppSqliteError::ConfigureBusyTimeout { source })?;

    apply_pragma(connection, "foreign_keys", "ON")?;
    apply_pragma(connection, "synchronous", "NORMAL")?;

    if matches!(target, DatabaseTarget::Path(_)) {
        connection
            .query_row("PRAGMA journal_mode = WAL", [], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|source| AppSqliteError::ApplyPragma {
                pragma: "journal_mode",
                source,
            })?;
    }

    apply_migrations(connection)
}

fn apply_pragma(
    connection: &Connection,
    pragma: &'static str,
    value: &str,
) -> Result<(), AppSqliteError> {
    let sql = format!("PRAGMA {pragma} = {value}");
    connection
        .execute_batch(&sql)
        .map_err(|source| AppSqliteError::ApplyPragma { pragma, source })
}

fn schema_version(connection: &Connection) -> Result<u32, AppSqliteError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|source| AppSqliteError::ReadSchemaVersion { source })
}

fn apply_migrations(connection: &mut Connection) -> Result<(), AppSqliteError> {
    let current_version = schema_version(connection)?;
    let latest_version = migrations::latest_schema_version();

    if current_version > latest_version {
        return Err(AppSqliteError::UnsupportedSchemaVersion {
            current: current_version,
            latest: latest_version,
        });
    }

    for (version, sql) in migrations::pending_migrations(current_version) {
        let transaction = connection
            .transaction()
            .map_err(|source| AppSqliteError::BeginMigration { version, source })?;

        transaction
            .execute_batch(sql)
            .map_err(|source| AppSqliteError::ExecuteMigration { version, source })?;
        transaction
            .pragma_update(None, "user_version", version)
            .map_err(|source| AppSqliteError::RecordSchemaVersion { version, source })?;
        transaction
            .commit()
            .map_err(|source| AppSqliteError::CommitMigration { version, source })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppSqliteStore, DatabaseTarget, latest_schema_version, migrations};
    use rusqlite::{Connection, params};
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn file_store_bootstrap_applies_pragmas_and_migrations() {
        let path = temp_database_path("bootstrap");
        let store =
            AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("store should open");
        let connection = store.connection();

        assert_eq!(
            store.schema_version().expect("schema version"),
            latest_schema_version()
        );
        assert_eq!(pragma_i64(connection, "foreign_keys"), 1);
        assert_eq!(pragma_text(connection, "journal_mode"), "wal");
        assert!(table_exists(connection, "farms"));
        assert!(table_exists(connection, "products"));
        assert!(table_exists(connection, "orders"));
        assert!(table_exists(connection, "local_outbox"));
        assert!(table_exists(connection, "local_conflicts"));
        assert!(table_exists(connection, "sync_checkpoints"));
        assert!(table_exists(connection, "app_relay_ingest_freshness"));
        assert!(table_exists(connection, "activity_events"));
        assert!(table_exists(connection, "account_surface_activations"));
        assert!(table_exists(connection, "account_farm_setups"));
        assert!(table_exists(connection, "farm_operating_rules"));
        assert!(table_exists(connection, "pickup_locations"));
        assert!(table_exists(connection, "blackout_periods"));
        assert!(table_exists(connection, "order_lines"));
        assert!(table_exists(connection, "buyer_carts"));
        assert!(table_exists(connection, "buyer_cart_lines"));
        assert!(table_exists(connection, "reminder_schedules"));
        assert!(table_exists(connection, "reminder_log_entries"));
        assert!(table_exists(connection, "order_recovery_records"));
        assert!(table_exists(connection, "buyer_order_coordination_records"));
        assert!(table_exists(connection, "order_validation_receipts"));
        assert!(column_exists(connection, "farms", "timezone"));
        assert!(column_exists(connection, "farms", "currency_code"));
        assert!(column_exists(connection, "local_outbox", "account_id"));
        assert!(column_exists(connection, "local_outbox", "operation_key"));
        assert!(column_exists(connection, "local_outbox", "state"));
        assert!(column_exists(
            connection,
            "local_outbox",
            "last_error_message"
        ));
        assert!(column_exists(connection, "local_conflicts", "account_id"));
        assert!(column_exists(connection, "local_conflicts", "severity"));
        assert!(column_exists(
            connection,
            "local_conflicts",
            "resolution_status"
        ));
        assert!(column_exists(connection, "sync_checkpoints", "account_id"));
        assert!(column_exists(connection, "sync_checkpoints", "state"));
        assert!(column_exists(
            connection,
            "app_relay_ingest_freshness",
            "scope_key"
        ));
        assert!(column_exists(
            connection,
            "app_relay_ingest_freshness",
            "relay_url"
        ));
        assert!(column_exists(
            connection,
            "app_relay_ingest_freshness",
            "cursor_since_unix_seconds"
        ));
        assert!(column_exists(
            connection,
            "fulfillment_windows",
            "pickup_location_id"
        ));
        assert!(column_exists(connection, "fulfillment_windows", "label"));
        assert!(column_exists(
            connection,
            "fulfillment_windows",
            "order_cutoff_at"
        ));
        assert!(column_exists(connection, "order_lines", "quantity_value"));
        assert!(column_exists(
            connection,
            "order_lines",
            "quantity_unit_label"
        ));
        assert!(column_exists(connection, "order_lines", "quantity_display"));
        assert!(column_exists(connection, "order_lines", "listing_bin_id"));
        assert!(column_exists(
            connection,
            "order_lines",
            "unit_price_minor_units"
        ));
        assert!(column_exists(connection, "order_lines", "price_currency"));
        assert!(column_exists(connection, "order_lines", "listing_addr"));
        assert!(column_exists(
            connection,
            "order_lines",
            "listing_relays_json"
        ));
        assert!(column_exists(connection, "products", "category"));
        assert!(column_exists(connection, "products", "listing_bin_id"));
        assert!(column_exists(connection, "buyer_carts", "buyer_email"));
        assert!(column_exists(connection, "buyer_carts", "buyer_phone"));
        assert!(column_exists(connection, "buyer_carts", "buyer_order_note"));
        assert!(column_exists(
            connection,
            "buyer_cart_lines",
            "listing_bin_id"
        ));
        assert!(column_exists(
            connection,
            "buyer_cart_lines",
            "quantity_unit_label"
        ));
        assert!(column_exists(
            connection,
            "buyer_cart_lines",
            "unit_price_minor_units"
        ));
        assert!(column_exists(connection, "buyer_cart_lines", "farm_key"));
        assert!(column_exists(
            connection,
            "buyer_cart_lines",
            "listing_event_id"
        ));
        assert!(column_exists(
            connection,
            "buyer_cart_lines",
            "listing_relays_json"
        ));
        assert!(column_exists(connection, "orders", "buyer_context_key"));
        assert!(column_exists(connection, "orders", "buyer_email"));
        assert!(column_exists(connection, "orders", "buyer_phone"));
        assert!(column_exists(connection, "orders", "buyer_order_note"));
        assert!(column_exists(
            connection,
            "reminder_schedules",
            "account_id"
        ));
        assert!(column_exists(
            connection,
            "reminder_schedules",
            "delivery_state"
        ));
        assert!(column_exists(
            connection,
            "reminder_log_entries",
            "recorded_at"
        ));
        assert!(column_exists(
            connection,
            "order_recovery_records",
            "recovery_kind"
        ));
        assert!(column_exists(
            connection,
            "buyer_order_coordination_records",
            "state"
        ));
        assert!(column_exists(
            connection,
            "buyer_order_coordination_records",
            "payload_json"
        ));
        assert!(column_exists(
            connection,
            "buyer_order_coordination_records",
            "last_error_message"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "event_id"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "order_id"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "raw_order_id"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "root_event_id"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "target_event_id"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "result"
        ));
        assert!(column_exists(
            connection,
            "order_validation_receipts",
            "proof_system"
        ));
        assert!(column_exists(
            connection,
            "order_recovery_records",
            "recovery_state"
        ));
        connection
            .execute(
                "INSERT INTO local_interop_imports (
                    record_id,
                    local_seq,
                    record_family,
                    local_status,
                    source_runtime,
                    projected_kind,
                    outbox_status,
                    imported_at
                 ) VALUES (
                    'schema_validation_receipt_projection_kind',
                    0,
                    'signed_event',
                    'published',
                    'cli',
                    'validation_receipt',
                    'acknowledged',
                    '2026-01-01T00:00:00Z'
                 )",
                [],
            )
            .expect("local interop imports should accept validation receipt projections");
        assert_eq!(row_count(connection, "sync_checkpoints"), 0);

        drop(store);
        remove_database_artifacts(&path);
    }

    #[test]
    fn reopening_existing_store_is_idempotent() {
        let path = temp_database_path("reopen");
        AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("first open should work");
        let reopened = AppSqliteStore::open(DatabaseTarget::Path(path.clone()))
            .expect("second open should work");

        assert_eq!(
            reopened.schema_version().expect("schema version"),
            latest_schema_version()
        );
        assert_eq!(row_count(reopened.connection(), "sync_checkpoints"), 0);

        drop(reopened);
        remove_database_artifacts(&path);
    }

    #[test]
    fn in_memory_store_bootstraps_without_file_only_pragmas() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");

        assert_eq!(
            store.schema_version().expect("schema version"),
            latest_schema_version()
        );
        assert_eq!(pragma_i64(store.connection(), "foreign_keys"), 1);
        assert!(table_exists(store.connection(), "farms"));
    }

    #[test]
    fn workflow_payment_display_schema_accepts_pending_and_settled_states() {
        let store = AppSqliteStore::open(DatabaseTarget::InMemory).expect("store should open");
        let connection = store.connection();
        connection
            .execute(
                "INSERT INTO farms (id, display_name, readiness, created_at, updated_at)
                 VALUES (?1, 'Schema Farm', 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                params!["farm_schema"],
            )
            .expect("farm should insert");

        for (order_id, workflow_payment) in [
            ("order_payment_pending", "pending"),
            ("order_payment_settled", "settled"),
        ] {
            connection
                .execute(
                    "INSERT INTO orders (
                        id,
                        farm_id,
                        order_number,
                        customer_display_name,
                        status,
                        updated_at,
                        workflow_payment
                     ) VALUES (?1, 'farm_schema', ?2, 'Buyer', 'scheduled', '2026-01-01T00:00:00Z', ?3)",
                    params![order_id, order_id, workflow_payment],
                )
                .expect("expanded workflow payment state should insert");
        }

        connection
            .execute(
                "INSERT INTO orders (
                    id,
                    farm_id,
                    order_number,
                    customer_display_name,
                    status,
                    updated_at,
                    workflow_receipt_event_id,
                    workflow_receipt_received,
                    workflow_receipt_issue,
                    workflow_receipt_received_at
                 ) VALUES (
                    'order_issue_receipt',
                    'farm_schema',
                    'issue receipt',
                    'Buyer',
                    'needs_review',
                    '2026-01-01T00:00:00Z',
                    'receipt-event-1',
                    0,
                    'items need review',
                    1777665700
                 )",
                [],
            )
            .expect("receipt projection should insert");

        let invalid_result = connection.execute(
            "INSERT INTO orders (
                id,
                farm_id,
                order_number,
                customer_display_name,
                status,
                updated_at,
                workflow_payment
             ) VALUES ('order_payment_invalid', 'farm_schema', 'invalid', 'Buyer', 'scheduled', '2026-01-01T00:00:00Z', 'collect')",
            [],
        );
        assert!(invalid_result.is_err());

        let invalid_receipt_result = connection.execute(
            "INSERT INTO orders (
                id,
                farm_id,
                order_number,
                customer_display_name,
                status,
                updated_at,
                workflow_receipt_event_id,
                workflow_receipt_received,
                workflow_receipt_received_at
             ) VALUES (
                'order_receipt_invalid',
                'farm_schema',
                'invalid receipt',
                'Buyer',
                'needs_review',
                '2026-01-01T00:00:00Z',
                'receipt-event-invalid',
                2,
                1777665700
             )",
            [],
        );
        assert!(invalid_receipt_result.is_err());
    }

    #[test]
    fn legacy_sync_scaffolding_migrates_to_account_scoped_contract() {
        let path = temp_database_path("legacy-sync-contract");
        fs::create_dir_all(path.parent().expect("temp database should have a parent"))
            .expect("legacy database parent should exist");
        let connection = Connection::open(&path).expect("legacy database should open");

        for (version, sql) in migrations::pending_migrations(0)
            .filter(|(version, _)| *version < latest_schema_version())
        {
            connection
                .execute_batch(sql)
                .expect("legacy migration should apply");
            connection
                .pragma_update(None, "user_version", version)
                .expect("legacy schema version should record");
        }

        drop(connection);

        let store =
            AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("store should open");
        let connection = store.connection();

        assert_eq!(
            store.schema_version().expect("schema version"),
            latest_schema_version()
        );
        assert!(column_exists(connection, "local_outbox", "account_id"));
        assert!(column_exists(connection, "local_outbox", "operation_key"));
        assert!(column_exists(connection, "local_outbox", "state"));
        assert!(column_exists(connection, "local_conflicts", "severity"));
        assert!(column_exists(
            connection,
            "local_conflicts",
            "resolution_status"
        ));
        assert!(column_exists(connection, "sync_checkpoints", "state"));
        assert!(table_exists(connection, "app_relay_ingest_freshness"));
        assert_eq!(row_count(connection, "sync_checkpoints"), 0);

        drop(store);
        remove_database_artifacts(&path);
    }

    #[test]
    fn legacy_orders_status_migration_preserves_child_rows_and_accepts_declined() {
        let path = temp_database_path("legacy-declined-orders");
        fs::create_dir_all(path.parent().expect("temp database should have a parent"))
            .expect("legacy database parent should exist");
        let connection = Connection::open(&path).expect("legacy database should open");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("foreign keys should enable");

        for (version, sql) in migrations::pending_migrations(0).filter(|(version, _)| *version < 20)
        {
            connection
                .execute_batch(sql)
                .expect("legacy migration should apply");
            connection
                .pragma_update(None, "user_version", version)
                .expect("legacy schema version should record");
        }

        connection
            .execute(
                "INSERT INTO farms (id, display_name, readiness, created_at, updated_at)
                 VALUES (?1, 'Legacy Farm', 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                params!["farm_legacy"],
            )
            .expect("legacy farm should insert");
        connection
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
                 ) VALUES (
                    'order_legacy',
                    'farm_legacy',
                    NULL,
                    'R-900',
                    'Legacy Buyer',
                    'needs_action',
                    '2026-01-01T00:00:00Z',
                    'account:buyer',
                    '',
                    '',
                    ''
                 )",
                [],
            )
            .expect("legacy order should insert");
        connection
            .execute(
                "INSERT INTO order_lines (
                    id,
                    order_id,
                    title,
                    quantity_value,
                    quantity_display
                 ) VALUES (
                    'line_legacy',
                    'order_legacy',
                    'Legacy Eggs',
                    2,
                    '2 each'
                 )",
                [],
            )
            .expect("legacy order line should insert");
        connection
            .execute(
                "INSERT INTO buyer_order_coordination_records (
                    order_id,
                    buyer_context_key,
                    state,
                    created_at,
                    updated_at
                 ) VALUES (
                    'order_legacy',
                    'account:buyer',
                    'pending',
                    '2026-01-01T00:00:00Z',
                    '2026-01-01T00:00:00Z'
                 )",
                [],
            )
            .expect("legacy buyer coordination should insert");

        drop(connection);

        let store =
            AppSqliteStore::open(DatabaseTarget::Path(path.clone())).expect("store should open");
        let connection = store.connection();

        assert_eq!(
            store.schema_version().expect("schema version"),
            latest_schema_version()
        );
        assert_eq!(row_count(connection, "orders"), 1);
        assert_eq!(row_count(connection, "order_lines"), 1);
        assert_eq!(row_count(connection, "buyer_order_coordination_records"), 1);
        assert_eq!(foreign_key_violation_count(connection), 0);

        connection
            .execute(
                "UPDATE orders SET status = 'declined' WHERE id = 'order_legacy'",
                [],
            )
            .expect("declined status should satisfy migrated check");
        connection
            .execute(
                "UPDATE orders SET status = 'needs_review' WHERE id = 'order_legacy'",
                [],
            )
            .expect("needs review status should satisfy migrated check");

        let status: String = connection
            .query_row(
                "SELECT status FROM orders WHERE id = 'order_legacy'",
                [],
                |row| row.get(0),
            )
            .expect("status should load");
        assert_eq!(status, "needs_review");

        drop(store);
        remove_database_artifacts(&path);
    }

    fn table_exists(connection: &Connection, table_name: &str) -> bool {
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table_name],
                |row| row.get::<_, i64>(0),
            )
            .expect("table existence query should succeed")
            == 1
    }

    fn row_count(connection: &Connection, table_name: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("row count query should succeed")
    }

    fn column_exists(connection: &Connection, table_name: &str, column_name: &str) -> bool {
        let sql = format!("PRAGMA table_info({table_name})");
        let mut statement = connection
            .prepare(&sql)
            .expect("table info statement should prepare");
        let mut rows = statement
            .query([])
            .expect("table info query should succeed");

        while let Some(row) = rows.next().expect("table info row should load") {
            if row
                .get::<_, String>(1)
                .expect("table info name should load")
                == column_name
            {
                return true;
            }
        }

        false
    }

    fn foreign_key_violation_count(connection: &Connection) -> usize {
        let mut statement = connection
            .prepare("PRAGMA foreign_key_check")
            .expect("foreign key check should prepare");
        let mut rows = statement.query([]).expect("foreign key check should run");
        let mut count = 0;
        while rows
            .next()
            .expect("foreign key check row should load")
            .is_some()
        {
            count += 1;
        }
        count
    }

    fn pragma_i64(connection: &Connection, pragma_name: &str) -> i64 {
        let sql = format!("PRAGMA {pragma_name}");
        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("pragma query should succeed")
    }

    fn pragma_text(connection: &Connection, pragma_name: &str) -> String {
        let sql = format!("PRAGMA {pragma_name}");
        connection
            .query_row(&sql, [], |row| row.get(0))
            .expect("pragma query should succeed")
    }

    fn temp_database_path(test_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();

        env::temp_dir()
            .join("radroots_studio_app_sqlite_tests")
            .join(format!("{test_name}-{nonce}"))
            .join("app.sqlite3")
    }

    fn remove_database_artifacts(database_path: &std::path::Path) {
        if let Some(parent) = database_path.parent() {
            let wal_path = database_path.with_extension("sqlite3-wal");
            let shm_path = database_path.with_extension("sqlite3-shm");

            let _ = fs::remove_file(&wal_path);
            let _ = fs::remove_file(&shm_path);
            let _ = fs::remove_file(database_path);
            let _ = fs::remove_dir_all(parent);
        }
    }
}
