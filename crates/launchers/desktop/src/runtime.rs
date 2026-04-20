use std::collections::BTreeSet;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use radroots_studio_app_core::{AppDesktopRuntimePaths, AppRuntimePathsError, AppSharedAccountsPaths};
use radroots_studio_app_models::{
    ActiveSurface, AppActivityContext, AppActivityKind, AppIdentityProjection, AppStartupGate,
    FarmId, FarmOrderMethod, FarmProfileRecord, FarmReadiness, FarmRulesProjection, FarmSetupDraft,
    FarmSetupProjection, FarmSummary, FarmerSection, FulfillmentWindowId,
    LoggedOutStartupProjection, OrderDetailProjection, OrderId, OrdersFilter, OrdersListProjection,
    OrdersScreenQueryState, PackDayProjection, PackDayScreenQueryState, PersonalSection,
    PickupLocationRecord, ProductEditorDraft, ProductId, ProductsFilter, ProductsListProjection,
    ProductsSort, SettingsAccountProjection, SettingsPreference, SettingsSection, ShellSection,
    TodayAgendaProjection,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
};
use radroots_studio_app_sqlite::{
    APP_ACTIVITY_CONTEXT_LIMIT, AppSqliteError, AppSqliteStore, DatabaseTarget,
    derive_farm_rules_readiness,
};
use radroots_studio_app_state::{
    AppShellProjection, AppStateCommand, AppStateStore, AppStateStoreError,
    BuyerBrowseScreenProjection, BuyerCartScreenProjection, BuyerOrdersScreenProjection,
    BuyerSearchScreenProjection, BuyerSearchScreenQueryState, FarmSetupFlowStage,
    FarmWorkspaceReadinessProjection, HomeRoute, InMemoryAppStateRepository,
    OrdersScreenProjection, PackDayScreenProjection, PersonalWorkspaceProjection,
    ProductsScreenProjection, ProductsScreenQueryState,
};
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
use thiserror::Error;
use tracing::error;

use crate::accounts::{
    DesktopAccountsBootstrapError, DesktopAccountsCommandError, DesktopAccountsProjectionError,
    DesktopLocalIdentityImportRequest, bootstrap_desktop_accounts, generate_local_account,
    identity_projection_from_manager, import_local_account, remove_selected_local_key,
    reset_local_device_state, select_active_surface, select_local_account,
};
use crate::remote_signer::{
    DesktopRemoteSignerError, DesktopRemoteSignerPaths, activate_pending_session,
    apply_remote_signer_custody, clear_pending_session, load_pending_session, purge_all_state,
    reconcile_startup, store_pending_session,
};

const APP_DATABASE_FILE_NAME: &str = "app.sqlite3";

#[derive(Clone, Debug)]
pub struct DesktopAppRuntime {
    state: Arc<Mutex<DesktopAppRuntimeState>>,
}

impl DesktopAppRuntime {
    pub fn bootstrap(default_nostr_relay_url: String) -> Self {
        let state = match DesktopAppRuntimeState::try_bootstrap(default_nostr_relay_url) {
            Ok(state) => state,
            Err(error) => DesktopAppRuntimeState::degraded(error),
        };

        Self::from_state(state)
    }

    pub fn summary(&self) -> DesktopAppRuntimeSummary {
        let state = self.lock_state();

        DesktopAppRuntimeSummary {
            shell_projection: state.state_store.shell_projection().clone(),
            settings_account_projection: state.state_store.settings_account_projection(),
            startup_gate: state.state_store.startup_gate(),
            logged_out_startup: state.state_store.logged_out_startup_projection().clone(),
            home_route: state.state_store.home_route(),
            personal_projection: state.state_store.personal_projection().clone(),
            farm_setup_projection: state.state_store.farm_setup_projection().clone(),
            farm_readiness_projection: state.state_store.farm_readiness_projection().clone(),
            today_projection: state.state_store.today_projection().clone(),
            products_projection: state.state_store.products_projection().clone(),
            orders_projection: state.state_store.orders_projection().clone(),
            pack_day_projection: state.state_store.pack_day_projection().clone(),
            startup_issue: state.startup_issue.clone(),
        }
    }

    pub fn default_nostr_relay_url(&self) -> String {
        self.lock_state().default_nostr_relay_url.clone()
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

    pub fn select_personal_section(&self, section: PersonalSection) -> bool {
        self.lock_state_mut().select_personal_section(section)
    }

    pub fn select_farmer_section(&self, section: FarmerSection) -> bool {
        self.lock_state_mut().select_farmer_section(section)
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

    pub fn mark_order_packed(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().mark_order_packed(order_id)
    }

    pub fn mark_order_completed(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().mark_order_completed(order_id)
    }

    pub fn open_pack_day(
        &self,
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().open_pack_day(fulfillment_window_id)
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

#[derive(Clone, Debug)]
pub struct DesktopAppRuntimeSummary {
    pub shell_projection: AppShellProjection,
    pub settings_account_projection: SettingsAccountProjection,
    pub startup_gate: AppStartupGate,
    pub logged_out_startup: LoggedOutStartupProjection,
    pub home_route: HomeRoute,
    pub personal_projection: PersonalWorkspaceProjection,
    pub farm_setup_projection: FarmSetupProjection,
    pub farm_readiness_projection: FarmWorkspaceReadinessProjection,
    pub today_projection: TodayAgendaProjection,
    pub products_projection: ProductsScreenProjection,
    pub orders_projection: OrdersScreenProjection,
    pub pack_day_projection: PackDayScreenProjection,
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
    products_list: ProductsListProjection,
    orders_list: OrdersListProjection,
    order_detail: Option<OrderDetailProjection>,
    pack_day_projection: PackDayProjection,
}

struct DesktopAppRuntimeState {
    state_store: AppStateStore<InMemoryAppStateRepository>,
    default_nostr_relay_url: String,
    shared_accounts_paths: Option<AppSharedAccountsPaths>,
    remote_signer_paths: Option<DesktopRemoteSignerPaths>,
    accounts_manager: Option<RadrootsNostrAccountsManager>,
    sqlite_store: Option<AppSqliteStore>,
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
            .field("startup_issue", &self.startup_issue)
            .finish()
    }
}

impl DesktopAppRuntimeState {
    fn try_bootstrap(
        default_nostr_relay_url: String,
    ) -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let paths = AppDesktopRuntimePaths::current_desktop()?;
        Self::bootstrap_from_paths(paths, default_nostr_relay_url)
    }

    fn bootstrap_from_paths(
        paths: AppDesktopRuntimePaths,
        default_nostr_relay_url: String,
    ) -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let database_path = paths.app.data.join(APP_DATABASE_FILE_NAME);
        let sqlite_store = AppSqliteStore::open(DatabaseTarget::Path(database_path.clone()))?;
        let mut state_store = AppStateStore::load(InMemoryAppStateRepository::default())?;
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
        let selected_account_context = load_selected_account_context(
            &sqlite_store,
            &identity_projection,
            state_store.products_projection().query.clone(),
            state_store.orders_projection().query.clone(),
            state_store
                .orders_projection()
                .detail
                .as_ref()
                .map(|detail| detail.order_id),
            state_store.pack_day_projection().query.clone(),
        )?;
        let _ = state_store.apply_in_memory(AppStateCommand::replace_identity_projection(
            identity_projection.clone(),
        ));
        if identity_projection.startup_gate() == AppStartupGate::SetupRequired
            && load_pending_session(&remote_signer_paths)?.is_some()
        {
            let _ = state_store.apply_in_memory(AppStateCommand::show_startup_signer_entry());
        }
        let _ = state_store.apply_in_memory(AppStateCommand::replace_personal_projection(
            selected_account_context.personal_projection.clone(),
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_farm_rules_projection(
            selected_account_context.farm_rules_projection,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_farm_setup_projection(
            selected_account_context.farm_setup_projection,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_today_agenda(
            selected_account_context.today_projection,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_products_list(
            selected_account_context.products_list,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_orders_list(
            selected_account_context.orders_list,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_order_detail(
            selected_account_context.order_detail,
        ));
        let _ = state_store.apply_in_memory(AppStateCommand::replace_pack_day_projection(
            selected_account_context.pack_day_projection,
        ));

        Ok(Self {
            state_store,
            default_nostr_relay_url,
            shared_accounts_paths: Some(paths.shared_accounts.clone()),
            remote_signer_paths: Some(remote_signer_paths),
            accounts_manager: accounts_bootstrap.accounts_manager,
            sqlite_store: Some(sqlite_store),
            startup_issue: None,
        })
    }

    fn degraded(error: DesktopAppRuntimeBootstrapError) -> Self {
        Self {
            state_store: AppStateStore::in_memory(AppShellProjection::default()),
            default_nostr_relay_url: String::new(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: None,
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

    fn select_personal_section(&mut self, section: PersonalSection) -> bool {
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Personal(
                section,
            )));
        let editor_changed = self.close_product_editor();

        section_changed || editor_changed
    }

    fn set_personal_search_query(&mut self, search_query: &str) -> Result<bool, AppSqliteError> {
        let query = self.state_store.personal_projection().search.query.clone();
        if query.search_query == search_query {
            return Ok(false);
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
        let detail_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_order_detail(None));
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                FarmerSection::Orders,
            )));
        let editor_changed = self.close_product_editor();

        Ok(query_changed || detail_changed || section_changed || editor_changed)
    }

    fn open_order_detail(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };
        let Some(order_detail) = sqlite_store.load_order_detail(farm_id, order_id)? else {
            return Ok(false);
        };

        let detail_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_order_detail(Some(order_detail)));
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Farmer(
                FarmerSection::Orders,
            )));
        let editor_changed = self.close_product_editor();

        Ok(detail_changed || section_changed || editor_changed)
    }

    fn mark_order_packed(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };

        let updated = sqlite_store.mark_order_packed(farm_id, order_id)?;
        if !updated {
            return Ok(false);
        }

        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);

        Ok(updated || context_changed)
    }

    fn mark_order_completed(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(false);
        };

        let updated = sqlite_store.mark_order_completed(farm_id, order_id)?;
        if !updated {
            return Ok(false);
        }

        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);

        Ok(updated || context_changed)
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

    fn update_product_stock(
        &mut self,
        product_id: ProductId,
        stock_quantity: u32,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
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

        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);

        Ok(updated || context_changed)
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
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
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
            load_selected_account_context(
                sqlite_store,
                self.state_store.identity_projection(),
                self.state_store.products_projection().query.clone(),
                self.state_store.orders_projection().query.clone(),
                self.selected_order_detail_id(),
                self.state_store.pack_day_projection().query.clone(),
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
        let editor_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_product_editor_draft(
                    reloaded_draft,
                ));

        Ok(saved || context_changed || editor_changed)
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
        let account = self.selected_account_for_farm_setup()?;
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

        let selected_account_context = self.refresh_selected_account_context()?;
        self.apply_selected_account_context(&selected_account_context);

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
            load_selected_account_context(
                sqlite_store,
                self.state_store.identity_projection(),
                self.state_store.products_projection().query.clone(),
                self.state_store.orders_projection().query.clone(),
                self.selected_order_detail_id(),
                self.state_store.pack_day_projection().query.clone(),
            )?
        };
        self.apply_selected_account_context(&selected_account_context);

        Ok(saved_projection)
    }

    fn replace_identity_projection(
        &mut self,
        projection: AppIdentityProjection,
    ) -> Result<bool, DesktopAppRuntimeCommandError> {
        let projection = self.decorate_identity_projection(projection)?;
        let selected_account_context = load_selected_account_context(
            self.sqlite_store()?,
            &projection,
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
        )?;
        let identity_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_identity_projection(projection));
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let editor_changed = self.close_product_editor();

        Ok(identity_changed || context_changed || editor_changed)
    }

    fn refresh_selected_account_context(
        &self,
    ) -> Result<DesktopSelectedAccountContext, DesktopAppRuntimeFarmSetupError> {
        Ok(load_selected_account_context(
            self.sqlite_store_for_farm_setup()?,
            self.state_store.identity_projection(),
            self.state_store.products_projection().query.clone(),
            self.state_store.orders_projection().query.clone(),
            self.selected_order_detail_id(),
            self.state_store.pack_day_projection().query.clone(),
        )?)
    }

    fn apply_selected_account_context(&mut self, context: &DesktopSelectedAccountContext) -> bool {
        let personal_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_personal_projection(
                    context.personal_projection.clone(),
                ));
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
        let products_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_products_list(
                    context.products_list.clone(),
                ));
        let orders_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_orders_list(
                    context.orders_list.clone(),
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
        let editor_changed = if context.farm_setup_projection.has_saved_farm() {
            false
        } else {
            self.close_product_editor()
        };
        let shell_changed = self.sync_truthful_farmer_section();

        personal_changed
            || farm_setup_changed
            || farm_rules_changed
            || today_changed
            || products_changed
            || orders_changed
            || order_detail_changed
            || pack_day_changed
            || editor_changed
            || shell_changed
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
    ) -> Result<&radroots_studio_app_models::SelectedAccountProjection, DesktopAppRuntimeFarmSetupError>
    {
        self.state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .ok_or(DesktopAppRuntimeFarmSetupError::AccountRequired)
    }

    fn selected_account_for_farm_rules(
        &self,
    ) -> Result<&radroots_studio_app_models::SelectedAccountProjection, DesktopAppRuntimeFarmRulesError>
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
    ) -> Result<radroots_studio_app_models::BuyerListingsProjection, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(Default::default());
        };

        sqlite_store.load_buyer_listings(&query.search_query, &query.fulfillment_methods)
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
        self.state_store
            .identity_projection()
            .selected_account
            .as_ref()
            .and_then(|account| account.farmer_activation.farm_id)
            .or(self
                .state_store
                .farm_setup_projection()
                .saved_farm
                .as_ref()
                .map(|farm| farm.farm_id))
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

    fn fallback_farm_profile(&self, farm_id: FarmId) -> FarmProfileRecord {
        fallback_farm_profile_for_projection(farm_id, self.state_store.farm_setup_projection())
    }

    fn load_products_list_for_query(
        &self,
        query: &ProductsScreenQueryState,
    ) -> Result<ProductsListProjection, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(ProductsListProjection::default());
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(ProductsListProjection::default());
        };

        sqlite_store.load_products(farm_id, &query.search_query, query.filter, query.sort)
    }

    fn replace_orders_query(
        &mut self,
        query: OrdersScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let orders_list = self.load_orders_list_for_query(&query)?;
        let filter_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::select_orders_filter(query.filter));
        let fulfillment_window_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::select_orders_fulfillment_window(
                    query.fulfillment_window_id,
                ));
        let list_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::replace_orders_list(orders_list));

        Ok(filter_changed || fulfillment_window_changed || list_changed)
    }

    fn load_orders_list_for_query(
        &self,
        query: &OrdersScreenQueryState,
    ) -> Result<OrdersListProjection, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(OrdersListProjection::default());
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(OrdersListProjection::default());
        };

        sqlite_store.load_orders_list(farm_id, query)
    }

    fn replace_pack_day_query(
        &mut self,
        query: PackDayScreenQueryState,
    ) -> Result<bool, AppSqliteError> {
        let pack_day_projection = self.load_pack_day_for_query(&query)?;
        let fulfillment_window_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::set_pack_day_fulfillment_window(
                    query.fulfillment_window_id,
                ));
        let projection_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::replace_pack_day_projection(
                    pack_day_projection,
                ));

        Ok(fulfillment_window_changed || projection_changed)
    }

    fn load_pack_day_for_query(
        &self,
        query: &PackDayScreenQueryState,
    ) -> Result<PackDayProjection, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(PackDayProjection::default());
        };
        let Some(farm_id) = self.selected_farm_id() else {
            return Ok(PackDayProjection::default());
        };

        sqlite_store.load_pack_day(farm_id, query)
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
            ShellSection::Home | ShellSection::Personal(_) | ShellSection::Settings(_) => false,
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
}

#[derive(Debug, Error)]
pub enum DesktopAppRuntimeCommandError {
    #[error("desktop runtime commands are unavailable while the runtime is degraded")]
    RuntimeUnavailable,
    #[error(transparent)]
    Accounts(#[from] DesktopAccountsCommandError),
    #[error(transparent)]
    Projection(#[from] DesktopAccountsProjectionError),
    #[error(transparent)]
    RemoteSigner(#[from] DesktopRemoteSignerError),
    #[error(transparent)]
    Sqlite(#[from] AppSqliteError),
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

fn load_selected_account_context(
    sqlite_store: &AppSqliteStore,
    identity_projection: &AppIdentityProjection,
    products_query: ProductsScreenQueryState,
    orders_query: OrdersScreenQueryState,
    selected_order_id: Option<OrderId>,
    pack_day_query: PackDayScreenQueryState,
) -> Result<DesktopSelectedAccountContext, AppSqliteError> {
    let buyer_context = identity_projection.buyer_context();
    let buyer_fulfillment_methods = BTreeSet::new();
    let buyer_listings = sqlite_store.load_buyer_listings("", &buyer_fulfillment_methods)?;
    let buyer_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
    let buyer_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;
    let buyer_orders = sqlite_store.load_buyer_orders(&buyer_context)?;
    let personal_projection = PersonalWorkspaceProjection {
        browse: BuyerBrowseScreenProjection {
            listings: buyer_listings.clone(),
            detail: None,
        },
        search: BuyerSearchScreenProjection {
            query: BuyerSearchScreenQueryState::default(),
            listings: buyer_listings,
            detail: None,
        },
        cart: BuyerCartScreenProjection {
            cart: buyer_cart,
            checkout: buyer_checkout,
        },
        orders: BuyerOrdersScreenProjection {
            list: buyer_orders,
            detail: None,
        },
        ..PersonalWorkspaceProjection::default()
    };
    let Some(selected_account) = identity_projection.selected_account.as_ref() else {
        return Ok(DesktopSelectedAccountContext {
            personal_projection,
            ..DesktopSelectedAccountContext::default()
        });
    };
    let farm_setup_projection =
        sqlite_store.load_farm_setup(&selected_account.account.account_id)?;
    let today_farm_id = selected_account
        .farmer_activation
        .farm_id
        .or(farm_setup_projection
            .saved_farm
            .as_ref()
            .map(|farm| farm.farm_id));
    let farm_rules_projection = match today_farm_id {
        Some(farm_id) => {
            let fallback_profile =
                fallback_farm_profile_for_projection(farm_id, &farm_setup_projection);
            sqlite_store.load_farm_rules(farm_id).map(|projection| {
                prepare_loaded_farm_rules_projection(projection, &fallback_profile)
            })?
        }
        None => FarmRulesProjection::default(),
    };
    let today_projection = match today_farm_id {
        Some(farm_id) => sqlite_store.load_today_agenda(Some(farm_id))?,
        None => TodayAgendaProjection::default(),
    };
    let products_list = match today_farm_id {
        Some(farm_id) => sqlite_store.load_products(
            farm_id,
            &products_query.search_query,
            products_query.filter,
            products_query.sort,
        )?,
        None => ProductsListProjection::default(),
    };
    let orders_list = match today_farm_id {
        Some(farm_id) => sqlite_store.load_orders_list(farm_id, &orders_query)?,
        None => OrdersListProjection::default(),
    };
    let order_detail = match today_farm_id.zip(selected_order_id) {
        Some((farm_id, order_id)) => sqlite_store.load_order_detail(farm_id, order_id)?,
        None => None,
    };
    let pack_day_projection = match today_farm_id {
        Some(farm_id) => sqlite_store.load_pack_day(farm_id, &pack_day_query)?,
        None => PackDayProjection::default(),
    };

    Ok(DesktopSelectedAccountContext {
        personal_projection,
        farm_setup_projection,
        farm_rules_projection,
        today_projection,
        products_list,
        orders_list,
        order_detail,
        pack_day_projection,
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

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use radroots_studio_app_core::{
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePlatform,
        AppSharedAccountsPaths, SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITY_FILE_NAME,
    };
    use radroots_studio_app_models::{
        AccountSurfaceActivationProjection, ActiveSurface, AppActivityKind, AppStartupGate,
        BlackoutPeriodId, BlackoutPeriodRecord, FarmId, FarmOperatingRulesRecord, FarmOrderMethod,
        FarmProfileRecord, FarmReadiness, FarmReadinessBlocker, FarmSetupDraft,
        FarmSetupProjection, FarmSummary, FarmerActivationProjection, FarmerSection,
        FulfillmentWindowId, FulfillmentWindowRecord, LoggedOutStartupProjection, OrderId,
        OrderStatus, OrdersFilter, PersonalSection, PickupLocationId, PickupLocationRecord,
        ProductEditorDraft, ProductStatus, ProductsFilter, ProductsSort, SelectedSurfaceProjection,
        SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TodaySummary,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_sqlite::{AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_state::{
        AppStateRepositoryError, AppStateStore, AppStateStoreError, HomeRoute,
        InMemoryAppStateRepository,
    };
    use radroots_identity::RadrootsIdentity;
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory,
    };

    use crate::accounts::DesktopLocalIdentityImportRequest;

    use super::{
        APP_DATABASE_FILE_NAME, DesktopAppRuntime, DesktopAppRuntimeActivityContextError,
        DesktopAppRuntimeCommandError, DesktopAppRuntimeState, DesktopRemoteSignerPaths,
    };

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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            radroots_studio_app_models::LoggedOutStartupPhase::GenerateKeyStarting
        );
        assert_eq!(
            summary.logged_out_startup.signer_entry.source_input,
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example"
        );
    }

    #[test]
    fn clearing_startup_pending_remote_signer_session_is_idempotent_without_record() {
        let paths = temp_remote_signer_paths("clear_pending_none");
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: Some(paths.clone()),
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: Some(paths.clone()),
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            radroots_studio_app_models::LoggedOutStartupPhase::GenerateKeyStarting
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
            radroots_studio_app_models::LoggedOutStartupPhase::SignerEntry
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
            radroots_studio_app_models::LoggedOutStartupPhase::ContinuePrompt
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
    fn replacing_today_agenda_is_shared_without_clobbering_home_shell() {
        let runtime = DesktopAppRuntime::from_state(DesktopAppRuntimeState {
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
            startup_issue: None,
        });
        let cloned_runtime = runtime.clone();
        let today_agenda = TodayAgendaProjection {
            farm: Some(FarmSummary {
                farm_id: radroots_studio_app_models::FarmId::new(),
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            }),
            summary: Some(TodaySummary {
                farm_id: radroots_studio_app_models::FarmId::new(),
                orders_needing_action: 2,
                low_stock_products: 1,
                draft_products: 3,
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
            radroots_studio_app_models::ActiveSurface::Personal
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
            radroots_studio_app_models::ActiveSurface::Personal
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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

        assert!(runtime.select_personal_section(PersonalSection::Browse));

        let summary = runtime.summary();
        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Browse)
        );
        assert_eq!(
            summary.personal_projection.entry.state,
            radroots_studio_app_models::PersonalEntryState::Guest
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
    fn runtime_order_actions_refresh_repository_backed_orders_projection() {
        let runtime = memory_runtime();
        let (_, farm_id) = provision_ready_farmer_account(&runtime);
        let (_, order_id) = seed_order_workspace(&runtime, farm_id);

        let sql = format!(
            "update orders
             set status = 'scheduled', updated_at = '2026-04-17T12:00:00Z'
             where id = '{order_id}' and farm_id = '{farm_id}'"
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
                .open_order_detail(order_id)
                .expect("order detail should open")
        );
        assert!(
            runtime
                .mark_order_packed(order_id)
                .expect("order should mark packed")
        );
        let packed_summary = runtime.summary();
        assert_eq!(
            packed_summary
                .orders_projection
                .detail
                .as_ref()
                .expect("packed detail")
                .status,
            OrderStatus::Packed
        );
        assert_eq!(packed_summary.orders_projection.list.rows.len(), 0);
        assert_eq!(
            packed_summary
                .orders_projection
                .list
                .summary
                .scheduled_orders,
            0
        );
        assert_eq!(
            packed_summary.orders_projection.list.summary.packed_orders,
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
                .open_order_detail(order_id)
                .expect("packed detail should open")
        );
        assert!(
            runtime
                .mark_order_completed(order_id)
                .expect("order should mark completed")
        );
        let completed_summary = runtime.summary();
        assert_eq!(
            completed_summary
                .orders_projection
                .detail
                .as_ref()
                .expect("completed detail")
                .status,
            OrderStatus::Completed
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
            unit_label: "box".to_owned(),
            price_minor_units: Some(900),
            price_currency: "usd".to_owned(),
            stock_quantity: Some(14),
            availability_window_id: None,
            status: radroots_studio_app_models::ProductStatus::Published,
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
            .save_farm_rules_projection(radroots_studio_app_models::FarmRulesProjection {
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
            shared_accounts_paths: Some(paths),
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: Some(
                AppSqliteStore::open(DatabaseTarget::InMemory)
                    .expect("in-memory sqlite store should open"),
            ),
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
            state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            startup_issue: None,
        })
    }

    fn file_backed_runtime(label: &str) -> (DesktopAppRuntime, AppSharedAccountsPaths) {
        let paths = temp_shared_accounts_paths(label);
        fs::create_dir_all(paths.data_root.as_path()).expect("data root should create");
        fs::create_dir_all(paths.secrets_root.as_path()).expect("secrets root should create");

        (
            DesktopAppRuntime::from_state(DesktopAppRuntimeState {
                state_store: AppStateStore::load(InMemoryAppStateRepository::default())
                    .expect("in-memory state store should load"),
                default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            DesktopAppRuntimeState::bootstrap_from_paths(paths, "ws://127.0.0.1:8080".to_owned())
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
    ) -> radroots_studio_app_models::ProductId {
        let product_id = radroots_studio_app_models::ProductId::new();
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

    fn cleanup_paths(paths: &AppSharedAccountsPaths) {
        let Some(base) = paths.data_root.ancestors().nth(3).map(PathBuf::from) else {
            return;
        };
        let _ = fs::remove_dir_all(base);
    }
}
