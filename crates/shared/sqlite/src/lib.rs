#![forbid(unsafe_code)]

mod activation;
mod activity;
mod buyer;
mod error;
mod farm_rules;
mod farm_setup;
mod local_interop;
mod migrations;
mod orders;
mod products;
mod reminders;
mod sync;
mod today;

use std::{collections::BTreeSet, fs, path::PathBuf, time::Duration};

use radroots_studio_app_models::{
    AccountSurfaceActivationProjection, AppActivityContext, AppActivityEvent, AppActivityKind,
    BuyerCartProjection, BuyerCheckoutDraft, BuyerCheckoutProjection, BuyerContext,
    BuyerListingsProjection, BuyerOrderDetailProjection, BuyerOrdersProjection,
    BuyerProductDetailProjection, FarmId, FarmOrderMethod, FarmRulesProjection,
    FarmSetupProjection, FarmSummary, FulfillmentWindowId, OrderDetailProjection, OrderId,
    OrderRecoveryProjection, OrdersListProjection, OrdersScreenQueryState, PackDayOutputSource,
    PackDayProjection, PackDayScreenQueryState, ProductEditorDraft, ProductId,
    ProductPublishBlocker, ProductsFilter, ProductsListProjection, ProductsSort, RecoveryKind,
    RecoveryQueueProjection, ReminderFeedProjection, ReminderLogEntryProjection,
    ReminderLogProjection, TodayAgendaProjection,
};
use radroots_studio_app_sync::{
    PendingSyncOperation, SyncCheckpointStatus, SyncConflict, SyncConflictResolutionStatus,
};
use rusqlite::Connection;

pub use activation::AppActivationRepository;
pub use activity::{
    APP_ACTIVITY_CONTEXT_LIMIT, APP_ACTIVITY_RETENTION_LIMIT, AppActivityRepository,
};
pub use buyer::{
    AppBuyerRepository, BuyerOrderCoordinationRecord, BuyerOrderCoordinationState,
    BuyerOrderLocalEventExport, BuyerOrderLocalEventLine, BuyerRepeatDemandApplyOutcome,
};
pub use error::AppSqliteError;
pub use farm_rules::{AppFarmRulesRepository, derive_farm_rules_readiness};
pub use farm_setup::AppFarmSetupRepository;
pub use local_interop::{
    AppLocalInteropImportReport, AppLocalInteropRepository, StoredLocalInteropRecord,
};
pub use migrations::latest_schema_version;
pub use orders::AppOrdersRepository;
pub use products::AppProductsRepository;
pub use reminders::AppRemindersRepository;
pub use sync::{AppSyncRepository, StoredPendingSyncOperation, StoredSyncConflict};
pub use today::{
    AppTodayAgendaRepository, TODAY_AGENDA_LIST_LIMIT, TODAY_AGENDA_LOW_STOCK_THRESHOLD,
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

    pub fn mark_order_packed(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.orders_repository()
            .mark_order_packed(farm_id, order_id)
    }

    pub fn mark_order_completed(
        &self,
        farm_id: FarmId,
        order_id: OrderId,
    ) -> Result<bool, AppSqliteError> {
        self.orders_repository()
            .mark_order_completed(farm_id, order_id)
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

    pub fn load_buyer_checkout(
        &self,
        context: &BuyerContext,
    ) -> Result<BuyerCheckoutProjection, AppSqliteError> {
        self.buyer_repository().load_buyer_checkout(context)
    }

    pub fn save_buyer_checkout_draft(
        &self,
        context: &BuyerContext,
        draft: &BuyerCheckoutDraft,
    ) -> Result<(), AppSqliteError> {
        self.buyer_repository()
            .save_buyer_checkout_draft(context, draft)
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

    pub fn load_buyer_order_detail(
        &self,
        context: &BuyerContext,
        order_id: OrderId,
    ) -> Result<Option<BuyerOrderDetailProjection>, AppSqliteError> {
        self.buyer_repository()
            .load_buyer_order_detail(context, order_id)
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
    ) -> Result<bool, AppSqliteError> {
        self.sync_repository().update_pending_operation_retry(
            account_id,
            operation_id,
            available_at,
            attempt_count,
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
    use rusqlite::Connection;
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
        assert!(column_exists(connection, "farms", "timezone"));
        assert!(column_exists(connection, "farms", "currency_code"));
        assert!(column_exists(connection, "local_outbox", "account_id"));
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
            "order_recovery_records",
            "recovery_state"
        ));
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
        assert!(column_exists(connection, "local_conflicts", "severity"));
        assert!(column_exists(
            connection,
            "local_conflicts",
            "resolution_status"
        ));
        assert!(column_exists(connection, "sync_checkpoints", "state"));
        assert_eq!(row_count(connection, "sync_checkpoints"), 0);

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
