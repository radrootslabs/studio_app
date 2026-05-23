use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Duration, Utc};
use radroots_studio_app_core::{
    prepare_pack_day_export_bundle_at_data_root, write_prepared_pack_day_export_bundle,
    AppBuildIdentity, AppDesktopRuntimePaths, AppRuntimeCapture, AppRuntimeMode,
    AppRuntimePathsError, AppRuntimeSnapshot, AppSharedAccountsPaths, PackDayExportWriteError,
};
use radroots_studio_app_models::{
    ActiveSurface, AppActivityContext, AppActivityKind, AppIdentityProjection, AppStartupGate,
    BuyerCartLineProjection, BuyerCartProjection, BuyerCartReplaceConfirmationProjection,
    BuyerCheckoutDraft, BuyerContext, BuyerOrderDetailProjection, BuyerProductDetailProjection,
    FarmId, FarmOrderMethod, FarmProfileRecord, FarmReadiness, FarmRulesProjection, FarmSetupDraft,
    FarmSetupProjection, FarmSummary, FarmerSection, FulfillmentWindowId,
    LoggedOutStartupProjection, OrderDetailProjection, OrderId, OrderRecoveryProjection,
    OrdersFilter, OrdersListProjection, OrdersScreenQueryState, PackDayBatchPrintStatus,
    PackDayExportBundle, PackDayExportInstanceId, PackDayExportStatus, PackDayHostHandoffKind,
    PackDayHostHandoffStatus, PackDayPrintKind, PackDayPrintStatus, PackDayProjection,
    PackDayScreenQueryState, PersonalSection, PickupLocationRecord, ProductEditorDraft, ProductId,
    ProductsFilter, ProductsListProjection, ProductsSort, RecoveryKind, RecoveryQueueProjection,
    RecoveryRecordId, RecoveryState, ReminderDeadlineProjection, ReminderDeliveryState,
    ReminderFeedProjection, ReminderId, ReminderKind, ReminderLogEntryProjection,
    ReminderLogProjection, ReminderSurface, ReminderUrgency, SettingsAccountProjection,
    SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
};
use radroots_studio_app_sqlite::{
    APP_ACTIVITY_CONTEXT_LIMIT, AppLocalInteropImportReport, AppSqliteError, AppSqliteStore,
    BuyerRepeatDemandApplyOutcome, DatabaseTarget, StoredPendingSyncOperation, StoredSyncConflict,
    derive_farm_rules_readiness,
};
use radroots_studio_app_state::{
    derive_sync_projection, AppShellProjection, AppStateCommand, AppStatePersistenceRepository,
    AppStateStore, AppStateStoreError, BuyerBrowseScreenProjection, BuyerCartScreenProjection,
    BuyerOrdersScreenProjection, BuyerSearchScreenProjection, BuyerSearchScreenQueryState,
    FarmSetupFlowStage, FarmWorkspaceReadinessProjection, HomeRoute, OrdersScreenProjection,
    PackDayBatchPrintRequest, PackDayExportRequest, PackDayHostHandoffRequest, PackDayPrintRequest,
    PackDayScreenProjection, PersistedAppState, PersonalWorkspaceProjection,
    ProductsScreenProjection, ProductsScreenQueryState, APP_STATE_FILE_NAME,
};
use radroots_studio_app_sync::{
    AppSyncProjection, AppSyncRequest, AppSyncResult, AppSyncTransport, AppSyncTransportError,
    PendingSyncOperation, SyncAggregateRef, SyncCheckpointStatus, SyncConflictSeverity,
    SyncOperationKind, SyncTrigger,
};
use radroots_local_events::{
    LocalEventRecordInput, LocalEventsStore, LocalRecordFamily, LocalRecordStatus,
    PublishOutboxStatus, SourceRuntime,
};
use radroots_nostr_accounts::prelude::RadrootsNostrAccountsManager;
use radroots_sql_core::SqliteExecutor;
use serde_json::json;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use crate::accounts::{
    bootstrap_desktop_accounts, generate_local_account, identity_projection_from_manager,
    import_local_account, remove_selected_local_key, reset_local_device_state,
    select_active_surface, select_local_account, DesktopAccountsBootstrapError,
    DesktopAccountsCommandError, DesktopAccountsProjectionError, DesktopLocalIdentityImportRequest,
};
use crate::pack_day_host_handoff::{
    plan_pack_day_host_handoff, PackDayHostHandoffCommandPlan, PackDayHostHandoffError,
};
use crate::pack_day_print::{
    cleanup_prepared_customer_label_asset_root,
    cleanup_prepared_customer_label_assets_for_export_instance, plan_pack_day_batch_print,
    plan_pack_day_print, PackDayBatchPrintCommandPlan, PackDayBatchPrintError,
    PackDayPrintCommandPlan, PackDayPrintError,
};
use crate::remote_signer::{
    activate_pending_session, apply_remote_signer_custody, clear_pending_session,
    load_pending_session, purge_all_state, reconcile_startup, store_pending_session,
    DesktopRemoteSignerError, DesktopRemoteSignerPaths,
};

const APP_DATABASE_FILE_NAME: &str = "app.sqlite3";
const SHARED_LOCAL_EVENTS_DIR: &str = "local_events";
const SHARED_LOCAL_EVENTS_DB_FILE_NAME: &str = "local_events.sqlite";
const SYNC_TRANSPORT_UNAVAILABLE_MESSAGE: &str = "remote sync transport is not configured";

#[derive(Debug, Default)]
struct UnavailableAppSyncTransport;

impl AppSyncTransport for UnavailableAppSyncTransport {
    fn sync(&mut self, _request: AppSyncRequest) -> Result<AppSyncResult, AppSyncTransportError> {
        Err(AppSyncTransportError::unavailable(
            SYNC_TRANSPORT_UNAVAILABLE_MESSAGE,
        ))
    }
}

fn default_sync_transport() -> Box<dyn AppSyncTransport + Send> {
    Box::new(UnavailableAppSyncTransport)
}

#[derive(Clone, Debug)]
pub struct DesktopAppRuntime {
    state: Arc<Mutex<DesktopAppRuntimeState>>,
}

impl DesktopAppRuntime {
    pub fn bootstrap(
        default_nostr_relay_url: String,
        runtime_snapshot: AppRuntimeSnapshot,
    ) -> Self {
        let state = match DesktopAppRuntimeState::try_bootstrap(
            default_nostr_relay_url,
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

    pub fn save_personal_checkout_draft(
        &self,
        draft: BuyerCheckoutDraft,
    ) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().save_personal_checkout_draft(draft)
    }

    pub fn place_personal_order(&self) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().place_personal_order()
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

    pub fn mark_order_packed(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().mark_order_packed(order_id)
    }

    pub fn mark_order_completed(&self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        self.lock_state_mut().mark_order_completed(order_id)
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
    default_nostr_relay_url: String,
    shared_accounts_paths: Option<AppSharedAccountsPaths>,
    remote_signer_paths: Option<DesktopRemoteSignerPaths>,
    accounts_manager: Option<RadrootsNostrAccountsManager>,
    sqlite_store: Option<AppSqliteStore>,
    sync_transport: Box<dyn AppSyncTransport + Send>,
    runtime_metadata: DesktopAppRuntimeMetadataSummary,
    selected_account_pending_sync_write_count: usize,
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
                "selected_account_sync_conflicts",
                &self.selected_account_sync_conflicts,
            )
            .field("startup_issue", &self.startup_issue)
            .finish()
    }
}

impl DesktopAppRuntimeState {
    fn try_bootstrap(
        default_nostr_relay_url: String,
        runtime_snapshot: AppRuntimeSnapshot,
    ) -> Result<Self, DesktopAppRuntimeBootstrapError> {
        let paths = AppDesktopRuntimePaths::current_desktop()?;
        Self::bootstrap_from_paths(paths, default_nostr_relay_url, runtime_snapshot)
    }

    fn bootstrap_from_paths(
        paths: AppDesktopRuntimePaths,
        default_nostr_relay_url: String,
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
        let shared_local_events_database_path = shared_local_events_database_path(&paths)?;
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
        let selected_account_sync_context =
            load_selected_account_sync_context(&sqlite_store, &identity_projection)?;
        let _ = state_store.apply_in_memory(AppStateCommand::replace_identity_projection(
            identity_projection.clone(),
        ));
        if identity_projection.startup_gate() == AppStartupGate::SetupRequired
            && load_pending_session(&remote_signer_paths)?.is_some()
        {
            let _ = state_store.apply_in_memory(AppStateCommand::show_startup_signer_entry());
        }
        let pending_sync_write_count = selected_account_sync_context.pending_write_count;
        let selected_account_sync_conflicts = selected_account_sync_context.conflicts;
        let _ = state_store.apply_in_memory(AppStateCommand::replace_sync_projection(
            selected_account_sync_context.projection,
        ));
        let mut state = Self {
            state_store,
            default_nostr_relay_url,
            shared_accounts_paths: Some(paths.shared_accounts.clone()),
            remote_signer_paths: Some(remote_signer_paths),
            accounts_manager: accounts_bootstrap.accounts_manager,
            sqlite_store: Some(sqlite_store),
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::available(
                runtime_snapshot,
                &paths,
                database_path,
                database_schema_version,
            ),
            selected_account_pending_sync_write_count: pending_sync_write_count,
            selected_account_sync_conflicts,
            startup_issue: None,
        };
        let _ = state.apply_selected_account_context(&selected_account_context);

        Ok(state)
    }

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
            default_nostr_relay_url: String::new(),
            shared_accounts_paths: None,
            remote_signer_paths: None,
            accounts_manager: None,
            sqlite_store: None,
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::unavailable(runtime_snapshot),
            selected_account_pending_sync_write_count: 0,
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

    fn select_personal_section(&mut self, section: PersonalSection) -> bool {
        let section_changed = self
            .state_store
            .apply_in_memory(AppStateCommand::SelectSection(ShellSection::Personal(
                section,
            )));
        let editor_changed = self.close_product_editor();

        section_changed || editor_changed
    }

    fn open_personal_product_detail(
        &mut self,
        section: PersonalSection,
        product_id: ProductId,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let Some(detail) = sqlite_store.load_buyer_product_detail(product_id)? else {
            return Ok(false);
        };

        let section_changed = matches!(section, PersonalSection::Browse | PersonalSection::Search)
            && self.select_personal_section(section);
        let detail_changed = self.set_personal_product_detail(section, Some(detail));

        Ok(section_changed || detail_changed)
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
        let refreshed_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;
        let cart_changed = self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.checkout != refreshed_checkout {
                projection.cart.checkout = refreshed_checkout.clone();
                changed = true;
            }
            changed
        });
        let section_changed = self.select_personal_section(PersonalSection::Cart);

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
        let refreshed_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;

        Ok(self.refresh_personal_cart_and_checkout(refreshed_cart, refreshed_checkout))
    }

    fn save_personal_checkout_draft(
        &mut self,
        draft: BuyerCheckoutDraft,
    ) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        sqlite_store.save_buyer_checkout_draft(&buyer_context, &draft)?;
        let refreshed_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;

        Ok(self.mutate_personal_projection(|projection| {
            if projection.cart.checkout == refreshed_checkout {
                return false;
            }

            projection.cart.checkout = refreshed_checkout;
            true
        }))
    }

    fn place_personal_order(&mut self) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let order_id = sqlite_store.place_buyer_order(&buyer_context)?;
        let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
        let refreshed_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;
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
        let Some(order_detail) = sqlite_store.load_buyer_order_detail(&buyer_context, order_id)?
        else {
            return Err(AppSqliteError::InvalidProjection {
                reason: "buyer order write did not surface in buyer order detail",
            });
        };

        let personal_changed = self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.checkout != refreshed_checkout {
                projection.cart.checkout = refreshed_checkout.clone();
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

            changed
        });
        let section_changed = self.select_personal_section(PersonalSection::Orders);
        let pending_changed = if matches!(buyer_context, BuyerContext::Account(_)) {
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Order(order_id),
                order_sync_payload(
                    order_id,
                    order_detail.farm_id,
                    "place_personal_order",
                    Some("needs_action"),
                ),
            )])?
        } else {
            false
        };

        Ok(personal_changed || section_changed || pending_changed)
    }

    fn open_personal_order_detail(&mut self, order_id: OrderId) -> Result<bool, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(false);
        };
        let buyer_context = self.state_store.identity_projection().buyer_context();
        let Some(order_detail) = sqlite_store.load_buyer_order_detail(&buyer_context, order_id)?
        else {
            return Ok(false);
        };

        let detail_changed = self.set_personal_order_detail(Some(order_detail));
        let section_changed = self.select_personal_section(PersonalSection::Orders);

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

        match sqlite_store.apply_buyer_repeat_demand_to_cart(
            &buyer_context,
            order_id,
            replace_existing,
        )? {
            BuyerRepeatDemandApplyOutcome::Applied => {
                let refreshed_cart = sqlite_store.load_buyer_cart(&buyer_context)?;
                let refreshed_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;
                let refreshed_orders = sqlite_store.load_buyer_orders(&buyer_context)?;
                let refreshed_detail =
                    sqlite_store.load_buyer_order_detail(&buyer_context, order_id)?;
                let personal_changed = self.mutate_personal_projection(|projection| {
                    let mut changed = false;
                    if projection.cart.cart != refreshed_cart {
                        projection.cart.cart = refreshed_cart.clone();
                        changed = true;
                    }
                    if projection.cart.checkout != refreshed_checkout {
                        projection.cart.checkout = refreshed_checkout.clone();
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
                let section_changed = self.select_personal_section(PersonalSection::Cart);

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

        let continuity_state =
            self.continuity_state_with_order_detail(self.selected_order_detail_id());
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Order(order_id),
                order_sync_payload(order_id, farm_id, "mark_order_packed", Some("packed")),
            )])?;

        Ok(updated || context_changed || pending_changed)
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

        let continuity_state =
            self.continuity_state_with_order_detail(self.selected_order_detail_id());
        let selected_account_context = load_selected_account_context(
            sqlite_store,
            self.state_store.identity_projection(),
            &continuity_state,
        )?;
        let context_changed = self.apply_selected_account_context(&selected_account_context);
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Order(order_id),
                order_sync_payload(order_id, farm_id, "mark_order_completed", Some("completed")),
            )])?;

        Ok(updated || context_changed || pending_changed)
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
                    Some(failure) => AppStateCommand::fail_pack_day_print_with_kind(
                        request,
                        failure,
                    ),
                    None => AppStateCommand::fail_pack_day_print(request),
                };
                let _ = self
                    .state_store
                    .apply_in_memory(failure_command);
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
                    Some(failure) => AppStateCommand::fail_pack_day_print_with_kind(
                        request,
                        failure,
                    ),
                    None => AppStateCommand::fail_pack_day_print(request),
                };
                let _ = self
                    .state_store
                    .apply_in_memory(failure_command);
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
        let Some(farm_id) = self.selected_farm_id() else {
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
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Product(product_id),
                product_sync_payload(
                    product_id,
                    Some(farm_id),
                    "update_product_stock",
                    None,
                    Some(stock_quantity),
                    None,
                ),
            )])?;

        Ok(updated || context_changed || pending_changed)
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
        let draft_payload = draft.clone();
        let editor_changed =
            self.state_store
                .apply_in_memory(AppStateCommand::open_existing_product_editor(
                    product_id, draft,
                ));
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Product(product_id),
                product_sync_payload(
                    product_id,
                    Some(farm_id),
                    "open_new_product_editor",
                    Some(&draft_payload),
                    draft_payload.stock_quantity,
                    None,
                ),
            )])?;

        Ok(context_changed || section_changed || editor_changed || pending_changed)
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
        let app_local_changed =
            self.append_app_listing_local_work_record(product_id, &draft_payload)?;
        let pending_changed =
            self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
                SyncAggregateRef::Product(product_id),
                product_sync_payload(
                    product_id,
                    self.selected_farm_id(),
                    "save_product_editor_draft",
                    Some(&draft_payload),
                    draft_payload.stock_quantity,
                    None,
                ),
            )])?;

        Ok(saved || context_changed || editor_changed || app_local_changed || pending_changed)
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
        let _ = self.append_app_farm_local_work_record(&account, &projection, &saved_farm)?;

        let selected_account_context = self.refresh_selected_account_context()?;
        self.apply_selected_account_context(&selected_account_context);
        let _ = self.enqueue_selected_account_sync_operations(vec![pending_sync_upsert(
            SyncAggregateRef::Farm(saved_farm.farm_id),
            farm_sync_payload(
                saved_farm.farm_id,
                saved_farm.display_name.as_str(),
                Some(saved_farm.readiness),
                "finish_farm_setup",
            ),
        )])?;

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
        let previous_fulfillment_window_ids = {
            let sqlite_store = self.sqlite_store_for_farm_rules()?;
            sqlite_store
                .load_farm_rules(farm_id)?
                .fulfillment_windows
                .into_iter()
                .map(|window| window.fulfillment_window_id)
                .collect::<BTreeSet<_>>()
        };

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
        let current_fulfillment_window_ids = saved_projection
            .fulfillment_windows
            .iter()
            .map(|window| window.fulfillment_window_id)
            .collect::<BTreeSet<_>>();
        let mut pending_operations = Vec::with_capacity(
            1 + saved_projection.fulfillment_windows.len() + previous_fulfillment_window_ids.len(),
        );
        pending_operations.push(pending_sync_upsert(
            SyncAggregateRef::Farm(farm_id),
            farm_sync_payload(
                farm_id,
                saved_projection
                    .farm_profile
                    .as_ref()
                    .map(|profile| profile.display_name.as_str())
                    .unwrap_or_default(),
                Some(if saved_projection.is_ready() {
                    FarmReadiness::Ready
                } else {
                    FarmReadiness::Incomplete
                }),
                "save_farm_rules_projection",
            ),
        ));
        for window in &saved_projection.fulfillment_windows {
            pending_operations.push(pending_sync_upsert(
                SyncAggregateRef::FulfillmentWindow(window.fulfillment_window_id),
                fulfillment_window_sync_payload(window.fulfillment_window_id, farm_id, "upsert"),
            ));
        }
        for fulfillment_window_id in previous_fulfillment_window_ids
            .difference(&current_fulfillment_window_ids)
            .copied()
        {
            pending_operations.push(pending_sync_delete(
                SyncAggregateRef::FulfillmentWindow(fulfillment_window_id),
                fulfillment_window_sync_payload(fulfillment_window_id, farm_id, "delete"),
            ));
        }
        let _ = self.enqueue_selected_account_sync_operations(pending_operations)?;

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
        let selected_account_sync_context =
            load_selected_account_sync_context(self.sqlite_store()?, &projection)?;
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

        load_selected_account_sync_context(sqlite_store, self.state_store.identity_projection())
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
        let conflicts_changed = self.selected_account_sync_conflicts != context.conflicts;

        self.selected_account_pending_sync_write_count = context.pending_write_count;
        self.selected_account_sync_conflicts = context.conflicts.clone();

        projection_changed || pending_changed || conflicts_changed
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

        match self.sync_transport.sync(request) {
            Ok(result) => {
                changed |= self.apply_sync_result(
                    prepared.account_id.as_str(),
                    &prepared.pending_operations,
                    &result,
                )?;
            }
            Err(error) => {
                changed |= self.apply_sync_transport_error(
                    prepared.account_id.as_str(),
                    &prepared.checkpoint,
                    &prepared.pending_operations,
                    started_at.as_str(),
                    error,
                )?;
            }
        }

        Ok(changed)
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
        }

        self.refresh_selected_account_sync()
    }

    fn apply_sync_transport_error(
        &mut self,
        account_id: &str,
        previous_checkpoint: &SyncCheckpointStatus,
        pending_operations: &[StoredPendingSyncOperation],
        started_at: &str,
        error: AppSyncTransportError,
    ) -> Result<bool, AppSqliteError> {
        let failed_checkpoint = SyncCheckpointStatus::failed(
            Some(started_at.to_owned()),
            previous_checkpoint.last_sync_completed_at.clone(),
            previous_checkpoint.last_remote_cursor.clone(),
            error.to_string(),
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
    ) -> Result<radroots_studio_app_models::BuyerListingsProjection, AppSqliteError> {
        let Some(sqlite_store) = self.sqlite_store.as_ref() else {
            return Ok(Default::default());
        };

        sqlite_store.load_buyer_listings(&query.search_query, &query.fulfillment_methods)
    }

    fn refresh_personal_cart_and_checkout(
        &mut self,
        refreshed_cart: BuyerCartProjection,
        refreshed_checkout: radroots_studio_app_models::BuyerCheckoutProjection,
    ) -> bool {
        self.mutate_personal_projection(|projection| {
            let mut changed = false;
            if projection.cart.cart != refreshed_cart {
                projection.cart.cart = refreshed_cart.clone();
                changed = true;
            }
            if projection.cart.checkout != refreshed_checkout {
                projection.cart.checkout = refreshed_checkout.clone();
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


    fn append_app_farm_local_work_record(
        &self,
        account: &radroots_studio_app_models::SelectedAccountProjection,
        projection: &FarmSetupProjection,
        saved_farm: &FarmSummary,
    ) -> Result<bool, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(false);
        };
        let timestamp = current_runtime_time_ms()?;
        let farm_d_tag = d_tag_from_uuid(saved_farm.farm_id.as_uuid());
        let owner_pubkey = self.local_events_owner_pubkey(account);
        let exportability = app_local_work_exportability(owner_pubkey.as_deref());
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
        let input = LocalEventRecordInput {
            record_id: format!("app:local_work:farm:{farm_d_tag}:{}", Uuid::now_v7()),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::App,
            created_at_ms: timestamp,
            inserted_at_ms: timestamp,
            owner_account_id: Some(account.account.account_id.clone()),
            owner_pubkey,
            farm_id: Some(farm_d_tag),
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
        };

        self.append_app_local_work_record(shared_accounts_paths, &input)?;
        Ok(true)
    }

    fn append_app_listing_local_work_record(
        &self,
        product_id: ProductId,
        draft: &ProductEditorDraft,
    ) -> Result<bool, AppSqliteError> {
        let Some(shared_accounts_paths) = self.shared_accounts_paths.as_ref() else {
            return Ok(false);
        };
        let Some(account) = self
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
        let timestamp = current_runtime_time_ms()?;
        let farm_d_tag = d_tag_from_uuid(farm_id.as_uuid());
        let listing_d_tag = d_tag_from_uuid(product_id.as_uuid());
        let owner_pubkey = self.local_events_owner_pubkey(account);
        let listing_addr = owner_pubkey
            .as_ref()
            .map(|pubkey| format!("30402:{pubkey}:{listing_d_tag}"));
        let exportability = app_local_work_exportability(owner_pubkey.as_deref());
        let farm_setup = self.state_store.farm_setup_projection();
        let delivery_method = farm_setup
            .draft
            .order_methods
            .iter()
            .next()
            .map(|method| method.storage_key())
            .unwrap_or("pickup");
        let location_primary = if farm_setup.draft.location_or_service_area.trim().is_empty() {
            "local pickup"
        } else {
            farm_setup.draft.location_or_service_area.as_str()
        };
        let unit_label = if draft.unit_label.trim().is_empty() {
            "each"
        } else {
            draft.unit_label.as_str()
        };
        let price_amount = draft
            .price_minor_units
            .map(decimal_from_minor_units)
            .unwrap_or_else(|| "0".to_owned());
        let available = draft
            .stock_quantity
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0".to_owned());
        let payload = json!({
            "record_kind": "listing_draft_v1",
            "exportability": exportability,
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
                    "category": "produce",
                    "summary": draft.subtitle,
                },
                "primary_bin": {
                    "bin_id": "bin-1",
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
        let input = LocalEventRecordInput {
            record_id: format!("app:local_work:listing:{listing_d_tag}:{}", Uuid::now_v7()),
            family: LocalRecordFamily::LocalWork,
            status: LocalRecordStatus::LocalSaved,
            source_runtime: SourceRuntime::App,
            created_at_ms: timestamp,
            inserted_at_ms: timestamp,
            owner_account_id: Some(account.account.account_id.clone()),
            owner_pubkey,
            farm_id: Some(farm_d_tag),
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
        };

        self.append_app_local_work_record(shared_accounts_paths, &input)?;
        Ok(true)
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

    fn local_events_owner_pubkey(
        &self,
        account: &radroots_studio_app_models::SelectedAccountProjection,
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
        let sync_changed = self.attempt_sync(SyncTrigger::ForegroundResume)?;

        Ok(local_changed || context_changed || sync_changed)
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
    #[error(transparent)]
    RemoteSigner(#[from] DesktopRemoteSignerError),
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
    #[error("desktop app data root must be nested under the Radroots data root")]
    SharedLocalEventsPath,
}

fn shared_local_events_database_path(
    paths: &AppDesktopRuntimePaths,
) -> Result<PathBuf, DesktopAppRuntimeBootstrapError> {
    let data_root = paths
        .app
        .data
        .parent()
        .and_then(|apps_root| apps_root.parent())
        .ok_or(DesktopAppRuntimeBootstrapError::SharedLocalEventsPath)?;
    Ok(data_root
        .join("shared")
        .join(SHARED_LOCAL_EVENTS_DIR)
        .join(SHARED_LOCAL_EVENTS_DB_FILE_NAME))
}

fn shared_local_events_database_path_from_shared_accounts(
    paths: &AppSharedAccountsPaths,
) -> Option<PathBuf> {
    Some(
        paths
            .data_root
            .parent()?
            .join(SHARED_LOCAL_EVENTS_DIR)
            .join(SHARED_LOCAL_EVENTS_DB_FILE_NAME),
    )
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

fn d_tag_from_uuid(uuid: Uuid) -> String {
    base64_url_no_pad(uuid.as_bytes())
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

fn is_hex_64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn app_local_work_exportability(owner_pubkey: Option<&str>) -> serde_json::Value {
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

fn load_selected_account_context_with_options(
    sqlite_store: &AppSqliteStore,
    identity_projection: &AppIdentityProjection,
    continuity_state: &PersistedAppState,
    allow_auto_present: bool,
) -> Result<DesktopSelectedAccountContext, AppSqliteError> {
    let buyer_context = identity_projection.buyer_context();
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
    let buyer_checkout = sqlite_store.load_buyer_checkout(&buyer_context)?;
    let buyer_orders = sqlite_store.load_buyer_orders(&buyer_context)?;
    let buyer_order_detail = match continuity_state.buyer.orders_detail_order_id {
        Some(order_id) => sqlite_store.load_buyer_order_detail(&buyer_context, order_id)?,
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
            checkout: buyer_checkout,
        },
        orders: BuyerOrdersScreenProjection {
            list: buyer_orders,
            detail: buyer_order_detail,
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

fn load_selected_account_reminder_context(
    sqlite_store: &AppSqliteStore,
    account_id: &str,
    farm_id: FarmId,
    today_projection: &TodayAgendaProjection,
    canonical_orders_list: &OrdersListProjection,
    pack_day_projection: &PackDayProjection,
    selected_order_detail: Option<&OrderDetailProjection>,
) -> Result<DesktopSellerReminderContext, AppSqliteError> {
    load_selected_account_reminder_context_with_options(
        sqlite_store,
        account_id,
        farm_id,
        today_projection,
        canonical_orders_list,
        pack_day_projection,
        selected_order_detail,
        true,
    )
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
        (RecoveryKind::RefundFollowUp, RecoveryState::Open) => "Refund follow-up is open",
        (RecoveryKind::RefundFollowUp, RecoveryState::InReview) => "Refund follow-up is in review",
        (RecoveryKind::RefundFollowUp, RecoveryState::Resolved) => "Refund follow-up is resolved",
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
            "Review the situation and handle any refund outside the app."
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::InReview) => {
            "Confirm the outcome and keep payment handling outside the app."
        }
        (RecoveryKind::RefundFollowUp, RecoveryState::Resolved) => {
            "The refund follow-up was handled outside the app."
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

    Ok(DesktopSelectedAccountSyncContext {
        projection: derive_sync_projection(&checkpoint, &conflicts),
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

fn pending_sync_upsert(aggregate: SyncAggregateRef, payload_json: String) -> PendingSyncOperation {
    let created_at = current_utc_timestamp();

    PendingSyncOperation {
        aggregate,
        operation: SyncOperationKind::Upsert,
        payload_json,
        created_at: created_at.clone(),
        available_at: created_at,
        attempt_count: 0,
    }
}

fn pending_sync_delete(aggregate: SyncAggregateRef, payload_json: String) -> PendingSyncOperation {
    let created_at = current_utc_timestamp();

    PendingSyncOperation {
        aggregate,
        operation: SyncOperationKind::Delete,
        payload_json,
        created_at: created_at.clone(),
        available_at: created_at,
        attempt_count: 0,
    }
}

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

fn fulfillment_window_sync_payload(
    fulfillment_window_id: FulfillmentWindowId,
    farm_id: FarmId,
    source: &str,
) -> String {
    json!({
        "aggregate_kind": "fulfillment_window",
        "fulfillment_window_id": fulfillment_window_id.to_string(),
        "farm_id": farm_id.to_string(),
        "source": source,
    })
    .to_string()
}

fn product_sync_payload(
    product_id: ProductId,
    farm_id: Option<FarmId>,
    source: &str,
    draft: Option<&ProductEditorDraft>,
    stock_quantity: Option<u32>,
    status: Option<&str>,
) -> String {
    json!({
        "aggregate_kind": "product",
        "product_id": product_id.to_string(),
        "farm_id": farm_id.map(|value| value.to_string()),
        "title": draft.map(|value| value.title.clone()),
        "subtitle": draft.map(|value| value.subtitle.clone()),
        "unit_label": draft.map(|value| value.unit_label.clone()),
        "price_minor_units": draft.and_then(|value| value.price_minor_units),
        "price_currency": draft.map(|value| value.price_currency.clone()),
        "stock_quantity": stock_quantity.or_else(|| draft.and_then(|value| value.stock_quantity)),
        "availability_window_id": draft
            .and_then(|value| value.availability_window_id)
            .map(|value| value.to_string()),
        "status": status.or_else(|| draft.map(|value| value.status.storage_key())),
        "source": source,
    })
    .to_string()
}

fn order_sync_payload(
    order_id: OrderId,
    farm_id: FarmId,
    source: &str,
    status: Option<&str>,
) -> String {
    json!({
        "aggregate_kind": "order",
        "order_id": order_id.to_string(),
        "farm_id": farm_id.to_string(),
        "status": status,
        "source": source,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{Duration, Utc};
    use radroots_studio_app_core::{
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePlatform,
        AppSharedAccountsPaths, SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITY_FILE_NAME,
    };
    use radroots_studio_app_models::{
        AccountCustody, AccountSummary, AccountSurfaceActivationProjection, ActiveSurface,
        AppActivityKind, AppIdentityProjection, AppStartupGate, BlackoutPeriodId,
        BlackoutPeriodRecord, BuyerCheckoutDraft, FarmId, FarmOperatingRulesRecord,
        FarmOrderMethod, FarmProfileRecord, FarmReadiness, FarmReadinessBlocker, FarmSetupDraft,
        FarmSetupProjection, FarmSummary, FarmerActivationProjection, FarmerSection,
        FulfillmentWindowId, FulfillmentWindowRecord, LoggedOutStartupProjection, OrderId,
        OrderStatus, OrdersFilter, PackDayBatchPrintArtifact, PackDayBatchPrintFailureKind,
        PackDayBatchPrintStatus, PackDayExportInstanceId, PackDayExportStatus,
        PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPackListRow,
        PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintStatus, PackDayProductTotalRow,
        PackDayProjection, PackDayRosterRow, PersonalSection, PickupLocationId,
        PickupLocationRecord, ProductEditorDraft, ProductId, ProductStatus, ProductsFilter,
        ProductsSort, RecoveryKind, RecoveryRecordId, ReminderDeliveryState,
        ReminderFeedProjection, ReminderKind, SelectedAccountProjection, SelectedSurfaceProjection,
        SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TodaySummary,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_sqlite::{latest_schema_version, AppSqliteStore, DatabaseTarget};
    use radroots_studio_app_state::{
        AppStateCommand, AppStatePersistenceRepository, AppStateRepository,
        AppStateRepositoryError, AppStateStore, AppStateStoreError, FileBackedAppStateRepository,
        HomeRoute, APP_STATE_FILE_NAME,
    };
    use radroots_studio_app_sync::{
        AppSyncRequest, AppSyncResult, AppSyncRunStatus, AppSyncTransport, AppSyncTransportError,
        PendingSyncOperation, RecordedAppSyncTransport, SyncAggregateRef, SyncCheckpointState,
        SyncCheckpointStatus, SyncConflict, SyncConflictKind, SyncConflictResolutionStatus,
        SyncConflictSeverity, SyncOperationKind, SyncTrigger,
    };
    use radroots_identity::RadrootsIdentity;
    use radroots_local_events::{
        LocalEventRecord, LocalEventRecordInput, LocalEventsStore, LocalRecordFamily,
        LocalRecordStatus, PublishOutboxStatus, SourceRuntime,
    };
    use radroots_nostr_accounts::prelude::{
        RadrootsNostrAccountsManager, RadrootsNostrFileAccountStore,
        RadrootsNostrMemoryAccountStore, RadrootsNostrSecretVaultMemory,
    };
    use radroots_sql_core::SqliteExecutor;
    use serde_json::json;

    use crate::accounts::DesktopLocalIdentityImportRequest;

    use super::{
        default_sync_transport, DesktopAppRuntime, DesktopAppRuntimeActivityContextError,
        DesktopAppRuntimeCommandError, DesktopAppRuntimeMetadataSummary, DesktopAppRuntimeState,
        DesktopAppSyncStatusSummary, DesktopRemoteSignerPaths, APP_DATABASE_FILE_NAME,
        SYNC_TRANSPORT_UNAVAILABLE_MESSAGE, is_hex_64,
    };
    use crate::pack_day_host_handoff::PackDayHostHandoffError;
    use crate::pack_day_print::{
        execute_pack_day_batch_print_plan_with, prepared_customer_label_asset_root,
        PackDayBatchPrintError, PackDayPrintCommandResult, PackDayPrintError,
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

    fn install_recorded_sync_transport(
        runtime: &DesktopAppRuntime,
        transport: RecordedAppSyncTransport,
    ) -> Arc<Mutex<RecordedAppSyncTransport>> {
        let shared = Arc::new(Mutex::new(transport));
        runtime.lock_state_mut().sync_transport =
            Box::new(SharedRecordedSyncTransport(shared.clone()));
        shared
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
        assert!(summary
            .settings_account_projection
            .selected_account
            .is_none());
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            radroots_studio_app_models::LoggedOutStartupPhase::GenerateKeyStarting
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
                    &PendingSyncOperation {
                        aggregate: SyncAggregateRef::Farm(farm_id),
                        operation: SyncOperationKind::Upsert,
                        payload_json: "{\"farm\":\"queued\"}".to_owned(),
                        created_at: "2026-04-20T19:02:00Z".to_owned(),
                        available_at: "2026-04-20T19:02:00Z".to_owned(),
                        attempt_count: 0,
                    },
                )
                .expect("pending sync operation should save");
        }

        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

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
    fn runtime_local_product_mutations_enqueue_pending_sync_without_transport_calls() {
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
            }),
        );

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

        let summary = runtime.summary();
        let pending_operations = runtime
            .lock_state()
            .sqlite_store
            .as_ref()
            .expect("sqlite store")
            .load_pending_sync_operations(account_id.as_str())
            .expect("pending sync operations should load");

        assert_eq!(recorded.lock().expect("recorded transport").call_count(), 0);
        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert_eq!(pending_operations.len(), 1);
        assert!(matches!(
            pending_operations[0].operation.aggregate,
            SyncAggregateRef::Product(_)
        ));
    }

    #[test]
    fn runtime_launch_sync_attempt_dequeues_pushed_operations() {
        let runtime = memory_runtime();
        let (account_id, _) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

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
            }),
        );

        assert!(runtime
            .sync_on_app_launch()
            .expect("launch sync should succeed"));

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
    fn runtime_foreground_resume_sync_uses_the_resume_trigger() {
        let runtime = memory_runtime();
        let (_, _) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

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
            }),
        );

        assert!(runtime
            .sync_on_foreground_resume()
            .expect("resume sync should succeed"));

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
        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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
        assert!(runtime
            .select_products_filter(ProductsFilter::Drafts)
            .expect("draft products filter should reload"));
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
        assert_eq!(listing_payload["document"]["kind"], "listing_draft_v1");
        assert_eq!(listing_payload["document"]["seller_actor"]["pubkey"], owner_pubkey);
        assert_eq!(listing_payload["document"]["product"]["title"], "Eggs");
        assert_eq!(
            listing_payload["document"]["primary_bin"]["price_amount"],
            "7.50"
        );
        assert_eq!(listing_payload["document"]["inventory"]["available"], "12");
        assert!(listing_payload.get("draft").is_none());
        assert!(listing_payload.get("editor").is_none());

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
        assert!(app_records.iter().all(|record| record.owner_pubkey.is_none()));
        assert!(
            app_records
                .iter()
                .all(|record| record
                    .local_work_json
                    .as_ref()
                    .is_some_and(|payload| payload["exportability"]["state"] == "identity_unresolved"
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
        let (account_id, _) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

        assert!(runtime
            .sync_on_manual_refresh()
            .expect("manual refresh should complete"));

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
        assert!(summary
            .sync_status
            .projection
            .checkpoint
            .last_error_message
            .as_deref()
            .is_some_and(|message| { message.contains(SYNC_TRANSPORT_UNAVAILABLE_MESSAGE) }));
        assert_eq!(pending_operations.len(), 1);
        assert_eq!(pending_operations[0].operation.attempt_count, 1);
    }

    #[test]
    fn runtime_sync_attempts_stop_when_blocking_conflicts_are_present() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

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
        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

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
            }),
        );

        assert!(!runtime
            .sync_on_app_launch()
            .expect("blocked launch sync should skip"));

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
        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

        assert!(runtime
            .resolve_sync_conflict(
                conflict_id.as_str(),
                SyncConflictResolutionStatus::AcceptedLocal,
            )
            .expect("conflict resolution should succeed"));

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
        assert!(summary.sync_status.conflicts[0]
            .conflict
            .resolved_at
            .as_deref()
            .is_some());
    }

    #[test]
    fn runtime_review_required_conflicts_do_not_block_manual_refresh() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

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
        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

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
            }),
        );

        assert!(runtime
            .sync_on_manual_refresh()
            .expect("manual refresh should succeed"));

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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            selected_account_sync_conflicts: Vec::new(),
            startup_issue: None,
        });

        assert!(runtime
            .clear_startup_pending_remote_signer_session()
            .expect("clear pending should succeed"));
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

        assert!(runtime
            .store_startup_pending_remote_signer_session(&pending_session)
            .expect("store pending should succeed"));

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

        assert!(runtime
            .store_startup_pending_remote_signer_session(&pending_session)
            .expect("store pending should succeed"));
        assert!(runtime
            .clear_startup_pending_remote_signer_session()
            .expect("clear pending should succeed"));

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
            radroots_studio_app_models::LoggedOutStartupPhase::SignerEntry
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
            radroots_studio_app_models::LoggedOutStartupPhase::IdentityChoice
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn buyer_search_query_and_detail_recover_after_runtime_restart() {
        let (runtime, paths) = bootstrapped_runtime("restart_buyer_search_detail");
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);

        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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

        assert!(runtime
            .set_personal_search_query("salad")
            .expect("buyer search query should update"));
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Search, product_id)
            .expect("buyer search detail should open"));

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

        assert!(runtime
            .set_products_search_query("pea")
            .expect("products query should update"));
        assert!(runtime
            .select_products_filter(ProductsFilter::Drafts)
            .expect("products filter should update"));
        assert!(runtime
            .select_products_sort(ProductsSort::Name)
            .expect("products sort should update"));
        assert!(runtime
            .open_existing_product_editor(product_id)
            .expect("product editor should open"));

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

        assert!(runtime
            .select_orders_filter(OrdersFilter::Packed)
            .expect("orders filter should update"));
        assert!(runtime
            .open_order_detail(order_id)
            .expect("order detail should open"));

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

        assert!(runtime
            .open_orders_fulfillment_window(fulfillment_window_id)
            .expect("orders window should open"));
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
        assert!(summary
            .orders_projection
            .list
            .rows
            .iter()
            .any(|row| { row.fulfillment_window_id == Some(fulfillment_window_id) }));

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
        assert!(summary
            .pack_day_projection
            .projection
            .fulfillment_window
            .is_some());
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            selected_account_sync_conflicts: Vec::new(),
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
            state_store: AppStateStore::load(AppStatePersistenceRepository::in_memory())
                .expect("in-memory state store should load"),
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
        assert!(!runtime
            .open_pack_day(None)
            .expect("pack day route should stay blocked"));
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );
        assert!(runtime
            .summary()
            .pack_day_projection
            .projection
            .fulfillment_window
            .is_none());
    }

    #[test]
    fn runtime_routes_between_farmer_home_and_products_through_explicit_methods() {
        let runtime = memory_runtime();

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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
        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));

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
        assert!(summary
            .personal_projection
            .search
            .query
            .fulfillment_methods
            .is_empty());

        assert!(runtime
            .set_personal_search_query("pea")
            .expect("buyer search query should apply"));
        let searched = runtime.summary();
        assert_eq!(searched.personal_projection.search.listings.rows.len(), 1);
        assert_eq!(
            searched.personal_projection.search.listings.rows[0].title,
            "Pea shoots"
        );

        assert!(runtime
            .set_personal_search_fulfillment_method(FarmOrderMethod::Pickup, true)
            .expect("buyer fulfillment filter should apply"));
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
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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

        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, product_id)
            .expect("buyer detail should open"));
        assert!(runtime.increase_personal_product_quantity(PersonalSection::Browse));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("buyer product should add to cart"));

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
        assert!(summary
            .personal_projection
            .cart
            .cart
            .replace_confirmation
            .is_none());
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
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, first_product_id)
            .expect("first buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("first buyer product should add to cart"));

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

        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, second_product_id)
            .expect("second buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("cross-farm add should require confirmation"));

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

        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, true)
            .expect("confirmed cross-farm add should replace the cart"));
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
        assert!(replaced_summary
            .personal_projection
            .cart
            .cart
            .replace_confirmation
            .is_none());
    }

    #[test]
    fn runtime_removing_buyer_cart_line_clears_cart_and_checkout_readiness() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, product_id)
            .expect("buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("buyer product should add to cart"));

        assert!(runtime
            .remove_personal_cart_line(product_id)
            .expect("buyer cart line should remove"));

        let summary = runtime.summary();
        assert!(summary.personal_projection.cart.cart.lines.is_empty());
        assert!(summary.personal_projection.cart.cart.farm_id.is_none());
        assert!(!summary.personal_projection.cart.checkout.can_place_order);
        assert_eq!(
            summary.personal_projection.cart.checkout.summary.line_count,
            0
        );
    }

    #[test]
    fn runtime_places_buyer_order_and_routes_into_personal_orders() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, product_id)
            .expect("buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("buyer product should add to cart"));
        assert!(runtime
            .save_personal_checkout_draft(BuyerCheckoutDraft {
                name: "Casey Buyer".to_owned(),
                email: "casey@example.com".to_owned(),
                phone: "555-0101".to_owned(),
                order_note: "Leave by the cooler".to_owned(),
            })
            .expect("buyer checkout draft should save"));
        assert!(runtime
            .place_personal_order()
            .expect("buyer order should place"));

        let summary = runtime.summary();
        assert_eq!(
            summary.shell_projection.selected_section,
            ShellSection::Personal(PersonalSection::Orders)
        );
        assert!(summary.personal_projection.cart.cart.lines.is_empty());
        assert!(!summary.personal_projection.cart.checkout.can_place_order);
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
    fn runtime_opens_buyer_order_detail_from_personal_orders() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, product_id)
            .expect("buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("buyer product should add to cart"));
        assert!(runtime
            .save_personal_checkout_draft(BuyerCheckoutDraft {
                name: "Casey Buyer".to_owned(),
                email: "casey@example.com".to_owned(),
                phone: String::new(),
                order_note: String::new(),
            })
            .expect("buyer checkout draft should save"));
        assert!(runtime
            .place_personal_order()
            .expect("buyer order should place"));
        let order_id = runtime.summary().personal_projection.orders.list.rows[0].order_id;
        assert!(runtime.select_personal_section(PersonalSection::Browse));
        assert!(runtime.lock_state_mut().set_personal_order_detail(None));

        assert!(runtime
            .open_personal_order_detail(order_id)
            .expect("buyer order detail should open"));

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
    fn runtime_repeat_personal_order_readds_only_currently_eligible_items() {
        let runtime = memory_runtime();
        let (account_id, farm_id) = provision_ready_farmer_account(&runtime);
        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should switch into marketplace"));
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
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, available_product_id)
            .expect("available buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("available buyer product should add to cart"));
        assert!(runtime
            .open_personal_product_detail(PersonalSection::Browse, unavailable_product_id)
            .expect("unavailable buyer detail should open"));
        assert!(runtime
            .add_personal_product_to_cart(PersonalSection::Browse, false)
            .expect("second buyer product should add to cart"));
        assert!(runtime
            .save_personal_checkout_draft(BuyerCheckoutDraft {
                name: "Casey Buyer".to_owned(),
                email: "casey@example.com".to_owned(),
                phone: String::new(),
                order_note: String::new(),
            })
            .expect("buyer checkout draft should save"));
        assert!(runtime
            .place_personal_order()
            .expect("buyer order should place"));
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

        assert!(runtime
            .open_personal_order_detail(order_id)
            .expect("buyer order detail should reopen"));
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

        assert!(runtime
            .repeat_personal_order(order_id, false)
            .expect("repeat demand should add available items to cart"));

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
        assert!(summary
            .personal_projection
            .cart
            .cart
            .replace_confirmation
            .is_none());
    }

    #[test]
    fn runtime_products_queries_refresh_the_repository_backed_projection() {
        let runtime = memory_runtime();

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));

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

        assert!(runtime
            .select_products_filter(ProductsFilter::NeedAttention)
            .expect("filter should apply"));
        assert_eq!(runtime.summary().products_projection.list.rows.len(), 2);

        assert!(runtime
            .set_products_search_query("pea")
            .expect("search should apply"));
        let searched = runtime.summary();
        assert_eq!(searched.products_projection.list.rows.len(), 1);
        assert_eq!(
            searched.products_projection.list.rows[0].title,
            "Pea shoots"
        );

        assert!(runtime
            .select_products_sort(ProductsSort::Name)
            .expect("sort should apply"));
        assert_eq!(
            runtime.summary().products_projection.query.sort,
            ProductsSort::Name
        );
    }

    #[test]
    fn runtime_open_products_filter_routes_today_follow_ons_into_products() {
        let runtime = memory_runtime();

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        assert_eq!(
            runtime.summary().shell_projection.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );

        assert!(runtime
            .open_products_filter(ProductsFilter::Drafts)
            .expect("products follow-on should route"));
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

        assert!(runtime
            .open_order_detail(order_id)
            .expect("order detail should open"));
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

        assert!(!runtime
            .export_pack_day()
            .expect("missing pack day context should no-op"));
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

        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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

            assert!(runtime
                .finish_pack_day_host_handoff(prepared.0, Ok(()))
                .expect("host handoff success should apply"));
        }

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_host_handoff_records_failures_in_state() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_host_handoff_failure");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

        let (request, _) = runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff should prepare")
            .expect("host handoff should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_host_handoff());

        assert!(!runtime
            .finish_pack_day_host_handoff(request, Ok(()))
            .expect("stale completion should no-op"));
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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
                    assert!(prepared
                        .1
                        .target_path
                        .ends_with("customer_labels_avery_5160_letter_30_up.ps"));
                    assert!(!prepared
                        .1
                        .target_path
                        .starts_with(PathBuf::from(&export_bundle.bundle_directory)));
                    assert!(prepared
                        .1
                        .target_path
                        .to_string_lossy()
                        .contains(export_bundle.export_instance_id.to_string().as_str()));
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

            assert!(runtime
                .finish_pack_day_print(prepared.0, Ok(()))
                .expect("print success should apply"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(
            plan.plans
                .iter()
                .all(|plan| plan.command_program == "lp")
        );

        assert!(runtime
            .finish_pack_day_batch_print(request, Ok(()))
            .expect("batch print success should apply"));
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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

        let (_, _) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");
        assert!(runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPackSheet)
            .expect("print prepare should not fail")
            .is_none());
        assert!(runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff prepare should not fail")
            .is_none());

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_batch_print());

        let (print_request, _) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPackSheet)
            .expect("print should prepare")
            .expect("print should produce a plan");
        assert!(runtime
            .prepare_pack_day_batch_print()
            .expect("batch print prepare should not fail")
            .is_none());
        assert!(runtime
            .finish_pack_day_print(print_request, Ok(()))
            .expect("print success should apply"));
        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        let (_, _) = runtime
            .prepare_pack_day_host_handoff(PackDayHostHandoffKind::RevealBundle)
            .expect("host handoff should prepare")
            .expect("host handoff should produce a plan");
        assert!(runtime
            .prepare_pack_day_batch_print()
            .expect("batch print prepare should not fail")
            .is_none());

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_batch_print_records_failures_and_cleans_prepared_assets() {
        let (runtime, paths) = bootstrapped_runtime("pack_day_batch_print_failure_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

        let (request, _) = runtime
            .prepare_pack_day_batch_print()
            .expect("batch print should prepare")
            .expect("batch print should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_batch_print());

        assert!(!runtime
            .finish_pack_day_batch_print(request, Ok(()))
            .expect("stale completion should no-op"));
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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(runtime
            .finish_pack_day_batch_print(request.clone(), Ok(()))
            .expect("batch print success should apply"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
            DesktopAppRuntimeCommandError::PackDayBatchPrint(PackDayBatchPrintError::QueueExit {
                ..
            })
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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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
            print.request.as_ref().map(|request| request.export_instance_id),
            Some(bundle.export_instance_id)
        );
        assert_eq!(
            print.failure,
            Some(PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow)
        );

        cleanup_bootstrapped_runtime_paths(&paths);
    }

    #[test]
    fn runtime_finish_pack_day_print_cleans_customer_label_assets_and_keeps_cleanup_failures_best_effort(
    ) {
        let (runtime, paths) = bootstrapped_runtime("pack_day_print_cleanup");
        let (_, farm_id) = provision_ready_farmer_account(&runtime);

        seed_order_workspace(&runtime, farm_id);
        assert!(runtime.open_pack_day(None).expect("pack day should open"));
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

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

        assert!(runtime
            .finish_pack_day_print(success_request, Ok(()))
            .expect("print success should apply"));
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
        assert!(runtime
            .export_pack_day()
            .expect("initial pack day export should succeed"));
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

        assert!(runtime
            .export_pack_day()
            .expect("replacement pack day export should succeed"));

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

        assert!(runtime
            .open_pack_day(Some(fulfillment_window_id))
            .expect("first pack day window should open"));
        assert!(runtime
            .export_pack_day()
            .expect("initial pack day export should succeed"));

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

        assert!(runtime
            .open_pack_day(Some(other_fulfillment_window_id))
            .expect("second pack day window should open"));

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
        assert!(runtime
            .export_pack_day()
            .expect("pack day export should succeed"));

        let (request, _) = runtime
            .prepare_pack_day_print(PackDayPrintKind::PrintPickupRoster)
            .expect("print should prepare")
            .expect("print should produce a plan");

        let _ = runtime
            .lock_state_mut()
            .state_store
            .apply_in_memory(AppStateCommand::reset_pack_day_print());

        assert!(!runtime
            .finish_pack_day_print(request, Ok(()))
            .expect("stale completion should no-op"));
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

        assert!(runtime.open_orders().expect("orders should open"));
        assert!(runtime
            .mark_order_packed(order_id)
            .expect("mark packed should succeed"));
        let summary = runtime.summary();

        assert_eq!(summary.sync_status.pending_write_count, 1);
        assert!(summary
            .orders_projection
            .reminders
            .items
            .iter()
            .any(|item| item.kind == ReminderKind::SyncImpact
                && item.title == "Pending local changes"));
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

        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

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
        assert!(runtime
            .lock_state_mut()
            .refresh_selected_account_sync()
            .expect("sync status should refresh"));

        let reminder_id = runtime
            .summary()
            .orders_projection
            .reminders
            .items
            .iter()
            .find(|item| item.kind == ReminderKind::SyncImpact)
            .expect("sync reminder")
            .reminder_id;
        assert!(runtime
            .acknowledge_reminder(reminder_id)
            .expect("reminder should acknowledge"));

        let acknowledged_summary = runtime.summary();
        assert!(acknowledged_summary
            .orders_projection
            .reminders
            .items
            .iter()
            .any(|item| {
                item.reminder_id == reminder_id
                    && item.delivery_state == ReminderDeliveryState::Acknowledged
            }));
        assert!(acknowledged_summary
            .reminder_log
            .entries
            .iter()
            .any(|entry| {
                entry.reminder_id == reminder_id
                    && entry.delivery_state == ReminderDeliveryState::Acknowledged
            }));

        assert!(runtime
            .resolve_sync_conflict(
                conflict_id.as_str(),
                SyncConflictResolutionStatus::AcceptedLocal,
            )
            .expect("conflict resolution should succeed"));

        let resolved_summary = runtime.summary();
        assert!(resolved_summary
            .orders_projection
            .reminders
            .items
            .iter()
            .all(|item| { item.reminder_id != reminder_id }));
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

        assert!(runtime
            .open_order_detail(order_id)
            .expect("order detail should open"));
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

        assert!(runtime
            .select_orders_filter(OrdersFilter::Packed)
            .expect("orders filter should update"));
        assert!(runtime
            .open_order_detail(order_id)
            .expect("order detail should open"));

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

        assert!(runtime
            .open_orders_fulfillment_window(fulfillment_window_id)
            .expect("orders window follow-on should route"));
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

        assert!(runtime
            .select_orders_filter(OrdersFilter::Scheduled)
            .expect("scheduled filter should apply"));
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Scheduled
        );

        assert!(runtime
            .open_order_detail(order_id)
            .expect("order detail should open"));
        assert!(runtime
            .mark_order_packed(order_id)
            .expect("order should mark packed"));
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

        assert!(runtime
            .select_orders_filter(OrdersFilter::Packed)
            .expect("packed filter should apply"));
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Packed
        );

        assert!(runtime
            .open_order_detail(order_id)
            .expect("packed detail should open"));
        assert!(runtime
            .mark_order_completed(order_id)
            .expect("order should mark completed"));
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

        assert!(runtime
            .select_orders_filter(OrdersFilter::Completed)
            .expect("completed filter should apply"));
        assert_eq!(runtime.summary().orders_projection.list.rows.len(), 1);
        assert_eq!(
            runtime.summary().orders_projection.list.rows[0].status,
            OrderStatus::Completed
        );
    }

    #[test]
    fn runtime_stock_updates_refresh_today_and_products_projections() {
        let runtime = memory_runtime();

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        let product_id = runtime.summary().products_projection.list.rows[0].product_id;

        assert_eq!(
            runtime.summary().today_projection.low_stock_products.len(),
            1
        );
        assert!(runtime
            .update_product_stock(product_id, 12)
            .expect("stock update should succeed"));

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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        assert_eq!(
            runtime
                .summary()
                .products_projection
                .list
                .summary
                .total_products,
            0
        );

        assert!(runtime
            .open_new_product_editor()
            .expect("new product editor should open"));

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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        assert!(runtime
            .open_existing_product_editor(product_id)
            .expect("existing product editor should open"));

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

        assert!(runtime
            .save_product_editor_draft(saved_draft.clone())
            .expect("product editor draft should save"));

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

        assert!(runtime
            .generate_local_account(Some("First".to_owned()))
            .expect("first account should generate"));
        let first_summary = runtime.summary();
        let first_account_id = first_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();

        assert!(runtime
            .generate_local_account(Some("Second".to_owned()))
            .expect("second account should generate"));
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
        assert!(runtime
            .select_local_account(second_account_id.as_str())
            .expect("selection should succeed"));
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

        assert!(runtime
            .remove_selected_local_key()
            .expect("selected local key should remove"));
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
        assert!(runtime
            .import_local_account(DesktopLocalIdentityImportRequest::raw_secret_key(
                imported_identity.nsec(),
            ))
            .expect("raw import should succeed"));
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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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
        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        assert_eq!(runtime.summary().startup_gate, AppStartupGate::Farmer);

        assert!(runtime
            .select_active_surface(ActiveSurface::Personal)
            .expect("surface should select"));
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

        assert!(runtime
            .select_active_surface(ActiveSurface::Farmer)
            .expect("surface should reselect"));
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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
        let summary = runtime.summary();

        assert_eq!(summary.startup_gate, AppStartupGate::Farmer);
        assert_eq!(summary.home_route, HomeRoute::FarmSetupForm);
        assert_eq!(summary.farm_setup_projection, projection);
    }

    #[test]
    fn finishing_farm_setup_persists_saved_farm_and_today_projection() {
        let runtime = memory_runtime();

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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
        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));
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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));

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

        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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

        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));

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

        assert!(runtime
            .generate_local_account(Some("First".to_owned()))
            .expect("first account should generate"));
        let first_account_id = runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .expect("first selected account")
            .account
            .account_id
            .clone();
        assert!(runtime
            .generate_local_account(Some("Second".to_owned()))
            .expect("second account should generate"));
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

        assert!(runtime
            .reset_local_device_state()
            .expect("device state should reset"));
        let summary = runtime.summary();

        assert_eq!(summary.startup_gate, AppStartupGate::SetupRequired);
        assert!(summary.settings_account_projection.roster.is_empty());
        assert!(summary
            .settings_account_projection
            .selected_account
            .is_none());
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
            default_nostr_relay_url: "ws://127.0.0.1:8080".to_owned(),
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
            sync_transport: default_sync_transport(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            selected_account_pending_sync_write_count: 0,
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
                sync_transport: default_sync_transport(),
                runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
                selected_account_pending_sync_write_count: 0,
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
                "ws://127.0.0.1:8080".to_owned(),
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
        let database_path =
            super::shared_local_events_database_path(paths).expect("shared local events path");
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


    fn shared_local_event_records(paths: &AppDesktopRuntimePaths) -> Vec<LocalEventRecord> {
        let database_path =
            super::shared_local_events_database_path(paths).expect("shared local events path");
        let executor =
            SqliteExecutor::open(database_path.as_path()).expect("open shared local events db");
        let store = LocalEventsStore::new(executor);
        store
            .list_records_after_seq(0, 100)
            .expect("shared local records should list")
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
        assert!(runtime
            .generate_local_account(Some("Farmer".to_owned()))
            .expect("account should generate"));
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
        assert!(runtime
            .select_local_account(account_id.as_str())
            .expect("account should select"));

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
