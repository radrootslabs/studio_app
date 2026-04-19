#![forbid(unsafe_code)]

mod activation;
mod activity;
mod error;
mod farm_rules;
mod farm_setup;
mod migrations;
mod orders;
mod products;
mod today;

use std::{fs, path::PathBuf, time::Duration};

use radroots_studio_app_models::{
    AccountSurfaceActivationProjection, AppActivityContext, AppActivityEvent, AppActivityKind,
    FarmId, FarmRulesProjection, FarmSetupProjection, FarmSummary, OrderDetailProjection, OrderId,
    OrdersListProjection, OrdersScreenQueryState, PackDayProjection, PackDayScreenQueryState,
    ProductEditorDraft, ProductId, ProductPublishBlocker, ProductsFilter, ProductsListProjection,
    ProductsSort, TodayAgendaProjection,
};
use rusqlite::Connection;

pub use activation::AppActivationRepository;
pub use activity::{
    APP_ACTIVITY_CONTEXT_LIMIT, APP_ACTIVITY_RETENTION_LIMIT, AppActivityRepository,
};
pub use error::AppSqliteError;
pub use farm_rules::{AppFarmRulesRepository, derive_farm_rules_readiness};
pub use farm_setup::AppFarmSetupRepository;
pub use migrations::latest_schema_version;
pub use orders::AppOrdersRepository;
pub use products::AppProductsRepository;
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

    pub fn products_repository(&self) -> AppProductsRepository<'_> {
        AppProductsRepository::new(&self.connection)
    }

    pub fn orders_repository(&self) -> AppOrdersRepository<'_> {
        AppOrdersRepository::new(&self.connection)
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
    use super::{AppSqliteStore, DatabaseTarget, latest_schema_version};
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
        assert!(column_exists(connection, "farms", "timezone"));
        assert!(column_exists(connection, "farms", "currency_code"));
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
        assert_eq!(row_count(connection, "sync_checkpoints"), 1);

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
        assert_eq!(row_count(reopened.connection(), "sync_checkpoints"), 1);

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
