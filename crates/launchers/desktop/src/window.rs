use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, Bounds, ClickEvent, Context, Entity,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled, Subscription,
    Timer, Window, WindowBackgroundAppearance, WindowBounds, WindowOptions, div,
    prelude::FluentBuilder, px, relative, rgb, size,
};
use gpui_component::{
    IconName, Root,
    input::{InputEvent, InputState},
};
use radroots_studio_app_i18n::{AppTextKey, app_text};
pub use radroots_studio_app_models::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_models::{
    AppStartupGate, BlackoutPeriodId, BlackoutPeriodRecord, BuyerCartProjection,
    BuyerCartReplaceConfirmationProjection, BuyerCheckoutDraft, BuyerCheckoutSummaryProjection,
    BuyerListingRow, BuyerOrderDetailProjection, BuyerOrderStatus, BuyerOrdersListRow,
    BuyerProductDetailProjection, FarmId, FarmOperatingRulesRecord, FarmOrderMethod,
    FarmProfileRecord, FarmReadinessBlocker, FarmRulesProjection, FarmRulesReadiness,
    FarmSetupBlocker, FarmSetupDraft, FarmSummary, FarmTimingConflictKind, FarmerSection,
    FulfillmentWindowId, FulfillmentWindowRecord, FulfillmentWindowSummary, LoggedOutStartupPhase,
    OrderDetailItemRow, OrderDetailProjection, OrderId, OrderListRow, OrderPrimaryAction,
    OrderStatus, OrdersFilter, OrdersListRow, PackDayPackListRow, PackDayProductTotalRow,
    PackDayRosterRow, PersonalEntryState, PersonalSection, PickupLocationId, PickupLocationRecord,
    ProductAttentionState, ProductEditorDraft, ProductId, ProductListRow, ProductPricePresentation,
    ProductPublishBlocker, ProductStatus, ProductsFilter, ProductsListRow, ProductsSort,
    ShellSection, TodayAgendaProjection, TodaySetupTaskKind,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingPollOutcome,
    RadrootsAppRemoteSignerPendingSession, radroots_studio_app_remote_signer_connect_pending,
    radroots_studio_app_remote_signer_poll_pending_session_with_progress,
    radroots_studio_app_remote_signer_preview, radroots_studio_app_remote_signer_requested_permissions,
};
use radroots_studio_app_sqlite::derive_farm_rules_readiness;
use radroots_studio_app_state::{
    FarmSetupFlowStage, FarmWorkspaceStatus, HomeRoute, derive_product_publish_blockers,
};
use radroots_studio_app_sync::{AppSyncRunStatus, SyncCheckpointState};
use radroots_studio_app_ui::{
    APP_UI_THEME, AppCheckboxFieldSpec, AppFormFieldSpec,
    AppSegmentButtonIconSpec as IconSegmentButtonSpec, LabelValueRow, app_button_card,
    app_button_choice as choice_button, app_button_compact as action_button_compact,
    app_button_icon as action_icon_button, app_button_list_row as list_row_button,
    app_button_primary as action_button_primary,
    app_button_primary_disabled as action_button_primary_disabled,
    app_button_secondary as action_button, app_button_text as text_button, app_checkbox_field,
    app_cluster, app_detail_row, app_divider as section_divider, app_form_field,
    app_form_input_text, app_form_section, app_heading_section, app_heading_view,
    app_input_text as app_text_input, app_scroll_panel,
    app_segment_button_icon as icon_segment_button, app_shared_label_text, app_shared_text,
    app_split_shell, app_stack_h, app_stack_v, app_status_indicator as status_indicator,
    app_surface_card, app_surface_card_section as home_card, app_surface_panel,
    app_surface_sidebar, app_surface_window as app_window_shell,
    app_text_badge as settings_badge_text, app_text_body_subtle as home_body_text, app_text_label,
    app_text_label as home_farm_setup_field_label, app_text_value, label_value_list,
    runtime_metadata_rows, utility_title_row,
};
use radroots_nostr::prelude::RadrootsNostrClient;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};
use tracing::error;

use crate::runtime::{DesktopAppRuntime, DesktopAppRuntimeSummary, DesktopAppSyncStatusSummary};

const HOME_WINDOW_MIN_WIDTH_PX: f32 = 1080.0;
const HOME_WINDOW_MIN_HEIGHT_PX: f32 = 720.0;

pub fn home_titlebar_options() -> gpui::TitlebarOptions {
    gpui::TitlebarOptions {
        title: None,
        appears_transparent: true,
        ..Default::default()
    }
}

pub fn settings_titlebar_options() -> gpui::TitlebarOptions {
    gpui::TitlebarOptions {
        title: None,
        appears_transparent: true,
        ..Default::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryWindowTarget {
    Home,
    SettingsAccount,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HomeStage {
    Setup,
    BuyerWorkspace,
    FarmerWorkspace,
}

pub fn primary_window_target(_: &DesktopAppRuntimeSummary) -> PrimaryWindowTarget {
    PrimaryWindowTarget::Home
}

pub fn home_stage(summary: &DesktopAppRuntimeSummary) -> HomeStage {
    if summary.startup_issue.is_some() || summary.startup_gate == AppStartupGate::Blocked {
        HomeStage::Setup
    } else if summary.startup_gate == AppStartupGate::Farmer {
        HomeStage::FarmerWorkspace
    } else if matches!(
        summary.shell_projection.selected_section,
        ShellSection::Personal(_)
    ) || summary.startup_gate == AppStartupGate::Personal
    {
        HomeStage::BuyerWorkspace
    } else {
        HomeStage::Setup
    }
}

pub fn home_window_options(cx: &mut App) -> WindowOptions {
    let (launch_width_px, launch_height_px) = home_window_launch_size_px();
    let (minimum_width_px, minimum_height_px) = home_window_minimum_size_px();
    let bounds = Bounds::centered(None, size(px(launch_width_px), px(launch_height_px)), cx);

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(minimum_width_px), px(minimum_height_px))),
        titlebar: Some(home_titlebar_options()),
        ..Default::default()
    }
}

fn home_window_launch_size_px() -> (f32, f32) {
    (
        APP_UI_THEME.shells.home_min_width_px,
        APP_UI_THEME.shells.home_min_height_px,
    )
}

fn home_window_minimum_size_px() -> (f32, f32) {
    (HOME_WINDOW_MIN_WIDTH_PX, HOME_WINDOW_MIN_HEIGHT_PX)
}

pub fn settings_window_options(cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(
        None,
        size(
            px(APP_UI_THEME.shells.settings_width_px),
            px(APP_UI_THEME.shells.settings_height_px),
        ),
        cx,
    );

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(
            px(APP_UI_THEME.shells.settings_width_px),
            px(APP_UI_THEME.shells.settings_height_px),
        )),
        titlebar: Some(settings_titlebar_options()),
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    }
}

pub fn open_home_window(
    window: &mut Window,
    cx: &mut App,
    runtime: DesktopAppRuntime,
) -> gpui::Entity<Root> {
    let _ = runtime.record_home_opened();
    let view = cx.new(|_| HomeView::new(runtime));
    cx.new(|cx| Root::new(view, window, cx))
}

pub fn open_settings_window(
    window: &mut Window,
    cx: &mut App,
    runtime: DesktopAppRuntime,
    initial_view: SettingsPanelViewKey,
) -> gpui::Entity<Root> {
    let _ = runtime.sync_settings_section(initial_view);
    let _ = runtime.record_settings_opened(initial_view);
    let view = cx.new(|_| SettingsWindowView::new(runtime, initial_view));
    cx.new(|cx| Root::new(view, window, cx))
}

pub struct HomeView {
    runtime: DesktopAppRuntime,
    startup_view: StartupHomeView,
    startup_signer_entry: Option<StartupSignerEntryState>,
    startup_signer_connect_state: StartupSignerConnectState,
    startup_signer_task_token: u64,
    startup_signer_recovery_attempted: bool,
    farm_setup_form: Option<FarmSetupFormState>,
    personal_search: Option<PersonalSearchState>,
    buyer_checkout_form: Option<BuyerCheckoutFormState>,
    products_search: Option<ProductsSearchState>,
    products_stock_editor: Option<ProductsStockEditorState>,
    product_editor_form: Option<ProductEditorFormState>,
    relay_client: Option<RadrootsNostrClient>,
}

#[derive(Clone, Debug)]
enum StartupSignerConnectState {
    Idle,
    Connecting,
    PendingApproval {
        pending_session: RadrootsAppRemoteSignerPendingSession,
        auth_challenge_url: Option<String>,
    },
    Approved {
        pending_session: RadrootsAppRemoteSignerPendingSession,
        approved_session: RadrootsAppRemoteSignerApprovedSession,
        auth_challenge_url: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StartupSignerPreviewSummary {
    source_label: String,
    signer_npub: String,
    relays_label: String,
    permissions_label: String,
}

#[derive(Clone, Debug)]
struct StartupSignerPollCycleResult {
    auth_challenge_url: Option<String>,
    outcome: Result<RadrootsAppRemoteSignerPendingPollOutcome, String>,
}

impl HomeView {
    pub fn new(runtime: DesktopAppRuntime) -> Self {
        Self {
            runtime,
            startup_view: StartupHomeView::new(),
            startup_signer_entry: None,
            startup_signer_connect_state: StartupSignerConnectState::Idle,
            startup_signer_task_token: 0,
            startup_signer_recovery_attempted: false,
            farm_setup_form: None,
            personal_search: None,
            buyer_checkout_form: None,
            products_search: None,
            products_stock_editor: None,
            product_editor_form: None,
            relay_client: None,
        }
    }

    fn generate_local_account(&mut self, cx: &mut Context<Self>) -> bool {
        if self.runtime.generate_local_account(None).unwrap_or(false) {
            cx.refresh_windows();
            cx.notify();
            return true;
        }

        false
    }

    fn reset_startup_signer_flow(&mut self) {
        self.startup_signer_task_token = self.startup_signer_task_token.wrapping_add(1);
        self.startup_signer_connect_state = StartupSignerConnectState::Idle;
    }

    fn next_startup_signer_task_token(&mut self) -> u64 {
        self.startup_signer_task_token = self.startup_signer_task_token.wrapping_add(1);
        self.startup_signer_task_token
    }

    fn startup_signer_task_is_current(&self, task_token: u64) -> bool {
        self.startup_signer_task_token == task_token
    }

    fn show_startup_identity_choice(&mut self, cx: &mut Context<Self>) {
        self.startup_view.clear_notice();
        self.reset_startup_signer_flow();
        self.startup_signer_recovery_attempted = false;
        if self.runtime.show_startup_identity_choice() {
            cx.notify();
        }
    }

    fn cancel_startup_signer_flow(&mut self, cx: &mut Context<Self>) -> bool {
        self.reset_startup_signer_flow();
        if !self.clear_startup_pending_remote_signer_session(cx) {
            return false;
        }

        self.startup_signer_recovery_attempted = false;
        true
    }

    fn back_out_of_startup_signer_entry(&mut self, cx: &mut Context<Self>) {
        if !self.cancel_startup_signer_flow(cx) {
            return;
        }

        self.startup_view.clear_notice();
        if self.runtime.show_startup_identity_choice() {
            cx.notify();
        }
    }

    fn show_startup_signer_entry(&mut self, cx: &mut Context<Self>) {
        self.startup_view.clear_notice();
        self.reset_startup_signer_flow();
        self.startup_signer_recovery_attempted = false;
        if self.runtime.show_startup_signer_entry() {
            cx.notify();
        }
    }

    fn start_generate_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.cancel_startup_signer_flow(cx) {
            return;
        }
        if !self.runtime.begin_generate_key_startup() {
            return;
        }

        self.startup_view.clear_notice();
        let relay_url = self.runtime.default_nostr_relay_url();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let startup_task = cx
                .background_executor()
                .spawn(run_startup_app_init(relay_url));
            Timer::after(Duration::from_secs(1)).await;
            let startup_result = startup_task.await;
            let _ = this.update(cx, |this, cx| {
                this.finish_generate_key(startup_result, cx);
            });
        })
        .detach();
    }

    fn finish_generate_key(
        &mut self,
        startup_result: Result<StartupAppInitResult, String>,
        cx: &mut Context<Self>,
    ) {
        match startup_result {
            Ok(result) => {
                self.relay_client = Some(result.relay_client);
                self.startup_view.clear_notice();
                if !self.generate_local_account(cx) {
                    self.show_startup_identity_choice(cx);
                }
            }
            Err(error) => {
                self.runtime.show_startup_identity_choice();
                self.startup_view.set_notice(error);
                cx.notify();
            }
        }
    }

    fn sync_startup_signer_entry(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if runtime_summary.startup_gate != AppStartupGate::SetupRequired
            || runtime_summary.logged_out_startup.phase != LoggedOutStartupPhase::SignerEntry
        {
            if self.startup_signer_entry.is_some()
                || !matches!(
                    self.startup_signer_connect_state,
                    StartupSignerConnectState::Idle
                )
            {
                self.reset_startup_signer_flow();
            }
            self.startup_signer_recovery_attempted = false;
            self.startup_signer_entry = None;
            return;
        }

        let source_input = runtime_summary
            .logged_out_startup
            .signer_entry
            .source_input
            .as_str();

        match self.startup_signer_entry.as_mut() {
            Some(entry) => entry.sync(source_input, window, cx),
            None => {
                self.startup_signer_entry =
                    Some(StartupSignerEntryState::new(source_input, window, cx));
            }
        }

        if !self.startup_signer_recovery_attempted {
            self.startup_signer_recovery_attempted = true;
            self.restore_startup_pending_remote_signer_session(window, cx);
        }
    }

    fn submit_startup_signer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.startup_signer_entry.as_ref() else {
            return;
        };

        let source_input = entry.input.read(cx).value().to_string();
        match startup_signer_preview_summary(source_input.as_str()) {
            Ok(_) => {}
            Err(error) => {
                self.startup_view.set_notice(error);
                cx.notify();
                return;
            }
        }

        self.startup_view.clear_notice();
        let task_token = self.next_startup_signer_task_token();
        self.startup_signer_connect_state = StartupSignerConnectState::Connecting;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let connect_result = cx
                .background_executor()
                .spawn(run_startup_signer_connect(source_input))
                .await;
            let Some(pending_session) = this
                .update(cx, |this, cx| {
                    this.finish_startup_signer_connect(task_token, connect_result, cx)
                })
                .ok()
                .flatten()
            else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.spawn_startup_signer_pending_poll(window, task_token, pending_session, cx);
            });
        })
        .detach();
    }

    fn finish_startup_signer_connect(
        &mut self,
        task_token: u64,
        connect_result: Result<RadrootsAppRemoteSignerPendingSession, String>,
        cx: &mut Context<Self>,
    ) -> Option<RadrootsAppRemoteSignerPendingSession> {
        if !self.startup_signer_task_is_current(task_token) {
            return None;
        }

        match connect_result {
            Ok(pending_session) => {
                if let Err(error) = self
                    .runtime
                    .store_startup_pending_remote_signer_session(&pending_session)
                {
                    self.startup_signer_connect_state = StartupSignerConnectState::Idle;
                    self.startup_view.set_notice(error.to_string());
                    cx.notify();
                    return None;
                }
                self.startup_view.clear_notice();
                self.startup_signer_connect_state = StartupSignerConnectState::PendingApproval {
                    pending_session: pending_session.clone(),
                    auth_challenge_url: None,
                };
                cx.notify();
                Some(pending_session)
            }
            Err(error) => {
                self.startup_signer_connect_state = StartupSignerConnectState::Idle;
                self.startup_view.set_notice(error);
                cx.notify();
                None
            }
        }
    }

    fn apply_startup_signer_poll_result(
        &mut self,
        task_token: u64,
        pending_session: RadrootsAppRemoteSignerPendingSession,
        poll_result: StartupSignerPollCycleResult,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.startup_signer_task_is_current(task_token) {
            return false;
        }

        let auth_challenge_url = poll_result.auth_challenge_url;
        match poll_result.outcome {
            Ok(RadrootsAppRemoteSignerPendingPollOutcome::PendingApproval) => {
                self.startup_view.clear_notice();
                self.startup_signer_connect_state = StartupSignerConnectState::PendingApproval {
                    pending_session,
                    auth_challenge_url,
                };
                cx.notify();
                true
            }
            Ok(RadrootsAppRemoteSignerPendingPollOutcome::TransportFailure { message }) => {
                if startup_signer_transport_failure_requires_notice(message.as_str()) {
                    self.startup_view.set_notice(message);
                } else {
                    self.startup_view.clear_notice();
                }
                self.startup_signer_connect_state = StartupSignerConnectState::PendingApproval {
                    pending_session,
                    auth_challenge_url,
                };
                cx.notify();
                true
            }
            Ok(RadrootsAppRemoteSignerPendingPollOutcome::Approved(approved_session)) => match self
                .runtime
                .activate_startup_approved_remote_signer_session(
                    &pending_session,
                    &approved_session,
                ) {
                Ok(_) => {
                    self.startup_view.clear_notice();
                    self.startup_signer_connect_state = StartupSignerConnectState::Approved {
                        pending_session,
                        approved_session,
                        auth_challenge_url,
                    };
                    cx.notify();
                    false
                }
                Err(error) => {
                    self.startup_view.set_notice(error.to_string());
                    self.startup_signer_connect_state =
                        StartupSignerConnectState::PendingApproval {
                            pending_session,
                            auth_challenge_url,
                        };
                    cx.notify();
                    false
                }
            },
            Ok(RadrootsAppRemoteSignerPendingPollOutcome::Rejected { message })
            | Ok(RadrootsAppRemoteSignerPendingPollOutcome::FatalError { message })
            | Err(message) => {
                let _ = self.runtime.clear_startup_pending_remote_signer_session();
                self.startup_signer_connect_state = StartupSignerConnectState::Idle;
                self.startup_view.set_notice(message);
                cx.notify();
                false
            }
        }
    }

    fn spawn_startup_signer_pending_poll(
        &mut self,
        window: &mut Window,
        task_token: u64,
        pending_session: RadrootsAppRemoteSignerPendingSession,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                let poll_result = cx
                    .background_executor()
                    .spawn(run_startup_signer_pending_poll(
                        pending_session.record.clone(),
                        pending_session.client_secret_key_hex.clone(),
                    ))
                    .await;
                let should_continue = this
                    .update(cx, |this, cx| {
                        this.apply_startup_signer_poll_result(
                            task_token,
                            pending_session.clone(),
                            poll_result,
                            cx,
                        )
                    })
                    .ok()
                    .unwrap_or(false);
                if !should_continue {
                    return;
                }

                Timer::after(Duration::from_secs(1)).await;
            }
        })
        .detach();
    }

    fn restore_startup_pending_remote_signer_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pending_session = match self.runtime.load_startup_pending_remote_signer_session() {
            Ok(Some(pending_session)) => pending_session,
            Ok(None) => return,
            Err(error) => {
                self.startup_view.set_notice(error.to_string());
                cx.notify();
                return;
            }
        };

        let task_token = self.next_startup_signer_task_token();
        self.startup_view.clear_notice();
        self.startup_signer_connect_state = StartupSignerConnectState::PendingApproval {
            pending_session: pending_session.clone(),
            auth_challenge_url: None,
        };
        cx.notify();
        self.spawn_startup_signer_pending_poll(window, task_token, pending_session, cx);
    }

    fn clear_startup_pending_remote_signer_session(&mut self, cx: &mut Context<Self>) -> bool {
        match self.runtime.clear_startup_pending_remote_signer_session() {
            Ok(_) => true,
            Err(error) => {
                self.startup_view.set_notice(error.to_string());
                cx.notify();
                false
            }
        }
    }

    fn open_farm_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let runtime_summary = self.runtime.summary();
        let Some(account_id) = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            return;
        };

        if runtime_summary.farm_setup_projection.has_saved_farm() {
            self.farm_setup_form = Some(FarmSetupFormState::new(
                account_id,
                runtime_summary.farm_setup_projection.draft,
                window,
                cx,
            ));
            cx.notify();
            return;
        }

        let stage_changed = self
            .runtime
            .select_farm_setup_flow_stage(FarmSetupFlowStage::Editing);

        self.farm_setup_form = Some(FarmSetupFormState::new(
            account_id,
            runtime_summary.farm_setup_projection.draft,
            window,
            cx,
        ));
        if stage_changed || self.farm_setup_form.is_some() {
            cx.notify();
        }
    }

    fn sync_farm_setup_form(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(account_id) = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            self.farm_setup_form = None;
            return;
        };

        if runtime_summary.home_route != HomeRoute::FarmSetupForm && self.farm_setup_form.is_none()
        {
            self.farm_setup_form = None;
            return;
        }

        let draft = runtime_summary.farm_setup_projection.draft.clone();
        let should_reset = self
            .farm_setup_form
            .as_ref()
            .map(|form| form.account_id != account_id)
            .unwrap_or(true);

        if should_reset {
            self.farm_setup_form = Some(FarmSetupFormState::new(account_id, draft, window, cx));
        }
    }

    fn sync_products_search(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(account_id) = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            self.products_search = None;
            return;
        };

        if !runtime_summary.farm_setup_projection.has_saved_farm() {
            self.products_search = None;
            return;
        }

        let search_query = runtime_summary
            .products_projection
            .query
            .search_query
            .as_str();
        let should_reset = self
            .products_search
            .as_ref()
            .map(|state| state.account_id != account_id)
            .unwrap_or(true);

        if should_reset {
            self.products_search = Some(ProductsSearchState::new(
                account_id,
                search_query,
                window,
                cx,
            ));
            return;
        }

        if let Some(products_search) = self.products_search.as_mut() {
            products_search.sync(search_query, window, cx);
        }
    }

    fn sync_personal_search(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if home_stage(runtime_summary) != HomeStage::BuyerWorkspace
            || selected_personal_section(runtime_summary) != PersonalSection::Search
        {
            self.personal_search = None;
            return;
        }

        let workspace_id = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
            .unwrap_or_else(|| "guest".to_owned());
        let search_query = runtime_summary
            .personal_projection
            .search
            .query
            .search_query
            .as_str();
        let should_reset = self
            .personal_search
            .as_ref()
            .map(|state| state.workspace_id != workspace_id)
            .unwrap_or(true);

        if should_reset {
            self.personal_search = Some(PersonalSearchState::new(
                workspace_id,
                search_query,
                window,
                cx,
            ));
            return;
        }

        if let Some(personal_search) = self.personal_search.as_mut() {
            personal_search.sync(search_query, window, cx);
        }
    }

    fn sync_buyer_checkout_form(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if home_stage(runtime_summary) != HomeStage::BuyerWorkspace
            || selected_personal_section(runtime_summary) != PersonalSection::Cart
            || runtime_summary
                .personal_projection
                .cart
                .cart
                .lines
                .is_empty()
        {
            self.buyer_checkout_form = None;
            return;
        }

        let workspace_id = personal_workspace_id(runtime_summary);
        let draft = &runtime_summary.personal_projection.cart.checkout.draft;
        let should_reset = self
            .buyer_checkout_form
            .as_ref()
            .map(|form| form.workspace_id != workspace_id)
            .unwrap_or(false);

        if should_reset {
            self.buyer_checkout_form =
                Some(BuyerCheckoutFormState::new(workspace_id, draft, window, cx));
            return;
        }

        if let Some(form) = self.buyer_checkout_form.as_mut() {
            form.sync(draft, window, cx);
        }
    }

    fn sync_products_stock_editor(&mut self, runtime_summary: &DesktopAppRuntimeSummary) {
        let Some(editor) = self.products_stock_editor.as_ref() else {
            return;
        };
        let Some(account_id) = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.as_str())
        else {
            self.products_stock_editor = None;
            return;
        };

        let should_clear = editor.account_id != account_id
            || selected_farmer_section(runtime_summary) != FarmerSection::Products
            || !runtime_summary.farm_setup_projection.has_saved_farm()
            || !runtime_summary
                .products_projection
                .list
                .rows
                .iter()
                .any(|row| row.product_id == editor.product_id);

        if should_clear {
            self.products_stock_editor = None;
        }
    }

    fn sync_product_editor_form(
        &mut self,
        runtime_summary: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(account_id) = runtime_summary
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            self.product_editor_form = None;
            return;
        };

        if selected_farmer_section(runtime_summary) != FarmerSection::Products
            || !runtime_summary.farm_setup_projection.has_saved_farm()
        {
            self.product_editor_form = None;
            return;
        }

        let radroots_studio_app_state::ProductEditorState::Open(session) =
            &runtime_summary.products_projection.editor
        else {
            self.product_editor_form = None;
            return;
        };
        let Some(product_id) = session.selected_product_id else {
            self.product_editor_form = None;
            return;
        };
        let should_reset = self
            .product_editor_form
            .as_ref()
            .map(|form| form.account_id != account_id || form.product_id != product_id)
            .unwrap_or(true);

        if should_reset {
            self.product_editor_form = Some(ProductEditorFormState::new(
                account_id,
                product_id,
                session.draft.clone(),
                window,
                cx,
            ));
        }
    }

    fn select_farmer_section(&mut self, section: FarmerSection, cx: &mut Context<Self>) {
        if self.runtime.select_farmer_section(section) {
            self.products_stock_editor = None;
            if section != FarmerSection::Products {
                self.product_editor_form = None;
            }
            cx.notify();
        }
    }

    fn select_personal_section(&mut self, section: PersonalSection, cx: &mut Context<Self>) {
        if self.runtime.select_personal_section(section) {
            self.products_stock_editor = None;
            self.product_editor_form = None;
            cx.notify();
        }
    }

    fn switch_to_marketplace(&mut self, cx: &mut Context<Self>) {
        match self
            .runtime
            .select_active_surface(radroots_studio_app_models::ActiveSurface::Personal)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "shell",
                    event = "shell.switch_marketplace_failed",
                    error = %runtime_error,
                    "failed to switch into marketplace mode"
                );
            }
        }
    }

    fn switch_to_farmer_workspace(&mut self, cx: &mut Context<Self>) {
        match self
            .runtime
            .select_active_surface(radroots_studio_app_models::ActiveSurface::Farmer)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "shell",
                    event = "shell.switch_farm_failed",
                    error = %runtime_error,
                    "failed to switch into farm mode"
                );
            }
        }
    }

    fn open_account_entry(&mut self, cx: &mut Context<Self>) {
        if self.runtime.select_home() {
            self.products_stock_editor = None;
            self.product_editor_form = None;
            cx.notify();
        }
    }

    fn handle_startup_signer_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(entry) = self.startup_signer_entry.as_ref() else {
            return;
        };
        if entry.input != *state {
            return;
        }
        if !startup_signer_source_input_is_editable(&self.startup_signer_connect_state) {
            return;
        }

        let value = state.read(cx).value().to_string();
        if self.runtime.set_startup_signer_source_input(value.as_str()) {
            self.startup_view.clear_notice();
            self.reset_startup_signer_flow();
            cx.notify();
        }
    }

    fn handle_products_search_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let value = state.read(cx).value().to_string();
        match self.runtime.set_products_search_query(value.as_str()) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.search_query_update_failed",
                    error = %runtime_error,
                    "failed to update products search query"
                );
            }
        }
    }

    fn handle_personal_search_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let value = state.read(cx).value().to_string();
        match self.runtime.set_personal_search_query(value.as_str()) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.search_query_update_failed",
                    error = %runtime_error,
                    "failed to update buyer search query"
                );
            }
        }
    }

    fn handle_buyer_checkout_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(form) = self.buyer_checkout_form.as_ref() else {
            return;
        };
        let matches_input = form.name_input == *state
            || form.email_input == *state
            || form.phone_input == *state
            || form.order_note_input == *state;
        if !matches_input {
            return;
        }

        match self
            .runtime
            .save_personal_checkout_draft(form.current_draft(cx))
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.checkout_save_failed",
                    error = %runtime_error,
                    "failed to save buyer checkout draft"
                );
            }
        }
    }

    fn toggle_personal_search_fulfillment_method(
        &mut self,
        method: FarmOrderMethod,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .set_personal_search_fulfillment_method(method, enabled)
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.fulfillment_filter_update_failed",
                    error = %runtime_error,
                    method = method.storage_key(),
                    "failed to update buyer fulfillment filter"
                );
            }
        }
    }

    fn open_personal_product_detail(
        &mut self,
        section: PersonalSection,
        product_id: ProductId,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .open_personal_product_detail(section, product_id)
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.detail_open_failed",
                    error = %runtime_error,
                    "failed to open buyer product detail"
                );
            }
        }
    }

    fn close_personal_product_detail(&mut self, section: PersonalSection, cx: &mut Context<Self>) {
        if self.runtime.close_personal_product_detail(section) {
            cx.notify();
        }
    }

    fn increase_personal_product_quantity(
        &mut self,
        section: PersonalSection,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.increase_personal_product_quantity(section) {
            cx.notify();
        }
    }

    fn decrease_personal_product_quantity(
        &mut self,
        section: PersonalSection,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.decrease_personal_product_quantity(section) {
            cx.notify();
        }
    }

    fn add_personal_product_to_cart(
        &mut self,
        section: PersonalSection,
        replace_existing: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .add_personal_product_to_cart(section, replace_existing)
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.add_to_cart_failed",
                    error = %runtime_error,
                    "failed to add buyer product to cart"
                );
            }
        }
    }

    fn clear_personal_cart_replace_confirmation(&mut self, cx: &mut Context<Self>) {
        if self.runtime.clear_personal_cart_replace_confirmation() {
            cx.notify();
        }
    }

    fn open_personal_checkout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.buyer_checkout_form.is_some() {
            return;
        }

        let runtime_summary = self.runtime.summary();
        if home_stage(&runtime_summary) != HomeStage::BuyerWorkspace
            || selected_personal_section(&runtime_summary) != PersonalSection::Cart
            || runtime_summary
                .personal_projection
                .cart
                .cart
                .lines
                .is_empty()
        {
            return;
        }

        self.buyer_checkout_form = Some(BuyerCheckoutFormState::new(
            personal_workspace_id(&runtime_summary),
            &runtime_summary.personal_projection.cart.checkout.draft,
            window,
            cx,
        ));
        cx.notify();
    }

    fn close_personal_checkout(&mut self, cx: &mut Context<Self>) {
        if self.buyer_checkout_form.take().is_some() {
            cx.notify();
        }
    }

    fn remove_personal_cart_line(&mut self, product_id: ProductId, cx: &mut Context<Self>) {
        match self.runtime.remove_personal_cart_line(product_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.cart_remove_failed",
                    error = %runtime_error,
                    product_id = %product_id,
                    "failed to remove buyer cart line"
                );
            }
        }
    }

    fn place_personal_order(&mut self, cx: &mut Context<Self>) {
        match self.runtime.place_personal_order() {
            Ok(true) => {
                self.buyer_checkout_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.checkout_place_failed",
                    error = %runtime_error,
                    "failed to place buyer order"
                );
            }
        }
    }

    fn open_personal_order_detail(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.open_personal_order_detail(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.order_open_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to open buyer order detail"
                );
            }
        }
    }

    fn select_products_filter(&mut self, filter: ProductsFilter, cx: &mut Context<Self>) {
        match self.runtime.select_products_filter(filter) {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.filter_update_failed",
                    error = %runtime_error,
                    filter = filter.storage_key(),
                    "failed to update products filter"
                );
            }
        }
    }

    fn select_products_sort(&mut self, sort: ProductsSort, cx: &mut Context<Self>) {
        match self.runtime.select_products_sort(sort) {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.sort_update_failed",
                    error = %runtime_error,
                    sort = sort.storage_key(),
                    "failed to update products sort"
                );
            }
        }
    }

    fn open_products_filter(&mut self, filter: ProductsFilter, cx: &mut Context<Self>) {
        match self.runtime.open_products_filter(filter) {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.route_failed",
                    error = %runtime_error,
                    filter = filter.storage_key(),
                    "failed to route into products view"
                );
            }
        }
    }

    fn open_orders(&mut self, cx: &mut Context<Self>) {
        match self.runtime.open_orders() {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.route_failed",
                    error = %runtime_error,
                    "failed to route into orders view"
                );
            }
        }
    }

    fn open_orders_fulfillment_window(
        &mut self,
        fulfillment_window_id: FulfillmentWindowId,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .open_orders_fulfillment_window(fulfillment_window_id)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.route_failed",
                    error = %runtime_error,
                    fulfillment_window_id = %fulfillment_window_id,
                    "failed to route into orders view"
                );
            }
        }
    }

    fn open_pack_day(
        &mut self,
        fulfillment_window_id: Option<FulfillmentWindowId>,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.open_pack_day(fulfillment_window_id) {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.route_failed",
                    error = %runtime_error,
                    "failed to route into pack day view"
                );
            }
        }
    }

    fn open_today_next_window(
        &mut self,
        fulfillment_window_id: Option<FulfillmentWindowId>,
        cx: &mut Context<Self>,
    ) {
        let Some(fulfillment_window_id) = fulfillment_window_id else {
            return;
        };

        match self.runtime.open_pack_day(Some(fulfillment_window_id)) {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => self.open_orders_fulfillment_window(fulfillment_window_id, cx),
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.route_failed",
                    error = %runtime_error,
                    "failed to route into pack day view"
                );
            }
        }
    }

    fn select_orders_filter(&mut self, filter: OrdersFilter, cx: &mut Context<Self>) {
        match self.runtime.select_orders_filter(filter) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.filter_update_failed",
                    error = %runtime_error,
                    filter = filter.storage_key(),
                    "failed to update orders filter"
                );
            }
        }
    }

    fn open_order_detail(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.open_order_detail(order_id) {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.detail_open_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to open order detail"
                );
            }
        }
    }

    fn mark_order_packed(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.mark_order_packed(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.mark_packed_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to mark order packed"
                );
            }
        }
    }

    fn mark_order_completed(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.mark_order_completed(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.mark_completed_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to mark order completed"
                );
            }
        }
    }

    fn open_products_stock_editor(
        &mut self,
        product_id: ProductId,
        stock_quantity: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _ = self.runtime.close_product_editor();
        let Some(account_id) = self
            .runtime
            .summary()
            .settings_account_projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.clone())
        else {
            return;
        };

        if self
            .products_stock_editor
            .as_ref()
            .map(|editor| editor.product_id == product_id)
            .unwrap_or(false)
        {
            self.products_stock_editor = None;
            cx.notify();
            return;
        }

        self.products_stock_editor = Some(ProductsStockEditorState::new(
            account_id,
            product_id,
            stock_quantity,
            window,
            cx,
        ));
        self.product_editor_form = None;
        cx.notify();
    }

    fn close_products_stock_editor(&mut self, cx: &mut Context<Self>) {
        if self.products_stock_editor.take().is_some() {
            cx.notify();
        }
    }

    fn handle_products_stock_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(editor) = self.products_stock_editor.as_mut() else {
            return;
        };

        if editor.input != *state || !editor.save_failed {
            return;
        }

        editor.save_failed = false;
        cx.notify();
    }

    fn save_products_stock_editor(&mut self, cx: &mut Context<Self>) {
        let Some((product_id, stock_quantity)) =
            self.products_stock_editor.as_ref().and_then(|editor| {
                editor
                    .parsed_stock_quantity(cx)
                    .map(|stock_quantity| (editor.product_id, stock_quantity))
            })
        else {
            return;
        };

        match self
            .runtime
            .update_product_stock(product_id, stock_quantity)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.stock_update_failed",
                    error = %runtime_error,
                    product_id = %product_id,
                    stock_quantity,
                    "failed to update product stock"
                );

                if let Some(editor) = self.products_stock_editor.as_mut() {
                    editor.save_failed = true;
                }
                cx.notify();
            }
        }
    }

    fn open_new_product_editor(&mut self, cx: &mut Context<Self>) {
        match self.runtime.open_new_product_editor() {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.new_editor_open_failed",
                    error = %runtime_error,
                    "failed to open new product editor"
                );
            }
        }
    }

    fn open_existing_product_editor(&mut self, product_id: ProductId, cx: &mut Context<Self>) {
        match self.runtime.open_existing_product_editor(product_id) {
            Ok(true) => {
                self.products_stock_editor = None;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.editor_open_failed",
                    error = %runtime_error,
                    product_id = %product_id,
                    "failed to open existing product editor"
                );
            }
        }
    }

    fn close_product_editor(&mut self, cx: &mut Context<Self>) {
        let changed = self.runtime.close_product_editor();
        let cleared = self.product_editor_form.take().is_some();

        if changed || cleared {
            cx.notify();
        }
    }

    fn handle_product_editor_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(form) = self.product_editor_form.as_mut() else {
            return;
        };
        let matches_input = form.title_input == *state
            || form.subtitle_input == *state
            || form.unit_input == *state
            || form.price_input == *state
            || form.stock_input == *state;

        if !matches_input {
            return;
        }

        if form.save_failed {
            form.save_failed = false;
        }

        cx.notify();
    }

    fn select_product_editor_status(&mut self, status: ProductStatus, cx: &mut Context<Self>) {
        let Some(form) = self.product_editor_form.as_mut() else {
            return;
        };

        if form.status == status {
            return;
        }

        form.status = status;
        form.save_failed = false;
        cx.notify();
    }

    fn save_product_editor(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.product_editor_form.as_mut() else {
            return;
        };
        let Some(draft) = form.current_draft(cx) else {
            return;
        };

        match self.runtime.save_product_editor_draft(draft.clone()) {
            Ok(true) => {
                form.initial_draft = draft;
                form.save_failed = false;
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "products",
                    event = "products.editor_save_failed",
                    error = %runtime_error,
                    product_id = %form.product_id,
                    "failed to save product editor draft"
                );
                form.save_failed = true;
                cx.notify();
            }
        }
    }

    fn handle_farm_name_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, InputEvent::Change) {
            let value = state.read(cx).value().to_string();
            self.update_farm_setup_draft(cx, |draft| {
                draft.farm_name = value;
            });
        }
    }

    fn handle_location_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, InputEvent::Change) {
            let value = state.read(cx).value().to_string();
            self.update_farm_setup_draft(cx, |draft| {
                draft.location_or_service_area = value;
            });
        }
    }

    fn toggle_farm_order_method(
        &mut self,
        method: FarmOrderMethod,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        self.update_farm_setup_draft(cx, |draft| {
            if enabled {
                draft.order_methods.insert(method);
            } else {
                draft.order_methods.remove(&method);
            }
        });
    }

    fn update_farm_setup_draft(
        &mut self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut FarmSetupDraft),
    ) {
        let Some(form) = self.farm_setup_form.as_mut() else {
            return;
        };

        update(&mut form.draft);

        match self.runtime.save_farm_setup_draft(form.draft.clone()) {
            Ok(projection) => {
                form.draft = projection.draft;
                form.save_state = FarmSetupSaveState::SavedLocally;
            }
            Err(_) => {
                form.save_state = FarmSetupSaveState::SaveFailed;
            }
        }

        cx.notify();
    }

    fn finish_farm_setup(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.farm_setup_form.as_mut() else {
            return;
        };

        match self.runtime.finish_farm_setup() {
            Ok(_) => {
                form.save_state = FarmSetupSaveState::SavedLocally;
                self.farm_setup_form = None;
            }
            Err(_) => {
                form.save_state = FarmSetupSaveState::SaveFailed;
            }
        }

        cx.notify();
    }

    fn render_today_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let projection = &runtime.today_projection;
        let home_status = home_status_presentation(runtime);
        let setup_onboarding = farm_setup_onboarding_card_spec(runtime.home_route);
        let farm_state = farmer_home_farm_state(runtime);
        let next_fulfillment_window_id = projection
            .next_fulfillment_window
            .as_ref()
            .map(|window| window.fulfillment_window_id);
        let mut sections = Vec::<AnyElement>::new();

        if let Some(summary) = projection.summary.as_ref() {
            sections.push(home_summary_card(summary).into_any_element());
        }

        if let Some(issue) = runtime.startup_issue.as_ref() {
            sections.push(
                home_card(
                    app_shared_text(AppTextKey::MetadataStartupIssue),
                    home_body_text(issue.clone()),
                )
                .into_any_element(),
            );
        }

        if let Some(farm_setup_form) = self.farm_setup_form.as_ref() {
            sections.push(
                home_farm_setup_form_card(
                    farm_setup_form,
                    cx.listener(|this, checked: &bool, _, cx| {
                        this.toggle_farm_order_method(FarmOrderMethod::Pickup, *checked, cx)
                    }),
                    cx.listener(|this, checked: &bool, _, cx| {
                        this.toggle_farm_order_method(FarmOrderMethod::Delivery, *checked, cx)
                    }),
                    cx.listener(|this, checked: &bool, _, cx| {
                        this.toggle_farm_order_method(FarmOrderMethod::Shipping, *checked, cx)
                    }),
                    cx.listener(|this, _, _, cx| this.finish_farm_setup(cx)),
                    cx,
                )
                .into_any_element(),
            );
        } else if let Some(spec) = setup_onboarding {
            sections.push(
                home_farm_setup_onboarding_card(
                    spec,
                    cx.listener(|this, _, window, cx| this.open_farm_setup(window, cx)),
                    cx,
                )
                .into_any_element(),
            );
        } else if projection.needs_setup() {
            sections.push(
                home_setup_card(
                    projection,
                    matches!(farm_state, FarmerHomeFarmState::IncompleteFarm).then_some(
                        action_button_primary(
                            "home-farm-setup-continue",
                            app_shared_text(AppTextKey::HomeFarmSetupContinueAction),
                            cx.listener(|this, _, window, cx| this.open_farm_setup(window, cx)),
                            cx,
                        )
                        .into_any_element(),
                    ),
                )
                .into_any_element(),
            );
        }

        if let Some(saved_farm_summary_card) = home_saved_farm_summary_card(runtime) {
            sections.push(saved_farm_summary_card);
        }

        if let Some(next_window) = projection.next_fulfillment_window.as_ref() {
            sections.push(
                home_next_fulfillment_window_card(
                    next_window,
                    Some(
                        action_button_compact(
                            "home-today-open-pack-day",
                            app_shared_text(AppTextKey::HomeTodayOpenInPackDayAction),
                            cx.listener(move |this, _, _, cx| {
                                this.open_today_next_window(next_fulfillment_window_id, cx)
                            }),
                            cx,
                        )
                        .into_any_element(),
                    ),
                )
                .into_any_element(),
            );
        }

        if !projection.orders_needing_action.is_empty() {
            sections.push(
                home_list_card(
                    AppTextKey::HomeTodayOrdersNeedingAction,
                    projection
                        .orders_needing_action
                        .iter()
                        .enumerate()
                        .map(|(index, order)| {
                            home_order_row(
                                index,
                                order,
                                cx.listener({
                                    let order_id = order.order_id;
                                    move |this, _, _, cx| this.open_order_detail(order_id, cx)
                                }),
                                cx,
                            )
                        })
                        .collect::<Vec<_>>(),
                    Some(
                        action_button_compact(
                            "home-today-open-orders",
                            app_shared_text(AppTextKey::HomeTodayOpenInOrdersAction),
                            cx.listener(|this, _, _, cx| this.open_orders(cx)),
                            cx,
                        )
                        .into_any_element(),
                    ),
                )
                .into_any_element(),
            );
        }

        if !projection.low_stock_products.is_empty() {
            sections.push(
                home_list_card(
                    AppTextKey::HomeTodayLowStock,
                    projection
                        .low_stock_products
                        .iter()
                        .map(home_low_stock_row)
                        .collect::<Vec<_>>(),
                    Some(
                        action_button_compact(
                            "home-today-open-products-low-stock",
                            app_shared_text(AppTextKey::HomeTodayOpenInProductsAction),
                            cx.listener(|this, _, _, cx| {
                                this.open_products_filter(ProductsFilter::NeedAttention, cx)
                            }),
                            cx,
                        )
                        .into_any_element(),
                    ),
                )
                .into_any_element(),
            );
        }

        if !projection.draft_products.is_empty() {
            sections.push(
                home_list_card(
                    AppTextKey::HomeTodayDraftProducts,
                    projection
                        .draft_products
                        .iter()
                        .map(home_draft_row)
                        .collect::<Vec<_>>(),
                    Some(
                        action_button_compact(
                            "home-today-open-products-drafts",
                            app_shared_text(AppTextKey::HomeTodayOpenInProductsAction),
                            cx.listener(|this, _, _, cx| {
                                this.open_products_filter(ProductsFilter::Drafts, cx)
                            }),
                            cx,
                        )
                        .into_any_element(),
                    ),
                )
                .into_any_element(),
            );
        }

        if runtime.startup_issue.is_none() && runtime.startup_gate == AppStartupGate::SetupRequired
        {
            sections.push(
                home_empty_state_card(
                    AppTextKey::HomeTodayEmptySetupTitle,
                    AppTextKey::HomeTodayEmptySetupBody,
                )
                .into_any_element(),
            );
        } else if runtime.startup_issue.is_none()
            && farm_state == FarmerHomeFarmState::NoFarm
            && setup_onboarding.is_none()
        {
            sections.push(
                home_empty_state_card(
                    AppTextKey::HomeTodayEmptyNoFarmTitle,
                    AppTextKey::HomeTodayEmptyNoFarmBody,
                )
                .into_any_element(),
            );
        } else if runtime.startup_issue.is_none()
            && farm_state == FarmerHomeFarmState::ConfiguredFarm
            && !projection.needs_setup()
            && projection.next_fulfillment_window.is_none()
            && !projection.has_attention_items()
        {
            sections.push(
                home_empty_state_card(
                    AppTextKey::HomeTodayEmptyQuietTitle,
                    AppTextKey::HomeTodayEmptyQuietBody,
                )
                .into_any_element(),
            );
        }

        div()
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(app_shared_text(AppTextKey::HomeTodayTitle)),
                    )
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .line_height(relative(1.2))
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .when_some(home_saved_farm(runtime), |this, farm| {
                                this.child(farm.display_name.clone())
                            })
                            .when(home_saved_farm(runtime).is_none(), |this| {
                                this.child(app_shared_text(home_status.label_key))
                            }),
                    )
                    .child(home_status_row(&home_status)),
            )
            .children(sections)
            .into_any_element()
    }

    fn render_buyer_workspace(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected_personal_section = selected_personal_section(runtime);
        let main_content = match selected_personal_section {
            PersonalSection::Browse => self
                .render_buyer_browse_content(runtime, cx)
                .into_any_element(),
            PersonalSection::Search => self
                .render_buyer_search_content(runtime, cx)
                .into_any_element(),
            PersonalSection::Cart => self
                .render_buyer_cart_content(runtime, cx)
                .into_any_element(),
            PersonalSection::Orders => self
                .render_buyer_orders_content(runtime, cx)
                .into_any_element(),
        };

        app_split_shell(
            buyer_sidebar(
                runtime,
                cx.listener(|this, _, _, cx| {
                    this.select_personal_section(PersonalSection::Browse, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_personal_section(PersonalSection::Search, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_personal_section(PersonalSection::Cart, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_personal_section(PersonalSection::Orders, cx)
                }),
                cx,
            )
            .into_any_element(),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .size_full()
                .child(shared_shell_header(
                    runtime,
                    cx.listener(|this, _, _, cx| this.switch_to_marketplace(cx)),
                    cx.listener(|this, _, _, cx| this.switch_to_farmer_workspace(cx)),
                    cx.listener(|this, _, _, cx| this.open_account_entry(cx)),
                    cx,
                ))
                .child(
                    app_scroll_panel(
                        buyer_content_scroll_id(selected_personal_section),
                        0.0,
                        None,
                        main_content,
                    )
                    .into_any_element(),
                )
                .into_any_element(),
        )
        .into_any_element()
    }

    fn render_buyer_browse_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let listings = &runtime.personal_projection.browse.listings.rows;
        let selected_product_id = runtime
            .personal_projection
            .browse
            .detail
            .as_ref()
            .map(|detail| detail.listing.product_id);

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(buyer_workspace_title_block(
                AppTextKey::HomeNavBrowse,
                AppTextKey::PersonalBrowsePlaceholderBody,
            ))
            .child(if listings.is_empty() {
                home_empty_state_card(
                    AppTextKey::PersonalBrowseEmptyTitle,
                    AppTextKey::PersonalBrowseEmptyBody,
                )
                .into_any_element()
            } else {
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .when_some(
                        runtime.personal_projection.browse.detail.as_ref(),
                        |this, detail| {
                            this.child(buyer_product_detail_card(
                                detail,
                                runtime
                                    .personal_projection
                                    .cart
                                    .cart
                                    .replace_confirmation
                                    .as_ref(),
                                cx.listener(|this, _, _, cx| {
                                    this.close_personal_product_detail(PersonalSection::Browse, cx)
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.decrease_personal_product_quantity(
                                        PersonalSection::Browse,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.increase_personal_product_quantity(
                                        PersonalSection::Browse,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.add_personal_product_to_cart(
                                        PersonalSection::Browse,
                                        false,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.add_personal_product_to_cart(
                                        PersonalSection::Browse,
                                        true,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.clear_personal_cart_replace_confirmation(cx)
                                }),
                                cx,
                            ))
                        },
                    )
                    .child(buyer_listings_feed(
                        PersonalSection::Browse,
                        listings,
                        selected_product_id,
                        cx,
                    ))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_buyer_search_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let query = &runtime.personal_projection.search.query;
        let listings = &runtime.personal_projection.search.listings.rows;
        let selected_product_id = runtime
            .personal_projection
            .search
            .detail
            .as_ref()
            .map(|detail| detail.listing.product_id);

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(buyer_workspace_title_block(
                AppTextKey::HomeNavSearch,
                AppTextKey::PersonalSearchPlaceholderBody,
            ))
            .child(
                home_card(
                    app_shared_text(AppTextKey::PersonalSearchFiltersTitle),
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                        .when_some(self.personal_search.as_ref(), |this, personal_search| {
                            this.child(
                                app_text_input(&personal_search.input, false)
                                    .cleanable(true)
                                    .w_full(),
                            )
                        })
                        .child(
                            app_cluster(8.0)
                                .child(choice_button(
                                    "personal-search-pickup",
                                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodPickup),
                                    query.fulfillment_methods.contains(&FarmOrderMethod::Pickup),
                                    cx.listener(|this, _, _, cx| {
                                        let enabled = !this
                                            .runtime
                                            .summary()
                                            .personal_projection
                                            .search
                                            .query
                                            .fulfillment_methods
                                            .contains(&FarmOrderMethod::Pickup);
                                        this.toggle_personal_search_fulfillment_method(
                                            FarmOrderMethod::Pickup,
                                            enabled,
                                            cx,
                                        )
                                    }),
                                    cx,
                                ))
                                .child(choice_button(
                                    "personal-search-delivery",
                                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodDelivery),
                                    query
                                        .fulfillment_methods
                                        .contains(&FarmOrderMethod::Delivery),
                                    cx.listener(|this, _, _, cx| {
                                        let enabled = !this
                                            .runtime
                                            .summary()
                                            .personal_projection
                                            .search
                                            .query
                                            .fulfillment_methods
                                            .contains(&FarmOrderMethod::Delivery);
                                        this.toggle_personal_search_fulfillment_method(
                                            FarmOrderMethod::Delivery,
                                            enabled,
                                            cx,
                                        )
                                    }),
                                    cx,
                                ))
                                .child(choice_button(
                                    "personal-search-shipping",
                                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodShipping),
                                    query
                                        .fulfillment_methods
                                        .contains(&FarmOrderMethod::Shipping),
                                    cx.listener(|this, _, _, cx| {
                                        let enabled = !this
                                            .runtime
                                            .summary()
                                            .personal_projection
                                            .search
                                            .query
                                            .fulfillment_methods
                                            .contains(&FarmOrderMethod::Shipping);
                                        this.toggle_personal_search_fulfillment_method(
                                            FarmOrderMethod::Shipping,
                                            enabled,
                                            cx,
                                        )
                                    }),
                                    cx,
                                )),
                        ),
                )
                .into_any_element(),
            )
            .child(if listings.is_empty() {
                home_empty_state_card(
                    AppTextKey::PersonalSearchEmptyTitle,
                    AppTextKey::PersonalSearchEmptyBody,
                )
                .into_any_element()
            } else {
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .when_some(
                        runtime.personal_projection.search.detail.as_ref(),
                        |this, detail| {
                            this.child(buyer_product_detail_card(
                                detail,
                                runtime
                                    .personal_projection
                                    .cart
                                    .cart
                                    .replace_confirmation
                                    .as_ref(),
                                cx.listener(|this, _, _, cx| {
                                    this.close_personal_product_detail(PersonalSection::Search, cx)
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.decrease_personal_product_quantity(
                                        PersonalSection::Search,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.increase_personal_product_quantity(
                                        PersonalSection::Search,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.add_personal_product_to_cart(
                                        PersonalSection::Search,
                                        false,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.add_personal_product_to_cart(
                                        PersonalSection::Search,
                                        true,
                                        cx,
                                    )
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.clear_personal_cart_replace_confirmation(cx)
                                }),
                                cx,
                            ))
                        },
                    )
                    .child(buyer_listings_feed(
                        PersonalSection::Search,
                        listings,
                        selected_product_id,
                        cx,
                    ))
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_buyer_cart_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let cart = &runtime.personal_projection.cart.cart;
        let checkout = &runtime.personal_projection.cart.checkout;

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(buyer_workspace_title_block(
                AppTextKey::HomeNavCart,
                AppTextKey::PersonalCartSurfaceBody,
            ))
            .child(if cart.lines.is_empty() {
                app_surface_card(home_body_text(app_shared_text(
                    AppTextKey::PersonalCartPlaceholderBody,
                )))
                .into_any_element()
            } else {
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .child(buyer_cart_card(
                        cart,
                        &checkout.summary,
                        self.buyer_checkout_form.is_some(),
                        cx,
                    ))
                    .when_some(self.buyer_checkout_form.as_ref(), |this, form| {
                        this.child(buyer_checkout_card(
                            form,
                            checkout,
                            cx.listener(|this, _, _, cx| this.close_personal_checkout(cx)),
                            cx.listener(|this, _, _, cx| this.place_personal_order(cx)),
                            cx,
                        ))
                    })
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_buyer_orders_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let orders = &runtime.personal_projection.orders;
        let selected_order_id = orders.detail.as_ref().map(|detail| detail.order_id);

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(buyer_workspace_title_block(
                AppTextKey::HomeNavOrders,
                AppTextKey::PersonalOrdersSurfaceBody,
            ))
            .child(if orders.list.rows.is_empty() {
                home_empty_state_card(
                    AppTextKey::PersonalOrdersEmptyTitle,
                    AppTextKey::PersonalOrdersEmptyBody,
                )
                .into_any_element()
            } else {
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .child(buyer_orders_list_card(
                        &orders.list.rows,
                        selected_order_id,
                        cx,
                    ))
                    .child(
                        orders
                            .detail
                            .as_ref()
                            .map(buyer_order_detail_card)
                            .unwrap_or_else(|| buyer_order_detail_empty_card().into_any_element()),
                    )
                    .into_any_element()
            })
            .into_any_element()
    }

    fn render_farmer_workspace(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected_farmer_section = selected_farmer_section(runtime);
        let main_content = match selected_farmer_section {
            FarmerSection::Products if farmer_products_available(runtime) => {
                self.render_products_content(runtime, cx)
            }
            FarmerSection::Orders if farmer_products_available(runtime) => {
                self.render_orders_content(runtime, cx)
            }
            FarmerSection::PackDay if farmer_pack_day_available(runtime) => {
                self.render_pack_day_content(runtime, cx)
            }
            FarmerSection::Today
            | FarmerSection::Products
            | FarmerSection::Orders
            | FarmerSection::PackDay
            | FarmerSection::Farm => self.render_today_content(runtime, cx),
        };

        app_split_shell(
            home_sidebar(
                runtime,
                cx.listener(|this, _, _, cx| this.select_farmer_section(FarmerSection::Today, cx)),
                cx.listener(|this, _, _, cx| {
                    this.select_farmer_section(FarmerSection::Products, cx)
                }),
                cx.listener(|this, _, _, cx| this.open_orders(cx)),
                cx.listener(|this, _, _, cx| this.open_pack_day(None, cx)),
                cx,
            )
            .into_any_element(),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .size_full()
                .child(shared_shell_header(
                    runtime,
                    cx.listener(|this, _, _, cx| this.switch_to_marketplace(cx)),
                    cx.listener(|this, _, _, cx| this.switch_to_farmer_workspace(cx)),
                    cx.listener(|this, _, _, cx| this.open_account_entry(cx)),
                    cx,
                ))
                .child(
                    app_scroll_panel(
                        home_content_scroll_id(selected_farmer_section),
                        0.0,
                        None,
                        main_content,
                    )
                    .into_any_element(),
                )
                .into_any_element(),
        )
        .into_any_element()
    }

    fn render_products_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let projection = &runtime.products_projection;
        let summary = &projection.list.summary;

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(products_title_row(
                runtime,
                action_button_primary(
                    "products-add-product",
                    app_shared_text(AppTextKey::ProductsAddAction),
                    cx.listener(|this, _, _, cx| this.open_new_product_editor(cx)),
                    cx,
                )
                .into_any_element(),
            ))
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(home_summary_metric(
                        AppTextKey::ProductsSummaryTotal,
                        summary.total_products,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::ProductsSummaryLive,
                        summary.live_products,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::ProductsSummaryNeedAttention,
                        summary.need_attention_products,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::ProductsSummaryDrafts,
                        summary.draft_products,
                    )),
            )
            .child(products_controls_card(
                runtime,
                self.products_search.as_ref(),
                cx.listener(|this, _, _, cx| this.select_products_filter(ProductsFilter::All, cx)),
                cx.listener(|this, _, _, cx| this.select_products_filter(ProductsFilter::Live, cx)),
                cx.listener(|this, _, _, cx| {
                    this.select_products_filter(ProductsFilter::Drafts, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_products_filter(ProductsFilter::NeedAttention, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_products_filter(ProductsFilter::Paused, cx)
                }),
                cx.listener(|this, _, _, cx| {
                    this.select_products_filter(ProductsFilter::Archived, cx)
                }),
                cx.listener(|this, _, _, cx| this.select_products_sort(ProductsSort::Updated, cx)),
                cx.listener(|this, _, _, cx| this.select_products_sort(ProductsSort::Name, cx)),
                cx.listener(|this, _, _, cx| {
                    this.select_products_sort(ProductsSort::Availability, cx)
                }),
                cx.listener(|this, _, _, cx| this.select_products_sort(ProductsSort::Stock, cx)),
                cx.listener(|this, _, _, cx| this.select_products_sort(ProductsSort::Price, cx)),
                cx,
            ))
            .when_some(self.product_editor_form.as_ref(), |this, form| {
                this.child(products_editor_surface(
                    form,
                    runtime,
                    cx.listener(|this, _, _, cx| {
                        this.select_product_editor_status(ProductStatus::Draft, cx)
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.select_product_editor_status(ProductStatus::Published, cx)
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.select_product_editor_status(ProductStatus::Paused, cx)
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.select_product_editor_status(ProductStatus::Archived, cx)
                    }),
                    cx.listener(|this, _, _, cx| this.close_product_editor(cx)),
                    cx.listener(|this, _, _, cx| this.save_product_editor(cx)),
                    cx,
                ))
            })
            .child(if projection.list.is_empty() {
                products_empty_state_card(projection.query.filter).into_any_element()
            } else {
                self.render_products_table_card(&projection.list.rows, cx)
            })
            .into_any_element()
    }

    fn render_orders_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let projection = &runtime.orders_projection;
        let summary = &projection.list.summary;

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(app_text_value(app_shared_text(AppTextKey::OrdersTitle)))
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(home_summary_metric(
                        AppTextKey::OrdersSummaryTotal,
                        summary.total_orders,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::OrdersStatusNeedsAction,
                        summary.needs_action_orders,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::OrdersStatusScheduled,
                        summary.scheduled_orders,
                    ))
                    .child(home_summary_metric(
                        AppTextKey::OrdersStatusPacked,
                        summary.packed_orders,
                    )),
            )
            .child(home_card(
                app_shared_text(AppTextKey::OrdersFiltersTitle),
                app_cluster(APP_UI_THEME.foundation.spacing.tight_px)
                    .child(choice_button(
                        "orders-filter-all",
                        app_shared_text(AppTextKey::OrdersFilterAll),
                        projection.query.filter == OrdersFilter::All,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::All, cx)
                        }),
                        cx,
                    ))
                    .child(choice_button(
                        "orders-filter-needs-action",
                        app_shared_text(AppTextKey::OrdersStatusNeedsAction),
                        projection.query.filter == OrdersFilter::NeedsAction,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::NeedsAction, cx)
                        }),
                        cx,
                    ))
                    .child(choice_button(
                        "orders-filter-scheduled",
                        app_shared_text(AppTextKey::OrdersStatusScheduled),
                        projection.query.filter == OrdersFilter::Scheduled,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::Scheduled, cx)
                        }),
                        cx,
                    ))
                    .child(choice_button(
                        "orders-filter-packed",
                        app_shared_text(AppTextKey::OrdersStatusPacked),
                        projection.query.filter == OrdersFilter::Packed,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::Packed, cx)
                        }),
                        cx,
                    ))
                    .child(choice_button(
                        "orders-filter-completed",
                        app_shared_text(AppTextKey::OrdersStatusCompleted),
                        projection.query.filter == OrdersFilter::Completed,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::Completed, cx)
                        }),
                        cx,
                    ))
                    .child(choice_button(
                        "orders-filter-refunded",
                        app_shared_text(AppTextKey::OrdersStatusRefunded),
                        projection.query.filter == OrdersFilter::Refunded,
                        cx.listener(|this, _, _, cx| {
                            this.select_orders_filter(OrdersFilter::Refunded, cx)
                        }),
                        cx,
                    )),
            ))
            .child(if projection.list.is_empty() {
                orders_empty_state_card(projection.query.filter).into_any_element()
            } else {
                self.render_orders_table_card(
                    &projection.list.rows,
                    projection.detail.as_ref().map(|detail| detail.order_id),
                    cx,
                )
            })
            .when_some(projection.detail.as_ref(), |this, detail| {
                this.child(self.render_order_detail_card(detail, cx))
            })
            .when(
                projection.detail.is_none() && !projection.list.is_empty(),
                |this| {
                    this.child(
                        home_card(
                            app_shared_text(AppTextKey::OrdersDetailTitle),
                            home_body_text(app_shared_text(AppTextKey::OrdersDetailEmptyBody)),
                        )
                        .into_any_element(),
                    )
                },
            )
            .into_any_element()
    }

    fn render_pack_day_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        _: &mut Context<Self>,
    ) -> AnyElement {
        let projection = &runtime.pack_day_projection.projection;
        let Some(fulfillment_window) = projection.fulfillment_window.as_ref() else {
            return home_empty_state_card(
                AppTextKey::PackDayEmptyTitle,
                AppTextKey::PackDayEmptyBody,
            )
            .into_any_element();
        };

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(pack_day_title_row(runtime))
            .child(pack_day_window_summary_card(fulfillment_window))
            .when(!projection.totals_by_product.is_empty(), |this| {
                this.child(pack_day_totals_card(&projection.totals_by_product))
            })
            .when(!projection.pack_list.is_empty(), |this| {
                this.child(pack_day_pack_list_card(&projection.pack_list))
            })
            .when(!projection.pickup_roster.is_empty(), |this| {
                this.child(pack_day_pickup_roster_card(&projection.pickup_roster))
            })
            .when(projection.is_empty(), |this| {
                this.child(home_empty_state_card(
                    AppTextKey::PackDayEmptyTitle,
                    AppTextKey::PackDayEmptyBody,
                ))
            })
            .into_any_element()
    }

    fn render_products_table_card(
        &mut self,
        rows: &[ProductsListRow],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut items = Vec::with_capacity(rows.len().saturating_mul(2));
        for (index, row) in rows.iter().enumerate() {
            items.push(self.render_products_table_entry(index, row, cx));
            if index + 1 < rows.len() {
                items.push(section_divider().into_any_element());
            }
        }

        home_card(
            app_shared_text(AppTextKey::ProductsTableTitle),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(products_table_header())
                .child(section_divider())
                .children(items),
        )
        .into_any_element()
    }

    fn render_orders_table_card(
        &mut self,
        rows: &[OrdersListRow],
        selected_order_id: Option<OrderId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut items = Vec::with_capacity(rows.len().saturating_mul(2));
        for (index, row) in rows.iter().enumerate() {
            items.push(self.render_orders_table_entry(index, row, selected_order_id, cx));
            if index + 1 < rows.len() {
                items.push(section_divider().into_any_element());
            }
        }

        home_card(
            app_shared_text(AppTextKey::OrdersTableTitle),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(orders_table_header())
                .child(section_divider())
                .children(items),
        )
        .into_any_element()
    }

    fn render_order_detail_card(
        &mut self,
        detail: &OrderDetailProjection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let primary_action = match detail.primary_action {
            Some(OrderPrimaryAction::MarkPacked) => Some(
                action_button_primary(
                    "orders-detail-mark-packed",
                    app_shared_text(AppTextKey::OrdersActionMarkPacked),
                    cx.listener({
                        let order_id = detail.order_id;
                        move |this, _, _, cx| this.mark_order_packed(order_id, cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            Some(OrderPrimaryAction::MarkCompleted) => Some(
                action_button_primary(
                    "orders-detail-mark-completed",
                    app_shared_text(AppTextKey::OrdersActionMarkCompleted),
                    cx.listener({
                        let order_id = detail.order_id;
                        move |this, _, _, cx| this.mark_order_completed(order_id, cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            Some(OrderPrimaryAction::Review) | None => None,
        };

        home_card(
            app_shared_text(AppTextKey::OrdersDetailTitle),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .child(app_heading_section(detail.order_number.clone()))
                .child(home_body_text(detail.customer_display_name.clone()))
                .child(label_value_list([
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailCustomerLabel),
                        detail.customer_display_name.clone(),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailStatusLabel),
                        app_shared_text(orders_status_key(detail.status)),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailWindowLabel),
                        order_optional_text(detail.fulfillment_window_label.as_deref()),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailPickupLabel),
                        order_optional_text(detail.pickup_location_label.as_deref()),
                    ),
                ]))
                .child(app_form_section(
                    app_shared_text(AppTextKey::OrdersDetailItemsTitle),
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                        .children(
                            detail
                                .items
                                .iter()
                                .map(order_detail_item_row)
                                .collect::<Vec<_>>(),
                        )
                        .when(detail.items.is_empty(), |this| {
                            this.child(home_body_text(app_shared_text(AppTextKey::ValueNone)))
                        }),
                ))
                .when_some(primary_action, |this, primary_action| {
                    this.child(div().child(primary_action))
                }),
        )
        .into_any_element()
    }

    fn render_products_table_entry(
        &mut self,
        index: usize,
        row: &ProductsListRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_open = self
            .product_editor_form
            .as_ref()
            .map(|form| form.product_id == row.product_id)
            .unwrap_or(false);
        let is_editing = self
            .products_stock_editor
            .as_ref()
            .map(|editor| editor.product_id == row.product_id)
            .unwrap_or(false);
        let product = list_row_button(
            ("products-row-open", index),
            product_display_title(row.title.as_str()),
            row.subtitle.clone().map(SharedString::from),
            is_open,
            cx.listener({
                let product_id = row.product_id;
                move |this, _, _, cx| this.open_existing_product_editor(product_id, cx)
            }),
            cx,
        )
        .into_any_element();
        let action = if is_editing {
            action_button_compact(
                "products-stock-editor-cancel",
                app_shared_text(AppTextKey::ProductsStockEditorCancelAction),
                cx.listener(|this, _, _, cx| this.close_products_stock_editor(cx)),
                cx,
            )
            .into_any_element()
        } else {
            action_button_compact(
                ("products-row-stock-action", index),
                app_shared_text(AppTextKey::ProductsUpdateStockAction),
                cx.listener({
                    let product_id = row.product_id;
                    let stock_quantity = row.stock.quantity;
                    move |this, _, window, cx| {
                        this.open_products_stock_editor(product_id, stock_quantity, window, cx)
                    }
                }),
                cx,
            )
            .into_any_element()
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(products_table_row(product, row, action))
            .when(is_editing, |this| {
                this.when_some(self.products_stock_editor.as_ref(), |this, editor| {
                    this.child(products_stock_editor_card(
                        row,
                        editor,
                        cx.listener(|this, _, _, cx| this.save_products_stock_editor(cx)),
                        cx.listener(|this, _, _, cx| this.close_products_stock_editor(cx)),
                        cx,
                    ))
                })
            })
            .into_any_element()
    }

    fn render_orders_table_entry(
        &mut self,
        index: usize,
        row: &OrdersListRow,
        selected_order_id: Option<OrderId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_selected = selected_order_id.is_some_and(|order_id| order_id == row.order_id);
        let order = list_row_button(
            ("orders-row-open", index),
            row.order_number.clone(),
            Some(SharedString::from(row.customer_display_name.clone())),
            is_selected,
            cx.listener({
                let order_id = row.order_id;
                move |this, _, _, cx| this.open_order_detail(order_id, cx)
            }),
            cx,
        )
        .into_any_element();
        let action = orders_table_action(
            index,
            row,
            cx.listener({
                let order_id = row.order_id;
                move |this, _, _, cx| this.open_order_detail(order_id, cx)
            }),
            cx.listener({
                let order_id = row.order_id;
                move |this, _, _, cx| this.mark_order_packed(order_id, cx)
            }),
            cx.listener({
                let order_id = row.order_id;
                move |this, _, _, cx| this.mark_order_completed(order_id, cx)
            }),
            cx,
        );

        div()
            .w_full()
            .child(orders_table_row(order, row, action))
            .into_any_element()
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime_summary = self.runtime.summary();
        self.sync_startup_signer_entry(&runtime_summary, window, cx);
        self.sync_farm_setup_form(&runtime_summary, window, cx);
        self.sync_personal_search(&runtime_summary, window, cx);
        self.sync_buyer_checkout_form(&runtime_summary, window, cx);
        self.sync_products_search(&runtime_summary, window, cx);
        self.sync_products_stock_editor(&runtime_summary);
        self.sync_product_editor_form(&runtime_summary, window, cx);
        match home_stage(&runtime_summary) {
            HomeStage::Setup => self
                .startup_view
                .render(
                    &runtime_summary,
                    self.startup_signer_entry.as_ref(),
                    &self.startup_signer_connect_state,
                    cx.listener(|this, _, _, cx| this.show_startup_identity_choice(cx)),
                    cx.listener(|this, _, _, cx| {
                        this.select_personal_section(PersonalSection::Browse, cx)
                    }),
                    cx.listener(|this, _, window, cx| this.start_generate_key(window, cx)),
                    cx.listener(|this, _, _, cx| this.show_startup_signer_entry(cx)),
                    cx.listener(|this, _, window, cx| this.submit_startup_signer(window, cx)),
                    cx.listener(|this, _, _, cx| this.back_out_of_startup_signer_entry(cx)),
                    cx,
                )
                .into_any_element(),
            HomeStage::BuyerWorkspace => self.render_buyer_workspace(&runtime_summary, cx),
            HomeStage::FarmerWorkspace => self.render_farmer_workspace(&runtime_summary, cx),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FarmSetupSaveState {
    AutosavesLocally,
    SavedLocally,
    SaveFailed,
}

struct FarmSetupFormState {
    account_id: String,
    draft: FarmSetupDraft,
    farm_name_input: Entity<InputState>,
    location_input: Entity<InputState>,
    _farm_name_subscription: Subscription,
    _location_subscription: Subscription,
    save_state: FarmSetupSaveState,
}

impl FarmSetupFormState {
    fn new(
        account_id: String,
        draft: FarmSetupDraft,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let farm_name_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.farm_name.clone()));
        let location_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(draft.location_or_service_area.clone())
        });
        let farm_name_subscription = cx.subscribe_in(
            &farm_name_input,
            window,
            HomeView::handle_farm_name_input_event,
        );
        let location_subscription = cx.subscribe_in(
            &location_input,
            window,
            HomeView::handle_location_input_event,
        );
        let save_state = if draft.is_empty() {
            FarmSetupSaveState::AutosavesLocally
        } else {
            FarmSetupSaveState::SavedLocally
        };

        Self {
            account_id,
            draft,
            farm_name_input,
            location_input,
            _farm_name_subscription: farm_name_subscription,
            _location_subscription: location_subscription,
            save_state,
        }
    }
}

struct PersonalSearchState {
    workspace_id: String,
    input: Entity<InputState>,
    _input_subscription: Subscription,
}

impl PersonalSearchState {
    fn new(
        workspace_id: String,
        search_query: &str,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(app_shared_text(AppTextKey::PersonalSearchPlaceholder))
                .default_value(search_query.to_owned())
        });
        let input_subscription =
            cx.subscribe_in(&input, window, HomeView::handle_personal_search_input_event);

        Self {
            workspace_id,
            input,
            _input_subscription: input_subscription,
        }
    }

    fn sync(&mut self, search_query: &str, window: &mut Window, cx: &mut Context<HomeView>) {
        if self.input.read(cx).value().as_ref() == search_query {
            return;
        }

        self.input.update(cx, |input, cx| {
            input.set_value(search_query.to_owned(), window, cx);
        });
    }
}

struct BuyerCheckoutFormState {
    workspace_id: String,
    name_input: Entity<InputState>,
    email_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    order_note_input: Entity<InputState>,
    _name_subscription: Subscription,
    _email_subscription: Subscription,
    _phone_subscription: Subscription,
    _order_note_subscription: Subscription,
}

impl BuyerCheckoutFormState {
    fn new(
        workspace_id: String,
        draft: &BuyerCheckoutDraft,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).default_value(draft.name.clone()));
        let email_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.email.clone()));
        let phone_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.phone.clone()));
        let order_note_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.order_note.clone()));
        let name_subscription = cx.subscribe_in(
            &name_input,
            window,
            HomeView::handle_buyer_checkout_input_event,
        );
        let email_subscription = cx.subscribe_in(
            &email_input,
            window,
            HomeView::handle_buyer_checkout_input_event,
        );
        let phone_subscription = cx.subscribe_in(
            &phone_input,
            window,
            HomeView::handle_buyer_checkout_input_event,
        );
        let order_note_subscription = cx.subscribe_in(
            &order_note_input,
            window,
            HomeView::handle_buyer_checkout_input_event,
        );

        Self {
            workspace_id,
            name_input,
            email_input,
            phone_input,
            order_note_input,
            _name_subscription: name_subscription,
            _email_subscription: email_subscription,
            _phone_subscription: phone_subscription,
            _order_note_subscription: order_note_subscription,
        }
    }

    fn sync(
        &mut self,
        draft: &BuyerCheckoutDraft,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) {
        sync_checkout_input(&self.name_input, draft.name.as_str(), window, cx);
        sync_checkout_input(&self.email_input, draft.email.as_str(), window, cx);
        sync_checkout_input(&self.phone_input, draft.phone.as_str(), window, cx);
        sync_checkout_input(
            &self.order_note_input,
            draft.order_note.as_str(),
            window,
            cx,
        );
    }

    fn current_draft(&self, cx: &App) -> BuyerCheckoutDraft {
        BuyerCheckoutDraft {
            name: self.name_input.read(cx).value().to_string(),
            email: self.email_input.read(cx).value().to_string(),
            phone: self.phone_input.read(cx).value().to_string(),
            order_note: self.order_note_input.read(cx).value().to_string(),
        }
    }
}

fn sync_checkout_input(
    input: &Entity<InputState>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<HomeView>,
) {
    if input.read(cx).value().as_ref() == value {
        return;
    }

    input.update(cx, |input, cx| {
        input.set_value(value.to_owned(), window, cx);
    });
}

struct ProductsSearchState {
    account_id: String,
    input: Entity<InputState>,
    _input_subscription: Subscription,
}

impl ProductsSearchState {
    fn new(
        account_id: String,
        search_query: &str,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(app_shared_text(AppTextKey::ProductsSearchPlaceholder))
                .default_value(search_query.to_owned())
        });
        let input_subscription =
            cx.subscribe_in(&input, window, HomeView::handle_products_search_input_event);

        Self {
            account_id,
            input,
            _input_subscription: input_subscription,
        }
    }

    fn sync(&mut self, search_query: &str, window: &mut Window, cx: &mut Context<HomeView>) {
        if self.input.read(cx).value().as_ref() == search_query {
            return;
        }

        self.input.update(cx, |input, cx| {
            input.set_value(search_query.to_owned(), window, cx);
        });
    }
}

struct StartupSignerEntryState {
    input: Entity<InputState>,
    _input_subscription: Subscription,
}

impl StartupSignerEntryState {
    fn new(source_input: &str, window: &mut Window, cx: &mut Context<HomeView>) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(app_shared_text(
                    AppTextKey::HomeSetupSignerSourcePlaceholder,
                ))
                .default_value(source_input.to_owned())
        });
        let input_subscription =
            cx.subscribe_in(&input, window, HomeView::handle_startup_signer_input_event);

        Self {
            input,
            _input_subscription: input_subscription,
        }
    }

    fn sync(&mut self, source_input: &str, window: &mut Window, cx: &mut Context<HomeView>) {
        if self.input.read(cx).value().as_ref() == source_input {
            return;
        }

        self.input.update(cx, |input, cx| {
            input.set_value(source_input.to_owned(), window, cx);
        });
    }
}

struct ProductsStockEditorState {
    account_id: String,
    product_id: ProductId,
    initial_stock_quantity: Option<u32>,
    input: Entity<InputState>,
    _input_subscription: Subscription,
    save_failed: bool,
}

impl ProductsStockEditorState {
    fn new(
        account_id: String,
        product_id: ProductId,
        stock_quantity: Option<u32>,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(app_shared_text(AppTextKey::ProductsStockEditorFieldLabel))
                .default_value(
                    stock_quantity
                        .map(|quantity| quantity.to_string())
                        .unwrap_or_else(|| "0".to_owned()),
                )
        });
        let input_subscription =
            cx.subscribe_in(&input, window, HomeView::handle_products_stock_input_event);

        Self {
            account_id,
            product_id,
            initial_stock_quantity: stock_quantity,
            input,
            _input_subscription: input_subscription,
            save_failed: false,
        }
    }

    fn parsed_stock_quantity(&self, cx: &App) -> Option<u32> {
        parse_products_stock_quantity(self.input.read(cx).value().as_ref())
    }

    fn has_changes(&self, cx: &App) -> bool {
        self.parsed_stock_quantity(cx)
            .map(|stock_quantity| Some(stock_quantity) != self.initial_stock_quantity)
            .unwrap_or(false)
    }
}

struct ProductEditorFormState {
    account_id: String,
    product_id: ProductId,
    initial_draft: ProductEditorDraft,
    status: ProductStatus,
    title_input: Entity<InputState>,
    subtitle_input: Entity<InputState>,
    unit_input: Entity<InputState>,
    price_input: Entity<InputState>,
    stock_input: Entity<InputState>,
    _title_subscription: Subscription,
    _subtitle_subscription: Subscription,
    _unit_subscription: Subscription,
    _price_subscription: Subscription,
    _stock_subscription: Subscription,
    save_failed: bool,
}

impl ProductEditorFormState {
    fn new(
        account_id: String,
        product_id: ProductId,
        draft: ProductEditorDraft,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) -> Self {
        let title_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.title.clone()));
        let subtitle_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.subtitle.clone()));
        let unit_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.unit_label.clone()));
        let price_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(product_editor_price_input_value(draft.price_minor_units))
        });
        let stock_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                draft
                    .stock_quantity
                    .map(|quantity| quantity.to_string())
                    .unwrap_or_default(),
            )
        });
        let title_subscription = cx.subscribe_in(
            &title_input,
            window,
            HomeView::handle_product_editor_input_event,
        );
        let subtitle_subscription = cx.subscribe_in(
            &subtitle_input,
            window,
            HomeView::handle_product_editor_input_event,
        );
        let unit_subscription = cx.subscribe_in(
            &unit_input,
            window,
            HomeView::handle_product_editor_input_event,
        );
        let price_subscription = cx.subscribe_in(
            &price_input,
            window,
            HomeView::handle_product_editor_input_event,
        );
        let stock_subscription = cx.subscribe_in(
            &stock_input,
            window,
            HomeView::handle_product_editor_input_event,
        );

        Self {
            account_id,
            product_id,
            status: draft.status,
            initial_draft: draft,
            title_input,
            subtitle_input,
            unit_input,
            price_input,
            stock_input,
            _title_subscription: title_subscription,
            _subtitle_subscription: subtitle_subscription,
            _unit_subscription: unit_subscription,
            _price_subscription: price_subscription,
            _stock_subscription: stock_subscription,
            save_failed: false,
        }
    }

    fn current_draft(&self, cx: &App) -> Option<ProductEditorDraft> {
        Some(ProductEditorDraft {
            title: self.title_input.read(cx).value().to_string(),
            subtitle: self.subtitle_input.read(cx).value().to_string(),
            unit_label: self.unit_input.read(cx).value().to_string(),
            price_minor_units: parse_product_editor_price_input(
                self.price_input.read(cx).value().as_ref(),
            )?,
            price_currency: "USD".to_owned(),
            stock_quantity: parse_optional_product_editor_stock_input(
                self.stock_input.read(cx).value().as_ref(),
            )?,
            availability_window_id: self.initial_draft.availability_window_id,
            status: self.status,
        })
    }

    fn has_changes(&self, cx: &App) -> bool {
        self.current_draft(cx)
            .map(|draft| draft != self.initial_draft)
            .unwrap_or(false)
    }

    fn publish_blockers(&self, cx: &App) -> Vec<ProductPublishBlocker> {
        self.current_draft(cx)
            .map(|draft| draft.publish_blockers())
            .unwrap_or_default()
    }
}

struct StartupHomeView {
    startup_notice: Option<String>,
}

impl StartupHomeView {
    fn new() -> Self {
        Self {
            startup_notice: None,
        }
    }

    fn set_notice(&mut self, notice: String) {
        self.startup_notice = Some(notice);
    }

    fn clear_notice(&mut self) {
        self.startup_notice = None;
    }

    fn render(
        &self,
        runtime: &DesktopAppRuntimeSummary,
        signer_entry: Option<&StartupSignerEntryState>,
        connect_state: &StartupSignerConnectState,
        on_continue: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_browse_marketplace: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_generate_key: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_connect_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> impl IntoElement {
        startup_home_shell(
            runtime,
            self.startup_notice.as_deref(),
            signer_entry,
            connect_state,
            on_continue,
            on_browse_marketplace,
            on_generate_key,
            on_connect_signer,
            on_submit_signer,
            on_back,
            cx,
        )
    }
}

struct SettingsPickupLocationFormState {
    pickup_location_id: PickupLocationId,
    label_input: Entity<InputState>,
    address_input: Entity<InputState>,
    directions_input: Entity<InputState>,
    is_default: bool,
    can_remove: bool,
    _label_subscription: Subscription,
    _address_subscription: Subscription,
    _directions_subscription: Subscription,
}

impl SettingsPickupLocationFormState {
    fn new(
        record: &PickupLocationRecord,
        can_remove: bool,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) -> Self {
        let label_input =
            cx.new(|cx| InputState::new(window, cx).default_value(record.label.clone()));
        let address_input =
            cx.new(|cx| InputState::new(window, cx).default_value(record.address_line.clone()));
        let directions_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(record.directions.clone().unwrap_or_default())
        });
        let label_subscription = cx.subscribe_in(
            &label_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let address_subscription = cx.subscribe_in(
            &address_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let directions_subscription = cx.subscribe_in(
            &directions_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );

        Self {
            pickup_location_id: record.pickup_location_id,
            label_input,
            address_input,
            directions_input,
            is_default: record.is_default,
            can_remove,
            _label_subscription: label_subscription,
            _address_subscription: address_subscription,
            _directions_subscription: directions_subscription,
        }
    }

    fn current_draft(&self, cx: &App) -> SettingsPickupLocationDraft {
        SettingsPickupLocationDraft {
            pickup_location_id: self.pickup_location_id,
            label: self.label_input.read(cx).value().to_string(),
            address_line: self.address_input.read(cx).value().to_string(),
            directions: self.directions_input.read(cx).value().to_string(),
            is_default: self.is_default,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SettingsPickupLocationDraft {
    pickup_location_id: PickupLocationId,
    label: String,
    address_line: String,
    directions: String,
    is_default: bool,
}

impl SettingsPickupLocationDraft {
    fn from_record(record: &PickupLocationRecord) -> Self {
        Self {
            pickup_location_id: record.pickup_location_id,
            label: record.label.clone(),
            address_line: record.address_line.clone(),
            directions: record.directions.clone().unwrap_or_default(),
            is_default: record.is_default,
        }
    }

    fn into_record(self, farm_id: FarmId) -> PickupLocationRecord {
        let directions = self.directions.trim().to_owned();

        PickupLocationRecord {
            pickup_location_id: self.pickup_location_id,
            farm_id,
            label: self.label.trim().to_owned(),
            address_line: self.address_line.trim().to_owned(),
            directions: (!directions.is_empty()).then_some(directions),
            is_default: self.is_default,
        }
    }
}

struct SettingsOperatingRulesFormState {
    promise_lead_hours_input: Entity<InputState>,
    substitution_policy_input: Entity<InputState>,
    missed_pickup_policy_input: Entity<InputState>,
    _promise_lead_hours_subscription: Subscription,
    _substitution_policy_subscription: Subscription,
    _missed_pickup_policy_subscription: Subscription,
}

impl SettingsOperatingRulesFormState {
    fn new(
        record: Option<&FarmOperatingRulesRecord>,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) -> Self {
        let promise_lead_hours_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                record
                    .map(|record| record.promise_lead_hours.to_string())
                    .unwrap_or_default(),
            )
        });
        let substitution_policy_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                record
                    .map(|record| record.substitution_policy.clone())
                    .unwrap_or_default(),
            )
        });
        let missed_pickup_policy_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                record
                    .map(|record| record.missed_pickup_policy.clone())
                    .unwrap_or_default(),
            )
        });
        let promise_lead_hours_subscription = cx.subscribe_in(
            &promise_lead_hours_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let substitution_policy_subscription = cx.subscribe_in(
            &substitution_policy_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let missed_pickup_policy_subscription = cx.subscribe_in(
            &missed_pickup_policy_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );

        Self {
            promise_lead_hours_input,
            substitution_policy_input,
            missed_pickup_policy_input,
            _promise_lead_hours_subscription: promise_lead_hours_subscription,
            _substitution_policy_subscription: substitution_policy_subscription,
            _missed_pickup_policy_subscription: missed_pickup_policy_subscription,
        }
    }

    fn current_draft(&self, cx: &App) -> SettingsOperatingRulesDraft {
        SettingsOperatingRulesDraft {
            promise_lead_hours: self.promise_lead_hours_input.read(cx).value().to_string(),
            substitution_policy: self.substitution_policy_input.read(cx).value().to_string(),
            missed_pickup_policy: self.missed_pickup_policy_input.read(cx).value().to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SettingsOperatingRulesDraft {
    promise_lead_hours: String,
    substitution_policy: String,
    missed_pickup_policy: String,
}

impl SettingsOperatingRulesDraft {
    fn from_record(record: Option<&FarmOperatingRulesRecord>) -> Self {
        Self {
            promise_lead_hours: record
                .map(|record| record.promise_lead_hours.to_string())
                .unwrap_or_default(),
            substitution_policy: record
                .map(|record| record.substitution_policy.clone())
                .unwrap_or_default(),
            missed_pickup_policy: record
                .map(|record| record.missed_pickup_policy.clone())
                .unwrap_or_default(),
        }
    }

    fn is_empty(&self) -> bool {
        self.promise_lead_hours.trim().is_empty()
            && self.substitution_policy.trim().is_empty()
            && self.missed_pickup_policy.trim().is_empty()
    }
}

struct SettingsFulfillmentWindowFormState {
    fulfillment_window_id: FulfillmentWindowId,
    selected_pickup_location_id: Option<PickupLocationId>,
    label_input: Entity<InputState>,
    starts_at_input: Entity<InputState>,
    ends_at_input: Entity<InputState>,
    order_cutoff_input: Entity<InputState>,
    _label_subscription: Subscription,
    _starts_at_subscription: Subscription,
    _ends_at_subscription: Subscription,
    _order_cutoff_subscription: Subscription,
}

impl SettingsFulfillmentWindowFormState {
    fn new(
        draft: &SettingsFulfillmentWindowDraft,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) -> Self {
        let label_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.label.clone()));
        let starts_at_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.starts_at.clone()));
        let ends_at_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.ends_at.clone()));
        let order_cutoff_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.order_cutoff_at.clone()));
        let label_subscription = cx.subscribe_in(
            &label_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let starts_at_subscription = cx.subscribe_in(
            &starts_at_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let ends_at_subscription = cx.subscribe_in(
            &ends_at_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let order_cutoff_subscription = cx.subscribe_in(
            &order_cutoff_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );

        Self {
            fulfillment_window_id: draft.fulfillment_window_id,
            selected_pickup_location_id: draft.selected_pickup_location_id,
            label_input,
            starts_at_input,
            ends_at_input,
            order_cutoff_input,
            _label_subscription: label_subscription,
            _starts_at_subscription: starts_at_subscription,
            _ends_at_subscription: ends_at_subscription,
            _order_cutoff_subscription: order_cutoff_subscription,
        }
    }

    fn current_draft(&self, cx: &App) -> SettingsFulfillmentWindowDraft {
        SettingsFulfillmentWindowDraft {
            fulfillment_window_id: self.fulfillment_window_id,
            selected_pickup_location_id: self.selected_pickup_location_id,
            label: self.label_input.read(cx).value().to_string(),
            starts_at: self.starts_at_input.read(cx).value().to_string(),
            ends_at: self.ends_at_input.read(cx).value().to_string(),
            order_cutoff_at: self.order_cutoff_input.read(cx).value().to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SettingsFulfillmentWindowDraft {
    fulfillment_window_id: FulfillmentWindowId,
    selected_pickup_location_id: Option<PickupLocationId>,
    label: String,
    starts_at: String,
    ends_at: String,
    order_cutoff_at: String,
}

impl SettingsFulfillmentWindowDraft {
    fn from_record(record: &FulfillmentWindowRecord) -> Self {
        Self {
            fulfillment_window_id: record.fulfillment_window_id,
            selected_pickup_location_id: Some(record.pickup_location_id),
            label: record.label.clone(),
            starts_at: record.starts_at.clone(),
            ends_at: record.ends_at.clone(),
            order_cutoff_at: record.order_cutoff_at.clone(),
        }
    }
}

struct SettingsBlackoutPeriodFormState {
    blackout_period_id: BlackoutPeriodId,
    label_input: Entity<InputState>,
    starts_at_input: Entity<InputState>,
    ends_at_input: Entity<InputState>,
    _label_subscription: Subscription,
    _starts_at_subscription: Subscription,
    _ends_at_subscription: Subscription,
}

impl SettingsBlackoutPeriodFormState {
    fn new(
        draft: &SettingsBlackoutPeriodDraft,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) -> Self {
        let label_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.label.clone()));
        let starts_at_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.starts_at.clone()));
        let ends_at_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.ends_at.clone()));
        let label_subscription = cx.subscribe_in(
            &label_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let starts_at_subscription = cx.subscribe_in(
            &starts_at_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let ends_at_subscription = cx.subscribe_in(
            &ends_at_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );

        Self {
            blackout_period_id: draft.blackout_period_id,
            label_input,
            starts_at_input,
            ends_at_input,
            _label_subscription: label_subscription,
            _starts_at_subscription: starts_at_subscription,
            _ends_at_subscription: ends_at_subscription,
        }
    }

    fn current_draft(&self, cx: &App) -> SettingsBlackoutPeriodDraft {
        SettingsBlackoutPeriodDraft {
            blackout_period_id: self.blackout_period_id,
            label: self.label_input.read(cx).value().to_string(),
            starts_at: self.starts_at_input.read(cx).value().to_string(),
            ends_at: self.ends_at_input.read(cx).value().to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SettingsBlackoutPeriodDraft {
    blackout_period_id: BlackoutPeriodId,
    label: String,
    starts_at: String,
    ends_at: String,
}

impl SettingsBlackoutPeriodDraft {
    fn from_record(record: &BlackoutPeriodRecord) -> Self {
        Self {
            blackout_period_id: record.blackout_period_id,
            label: record.label.clone(),
            starts_at: record.starts_at.clone(),
            ends_at: record.ends_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SettingsFarmRulesDraft {
    farm_profile: FarmProfileRecord,
    pickup_locations: Vec<SettingsPickupLocationDraft>,
    operating_rules: SettingsOperatingRulesDraft,
    fulfillment_windows: Vec<SettingsFulfillmentWindowDraft>,
    blackout_periods: Vec<SettingsBlackoutPeriodDraft>,
}

impl SettingsFarmRulesDraft {
    fn from_projection(farm_id: FarmId, projection: &FarmRulesProjection) -> Self {
        let farm_profile = projection
            .farm_profile
            .as_ref()
            .cloned()
            .unwrap_or(FarmProfileRecord {
                farm_id,
                display_name: String::new(),
                timezone: String::new(),
                currency_code: String::new(),
            });

        Self {
            farm_profile,
            pickup_locations: projection
                .pickup_locations
                .iter()
                .map(SettingsPickupLocationDraft::from_record)
                .collect(),
            operating_rules: SettingsOperatingRulesDraft::from_record(
                projection.operating_rules.as_ref(),
            ),
            fulfillment_windows: projection
                .fulfillment_windows
                .iter()
                .map(SettingsFulfillmentWindowDraft::from_record)
                .collect(),
            blackout_periods: projection
                .blackout_periods
                .iter()
                .map(SettingsBlackoutPeriodDraft::from_record)
                .collect(),
        }
    }
}

struct SettingsFarmRulesEvaluation {
    projection: FarmRulesProjection,
    operating_rules_validation_keys: Vec<AppTextKey>,
    fulfillment_window_validation_keys: Vec<Vec<AppTextKey>>,
    blackout_period_validation_keys: Vec<Vec<AppTextKey>>,
    blocking_keys: Vec<AppTextKey>,
    readiness_keys: Vec<AppTextKey>,
}

impl SettingsFarmRulesEvaluation {
    fn has_blocking_errors(&self) -> bool {
        !self.blocking_keys.is_empty()
    }
}

fn push_unique_text_key(keys: &mut Vec<AppTextKey>, key: AppTextKey) {
    if !keys.contains(&key) {
        keys.push(key);
    }
}

struct SettingsFarmPanelState {
    account_id: String,
    farm_id: FarmId,
    initial_draft: SettingsFarmRulesDraft,
    farm_name_input: Entity<InputState>,
    timezone_input: Entity<InputState>,
    currency_input: Entity<InputState>,
    pickup_locations: Vec<SettingsPickupLocationFormState>,
    operating_rules: SettingsOperatingRulesFormState,
    fulfillment_windows: Vec<SettingsFulfillmentWindowFormState>,
    blackout_periods: Vec<SettingsBlackoutPeriodFormState>,
    _farm_name_subscription: Subscription,
    _timezone_subscription: Subscription,
    _currency_subscription: Subscription,
    save_failed: bool,
}

impl SettingsFarmPanelState {
    fn new(
        account_id: String,
        projection: FarmRulesProjection,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) -> Self {
        let farm_id = projection
            .farm_profile
            .as_ref()
            .map(|farm_profile| farm_profile.farm_id)
            .unwrap_or_else(FarmId::new);
        let initial_draft = SettingsFarmRulesDraft::from_projection(farm_id, &projection);
        let farm_name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_draft.farm_profile.display_name.clone())
        });
        let timezone_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(initial_draft.farm_profile.timezone.clone())
        });
        let currency_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_draft.farm_profile.currency_code.clone())
        });
        let farm_name_subscription = cx.subscribe_in(
            &farm_name_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let timezone_subscription = cx.subscribe_in(
            &timezone_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let currency_subscription = cx.subscribe_in(
            &currency_input,
            window,
            SettingsWindowView::handle_farm_rules_input_event,
        );
        let pickup_locations = projection
            .pickup_locations
            .iter()
            .map(|record| {
                let can_remove = projection.fulfillment_windows.iter().all(|window_record| {
                    window_record.pickup_location_id != record.pickup_location_id
                });
                SettingsPickupLocationFormState::new(record, can_remove, window, cx)
            })
            .collect();
        let operating_rules =
            SettingsOperatingRulesFormState::new(projection.operating_rules.as_ref(), window, cx);
        let fulfillment_windows = projection
            .fulfillment_windows
            .iter()
            .map(|record| {
                SettingsFulfillmentWindowFormState::new(
                    &SettingsFulfillmentWindowDraft::from_record(record),
                    window,
                    cx,
                )
            })
            .collect();
        let blackout_periods = projection
            .blackout_periods
            .iter()
            .map(|record| {
                SettingsBlackoutPeriodFormState::new(
                    &SettingsBlackoutPeriodDraft::from_record(record),
                    window,
                    cx,
                )
            })
            .collect();
        let mut state = Self {
            account_id,
            farm_id,
            initial_draft,
            farm_name_input,
            timezone_input,
            currency_input,
            pickup_locations,
            operating_rules,
            fulfillment_windows,
            blackout_periods,
            _farm_name_subscription: farm_name_subscription,
            _timezone_subscription: timezone_subscription,
            _currency_subscription: currency_subscription,
            save_failed: false,
        };
        state.sync_pickup_location_removability();
        state
    }

    fn add_pickup_location(&mut self, window: &mut Window, cx: &mut Context<SettingsWindowView>) {
        let record = PickupLocationRecord {
            pickup_location_id: PickupLocationId::new(),
            farm_id: self.farm_id,
            label: String::new(),
            address_line: String::new(),
            directions: None,
            is_default: self.pickup_locations.is_empty(),
        };
        let pickup_location = SettingsPickupLocationFormState::new(&record, true, window, cx);

        self.pickup_locations.push(pickup_location);
        self.sync_pickup_location_removability();
        self.save_failed = false;
    }

    fn set_default_pickup_location(&mut self, pickup_location_id: PickupLocationId) {
        for pickup_location in &mut self.pickup_locations {
            pickup_location.is_default = pickup_location.pickup_location_id == pickup_location_id;
        }
        self.save_failed = false;
    }

    fn remove_pickup_location(&mut self, pickup_location_id: PickupLocationId) {
        self.pickup_locations
            .retain(|pickup_location| pickup_location.pickup_location_id != pickup_location_id);
        if !self
            .pickup_locations
            .iter()
            .any(|pickup_location| pickup_location.is_default)
        {
            if let Some(first_pickup_location) = self.pickup_locations.first_mut() {
                first_pickup_location.is_default = true;
            }
        }
        self.sync_pickup_location_removability();
        self.save_failed = false;
    }

    fn add_fulfillment_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<SettingsWindowView>,
    ) {
        let selected_pickup_location_id = self
            .pickup_locations
            .iter()
            .find(|pickup_location| pickup_location.is_default)
            .or_else(|| self.pickup_locations.first())
            .map(|pickup_location| pickup_location.pickup_location_id);
        let fulfillment_window = SettingsFulfillmentWindowFormState::new(
            &SettingsFulfillmentWindowDraft {
                fulfillment_window_id: FulfillmentWindowId::new(),
                selected_pickup_location_id,
                label: String::new(),
                starts_at: String::new(),
                ends_at: String::new(),
                order_cutoff_at: String::new(),
            },
            window,
            cx,
        );

        self.fulfillment_windows.push(fulfillment_window);
        self.sync_pickup_location_removability();
        self.save_failed = false;
    }

    fn select_fulfillment_window_pickup_location(
        &mut self,
        fulfillment_window_id: FulfillmentWindowId,
        pickup_location_id: PickupLocationId,
    ) {
        if let Some(fulfillment_window) =
            self.fulfillment_windows
                .iter_mut()
                .find(|fulfillment_window| {
                    fulfillment_window.fulfillment_window_id == fulfillment_window_id
                })
        {
            fulfillment_window.selected_pickup_location_id = Some(pickup_location_id);
            self.sync_pickup_location_removability();
            self.save_failed = false;
        }
    }

    fn remove_fulfillment_window(&mut self, fulfillment_window_id: FulfillmentWindowId) {
        self.fulfillment_windows.retain(|fulfillment_window| {
            fulfillment_window.fulfillment_window_id != fulfillment_window_id
        });
        self.sync_pickup_location_removability();
        self.save_failed = false;
    }

    fn add_blackout_period(&mut self, window: &mut Window, cx: &mut Context<SettingsWindowView>) {
        let blackout_period = SettingsBlackoutPeriodFormState::new(
            &SettingsBlackoutPeriodDraft {
                blackout_period_id: BlackoutPeriodId::new(),
                label: String::new(),
                starts_at: String::new(),
                ends_at: String::new(),
            },
            window,
            cx,
        );

        self.blackout_periods.push(blackout_period);
        self.save_failed = false;
    }

    fn remove_blackout_period(&mut self, blackout_period_id: BlackoutPeriodId) {
        self.blackout_periods
            .retain(|blackout_period| blackout_period.blackout_period_id != blackout_period_id);
        self.save_failed = false;
    }

    fn current_draft(&self, cx: &App) -> SettingsFarmRulesDraft {
        SettingsFarmRulesDraft {
            farm_profile: FarmProfileRecord {
                farm_id: self.farm_id,
                display_name: self.farm_name_input.read(cx).value().to_string(),
                timezone: self.timezone_input.read(cx).value().to_string(),
                currency_code: self.currency_input.read(cx).value().to_string(),
            },
            pickup_locations: self
                .pickup_locations
                .iter()
                .map(|pickup_location| pickup_location.current_draft(cx))
                .collect(),
            operating_rules: self.operating_rules.current_draft(cx),
            fulfillment_windows: self
                .fulfillment_windows
                .iter()
                .map(|fulfillment_window| fulfillment_window.current_draft(cx))
                .collect(),
            blackout_periods: self
                .blackout_periods
                .iter()
                .map(|blackout_period| blackout_period.current_draft(cx))
                .collect(),
        }
    }

    fn evaluate(&self, cx: &App) -> SettingsFarmRulesEvaluation {
        let draft = self.current_draft(cx);
        let farm_profile = FarmProfileRecord {
            farm_id: self.farm_id,
            display_name: draft.farm_profile.display_name.trim().to_owned(),
            timezone: draft.farm_profile.timezone.trim().to_owned(),
            currency_code: draft.farm_profile.currency_code.trim().to_owned(),
        };
        let pickup_locations = draft
            .pickup_locations
            .clone()
            .into_iter()
            .map(|pickup_location| pickup_location.into_record(self.farm_id))
            .collect();
        let mut operating_rules_validation_keys = Vec::new();
        let operating_rules = if draft.operating_rules.is_empty() {
            None
        } else {
            let promise_lead_hours = match draft
                .operating_rules
                .promise_lead_hours
                .trim()
                .parse::<u16>()
            {
                Ok(promise_lead_hours) => promise_lead_hours,
                Err(_) if draft.operating_rules.promise_lead_hours.trim().is_empty() => 0,
                Err(_) => {
                    push_unique_text_key(
                        &mut operating_rules_validation_keys,
                        AppTextKey::SettingsOperatingRulesInvalidPromiseLeadTime,
                    );
                    0
                }
            };

            Some(FarmOperatingRulesRecord {
                farm_id: self.farm_id,
                promise_lead_hours,
                substitution_policy: draft.operating_rules.substitution_policy.trim().to_owned(),
                missed_pickup_policy: draft.operating_rules.missed_pickup_policy.trim().to_owned(),
            })
        };
        let mut fulfillment_windows = Vec::new();
        let mut fulfillment_window_validation_keys =
            Vec::with_capacity(draft.fulfillment_windows.len());
        for fulfillment_window in &draft.fulfillment_windows {
            let label = fulfillment_window.label.trim().to_owned();
            let starts_at = fulfillment_window.starts_at.trim().to_owned();
            let ends_at = fulfillment_window.ends_at.trim().to_owned();
            let order_cutoff_at = fulfillment_window.order_cutoff_at.trim().to_owned();
            let mut row_validation_keys = Vec::new();
            let missing_required_fields = label.is_empty()
                || starts_at.is_empty()
                || ends_at.is_empty()
                || order_cutoff_at.is_empty();

            if missing_required_fields {
                push_unique_text_key(
                    &mut row_validation_keys,
                    AppTextKey::SettingsFulfillmentWindowsValidationCompleteBeforeSave,
                );
            } else if fulfillment_window.selected_pickup_location_id.is_none() {
                push_unique_text_key(
                    &mut row_validation_keys,
                    AppTextKey::SettingsFulfillmentWindowsValidationChoosePickupLocation,
                );
            }

            if let Some(pickup_location_id) = fulfillment_window.selected_pickup_location_id {
                if !missing_required_fields {
                    if ends_at <= starts_at {
                        push_unique_text_key(
                            &mut row_validation_keys,
                            AppTextKey::SettingsReadinessFieldFulfillmentWindowEndsBeforeStart,
                        );
                    }
                    if order_cutoff_at >= starts_at {
                        push_unique_text_key(
                            &mut row_validation_keys,
                            AppTextKey::SettingsReadinessFieldFulfillmentWindowCutoffAfterStart,
                        );
                    }
                    fulfillment_windows.push(FulfillmentWindowRecord {
                        fulfillment_window_id: fulfillment_window.fulfillment_window_id,
                        farm_id: self.farm_id,
                        pickup_location_id,
                        label,
                        starts_at,
                        ends_at,
                        order_cutoff_at,
                    });
                }
            }

            fulfillment_window_validation_keys.push(row_validation_keys);
        }
        let mut blackout_periods = Vec::new();
        let mut blackout_period_validation_keys = Vec::with_capacity(draft.blackout_periods.len());
        for blackout_period in &draft.blackout_periods {
            let label = blackout_period.label.trim().to_owned();
            let starts_at = blackout_period.starts_at.trim().to_owned();
            let ends_at = blackout_period.ends_at.trim().to_owned();
            let mut row_validation_keys = Vec::new();

            if label.is_empty() || starts_at.is_empty() || ends_at.is_empty() {
                push_unique_text_key(
                    &mut row_validation_keys,
                    AppTextKey::SettingsBlackoutPeriodsValidationCompleteBeforeSave,
                );
            } else {
                if ends_at <= starts_at {
                    push_unique_text_key(
                        &mut row_validation_keys,
                        AppTextKey::SettingsReadinessFieldBlackoutPeriodEndsBeforeStart,
                    );
                }
                blackout_periods.push(BlackoutPeriodRecord {
                    blackout_period_id: blackout_period.blackout_period_id,
                    farm_id: self.farm_id,
                    label,
                    starts_at,
                    ends_at,
                });
            }

            blackout_period_validation_keys.push(row_validation_keys);
        }

        let mut projection = FarmRulesProjection {
            farm_profile: Some(farm_profile),
            pickup_locations,
            operating_rules,
            fulfillment_windows,
            blackout_periods,
            readiness: FarmRulesReadiness::ready(),
        };
        projection.readiness = derive_farm_rules_readiness(&projection);

        let mut blocking_keys = operating_rules_validation_keys.clone();
        for row_validation_keys in &fulfillment_window_validation_keys {
            for validation_key in row_validation_keys {
                push_unique_text_key(&mut blocking_keys, *validation_key);
            }
        }
        for row_validation_keys in &blackout_period_validation_keys {
            for validation_key in row_validation_keys {
                push_unique_text_key(&mut blocking_keys, *validation_key);
            }
        }
        for timing_conflict in &projection.readiness.timing_conflicts {
            push_unique_text_key(
                &mut blocking_keys,
                settings_timing_conflict_key(timing_conflict.kind),
            );
        }

        let mut readiness_keys = projection
            .readiness
            .blockers
            .iter()
            .copied()
            .map(settings_readiness_key)
            .collect::<Vec<_>>();
        for blocking_key in &blocking_keys {
            push_unique_text_key(&mut readiness_keys, *blocking_key);
        }

        SettingsFarmRulesEvaluation {
            projection,
            operating_rules_validation_keys,
            fulfillment_window_validation_keys,
            blackout_period_validation_keys,
            blocking_keys,
            readiness_keys,
        }
    }

    fn current_projection(&self, cx: &App) -> FarmRulesProjection {
        self.evaluate(cx).projection
    }

    fn has_changes(&self, cx: &App) -> bool {
        self.current_draft(cx) != self.initial_draft
    }

    fn save_ready(&self, cx: &App) -> bool {
        let evaluation = self.evaluate(cx);
        self.has_changes(cx) && !evaluation.has_blocking_errors()
    }

    fn save_status_key(&self, cx: &App) -> AppTextKey {
        if self.save_failed {
            AppTextKey::SettingsFarmSaveFailed
        } else if self.has_changes(cx) {
            let evaluation = self.evaluate(cx);
            if evaluation.has_blocking_errors() {
                AppTextKey::SettingsFarmSaveBlocked
            } else {
                AppTextKey::SettingsFarmSavePending
            }
        } else {
            AppTextKey::SettingsFarmSaveSaved
        }
    }

    fn sync_pickup_location_removability(&mut self) {
        let selected_pickup_location_ids = self
            .fulfillment_windows
            .iter()
            .filter_map(|fulfillment_window| fulfillment_window.selected_pickup_location_id)
            .collect::<Vec<_>>();

        for pickup_location in &mut self.pickup_locations {
            pickup_location.can_remove =
                !selected_pickup_location_ids.contains(&pickup_location.pickup_location_id);
        }
    }
}

pub struct SettingsWindowView {
    runtime: DesktopAppRuntime,
    farm_panel_state: Option<SettingsFarmPanelState>,
    farm_panel_error: Option<String>,
}

impl SettingsWindowView {
    pub fn new(runtime: DesktopAppRuntime, initial_view: SettingsPanelViewKey) -> Self {
        let _ = initial_view;
        Self {
            runtime,
            farm_panel_state: None,
            farm_panel_error: None,
        }
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        if self.runtime.select_settings_section(view) {
            cx.notify();
        }
    }

    fn selected_view(&self) -> SettingsPanelViewKey {
        self.runtime.selected_settings_section()
    }

    fn handle_farm_rules_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        if let Some(form) = self.farm_panel_state.as_mut() {
            form.save_failed = false;
        }

        cx.notify();
    }

    fn sync_farm_panel_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let runtime = self.runtime.summary();
        let Some((account_id, farm_id)) = settings_panel_farm_context(&runtime) else {
            self.farm_panel_state = None;
            self.farm_panel_error = None;
            return;
        };

        if self
            .farm_panel_state
            .as_ref()
            .is_some_and(|form| form.account_id == account_id && form.farm_id == farm_id)
        {
            return;
        }

        match self.runtime.load_farm_rules_projection() {
            Ok(projection) => {
                self.farm_panel_state = Some(SettingsFarmPanelState::new(
                    account_id, projection, window, cx,
                ));
                self.farm_panel_error = None;
            }
            Err(runtime_error) => {
                error!(
                    target: "settings",
                    event = "settings.farm.load_failed",
                    error = %runtime_error,
                    "failed to load farm settings projection"
                );
                self.farm_panel_state = None;
                self.farm_panel_error = Some(runtime_error.to_string());
            }
        }
    }

    fn add_pickup_location(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.add_pickup_location(window, cx);
        cx.notify();
    }

    fn select_default_pickup_location(
        &mut self,
        pickup_location_id: PickupLocationId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.set_default_pickup_location(pickup_location_id);
        cx.notify();
    }

    fn remove_pickup_location(
        &mut self,
        pickup_location_id: PickupLocationId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.remove_pickup_location(pickup_location_id);
        cx.notify();
    }

    fn add_fulfillment_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.add_fulfillment_window(window, cx);
        cx.notify();
    }

    fn select_fulfillment_window_pickup_location(
        &mut self,
        fulfillment_window_id: FulfillmentWindowId,
        pickup_location_id: PickupLocationId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.select_fulfillment_window_pickup_location(fulfillment_window_id, pickup_location_id);
        cx.notify();
    }

    fn remove_fulfillment_window(
        &mut self,
        fulfillment_window_id: FulfillmentWindowId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.remove_fulfillment_window(fulfillment_window_id);
        cx.notify();
    }

    fn add_blackout_period(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.add_blackout_period(window, cx);
        cx.notify();
    }

    fn remove_blackout_period(
        &mut self,
        blackout_period_id: BlackoutPeriodId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.farm_panel_state.as_mut() else {
            return;
        };

        form.remove_blackout_period(blackout_period_id);
        cx.notify();
    }

    fn save_farm_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((current_projection, save_ready)) = self
            .farm_panel_state
            .as_ref()
            .map(|form| (form.current_projection(cx), form.save_ready(cx)))
        else {
            return;
        };
        if !save_ready {
            return;
        }

        match self.runtime.save_farm_rules_projection(current_projection) {
            Ok(saved_projection) => {
                let account_id = self
                    .farm_panel_state
                    .as_ref()
                    .map(|form| form.account_id.clone())
                    .unwrap_or_default();
                self.farm_panel_state = Some(SettingsFarmPanelState::new(
                    account_id,
                    saved_projection,
                    window,
                    cx,
                ));
                self.farm_panel_error = None;
                cx.notify();
            }
            Err(runtime_error) => {
                error!(
                    target: "settings",
                    event = "settings.farm.save_failed",
                    error = %runtime_error,
                    "failed to save farm settings projection"
                );
                if let Some(form) = self.farm_panel_state.as_mut() {
                    form.save_failed = true;
                }
                cx.notify();
            }
        }
    }

    fn navigation_button(
        &mut self,
        view: SettingsPanelViewKey,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (navigation_id, navigation_icon) = settings_panel_spec(view);
        icon_segment_button(
            IconSegmentButtonSpec::new(
                navigation_id,
                app_shared_text(settings_panel_label_key(view)),
                navigation_icon,
            ),
            self.selected_view() == view,
            cx.listener(move |this, _, _, cx| this.select_view(view, cx)),
            cx,
        )
    }

    fn account_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let detail_text_px = APP_UI_THEME
            .foundation
            .typography
            .settings_account_detail_text_px;
        let account_status_color = APP_UI_THEME.components.app_status_indicator.offline;

        div()
            .size_full()
            .flex()
            .child(
                div()
                    .h_full()
                    .w(px(APP_UI_THEME.shells.settings_account_sidebar_width_px))
                    .p(px(APP_UI_THEME.shells.settings_account_sidebar_padding_px))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .w_full()
                            .bg(rgb(APP_UI_THEME.foundation.surfaces.chrome_background))
                            .rounded(px(APP_UI_THEME
                                .shells
                                .settings_account_sidebar_button_corner_radius_px))
                            .p(px(APP_UI_THEME
                                .shells
                                .settings_account_sidebar_button_padding_px))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(APP_UI_THEME
                                                .foundation
                                                .typography
                                                .settings_account_identity_text_px))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountNoSelectionTitle,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(APP_UI_THEME
                                                .foundation
                                                .typography
                                                .settings_account_identity_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                            .line_height(relative(1.2))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountNoSelectionBody,
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .pt(px(APP_UI_THEME
                                .shells
                                .settings_account_sidebar_footer_padding_top_px))
                            .flex()
                            .flex_col()
                            .gap(px(APP_UI_THEME
                                .shells
                                .settings_account_sidebar_footer_row_gap_px))
                            .child(section_divider())
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(APP_UI_THEME
                                        .shells
                                        .settings_account_sidebar_footer_button_gap_px))
                                    .child(action_button(
                                        "account-add",
                                        app_shared_text(AppTextKey::SettingsAccountAddAction),
                                        |_, _, _| {},
                                        cx,
                                    ))
                                    .child(action_icon_button(
                                        "account-more",
                                        IconName::ChevronDown,
                                        |_, _, _| {},
                                        cx,
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .w(px(APP_UI_THEME.foundation.borders.divider_thickness_px))
                    .bg(rgb(APP_UI_THEME.foundation.surfaces.divider)),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .p(px(APP_UI_THEME.shells.settings_account_main_padding_px))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_start()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(APP_UI_THEME
                                .shells
                                .settings_account_content_max_width_px))
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(APP_UI_THEME.shells.settings_account_main_stack_gap_px))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(APP_UI_THEME.shells.settings_account_main_stack_gap_px))
                                    .child(
                                        div()
                                            .size(px(APP_UI_THEME
                                                .shells
                                                .settings_account_profile_avatar_size_px))
                                            .bg(rgb(APP_UI_THEME
                                                .foundation
                                                .surfaces
                                                .card_background))
                                            .rounded(px(APP_UI_THEME
                                                .shells
                                                .settings_account_profile_avatar_size_px
                                                / 2.0)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(detail_text_px))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountNoSelectionTitle,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap(px(APP_UI_THEME.shells.settings_account_detail_row_gap_px))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountProfileLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountStatusLabel,
                                        ),
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(APP_UI_THEME
                                                .shells
                                                .settings_account_status_gap_px))
                                            .child(status_indicator(account_status_color))
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .text_color(rgb(APP_UI_THEME
                                                        .foundation
                                                        .text
                                                        .primary))
                                                    .child(app_shared_text(
                                                        AppTextKey::SettingsAccountStatusLoggedOut,
                                                    )),
                                            ),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountCustodyLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountSurfaceLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountActivationLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountActivationInactive,
                                            )),
                                    ))
                                    .child(home_body_text(app_shared_text(
                                        AppTextKey::SettingsAccountNoSelectionBody,
                                    )))
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .min_w_0()
                                            .items_center()
                                            .gap(px(APP_UI_THEME
                                                .shells
                                                .settings_account_action_row_gap_px))
                                            .child(div().child(action_button(
                                                "account-log-out",
                                                app_shared_text(
                                                    AppTextKey::SettingsAccountLogOutAction,
                                                ),
                                                |_, _, _| {},
                                                cx,
                                            )))
                                            .child(div().child(action_button(
                                                "account-open-workspace",
                                                app_shared_text(
                                                    AppTextKey::SettingsAccountOpenWorkspaceAction,
                                                ),
                                                |_, _, _| {},
                                                cx,
                                            ))),
                                    ),
                            ),
                    ),
            )
    }

    fn settings_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_farm_panel_state(window, cx);

        let runtime_summary = self.runtime.summary();
        let general_settings = runtime_summary.shell_projection.settings.general;
        let general_allow_relay_connections = general_settings.allow_relay_connections;
        let general_use_media_servers = general_settings.use_media_servers;
        let general_use_nip05 = general_settings.use_nip05;
        let general_launch_at_login = general_settings.launch_at_login;

        let mut cards = Vec::new();

        if let Some(error) = self.farm_panel_error.as_ref() {
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsNavSettings),
                    home_body_text(error.clone()),
                )
                .into_any_element(),
            );
        } else if let Some(form) = self.farm_panel_state.as_ref() {
            let evaluation = form.evaluate(cx);
            let save_ready = form.has_changes(cx) && !evaluation.has_blocking_errors();
            let save_action = if save_ready {
                action_button_primary(
                    "settings-farm-save",
                    app_shared_text(AppTextKey::SettingsFarmSaveAction),
                    cx.listener(|this, _, window, cx| this.save_farm_panel(window, cx)),
                    cx,
                )
                .into_any_element()
            } else {
                action_button_primary_disabled(
                    "settings-farm-save",
                    app_shared_text(AppTextKey::SettingsFarmSaveAction),
                    cx,
                )
                .into_any_element()
            };

            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsOperatingRulesSectionLabel),
                    app_stack_v(12.0)
                        .w_full()
                        .child(app_form_input_text(
                            AppFormFieldSpec::new(
                                app_shared_text(
                                    AppTextKey::SettingsOperatingRulesFieldPromiseLeadTime,
                                ),
                                Option::<SharedString>::None,
                            ),
                            &form.operating_rules.promise_lead_hours_input,
                            false,
                        ))
                        .child(app_form_input_text(
                            AppFormFieldSpec::new(
                                app_shared_text(
                                    AppTextKey::SettingsOperatingRulesFieldSubstitutionPolicy,
                                ),
                                Option::<SharedString>::None,
                            ),
                            &form.operating_rules.substitution_policy_input,
                            false,
                        ))
                        .child(app_form_input_text(
                            AppFormFieldSpec::new(
                                app_shared_text(
                                    AppTextKey::SettingsOperatingRulesFieldMissedPickupPolicy,
                                ),
                                Option::<SharedString>::None,
                            ),
                            &form.operating_rules.missed_pickup_policy_input,
                            false,
                        ))
                        .children(
                            evaluation
                                .operating_rules_validation_keys
                                .iter()
                                .copied()
                                .map(|key| home_body_text(app_shared_text(key)).into_any_element())
                                .collect::<Vec<_>>(),
                        ),
                )
                .into_any_element(),
            );
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsSectionLabel),
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .when(form.fulfillment_windows.is_empty(), |this| {
                            this.child(home_body_text(app_shared_text(
                                AppTextKey::SettingsFulfillmentWindowsEmptyBody,
                            )))
                        })
                        .when(form.pickup_locations.is_empty(), |this| {
                            this.child(home_body_text(app_shared_text(
                                AppTextKey::SettingsFulfillmentWindowsPickupLocationsBody,
                            )))
                        })
                        .children(
                            form.fulfillment_windows
                                .iter()
                                .enumerate()
                                .map(|(index, fulfillment_window)| {
                                    let fulfillment_window_id =
                                        fulfillment_window.fulfillment_window_id;
                                    let pickup_location_options = form
                                        .pickup_locations
                                        .iter()
                                        .enumerate()
                                        .map(|(pickup_index, pickup_location)| {
                                            let pickup_location_id =
                                                pickup_location.pickup_location_id;
                                            let is_selected = fulfillment_window
                                                .selected_pickup_location_id
                                                .is_some_and(|selected_pickup_location_id| {
                                                    selected_pickup_location_id
                                                        == pickup_location_id
                                                });
                                            choice_button(
                                                (
                                                    "settings-fulfillment-window-pickup-location",
                                                    index * 100 + pickup_index,
                                                ),
                                                settings_pickup_location_title(
                                                    pickup_index,
                                                    pickup_location,
                                                    cx,
                                                ),
                                                is_selected,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.select_fulfillment_window_pickup_location(
                                                        fulfillment_window_id,
                                                        pickup_location_id,
                                                        cx,
                                                    )
                                                }),
                                                cx,
                                            )
                                            .into_any_element()
                                        })
                                        .collect::<Vec<_>>();
                                    let validation_keys = evaluation
                                        .fulfillment_window_validation_keys
                                        .get(index)
                                        .cloned()
                                        .unwrap_or_default();

                                    settings_fulfillment_window_card(
                                        index,
                                        fulfillment_window,
                                        pickup_location_options,
                                        &validation_keys,
                                        cx.listener(move |this, _, _, cx| {
                                            this.remove_fulfillment_window(
                                                fulfillment_window_id,
                                                cx,
                                            )
                                        }),
                                        cx,
                                    )
                                    .into_any_element()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .child(
                            action_button_compact(
                                "settings-add-fulfillment-window",
                                app_shared_text(AppTextKey::SettingsFulfillmentWindowsAddAction),
                                cx.listener(|this, _, window, cx| {
                                    this.add_fulfillment_window(window, cx)
                                }),
                                cx,
                            )
                            .into_any_element(),
                        ),
                )
                .into_any_element(),
            );
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsBlackoutPeriodsSectionLabel),
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .when(form.blackout_periods.is_empty(), |this| {
                            this.child(home_body_text(app_shared_text(
                                AppTextKey::SettingsBlackoutPeriodsEmptyBody,
                            )))
                        })
                        .children(
                            form.blackout_periods
                                .iter()
                                .enumerate()
                                .map(|(index, blackout_period)| {
                                    let blackout_period_id = blackout_period.blackout_period_id;
                                    let validation_keys = evaluation
                                        .blackout_period_validation_keys
                                        .get(index)
                                        .cloned()
                                        .unwrap_or_default();

                                    settings_blackout_period_card(
                                        index,
                                        blackout_period,
                                        &validation_keys,
                                        cx.listener(move |this, _, _, cx| {
                                            this.remove_blackout_period(blackout_period_id, cx)
                                        }),
                                        cx,
                                    )
                                    .into_any_element()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .child(
                            action_button_compact(
                                "settings-add-blackout-period",
                                app_shared_text(AppTextKey::SettingsBlackoutPeriodsAddAction),
                                cx.listener(|this, _, window, cx| {
                                    this.add_blackout_period(window, cx)
                                }),
                                cx,
                            )
                            .into_any_element(),
                        ),
                )
                .into_any_element(),
            );
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsReadinessSectionLabel),
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .children(settings_farm_readiness_rows(&evaluation))
                        .child(section_divider())
                        .child(home_body_text(app_shared_text(form.save_status_key(cx))))
                        .child(div().child(save_action)),
                )
                .into_any_element(),
            );
        } else {
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsNavSettings),
                    home_body_text(app_shared_text(AppTextKey::SettingsFarmUnavailableBody)),
                )
                .into_any_element(),
            );
        }

        cards.push(
            home_card(
                app_shared_text(AppTextKey::SettingsGeneralSectionLabel),
                app_stack_v(16.0)
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(APP_UI_THEME.shells.settings_account_detail_value_gap_px))
                            .child(app_checkbox_field(
                                AppCheckboxFieldSpec::new(
                                    "settings-allow-relay-connections",
                                    app_shared_text(
                                        AppTextKey::SettingsGeneralAllowRelayConnections,
                                    ),
                                    Option::<SharedString>::None,
                                ),
                                general_allow_relay_connections,
                                cx,
                                |_, _, _| {},
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(APP_UI_THEME.shells.settings_account_detail_value_gap_px))
                            .child(app_checkbox_field(
                                AppCheckboxFieldSpec::new(
                                    "settings-use-media-servers",
                                    app_shared_text(AppTextKey::SettingsGeneralUseMediaServers),
                                    Option::<SharedString>::None,
                                ),
                                general_use_media_servers,
                                cx,
                                |_, _, _| {},
                            ))
                            .child(div().flex_none().child(action_button_compact(
                                "settings-manage-media-servers",
                                app_shared_text(AppTextKey::SettingsGeneralManageAction),
                                |_, _, _| {},
                                cx,
                            ))),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(APP_UI_THEME.shells.settings_account_detail_value_gap_px))
                            .child(app_checkbox_field(
                                AppCheckboxFieldSpec::new(
                                    "settings-use-nip05",
                                    app_shared_text(AppTextKey::SettingsGeneralUseNip05),
                                    Some(app_shared_text(AppTextKey::SettingsGeneralUseNip05Note)),
                                ),
                                general_use_nip05,
                                cx,
                                |_, _, _| {},
                            )),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(APP_UI_THEME.shells.settings_account_detail_value_gap_px))
                            .child(app_checkbox_field(
                                AppCheckboxFieldSpec::new(
                                    "settings-launch-at-login",
                                    app_shared_text(AppTextKey::SettingsGeneralLaunchAtLogin),
                                    Option::<SharedString>::None,
                                ),
                                general_launch_at_login,
                                cx,
                                |_, _, _| {},
                            )),
                    ),
            )
            .into_any_element(),
        );

        app_scroll_panel(
            "settings-panel-scroll",
            APP_UI_THEME.shells.settings_content_padding_px,
            Some(560.0),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .w_full()
                .child(home_body_text(app_shared_text(
                    AppTextKey::SettingsSettingsPanelBody,
                )))
                .children(cards),
        )
    }

    fn farm_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_farm_panel_state(window, cx);

        let mut cards = Vec::new();

        if let Some(error) = self.farm_panel_error.as_ref() {
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsNavFarm),
                    home_body_text(error.clone()),
                )
                .into_any_element(),
            );
            return app_scroll_panel(
                "settings-panel-scroll",
                APP_UI_THEME.shells.settings_content_padding_px,
                Some(560.0),
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .child(home_body_text(app_shared_text(
                        AppTextKey::SettingsFarmPanelBody,
                    )))
                    .children(cards),
            );
        }

        let Some(form) = self.farm_panel_state.as_ref() else {
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsNavFarm),
                    home_body_text(app_shared_text(AppTextKey::SettingsFarmUnavailableBody)),
                )
                .into_any_element(),
            );
            return app_scroll_panel(
                "settings-panel-scroll",
                APP_UI_THEME.shells.settings_content_padding_px,
                Some(560.0),
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .child(home_body_text(app_shared_text(
                        AppTextKey::SettingsFarmPanelBody,
                    )))
                    .children(cards),
            );
        };

        let evaluation = form.evaluate(cx);
        let save_action = if form.has_changes(cx) && !evaluation.has_blocking_errors() {
            action_button_primary(
                "settings-farm-save",
                app_shared_text(AppTextKey::SettingsFarmSaveAction),
                cx.listener(|this, _, window, cx| this.save_farm_panel(window, cx)),
                cx,
            )
            .into_any_element()
        } else {
            action_button_primary_disabled(
                "settings-farm-save",
                app_shared_text(AppTextKey::SettingsFarmSaveAction),
                cx,
            )
            .into_any_element()
        };

        cards.push(
            home_card(
                app_shared_text(AppTextKey::HomeFarmSetupSectionFarm),
                app_stack_v(12.0)
                    .w_full()
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::HomeFarmSetupFieldFarmName),
                            Option::<SharedString>::None,
                        ),
                        &form.farm_name_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::SettingsFarmFieldTimezone),
                            Option::<SharedString>::None,
                        ),
                        &form.timezone_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::SettingsFarmFieldCurrency),
                            Option::<SharedString>::None,
                        ),
                        &form.currency_input,
                        false,
                    )),
            )
            .into_any_element(),
        );
        cards.push(
            home_card(
                app_shared_text(AppTextKey::SettingsPickupLocationsSectionLabel),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .when(form.pickup_locations.is_empty(), |this| {
                        this.child(home_body_text(app_shared_text(
                            AppTextKey::SettingsPickupLocationsEmptyBody,
                        )))
                    })
                    .children(
                        form.pickup_locations
                            .iter()
                            .enumerate()
                            .map(|(index, pickup_location)| {
                                let pickup_location_id = pickup_location.pickup_location_id;
                                settings_pickup_location_card(
                                    index,
                                    pickup_location,
                                    cx.listener(move |this, _, _, cx| {
                                        this.select_default_pickup_location(pickup_location_id, cx)
                                    }),
                                    cx.listener(move |this, _, _, cx| {
                                        this.remove_pickup_location(pickup_location_id, cx)
                                    }),
                                    cx,
                                )
                                .into_any_element()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .child(
                        action_button_compact(
                            "settings-farm-add-pickup",
                            app_shared_text(AppTextKey::SettingsPickupLocationsAddAction),
                            cx.listener(|this, _, window, cx| this.add_pickup_location(window, cx)),
                            cx,
                        )
                        .into_any_element(),
                    )
                    .child(section_divider())
                    .child(home_body_text(app_shared_text(form.save_status_key(cx))))
                    .child(div().child(save_action)),
            )
            .into_any_element(),
        );

        app_scroll_panel(
            "settings-panel-scroll",
            APP_UI_THEME.shells.settings_content_padding_px,
            Some(560.0),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .w_full()
                .child(home_body_text(app_shared_text(
                    AppTextKey::SettingsFarmPanelBody,
                )))
                .children(cards),
        )
    }

    fn about_panel(&self) -> impl IntoElement {
        let runtime = self.runtime.summary();
        let status_rows = about_status_rows(&runtime);
        let conflict_rows = about_conflict_review_rows(&runtime.sync_status);
        let runtime_rows = about_runtime_rows(&runtime);

        app_scroll_panel(
            "settings-panel-scroll",
            APP_UI_THEME.shells.settings_content_padding_px,
            None,
            app_stack_v(APP_UI_THEME.shells.settings_account_main_stack_gap_px)
                .size_full()
                .py_12()
                .child(app_surface_card(
                    app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                        .w_full()
                        .child(app_heading_section(app_shared_text(
                            AppTextKey::SettingsAboutStatusSectionLabel,
                        )))
                        .child(label_value_list(status_rows)),
                ))
                .child(app_surface_card(
                    app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                        .w_full()
                        .child(app_heading_section(app_shared_text(
                            AppTextKey::SettingsAboutConflictReviewSectionLabel,
                        )))
                        .child(home_body_text(app_text(about_conflict_review_body_key(
                            &runtime.sync_status,
                        ))))
                        .child(label_value_list(conflict_rows)),
                ))
                .child(app_surface_card(
                    app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                        .w_full()
                        .child(app_heading_section(app_shared_text(
                            AppTextKey::SettingsAboutRuntimeSectionLabel,
                        )))
                        .child(label_value_list(runtime_rows)),
                )),
        )
    }

    fn settings_panel_content(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.selected_view() {
            SettingsPanelViewKey::Account => self.account_panel(cx).into_any_element(),
            SettingsPanelViewKey::Farm => self.farm_panel(window, cx).into_any_element(),
            SettingsPanelViewKey::Settings => self.settings_panel(window, cx).into_any_element(),
            SettingsPanelViewKey::About => self.about_panel().into_any_element(),
        }
    }
}

fn about_status_rows(runtime: &DesktopAppRuntimeSummary) -> Vec<LabelValueRow> {
    let mut rows = vec![
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSelectedAccount),
            selected_account_label(runtime.sync_status.account_id.as_deref()),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncRunStatus),
            about_sync_run_status_text(&runtime.sync_status),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncCheckpointState),
            about_sync_checkpoint_state_text(&runtime.sync_status),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncPendingWriteCount),
            runtime.sync_status.pending_write_count.to_string(),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictCount),
            runtime
                .sync_status
                .projection
                .conflict_status
                .unresolved_count
                .to_string(),
        ),
    ];

    if runtime
        .sync_status
        .projection
        .conflict_status
        .blocking_count
        > 0
    {
        rows.push(LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncBlockingConflictCount),
            runtime
                .sync_status
                .projection
                .conflict_status
                .blocking_count
                .to_string(),
        ));
    }

    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataStartupIssue),
        runtime
            .startup_issue
            .clone()
            .unwrap_or_else(|| app_text(AppTextKey::ValueNone)),
    ));

    rows
}

fn about_conflict_review_body_key(sync_status: &DesktopAppSyncStatusSummary) -> AppTextKey {
    if !sync_status.is_enabled() {
        AppTextKey::SettingsAboutConflictReviewUnavailable
    } else if sync_status
        .projection
        .conflict_status
        .has_blocking_conflicts()
    {
        AppTextKey::SettingsAboutConflictReviewBlocking
    } else if sync_status.projection.conflict_status.requires_attention() {
        AppTextKey::SettingsAboutConflictReviewNeedsAttention
    } else {
        AppTextKey::SettingsAboutConflictReviewClear
    }
}

fn about_conflict_review_rows(sync_status: &DesktopAppSyncStatusSummary) -> Vec<LabelValueRow> {
    let mut rows = vec![LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataSyncConflictCount),
        sync_status
            .projection
            .conflict_status
            .unresolved_count
            .to_string(),
    )];

    if sync_status.projection.conflict_status.blocking_count > 0 {
        rows.push(LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncBlockingConflictCount),
            sync_status
                .projection
                .conflict_status
                .blocking_count
                .to_string(),
        ));
    }

    rows
}

fn about_runtime_rows(runtime: &DesktopAppRuntimeSummary) -> Vec<LabelValueRow> {
    let mut rows = runtime_metadata_rows(&runtime.runtime_metadata.snapshot);
    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataDataRoot),
        path_or_none(runtime.runtime_metadata.data_root.as_ref()),
    ));
    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataLogsRoot),
        path_or_none(runtime.runtime_metadata.logs_root.as_ref()),
    ));
    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataDatabasePath),
        path_or_none(runtime.runtime_metadata.database_path.as_ref()),
    ));
    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataDatabaseSchemaVersion),
        runtime
            .runtime_metadata
            .database_schema_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| app_text(AppTextKey::ValueNone)),
    ));
    rows.push(LabelValueRow::new(
        app_shared_text(AppTextKey::MetadataShellSection),
        runtime
            .shell_projection
            .selected_section
            .storage_key()
            .to_owned(),
    ));
    rows
}

fn selected_account_label(account_id: Option<&str>) -> String {
    account_id
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| app_text(AppTextKey::ValueNone))
}

fn about_sync_run_status_text(sync_status: &DesktopAppSyncStatusSummary) -> String {
    if !sync_status.is_enabled() {
        return app_text(AppTextKey::ValueDisabled);
    }

    match sync_status.projection.run_status {
        AppSyncRunStatus::Idle => app_text(AppTextKey::ValueSyncRunStatusIdle),
        AppSyncRunStatus::Syncing => app_text(AppTextKey::ValueSyncRunStatusSyncing),
        AppSyncRunStatus::Succeeded => app_text(AppTextKey::ValueSyncRunStatusSucceeded),
        AppSyncRunStatus::Conflicted => app_text(AppTextKey::ValueSyncRunStatusConflicted),
        AppSyncRunStatus::Failed => app_text(AppTextKey::ValueSyncRunStatusFailed),
    }
}

fn about_sync_checkpoint_state_text(sync_status: &DesktopAppSyncStatusSummary) -> String {
    if !sync_status.is_enabled() {
        return app_text(AppTextKey::ValueNone);
    }

    match sync_status.projection.checkpoint.state {
        SyncCheckpointState::NeverSynced => app_text(AppTextKey::ValueSyncCheckpointNeverSynced),
        SyncCheckpointState::Syncing => app_text(AppTextKey::ValueSyncCheckpointSyncing),
        SyncCheckpointState::Current => app_text(AppTextKey::ValueSyncCheckpointCurrent),
        SyncCheckpointState::Failed => app_text(AppTextKey::ValueSyncCheckpointFailed),
    }
}

fn path_or_none(path: Option<&PathBuf>) -> String {
    path.map(|value| value.display().to_string())
        .unwrap_or_else(|| app_text(AppTextKey::ValueNone))
}

impl Render for SettingsWindowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let navigation_buttons = SETTINGS_NAVIGATION_ORDER
            .iter()
            .copied()
            .map(|view| self.navigation_button(view, cx).into_any_element())
            .collect::<Vec<_>>();

        app_window_shell(
            APP_UI_THEME.foundation.surfaces.panel_background,
            app_stack_v(0.0)
                .size_full()
                .bg(rgb(APP_UI_THEME.foundation.surfaces.panel_background))
                .overflow_hidden()
                .child(
                    app_stack_v(0.0)
                        .w_full()
                        .h(px(APP_UI_THEME.shells.settings_chrome_height_px))
                        .bg(rgb(APP_UI_THEME.foundation.surfaces.chrome_background))
                        .child(utility_title_row(app_shared_text(
                            AppTextKey::SettingsTitle,
                        )))
                        .child(
                            app_cluster(APP_UI_THEME.shells.settings_navigation_row_gap_px)
                                .w_full()
                                .justify_center()
                                .pt(px(APP_UI_THEME.shells.settings_navigation_row_padding_px))
                                .pb(px(APP_UI_THEME.shells.settings_navigation_row_padding_px))
                                .children(navigation_buttons),
                        ),
                )
                .child(section_divider())
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(self.settings_panel_content(window, cx)),
                ),
        )
    }
}

fn settings_panel_label_key(view: SettingsPanelViewKey) -> AppTextKey {
    match view {
        SettingsPanelViewKey::Account => AppTextKey::SettingsNavAccounts,
        SettingsPanelViewKey::Farm => AppTextKey::SettingsNavFarm,
        SettingsPanelViewKey::Settings => AppTextKey::SettingsNavSettings,
        SettingsPanelViewKey::About => AppTextKey::SettingsNavAbout,
    }
}

fn settings_panel_spec(view: SettingsPanelViewKey) -> (&'static str, IconName) {
    match view {
        SettingsPanelViewKey::Account => ("settings-nav-accounts", IconName::CircleUser),
        SettingsPanelViewKey::Farm => ("settings-nav-farm", IconName::Map),
        SettingsPanelViewKey::Settings => ("settings-nav-settings", IconName::Settings2),
        SettingsPanelViewKey::About => ("settings-nav-about", IconName::Info),
    }
}

#[derive(Clone, Copy)]
struct HomeStatusPresentation {
    indicator_color: u32,
    label_key: AppTextKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FarmSetupOnboardingCardSpec {
    title_key: AppTextKey,
    body_key: AppTextKey,
    action_key: Option<AppTextKey>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SettingsInventorySectionSpec {
    title_key: AppTextKey,
    field_keys: &'static [AppTextKey],
}

const SETTINGS_NAVIGATION_ORDER: &[SettingsPanelViewKey] = &[
    SettingsPanelViewKey::Account,
    SettingsPanelViewKey::Farm,
    SettingsPanelViewKey::Settings,
    SettingsPanelViewKey::About,
];

#[cfg(test)]
const SETTINGS_FARM_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::HomeFarmSetupFieldFarmName,
    AppTextKey::SettingsFarmFieldTimezone,
    AppTextKey::SettingsFarmFieldCurrency,
];

#[cfg(test)]
const SETTINGS_PICKUP_LOCATIONS_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::SettingsPickupLocationsFieldLabel,
    AppTextKey::SettingsPickupLocationsFieldAddress,
    AppTextKey::SettingsPickupLocationsFieldDirections,
    AppTextKey::SettingsPickupLocationsFieldDefault,
];

#[cfg(test)]
const SETTINGS_OPERATING_RULES_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::SettingsOperatingRulesFieldPromiseLeadTime,
    AppTextKey::SettingsOperatingRulesFieldSubstitutionPolicy,
    AppTextKey::SettingsOperatingRulesFieldMissedPickupPolicy,
];

#[cfg(test)]
const SETTINGS_FULFILLMENT_WINDOWS_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::SettingsFulfillmentWindowsFieldLabel,
    AppTextKey::SettingsFulfillmentWindowsFieldPickupLocation,
    AppTextKey::SettingsFulfillmentWindowsFieldStartsAt,
    AppTextKey::SettingsFulfillmentWindowsFieldEndsAt,
    AppTextKey::SettingsFulfillmentWindowsFieldOrderCutoff,
];

#[cfg(test)]
const SETTINGS_BLACKOUT_PERIODS_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::SettingsBlackoutPeriodsFieldLabel,
    AppTextKey::SettingsBlackoutPeriodsFieldStartsAt,
    AppTextKey::SettingsBlackoutPeriodsFieldEndsAt,
];

#[cfg(test)]
const SETTINGS_READINESS_SECTION_FIELDS: &[AppTextKey] = &[
    AppTextKey::SettingsReadinessFieldMissingProfileBasics,
    AppTextKey::SettingsReadinessFieldMissingPickupLocation,
    AppTextKey::SettingsReadinessFieldMissingFulfillmentWindow,
    AppTextKey::SettingsReadinessFieldMissingOperatingRules,
    AppTextKey::SettingsReadinessFieldInvalidTimingConflicts,
];

#[cfg(test)]
const SETTINGS_FARM_PANEL_SECTIONS: &[SettingsInventorySectionSpec] = &[
    SettingsInventorySectionSpec {
        title_key: AppTextKey::HomeFarmSetupSectionFarm,
        field_keys: SETTINGS_FARM_SECTION_FIELDS,
    },
    SettingsInventorySectionSpec {
        title_key: AppTextKey::SettingsPickupLocationsSectionLabel,
        field_keys: SETTINGS_PICKUP_LOCATIONS_SECTION_FIELDS,
    },
];

#[cfg(test)]
const SETTINGS_OPERATIONS_PANEL_SECTIONS: &[SettingsInventorySectionSpec] = &[
    SettingsInventorySectionSpec {
        title_key: AppTextKey::SettingsOperatingRulesSectionLabel,
        field_keys: SETTINGS_OPERATING_RULES_SECTION_FIELDS,
    },
    SettingsInventorySectionSpec {
        title_key: AppTextKey::SettingsFulfillmentWindowsSectionLabel,
        field_keys: SETTINGS_FULFILLMENT_WINDOWS_SECTION_FIELDS,
    },
    SettingsInventorySectionSpec {
        title_key: AppTextKey::SettingsBlackoutPeriodsSectionLabel,
        field_keys: SETTINGS_BLACKOUT_PERIODS_SECTION_FIELDS,
    },
    SettingsInventorySectionSpec {
        title_key: AppTextKey::SettingsReadinessSectionLabel,
        field_keys: SETTINGS_READINESS_SECTION_FIELDS,
    },
];

fn shared_shell_header(
    runtime: &DesktopAppRuntimeSummary,
    on_select_marketplace: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_farm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_account: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let can_enter_farmer_workspace = runtime.personal_projection.entry.can_enter_farmer_workspace;
    let is_marketplace_active =
        runtime.shell_projection.active_surface != radroots_studio_app_models::ActiveSurface::Farmer;
    let farm_name = home_saved_farm(runtime).map(|farm| farm.display_name.clone());
    let account_label = shell_account_label(runtime);

    app_surface_panel(
        div()
            .w_full()
            .px(px(APP_UI_THEME.shells.home_card_padding_px))
            .py(px(APP_UI_THEME.foundation.spacing.small_px))
            .flex()
            .justify_between()
            .items_center()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(app_text_label(app_shared_text(AppTextKey::AppName)))
                    .when_some(farm_name, |this, farm_name| {
                        this.child(home_body_text(farm_name))
                    }),
            )
            .child(
                app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                    .items_center()
                    .when(can_enter_farmer_workspace, |this| {
                        this.child(
                            shared_shell_mode_button(
                                "shell-mode-marketplace",
                                AppTextKey::HomeHeaderMarketplaceMode,
                                is_marketplace_active,
                                on_select_marketplace,
                                cx,
                            )
                            .into_any_element(),
                        )
                        .child(
                            shared_shell_mode_button(
                                "shell-mode-farm",
                                AppTextKey::HomeHeaderFarmMode,
                                !is_marketplace_active,
                                on_select_farm,
                                cx,
                            )
                            .into_any_element(),
                        )
                    })
                    .child(shell_account_entry(
                        runtime,
                        account_label,
                        on_open_account,
                        cx,
                    )),
            ),
    )
}

fn shared_shell_mode_button(
    id: &'static str,
    key: AppTextKey,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    if is_active {
        div()
            .id(id)
            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
            .child(app_shared_text(key))
            .into_any_element()
    } else {
        action_button_compact(id, app_shared_text(key), on_click, cx).into_any_element()
    }
}

fn shell_account_entry(
    runtime: &DesktopAppRuntimeSummary,
    account_label: String,
    on_open_account: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    if runtime.personal_projection.entry.state == PersonalEntryState::Guest {
        action_button_compact(
            "shell-account-entry",
            app_shared_text(AppTextKey::HomeHeaderAccountSetupAction),
            on_open_account,
            cx,
        )
        .into_any_element()
    } else {
        div()
            .id("shell-account-label")
            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
            .child(account_label)
            .into_any_element()
    }
}

fn shell_account_label(runtime: &DesktopAppRuntimeSummary) -> String {
    runtime
        .settings_account_projection
        .selected_account
        .as_ref()
        .and_then(|account| {
            account
                .account
                .label
                .as_ref()
                .map(|label| label.trim().to_owned())
                .filter(|label| !label.is_empty())
                .or_else(|| Some(account.account.npub.clone()))
        })
        .unwrap_or_else(|| app_shared_text(AppTextKey::HomeHeaderGuestLabel).to_string())
}

fn buyer_workspace_title_block(title_key: AppTextKey, body_key: AppTextKey) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(app_shared_text(title_key)),
        )
        .child(
            div()
                .w_full()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(app_shared_text(body_key)),
        )
}

fn buyer_listings_feed(
    section: PersonalSection,
    rows: &[BuyerListingRow],
    selected_product_id: Option<ProductId>,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
        .w_full()
        .children(
            rows.iter()
                .enumerate()
                .map(|(index, row)| {
                    buyer_listing_card(
                        index,
                        section,
                        row,
                        selected_product_id == Some(row.product_id),
                        cx,
                    )
                })
                .collect::<Vec<_>>(),
        )
}

fn buyer_listing_card(
    index: usize,
    section: PersonalSection,
    row: &BuyerListingRow,
    is_selected: bool,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    let subtitle = row
        .subtitle
        .as_deref()
        .map(str::trim)
        .filter(|subtitle| !subtitle.is_empty())
        .map(str::to_owned);
    app_button_card(
        ("buyer-listing-open", index),
        is_selected,
        cx.listener({
            let product_id = row.product_id;
            move |this, _, _, cx| this.open_personal_product_detail(section, product_id, cx)
        }),
        cx,
        div()
            .w_full()
            .min_w_0()
            .p(px(APP_UI_THEME.shells.home_card_padding_px))
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                    .child(product_display_title(row.title.as_str())),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .text_size(px(APP_UI_THEME
                                        .foundation
                                        .typography
                                        .utility_title_text_px))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(APP_UI_THEME.foundation.text.accent))
                                    .child(row.farm_display_name.clone()),
                            )
                            .when_some(subtitle, |this, subtitle| {
                                this.child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .text_size(px(APP_UI_THEME
                                            .foundation
                                            .typography
                                            .utility_title_text_px))
                                        .line_height(relative(1.2))
                                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                        .child(subtitle),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(buyer_listing_price_text(&row.price)),
                    ),
            )
            .child(
                app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .child(buyer_listing_chip(buyer_listing_next_window_text(row)))
                    .child(buyer_listing_chip(buyer_listing_fulfillment_methods_text(
                        &row.fulfillment_methods,
                    )))
                    .child(buyer_listing_chip(
                        buyer_listing_stock_or_availability_text(row),
                    )),
            ),
    )
    .into_any_element()
}

fn buyer_listing_chip(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .min_w_0()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.window_background))
        .rounded(px(APP_UI_THEME.foundation.radii.small_px))
        .px(px(8.0))
        .py(px(6.0))
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::MEDIUM)
        .line_height(relative(1.1))
        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
        .child(content.into())
}

fn buyer_listing_next_window_text(row: &BuyerListingRow) -> String {
    row.next_fulfillment_window_label
        .clone()
        .unwrap_or_else(|| row.availability.label.clone())
}

fn buyer_listing_fulfillment_methods_text(methods: &BTreeSet<FarmOrderMethod>) -> String {
    if methods.is_empty() {
        return app_shared_text(AppTextKey::ValueNone).to_string();
    }

    methods
        .iter()
        .map(|method| app_shared_text(home_farm_order_method_label_key(*method)).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn buyer_listing_stock_or_availability_text(row: &BuyerListingRow) -> String {
    match row.stock.quantity {
        Some(quantity) => match row.stock.unit_label.as_deref() {
            Some(unit_label) if !unit_label.trim().is_empty() => format!("{quantity} {unit_label}"),
            Some(_) | None => quantity.to_string(),
        },
        None => row.availability.label.clone(),
    }
}

fn buyer_listing_price_text(price: &ProductPricePresentation) -> String {
    let dollars = price.amount_minor_units / 100;
    let cents = price.amount_minor_units % 100;

    format!("${dollars}.{cents:02} / {}", price.unit_label)
}

fn buyer_product_detail_card(
    detail: &BuyerProductDetailProjection,
    replace_confirmation: Option<&BuyerCartReplaceConfirmationProjection>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_decrease_quantity: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_increase_quantity: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_add_to_cart: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm_replace: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_keep_current_cart: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_surface_card(
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        app_stack_v(4.0)
                            .flex_1()
                            .min_w_0()
                            .child(app_text_value(product_display_title(
                                detail.listing.title.as_str(),
                            )))
                            .child(settings_badge_text(
                                detail.listing.farm_display_name.clone(),
                            )),
                    )
                    .child(text_button(
                        "buyer-detail-back",
                        app_shared_text(AppTextKey::PersonalDetailBackAction),
                        on_close,
                        cx,
                    )),
            )
            .when_some(
                detail
                    .detail_text
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
                |this, detail_text| this.child(home_body_text(detail_text)),
            )
            .child(
                app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .child(buyer_listing_chip(buyer_listing_price_text(
                        &detail.listing.price,
                    )))
                    .child(buyer_listing_chip(buyer_listing_next_window_text(
                        &detail.listing,
                    )))
                    .child(buyer_listing_chip(buyer_listing_fulfillment_methods_text(
                        &detail.listing.fulfillment_methods,
                    )))
                    .child(buyer_listing_chip(
                        buyer_listing_stock_or_availability_text(&detail.listing),
                    )),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(app_text_label(app_shared_text(
                        AppTextKey::PersonalDetailQuantityLabel,
                    )))
                    .child(
                        app_stack_h(APP_UI_THEME.foundation.spacing.small_px)
                            .child(action_button_compact(
                                "buyer-detail-quantity-decrease",
                                SharedString::from("-"),
                                on_decrease_quantity,
                                cx,
                            ))
                            .child(
                                div()
                                    .min_w(px(36.0))
                                    .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                    .child(detail.selected_quantity.to_string()),
                            )
                            .child(action_button_compact(
                                "buyer-detail-quantity-increase",
                                SharedString::from("+"),
                                on_increase_quantity,
                                cx,
                            )),
                    ),
            )
            .when_some(replace_confirmation, |this, replace_confirmation| {
                this.child(app_surface_panel(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .p(px(APP_UI_THEME.shells.home_card_padding_px))
                        .child(app_text_label(app_shared_text(
                            AppTextKey::PersonalDetailReplaceCartTitle,
                        )))
                        .child(home_body_text(format!(
                            "{} {} {}.",
                            replace_confirmation.current_farm_display_name,
                            app_shared_text(AppTextKey::PersonalDetailReplaceCartBody),
                            replace_confirmation.incoming_farm_display_name,
                        )))
                        .child(
                            app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                                .w_full()
                                .child(action_button_primary(
                                    "buyer-detail-confirm-replace",
                                    app_shared_text(AppTextKey::PersonalDetailReplaceCartAction),
                                    on_confirm_replace,
                                    cx,
                                ))
                                .child(action_button_compact(
                                    "buyer-detail-keep-current",
                                    app_shared_text(
                                        AppTextKey::PersonalDetailKeepCurrentCartAction,
                                    ),
                                    on_keep_current_cart,
                                    cx,
                                )),
                        ),
                ))
            })
            .child(action_button_primary(
                "buyer-detail-add-to-cart",
                app_shared_text(AppTextKey::PersonalDetailAddToCartAction),
                on_add_to_cart,
                cx,
            )),
    )
}

fn buyer_cart_card(
    cart: &BuyerCartProjection,
    summary: &BuyerCheckoutSummaryProjection,
    checkout_open: bool,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    app_surface_card(
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .children(
                cart.lines
                    .iter()
                    .enumerate()
                    .map(|(index, line)| buyer_cart_line_card(index, line, cx).into_any_element())
                    .collect::<Vec<_>>(),
            )
            .child(app_surface_panel(
                app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .p(px(APP_UI_THEME.shells.home_card_padding_px))
                    .child(app_text_label(app_shared_text(
                        AppTextKey::PersonalOrderSummaryTitle,
                    )))
                    .child(label_value_list(buyer_order_summary_rows(summary))),
            ))
            .when(!checkout_open, |this| {
                this.child(action_button_primary(
                    "buyer-cart-open-checkout",
                    app_shared_text(AppTextKey::PersonalCartContinueCheckoutAction),
                    cx.listener(|this, _, window, cx| this.open_personal_checkout(window, cx)),
                    cx,
                ))
            }),
    )
}

fn buyer_cart_line_card(
    index: usize,
    line: &radroots_studio_app_models::BuyerCartLineProjection,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    app_surface_panel(
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .p(px(APP_UI_THEME.shells.home_card_padding_px))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        app_stack_v(4.0)
                            .flex_1()
                            .min_w_0()
                            .child(app_text_label(product_display_title(line.title.as_str())))
                            .child(settings_badge_text(line.farm_display_name.clone())),
                    )
                    .child(action_button_compact(
                        ("buyer-cart-remove-line", index),
                        app_shared_text(AppTextKey::PersonalCartRemoveLineAction),
                        cx.listener({
                            let product_id = line.product_id;
                            move |this, _, _, cx| this.remove_personal_cart_line(product_id, cx)
                        }),
                        cx,
                    )),
            )
            .child(label_value_list(vec![
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalCartLineQuantityLabel),
                    line.quantity.to_string(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalCartLineUnitPriceLabel),
                    buyer_listing_price_text(&line.unit_price),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalCartLineTotalLabel),
                    buyer_money_text(
                        line.line_total_minor_units,
                        line.unit_price.currency_code.as_str(),
                    ),
                ),
            ]))
            .child(buyer_listing_chip(line.fulfillment_summary.clone())),
    )
}

fn buyer_checkout_card(
    form: &BuyerCheckoutFormState,
    checkout: &radroots_studio_app_models::BuyerCheckoutProjection,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_place_order: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_surface_card(
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(app_text_value(app_shared_text(
                        AppTextKey::PersonalCheckoutTitle,
                    )))
                    .child(text_button(
                        "buyer-checkout-back",
                        app_shared_text(AppTextKey::PersonalCheckoutBackAction),
                        on_close,
                        cx,
                    )),
            )
            .child(home_body_text(app_shared_text(
                AppTextKey::PersonalCheckoutLocalOnlyBody,
            )))
            .child(app_surface_panel(
                app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .p(px(APP_UI_THEME.shells.home_card_padding_px))
                    .child(app_text_label(app_shared_text(
                        AppTextKey::PersonalOrderSummaryTitle,
                    )))
                    .child(label_value_list(buyer_order_summary_rows(
                        &checkout.summary,
                    ))),
            ))
            .child(app_surface_panel(
                app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .p(px(APP_UI_THEME.shells.home_card_padding_px))
                    .child(app_text_label(app_shared_text(
                        AppTextKey::PersonalFulfillmentTitle,
                    )))
                    .child(home_body_text(
                        checkout
                            .summary
                            .fulfillment_summary
                            .clone()
                            .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone).to_string()),
                    )),
            ))
            .child(app_form_section(
                app_shared_text(AppTextKey::PersonalCheckoutContactTitle),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalCheckoutFieldName),
                            Option::<SharedString>::None,
                        ),
                        &form.name_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalCheckoutFieldEmail),
                            Option::<SharedString>::None,
                        ),
                        &form.email_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalCheckoutFieldPhone),
                            Option::<SharedString>::None,
                        ),
                        &form.phone_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalCheckoutFieldOrderNote),
                            Option::<SharedString>::None,
                        ),
                        &form.order_note_input,
                        false,
                    )),
            ))
            .child(if checkout.can_place_order {
                action_button_primary(
                    "buyer-checkout-place-order",
                    app_shared_text(AppTextKey::PersonalCheckoutPlaceOrderAction),
                    on_place_order,
                    cx,
                )
                .into_any_element()
            } else {
                action_button_primary_disabled(
                    "buyer-checkout-place-order",
                    app_shared_text(AppTextKey::PersonalCheckoutPlaceOrderAction),
                    cx,
                )
                .into_any_element()
            }),
    )
}

fn buyer_order_summary_rows(summary: &BuyerCheckoutSummaryProjection) -> Vec<LabelValueRow> {
    vec![
        LabelValueRow::new(
            app_shared_text(AppTextKey::PersonalSummaryFarmLabel),
            summary
                .farm_display_name
                .clone()
                .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone).to_string()),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::PersonalSummaryItemsLabel),
            summary.line_count.to_string(),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::PersonalSummarySubtotalLabel),
            summary
                .subtotal_minor_units
                .zip(summary.currency_code.as_deref())
                .map(|(amount, currency_code)| buyer_money_text(amount, currency_code))
                .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone).to_string()),
        ),
    ]
}

fn buyer_money_text(amount_minor_units: u32, currency_code: &str) -> String {
    let dollars = amount_minor_units / 100;
    let cents = amount_minor_units % 100;

    if currency_code == "USD" {
        format!("${dollars}.{cents:02}")
    } else {
        format!("{currency_code} {dollars}.{cents:02}")
    }
}

fn buyer_orders_list_card(
    rows: &[BuyerOrdersListRow],
    selected_order_id: Option<OrderId>,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    home_card(
        app_shared_text(AppTextKey::PersonalOrdersListTitle),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .children(
                rows.iter()
                    .enumerate()
                    .map(|(index, row)| {
                        buyer_orders_list_entry(
                            index,
                            row,
                            selected_order_id == Some(row.order_id),
                            cx,
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
    )
    .into_any_element()
}

fn buyer_orders_list_entry(
    index: usize,
    row: &BuyerOrdersListRow,
    is_selected: bool,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    app_button_card(
        ("buyer-order-open", index),
        is_selected,
        cx.listener({
            let order_id = row.order_id;
            move |this, _, _, cx| this.open_personal_order_detail(order_id, cx)
        }),
        cx,
        div()
            .w_full()
            .min_w_0()
            .p(px(APP_UI_THEME.shells.home_card_padding_px))
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.foundation.spacing.small_px))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        app_stack_v(4.0)
                            .flex_1()
                            .min_w_0()
                            .child(app_text_label(row.order_number.clone()))
                            .child(settings_badge_text(row.farm_display_name.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(status_indicator(buyer_orders_status_color(row.status)))
                            .child(
                                div()
                                    .text_size(px(APP_UI_THEME
                                        .foundation
                                        .typography
                                        .utility_title_text_px))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                    .child(app_shared_text(buyer_orders_status_key(row.status))),
                            ),
                    ),
            )
            .child(buyer_listing_chip(row.fulfillment_summary.clone())),
    )
    .into_any_element()
}

fn buyer_order_detail_card(detail: &BuyerOrderDetailProjection) -> AnyElement {
    home_card(
        app_shared_text(AppTextKey::PersonalOrdersDetailTitle),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(app_heading_section(detail.order_number.clone()))
            .child(settings_badge_text(detail.farm_display_name.clone()))
            .child(label_value_list([
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailFarmLabel),
                    detail.farm_display_name.clone(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailStatusLabel),
                    app_shared_text(buyer_orders_status_key(detail.status)),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailFulfillmentLabel),
                    detail.fulfillment_summary.clone(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailNoteLabel),
                    order_optional_text(detail.order_note.as_deref()),
                ),
            ]))
            .child(app_form_section(
                app_shared_text(AppTextKey::PersonalOrdersDetailItemsTitle),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .children(
                        detail
                            .items
                            .iter()
                            .map(order_detail_item_row)
                            .collect::<Vec<_>>(),
                    )
                    .when(detail.items.is_empty(), |this| {
                        this.child(home_body_text(app_shared_text(AppTextKey::ValueNone)))
                    }),
            )),
    )
    .into_any_element()
}

fn buyer_order_detail_empty_card() -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::PersonalOrdersDetailTitle),
        home_body_text(app_shared_text(AppTextKey::PersonalOrdersDetailEmptyBody)),
    )
}

fn buyer_orders_status_key(status: BuyerOrderStatus) -> AppTextKey {
    match status {
        BuyerOrderStatus::Placed => AppTextKey::PersonalOrdersStatusPlaced,
        BuyerOrderStatus::Scheduled => AppTextKey::PersonalOrdersStatusScheduled,
        BuyerOrderStatus::Ready => AppTextKey::PersonalOrdersStatusReady,
        BuyerOrderStatus::Completed => AppTextKey::PersonalOrdersStatusCompleted,
        BuyerOrderStatus::Refunded => AppTextKey::PersonalOrdersStatusRefunded,
    }
}

fn buyer_orders_status_color(status: BuyerOrderStatus) -> u32 {
    match status {
        BuyerOrderStatus::Placed => APP_UI_THEME.components.app_status_indicator.attention,
        BuyerOrderStatus::Scheduled | BuyerOrderStatus::Ready => {
            APP_UI_THEME.components.app_status_indicator.online
        }
        BuyerOrderStatus::Completed | BuyerOrderStatus::Refunded => {
            APP_UI_THEME.components.app_status_indicator.offline
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupHomeSurface {
    IssueCard,
    ContinuePrompt,
    IdentityChoice,
    GenerateKeyStarting,
    SignerEntry,
}

fn startup_home_surface(runtime: &DesktopAppRuntimeSummary) -> StartupHomeSurface {
    if runtime.startup_issue.is_some() || runtime.startup_gate != AppStartupGate::SetupRequired {
        return StartupHomeSurface::IssueCard;
    }

    match runtime.logged_out_startup.phase {
        LoggedOutStartupPhase::ContinuePrompt => StartupHomeSurface::ContinuePrompt,
        LoggedOutStartupPhase::IdentityChoice => StartupHomeSurface::IdentityChoice,
        LoggedOutStartupPhase::GenerateKeyStarting => StartupHomeSurface::GenerateKeyStarting,
        LoggedOutStartupPhase::SignerEntry => StartupHomeSurface::SignerEntry,
    }
}

fn startup_home_shell(
    runtime: &DesktopAppRuntimeSummary,
    startup_notice: Option<&str>,
    signer_entry: Option<&StartupSignerEntryState>,
    connect_state: &StartupSignerConnectState,
    on_continue: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_browse_marketplace: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_generate_key: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_connect_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let surface = startup_home_surface(runtime);

    app_window_shell(
        APP_UI_THEME.foundation.surfaces.window_background,
        div()
            .size_full()
            .bg(rgb(APP_UI_THEME.foundation.surfaces.window_background))
            .child(
                div()
                    .size_full()
                    .p(px(APP_UI_THEME.shells.home_window_padding_px))
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                app_stack_v(APP_UI_THEME.shells.startup_stack_gap_px)
                                    .w_full()
                                    .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
                                    .mx_auto()
                                    .items_center()
                                    .child(startup_home_title(surface))
                                    .child(startup_home_tagline())
                                    .child(match surface {
                                        StartupHomeSurface::ContinuePrompt => app_stack_v(
                                            APP_UI_THEME.shells.startup_stack_gap_px,
                                        )
                                        .items_center()
                                        .child(action_button_primary(
                                            "home-continue",
                                            app_shared_text(AppTextKey::HomeSetupContinueAction),
                                            on_continue,
                                            cx,
                                        ))
                                        .child(action_button(
                                            "home-browse-marketplace",
                                            app_shared_text(
                                                AppTextKey::HomeSetupBrowseMarketplaceAction,
                                            ),
                                            on_browse_marketplace,
                                            cx,
                                        ))
                                        .when_some(startup_notice, |this, error| {
                                            this.child(
                                                div()
                                                    .w_full()
                                                    .text_center()
                                                    .child(home_body_text(error.to_owned())),
                                            )
                                        })
                                        .into_any_element(),
                                        StartupHomeSurface::IdentityChoice => {
                                            app_stack_v(APP_UI_THEME.shells.startup_stack_gap_px)
                                                .items_center()
                                                .child(action_button_primary(
                                                    "home-generate-key",
                                                    app_shared_text(
                                                        AppTextKey::HomeSetupGenerateKeyAction,
                                                    ),
                                                    on_generate_key,
                                                    cx,
                                                ))
                                                .child(action_button(
                                                    "home-connect-signer",
                                                    app_shared_text(
                                                        AppTextKey::HomeSetupConnectSignerAction,
                                                    ),
                                                    on_connect_signer,
                                                    cx,
                                                ))
                                                .when_some(startup_notice, |this, error| {
                                                    this.child(
                                                        div().w_full().text_center().child(
                                                            home_body_text(error.to_owned()),
                                                        ),
                                                    )
                                                })
                                                .into_any_element()
                                        }
                                        StartupHomeSurface::GenerateKeyStarting => {
                                            app_stack_v(APP_UI_THEME.shells.startup_stack_gap_px)
                                                .items_center()
                                                .child(action_button_primary_disabled(
                                                    "home-generate-key",
                                                    app_shared_text(
                                                        AppTextKey::HomeSetupGenerateKeyAction,
                                                    ),
                                                    cx,
                                                ))
                                                .into_any_element()
                                        }
                                        StartupHomeSurface::SignerEntry => {
                                            startup_signer_entry_surface(
                                                signer_entry,
                                                connect_state,
                                                startup_notice,
                                                on_submit_signer,
                                                on_back,
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        StartupHomeSurface::IssueCard => app_surface_card(
                                            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                                                .w_full()
                                                .items_center()
                                                .child(app_heading_section(app_shared_text(
                                                    AppTextKey::MetadataStartupIssue,
                                                )))
                                                .child(startup_home_body(runtime)),
                                        )
                                        .into_any_element(),
                                    }),
                            ),
                    ),
            ),
    )
}

fn startup_home_title(surface: StartupHomeSurface) -> impl IntoElement {
    let (animation_id, title_key) = if surface == StartupHomeSurface::GenerateKeyStarting {
        ("startup-title-starting", AppTextKey::HomeSetupStarting)
    } else {
        ("startup-title-radroots", AppTextKey::HomeSetupTitle)
    };

    div()
        .text_center()
        .child(app_heading_view(app_shared_text(title_key)))
        .with_animation(
            animation_id,
            Animation::new(Duration::from_millis(180)),
            |this, delta| this.opacity(delta),
        )
}

fn startup_home_tagline() -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME
            .foundation
            .typography
            .startup_tagline_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .text_center()
        .child(app_shared_text(AppTextKey::HomeSetupTagline))
}

fn startup_signer_entry_surface(
    signer_entry: Option<&StartupSignerEntryState>,
    connect_state: &StartupSignerConnectState,
    startup_notice: Option<&str>,
    on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let source_input = signer_entry
        .map(|signer_entry| signer_entry.input.read(cx).value().to_string())
        .unwrap_or_default();
    let preview =
        startup_signer_preview_summary_for_connect_state(source_input.as_str(), connect_state);
    let parse_error = if source_input.trim().is_empty()
        || !matches!(connect_state, StartupSignerConnectState::Idle)
    {
        None
    } else {
        preview.as_ref().err().cloned()
    };
    let submit_enabled =
        preview.is_ok() && matches!(connect_state, StartupSignerConnectState::Idle);
    let source_input_is_editable = startup_signer_source_input_is_editable(connect_state);

    app_stack_v(APP_UI_THEME.shells.startup_stack_gap_px)
        .w_full()
        .items_center()
        .when_some(signer_entry, |this, signer_entry| {
            this.child(
                div()
                    .w_full()
                    .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
                    .id("home-signer-source-input")
                    .child(
                        app_text_input(&signer_entry.input, !source_input_is_editable)
                            .disabled(!source_input_is_editable)
                            .w_full(),
                    ),
            )
        })
        .when_some(preview.as_ref().ok(), |this, preview| {
            this.child(app_surface_card(
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .items_center()
                    .child(app_heading_section(app_shared_text(
                        AppTextKey::HomeSetupSignerReviewTitle,
                    )))
                    .child(label_value_list([
                        LabelValueRow::new(
                            app_shared_text(AppTextKey::HomeSetupSignerSourceLabel),
                            preview.source_label.clone(),
                        ),
                        LabelValueRow::new(
                            app_shared_text(AppTextKey::HomeSetupSignerSignerLabel),
                            preview.signer_npub.clone(),
                        ),
                        LabelValueRow::new(
                            app_shared_text(AppTextKey::HomeSetupSignerRelaysLabel),
                            preview.relays_label.clone(),
                        ),
                        LabelValueRow::new(
                            app_shared_text(AppTextKey::HomeSetupSignerPermissionsLabel),
                            preview.permissions_label.clone(),
                        ),
                    ])),
            ))
        })
        .when_some(startup_signer_status_spec(connect_state), |this, status| {
            this.child(app_surface_card(
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .items_center()
                    .child(app_heading_section(app_shared_text(status.0)))
                    .child(
                        status
                            .1
                            .map(|body| {
                                div()
                                    .w_full()
                                    .text_center()
                                    .child(home_body_text(body))
                                    .into_any_element()
                            })
                            .unwrap_or_else(|| div().into_any_element()),
                    ),
            ))
        })
        .when_some(parse_error, |this, error| {
            this.child(div().w_full().text_center().child(home_body_text(error)))
        })
        .child(if submit_enabled {
            action_button_primary(
                "home-connect-signer-submit",
                app_shared_text(AppTextKey::HomeSetupSignerConnectAction),
                on_submit_signer,
                cx,
            )
            .into_any_element()
        } else {
            action_button_primary_disabled(
                "home-connect-signer-submit",
                app_shared_text(AppTextKey::HomeSetupSignerConnectAction),
                cx,
            )
            .into_any_element()
        })
        .child(text_button(
            "home-signer-back",
            app_shared_text(AppTextKey::HomeSetupBackAction),
            on_back,
            cx,
        ))
        .when_some(startup_notice, |this, notice| {
            this.child(
                div()
                    .w_full()
                    .text_center()
                    .child(home_body_text(notice.to_owned())),
            )
        })
}

fn startup_signer_preview_summary(input: &str) -> Result<StartupSignerPreviewSummary, String> {
    let target = radroots_studio_app_remote_signer_preview(input).map_err(|error| error.to_string())?;
    let requested_permissions = target.requested_permission_labels();

    Ok(StartupSignerPreviewSummary {
        source_label: target.source_label().to_owned(),
        signer_npub: target.signer_identity.public_key_npub.clone(),
        relays_label: startup_signer_csv_or_none(target.relays.as_slice()),
        permissions_label: startup_signer_csv_or_none(requested_permissions.as_slice()),
    })
}

fn startup_signer_preview_summary_for_connect_state(
    input: &str,
    connect_state: &StartupSignerConnectState,
) -> Result<StartupSignerPreviewSummary, String> {
    let mut preview = startup_signer_preview_summary(input)?;

    match connect_state {
        StartupSignerConnectState::Idle | StartupSignerConnectState::Connecting => {}
        StartupSignerConnectState::PendingApproval {
            pending_session, ..
        } => {
            preview.signer_npub = pending_session
                .record
                .signer_identity
                .public_key_npub
                .clone();
            preview.relays_label =
                startup_signer_csv_or_none(pending_session.record.relays.as_slice());
            preview.permissions_label = startup_signer_requested_permissions_label();
        }
        StartupSignerConnectState::Approved {
            pending_session,
            approved_session,
            ..
        } => {
            preview.signer_npub = pending_session
                .record
                .signer_identity
                .public_key_npub
                .clone();
            preview.relays_label = startup_signer_csv_or_none(approved_session.relays.as_slice());
            preview.permissions_label = startup_signer_permissions_label(
                approved_session
                    .approved_permissions
                    .as_slice()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            );
        }
    }

    Ok(preview)
}

fn startup_signer_source_input_is_editable(connect_state: &StartupSignerConnectState) -> bool {
    matches!(connect_state, StartupSignerConnectState::Idle)
}

fn startup_signer_csv_or_none(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values.join(", ")
}

fn startup_signer_requested_permissions_label() -> String {
    startup_signer_permissions_label(
        radroots_studio_app_remote_signer_requested_permissions()
            .as_slice()
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
}

fn startup_signer_permissions_label(permissions: Vec<String>) -> String {
    startup_signer_csv_or_none(permissions.as_slice())
}

fn startup_signer_status_spec(
    connect_state: &StartupSignerConnectState,
) -> Option<(AppTextKey, Option<String>)> {
    match connect_state {
        StartupSignerConnectState::Idle => None,
        StartupSignerConnectState::Connecting => {
            Some((AppTextKey::HomeSetupSignerConnectingTitle, None))
        }
        StartupSignerConnectState::PendingApproval {
            auth_challenge_url, ..
        } => Some(match auth_challenge_url {
            Some(url) => (
                AppTextKey::HomeSetupSignerAuthChallengeTitle,
                Some(url.clone()),
            ),
            None => (AppTextKey::HomeSetupSignerPendingTitle, None),
        }),
        StartupSignerConnectState::Approved {
            auth_challenge_url, ..
        } => Some((
            AppTextKey::HomeSetupSignerApprovedTitle,
            auth_challenge_url.clone(),
        )),
    }
}

fn startup_signer_transport_failure_requires_notice(message: &str) -> bool {
    message != "remote signer did not respond yet"
}

fn startup_home_body(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let body = runtime
        .startup_issue
        .clone()
        .unwrap_or_else(|| app_shared_text(AppTextKey::HomeTodayEmptySetupBody).to_string());

    div().w_full().text_center().child(home_body_text(body))
}

async fn connect_default_relay(relay_url: String) -> Result<RadrootsNostrClient, String> {
    let client = RadrootsNostrClient::new_signerless();
    client
        .add_relay(relay_url.as_str())
        .await
        .map_err(|error| format!("failed to add relay `{relay_url}`: {error}"))?;
    client.connect().await;
    Ok(client)
}

struct StartupAppInitResult {
    relay_client: RadrootsNostrClient,
}

async fn run_startup_app_init(relay_url: String) -> Result<StartupAppInitResult, String> {
    let relay_client = connect_default_relay(relay_url).await?;
    Ok(StartupAppInitResult { relay_client })
}

async fn run_startup_signer_connect(
    source_input: String,
) -> Result<RadrootsAppRemoteSignerPendingSession, String> {
    radroots_studio_app_remote_signer_connect_pending(source_input.as_str())
        .map_err(|error| error.to_string())
}

async fn run_startup_signer_pending_poll(
    record: radroots_studio_app_remote_signer::RadrootsAppRemoteSignerSessionRecord,
    client_secret_key_hex: String,
) -> StartupSignerPollCycleResult {
    let mut auth_challenge_url = None;
    let outcome = radroots_studio_app_remote_signer_poll_pending_session_with_progress(
        &record,
        client_secret_key_hex.as_str(),
        |progress| match progress {
            radroots_studio_app_remote_signer::RadrootsAppRemoteSignerProgressUpdate::AuthChallenge {
                url,
            } => auth_challenge_url = Some(url),
        },
    )
    .map_err(|error| error.to_string());

    StartupSignerPollCycleResult {
        auth_challenge_url,
        outcome,
    }
}

fn home_sidebar(
    runtime: &DesktopAppRuntimeSummary,
    on_select_today: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_orders: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_pack_day: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let selected_section = selected_farmer_section(runtime);
    let workspace_available = farmer_products_available(runtime);
    let pack_day_available = farmer_pack_day_available(runtime);
    let navigation_sections =
        home_sidebar_navigation_sections(selected_section, workspace_available, pack_day_available);
    let on_select_today = Arc::new(on_select_today);
    let on_select_products = Arc::new(on_select_products);
    let on_select_orders = Arc::new(on_select_orders);
    let on_select_pack_day = Arc::new(on_select_pack_day);
    let mut navigation_elements = Vec::with_capacity(navigation_sections.len());
    for section in navigation_sections {
        let element = match section {
            FarmerSection::Today => {
                let on_click = Arc::clone(&on_select_today);
                home_sidebar_nav_button(
                    "home-nav-today",
                    AppTextKey::HomeNavToday,
                    true,
                    selected_section == FarmerSection::Today,
                    move |event, window, app| on_click(event, window, app),
                    cx,
                )
                .into_any_element()
            }
            FarmerSection::Products => {
                let on_click = Arc::clone(&on_select_products);
                home_sidebar_nav_button(
                    "home-nav-products",
                    AppTextKey::HomeNavProducts,
                    true,
                    selected_section == FarmerSection::Products,
                    move |event, window, app| on_click(event, window, app),
                    cx,
                )
                .into_any_element()
            }
            FarmerSection::Orders => {
                let on_click = Arc::clone(&on_select_orders);
                home_sidebar_nav_button(
                    "home-nav-orders",
                    AppTextKey::HomeNavOrders,
                    true,
                    selected_section == FarmerSection::Orders,
                    move |event, window, app| on_click(event, window, app),
                    cx,
                )
                .into_any_element()
            }
            FarmerSection::PackDay => {
                let on_click = Arc::clone(&on_select_pack_day);
                home_sidebar_nav_button(
                    "home-nav-pack-day",
                    AppTextKey::PackDayTitle,
                    true,
                    selected_section == FarmerSection::PackDay,
                    move |event, window, app| on_click(event, window, app),
                    cx,
                )
                .into_any_element()
            }
            FarmerSection::Farm => unreachable!(),
        };
        navigation_elements.push(element);
    }

    app_surface_sidebar(
        div()
            .h_full()
            .w(px(APP_UI_THEME.shells.home_sidebar_width_px))
            .p(px(APP_UI_THEME.shells.home_window_padding_px))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_start()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .children(navigation_elements),
            )
            .child(
                div().child(div().when_some(home_saved_farm(runtime), |this, farm| {
                    this.child(home_body_text(farm.display_name.clone()))
                })),
            ),
    )
}

fn buyer_sidebar(
    runtime: &DesktopAppRuntimeSummary,
    on_select_browse: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_search: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_cart: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_orders: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let selected_section = selected_personal_section(runtime);

    app_surface_sidebar(
        div()
            .h_full()
            .w(px(APP_UI_THEME.shells.home_sidebar_width_px))
            .p(px(APP_UI_THEME.shells.home_window_padding_px))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        buyer_sidebar_nav_button(
                            "buyer-nav-browse",
                            AppTextKey::HomeNavBrowse,
                            selected_section == PersonalSection::Browse,
                            on_select_browse,
                            cx,
                        )
                        .into_any_element(),
                    )
                    .child(
                        buyer_sidebar_nav_button(
                            "buyer-nav-search",
                            AppTextKey::HomeNavSearch,
                            selected_section == PersonalSection::Search,
                            on_select_search,
                            cx,
                        )
                        .into_any_element(),
                    )
                    .child(
                        buyer_sidebar_nav_button(
                            "buyer-nav-cart",
                            AppTextKey::HomeNavCart,
                            selected_section == PersonalSection::Cart,
                            on_select_cart,
                            cx,
                        )
                        .into_any_element(),
                    )
                    .child(
                        buyer_sidebar_nav_button(
                            "buyer-nav-orders",
                            AppTextKey::HomeNavOrders,
                            selected_section == PersonalSection::Orders,
                            on_select_orders,
                            cx,
                        )
                        .into_any_element(),
                    ),
            )
            .child(
                div().child(div().when_some(home_saved_farm(runtime), |this, farm| {
                    this.child(home_body_text(farm.display_name.clone()))
                })),
            ),
    )
}

fn buyer_sidebar_nav_button(
    id: &'static str,
    key: AppTextKey,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    if is_active {
        div()
            .id(id)
            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
            .child(app_shared_text(key))
            .into_any_element()
    } else {
        action_button(id, app_shared_text(key), on_click, cx).into_any_element()
    }
}

fn home_sidebar_navigation_sections(
    selected_section: FarmerSection,
    workspace_available: bool,
    pack_day_available: bool,
) -> Vec<FarmerSection> {
    let mut sections = vec![FarmerSection::Today];
    if workspace_available {
        sections.push(FarmerSection::Products);
        sections.push(FarmerSection::Orders);
    }
    if pack_day_available {
        sections.push(FarmerSection::PackDay);
    }

    if let Some(selected_index) = sections
        .iter()
        .position(|section| *section == selected_section)
    {
        let selected = sections.remove(selected_index);
        sections.insert(0, selected);
    }

    sections
}

fn selected_farmer_section(runtime: &DesktopAppRuntimeSummary) -> FarmerSection {
    match runtime.shell_projection.selected_section {
        ShellSection::Farmer(section) => section,
        ShellSection::Home | ShellSection::Personal(_) | ShellSection::Settings(_) => {
            FarmerSection::Today
        }
    }
}

fn selected_personal_section(runtime: &DesktopAppRuntimeSummary) -> PersonalSection {
    match runtime.shell_projection.selected_section {
        ShellSection::Personal(section) => section,
        ShellSection::Home | ShellSection::Farmer(_) | ShellSection::Settings(_) => {
            PersonalSection::Browse
        }
    }
}

fn personal_workspace_id(runtime: &DesktopAppRuntimeSummary) -> String {
    runtime
        .settings_account_projection
        .selected_account
        .as_ref()
        .map(|account| account.account.account_id.clone())
        .unwrap_or_else(|| "guest".to_owned())
}

fn farmer_products_available(runtime: &DesktopAppRuntimeSummary) -> bool {
    runtime.farm_setup_projection.has_saved_farm()
}

fn farmer_pack_day_available(runtime: &DesktopAppRuntimeSummary) -> bool {
    runtime
        .pack_day_projection
        .projection
        .fulfillment_window
        .is_some()
}

fn home_content_scroll_id(section: FarmerSection) -> &'static str {
    match section {
        FarmerSection::Products => "home-products-scroll",
        FarmerSection::Orders => "home-orders-scroll",
        FarmerSection::PackDay => "home-pack-day-scroll",
        FarmerSection::Today | FarmerSection::Farm => "home-today-scroll",
    }
}

fn buyer_content_scroll_id(section: PersonalSection) -> &'static str {
    match section {
        PersonalSection::Browse => "buyer-browse-scroll",
        PersonalSection::Search => "buyer-search-scroll",
        PersonalSection::Cart => "buyer-cart-scroll",
        PersonalSection::Orders => "buyer-orders-scroll",
    }
}

fn home_sidebar_nav_button(
    id: &'static str,
    key: AppTextKey,
    is_available: bool,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    if !is_available {
        return div().id(id).into_any_element();
    }

    if is_active {
        div()
            .id(id)
            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
            .font_weight(gpui::FontWeight::BOLD)
            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
            .child(app_shared_text(key))
            .into_any_element()
    } else {
        action_button(id, app_shared_text(key), on_click, cx).into_any_element()
    }
}

fn products_title_row(
    runtime: &DesktopAppRuntimeSummary,
    add_product_action: AnyElement,
) -> impl IntoElement {
    app_stack_h(APP_UI_THEME.shells.home_stack_gap_px)
        .w_full()
        .items_end()
        .justify_between()
        .child(
            app_stack_v(4.0)
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(app_shared_text(AppTextKey::ProductsTitle)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .when_some(home_saved_farm(runtime), |this, farm| {
                            this.child(farm.display_name.clone())
                        }),
                ),
        )
        .child(add_product_action)
}

fn products_controls_card(
    runtime: &DesktopAppRuntimeSummary,
    products_search: Option<&ProductsSearchState>,
    on_select_all_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_live_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_draft_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_products_needing_attention: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_paused_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_archived_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_sort_products_by_updated: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_sort_products_by_name: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_sort_products_by_availability: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_sort_products_by_stock: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_sort_products_by_price: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let selected_filter = runtime.products_projection.query.filter;
    let selected_sort = runtime.products_projection.query.sort;

    home_card(
        app_shared_text(AppTextKey::ProductsFiltersTitle),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .when_some(products_search, |this, products_search| {
                this.child(
                    app_text_input(&products_search.input, false)
                        .cleanable(true)
                        .w_full(),
                )
            })
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(choice_button(
                        "products-filter-all",
                        app_shared_text(AppTextKey::ProductsFilterAll),
                        selected_filter == ProductsFilter::All,
                        on_select_all_products,
                        cx,
                    ))
                    .child(choice_button(
                        "products-filter-live",
                        app_shared_text(AppTextKey::ProductsFilterLive),
                        selected_filter == ProductsFilter::Live,
                        on_select_live_products,
                        cx,
                    ))
                    .child(choice_button(
                        "products-filter-drafts",
                        app_shared_text(AppTextKey::ProductsFilterDrafts),
                        selected_filter == ProductsFilter::Drafts,
                        on_select_draft_products,
                        cx,
                    ))
                    .child(choice_button(
                        "products-filter-need-attention",
                        app_shared_text(AppTextKey::ProductsFilterNeedAttention),
                        selected_filter == ProductsFilter::NeedAttention,
                        on_select_products_needing_attention,
                        cx,
                    ))
                    .child(choice_button(
                        "products-filter-paused",
                        app_shared_text(AppTextKey::ProductsFilterPaused),
                        selected_filter == ProductsFilter::Paused,
                        on_select_paused_products,
                        cx,
                    ))
                    .child(choice_button(
                        "products-filter-archived",
                        app_shared_text(AppTextKey::ProductsFilterArchived),
                        selected_filter == ProductsFilter::Archived,
                        on_select_archived_products,
                        cx,
                    )),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                            .child(app_shared_text(AppTextKey::ProductsSortTitle)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(choice_button(
                                "products-sort-updated",
                                app_shared_text(AppTextKey::ProductsSortUpdated),
                                selected_sort == ProductsSort::Updated,
                                on_sort_products_by_updated,
                                cx,
                            ))
                            .child(choice_button(
                                "products-sort-name",
                                app_shared_text(AppTextKey::ProductsSortName),
                                selected_sort == ProductsSort::Name,
                                on_sort_products_by_name,
                                cx,
                            ))
                            .child(choice_button(
                                "products-sort-availability",
                                app_shared_text(AppTextKey::ProductsSortAvailability),
                                selected_sort == ProductsSort::Availability,
                                on_sort_products_by_availability,
                                cx,
                            ))
                            .child(choice_button(
                                "products-sort-stock",
                                app_shared_text(AppTextKey::ProductsSortStock),
                                selected_sort == ProductsSort::Stock,
                                on_sort_products_by_stock,
                                cx,
                            ))
                            .child(choice_button(
                                "products-sort-price",
                                app_shared_text(AppTextKey::ProductsSortPrice),
                                selected_sort == ProductsSort::Price,
                                on_sort_products_by_price,
                                cx,
                            )),
                    ),
            ),
    )
}

fn products_table_header() -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnProduct,
            None,
            true,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnStatus,
            Some(112.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnAvailability,
            Some(192.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnStock,
            Some(128.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnPrice,
            Some(128.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnUpdated,
            Some(164.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::ProductsColumnAction,
            Some(120.0),
            false,
        ))
}

fn products_table_header_column(
    key: AppTextKey,
    width_px: Option<f32>,
    grows: bool,
) -> impl IntoElement {
    div()
        .when_some(width_px, |this, width_px| this.w(px(width_px)))
        .when(grows, |this| this.flex_1().min_w_0())
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
        .child(app_shared_text(key))
}

fn products_table_row(
    product: AnyElement,
    row: &ProductsListRow,
    action: AnyElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(product)
        .child(
            div()
                .w(px(112.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status_indicator(products_row_status_color(row)))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(app_shared_text(products_status_key(row.status))),
                ),
        )
        .child(
            div()
                .w(px(192.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(row.availability.label.clone()),
        )
        .child(
            div()
                .w(px(128.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(products_stock_text(row)),
        )
        .child(
            div()
                .w(px(128.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(products_price_text(row)),
        )
        .child(
            div()
                .w(px(164.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(row.updated_at.clone()),
        )
        .child(div().w(px(120.0)).flex().justify_end().child(action))
}

fn orders_table_header() -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnOrder,
            None,
            true,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnStatus,
            Some(128.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnWindow,
            Some(196.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnPickup,
            Some(196.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnAction,
            Some(132.0),
            false,
        ))
}

fn orders_table_row(
    order: AnyElement,
    row: &OrdersListRow,
    action: AnyElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(order)
        .child(
            div()
                .w(px(128.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status_indicator(orders_status_color(row.status)))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(app_shared_text(orders_status_key(row.status))),
                ),
        )
        .child(
            div()
                .w(px(196.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(order_optional_text(row.fulfillment_window_label.as_deref())),
        )
        .child(
            div()
                .w(px(196.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(order_optional_text(row.pickup_location_label.as_deref())),
        )
        .child(div().w(px(132.0)).flex().justify_end().child(action))
}

fn orders_table_action(
    index: usize,
    row: &OrdersListRow,
    on_review: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_mark_packed: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_mark_completed: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    match row.primary_action {
        Some(OrderPrimaryAction::Review) => action_button_compact(
            ("orders-row-action-review", index),
            app_shared_text(AppTextKey::OrdersActionReview),
            on_review,
            cx,
        )
        .into_any_element(),
        Some(OrderPrimaryAction::MarkPacked) => action_button_compact(
            ("orders-row-action-mark-packed", index),
            app_shared_text(AppTextKey::OrdersActionMarkPacked),
            on_mark_packed,
            cx,
        )
        .into_any_element(),
        Some(OrderPrimaryAction::MarkCompleted) => action_button_compact(
            ("orders-row-action-mark-completed", index),
            app_shared_text(AppTextKey::OrdersActionMarkCompleted),
            on_mark_completed,
            cx,
        )
        .into_any_element(),
        None => div()
            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
            .child(app_shared_text(AppTextKey::ValueNone))
            .into_any_element(),
    }
}

fn orders_empty_state_card(filter: OrdersFilter) -> impl IntoElement {
    let (title_key, body_key) = if filter == OrdersFilter::NeedsAction {
        (
            AppTextKey::OrdersEmptyNeedsActionTitle,
            AppTextKey::OrdersEmptyNeedsActionBody,
        )
    } else {
        (AppTextKey::OrdersEmptyTitle, AppTextKey::OrdersEmptyBody)
    };

    home_empty_state_card(title_key, body_key)
}

fn orders_status_key(status: OrderStatus) -> AppTextKey {
    match status {
        OrderStatus::NeedsAction => AppTextKey::OrdersStatusNeedsAction,
        OrderStatus::Scheduled => AppTextKey::OrdersStatusScheduled,
        OrderStatus::Packed => AppTextKey::OrdersStatusPacked,
        OrderStatus::Completed => AppTextKey::OrdersStatusCompleted,
        OrderStatus::Refunded => AppTextKey::OrdersStatusRefunded,
    }
}

fn orders_status_color(status: OrderStatus) -> u32 {
    match status {
        OrderStatus::NeedsAction => APP_UI_THEME.components.app_status_indicator.attention,
        OrderStatus::Scheduled | OrderStatus::Packed => {
            APP_UI_THEME.components.app_status_indicator.online
        }
        OrderStatus::Completed | OrderStatus::Refunded => {
            APP_UI_THEME.components.app_status_indicator.offline
        }
    }
}

fn pack_day_title_row(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    app_stack_v(4.0)
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(app_shared_text(AppTextKey::PackDayTitle)),
        )
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .when_some(home_saved_farm(runtime), |this, farm| {
                    this.child(farm.display_name.clone())
                }),
        )
}

fn pack_day_window_summary_card(fulfillment_window: &FulfillmentWindowSummary) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::PackDayWindowSummaryTitle),
        label_value_list([
            LabelValueRow::new(
                app_shared_text(AppTextKey::HomeTodayWindowStartsLabel),
                fulfillment_window.starts_at.clone(),
            ),
            LabelValueRow::new(
                app_shared_text(AppTextKey::HomeTodayWindowEndsLabel),
                fulfillment_window.ends_at.clone(),
            ),
        ]),
    )
}

fn pack_day_totals_card(rows: &[PackDayProductTotalRow]) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::PackDayTotalsTitle),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
            .children(
                rows.iter()
                    .map(pack_day_product_total_row)
                    .collect::<Vec<_>>(),
            ),
    )
}

fn pack_day_pack_list_card(rows: &[PackDayPackListRow]) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::PackDayPackListTitle),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
            .children(rows.iter().map(pack_day_pack_list_row).collect::<Vec<_>>()),
    )
}

fn pack_day_pickup_roster_card(rows: &[PackDayRosterRow]) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::PackDayPickupRosterTitle),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
            .children(rows.iter().map(pack_day_roster_row).collect::<Vec<_>>()),
    )
}

fn pack_day_product_total_row(row: &PackDayProductTotalRow) -> AnyElement {
    pack_day_label_value_row(row.title.as_str(), row.quantity_display.as_str())
}

fn pack_day_pack_list_row(row: &PackDayPackListRow) -> AnyElement {
    pack_day_label_value_row(row.title.as_str(), row.quantity_display.as_str())
}

fn pack_day_roster_row(row: &PackDayRosterRow) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(row.order_number.clone()),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                        .child(row.customer_display_name.clone()),
                ),
        )
        .into_any_element()
}

fn pack_day_label_value_row(label: &str, value: &str) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(value.to_owned()),
        )
        .into_any_element()
}

fn products_empty_state_card(filter: ProductsFilter) -> impl IntoElement {
    let (title_key, body_key) = if filter == ProductsFilter::NeedAttention {
        (
            AppTextKey::ProductsEmptyNeedAttentionTitle,
            AppTextKey::ProductsEmptyNeedAttentionBody,
        )
    } else {
        (
            AppTextKey::ProductsEmptyTitle,
            AppTextKey::ProductsEmptyBody,
        )
    };

    home_empty_state_card(title_key, body_key)
}

fn products_status_key(status: ProductStatus) -> AppTextKey {
    match status {
        ProductStatus::Draft => AppTextKey::ProductsStatusDraft,
        ProductStatus::Published => AppTextKey::ProductsStatusLive,
        ProductStatus::Paused => AppTextKey::ProductsStatusPaused,
        ProductStatus::Archived => AppTextKey::ProductsStatusArchived,
    }
}

fn products_row_status_color(row: &ProductsListRow) -> u32 {
    if row.attention_state != ProductAttentionState::Healthy {
        APP_UI_THEME.components.app_status_indicator.attention
    } else {
        match row.status {
            ProductStatus::Published => APP_UI_THEME.components.app_status_indicator.online,
            ProductStatus::Draft | ProductStatus::Paused | ProductStatus::Archived => {
                APP_UI_THEME.components.app_status_indicator.offline
            }
        }
    }
}

fn products_stock_text(row: &ProductsListRow) -> String {
    match row.stock.quantity {
        Some(quantity) => match row.stock.unit_label.as_ref() {
            Some(unit_label) => format!("{quantity} {unit_label}"),
            None => quantity.to_string(),
        },
        None => app_shared_text(AppTextKey::ValueNone).to_string(),
    }
}

fn products_price_text(row: &ProductsListRow) -> String {
    let Some(price) = row.price.as_ref() else {
        return app_shared_text(AppTextKey::ValueNone).to_string();
    };
    let dollars = price.amount_minor_units / 100;
    let cents = price.amount_minor_units % 100;

    format!("${dollars}.{cents:02} / {}", price.unit_label)
}

fn products_stock_editor_card(
    row: &ProductsListRow,
    editor: &ProductsStockEditorState,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let validation_key = products_stock_editor_validation_key(editor, cx);
    let save_ready = editor.has_changes(cx) && editor.parsed_stock_quantity(cx).is_some();

    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.window_background))
        .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(app_shared_text(AppTextKey::ProductsStockEditorTitle)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                        .child(product_display_title(row.title.as_str())),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_end()
                .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .utility_title_text_px))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(app_shared_text(AppTextKey::ProductsStockEditorFieldLabel)),
                        )
                        .child(app_text_input(&editor.input, false).w_full())
                        .when_some(validation_key, |this, key| {
                            this.child(
                                div()
                                    .text_size(px(APP_UI_THEME
                                        .foundation
                                        .typography
                                        .utility_title_text_px))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                    .child(app_shared_text(key)),
                            )
                        })
                        .when(editor.save_failed, |this| {
                            this.child(
                                div()
                                    .text_size(px(APP_UI_THEME
                                        .foundation
                                        .typography
                                        .utility_title_text_px))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                    .child(app_shared_text(
                                        AppTextKey::ProductsStockEditorSaveFailed,
                                    )),
                            )
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(action_button_compact(
                            "products-stock-editor-close",
                            app_shared_text(AppTextKey::ProductsStockEditorCancelAction),
                            on_cancel,
                            cx,
                        ))
                        .child(if save_ready {
                            action_button_primary(
                                "products-stock-editor-save",
                                app_shared_text(AppTextKey::ProductsStockEditorSaveAction),
                                on_save,
                                cx,
                            )
                            .into_any_element()
                        } else {
                            action_button_primary_disabled(
                                "products-stock-editor-save",
                                app_shared_text(AppTextKey::ProductsStockEditorSaveAction),
                                cx,
                            )
                            .into_any_element()
                        }),
                ),
        )
}

fn products_stock_editor_validation_key(
    editor: &ProductsStockEditorState,
    cx: &App,
) -> Option<AppTextKey> {
    if editor.parsed_stock_quantity(cx).is_some() {
        return None;
    }

    Some(AppTextKey::ProductsStockEditorInvalidQuantity)
}

fn products_editor_surface(
    form: &ProductEditorFormState,
    runtime: &DesktopAppRuntimeSummary,
    on_select_draft: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_live: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_paused: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_archived: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let validation_keys = products_editor_validation_keys(form, cx);
    let save_ready = form.has_changes(cx) && validation_keys.is_empty();

    div().w_full().flex().justify_center().child(
        div().w_full().max_w(px(520.0)).child(home_card(
            app_shared_text(AppTextKey::ProductsEditorTitle),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                .child(home_body_text(app_shared_text(
                    AppTextKey::ProductsEditorBody,
                )))
                .child(app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::ProductsEditorFieldTitle),
                        Option::<SharedString>::None,
                    ),
                    &form.title_input,
                    false,
                ))
                .child(app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::ProductsEditorFieldSubtitle),
                        Option::<SharedString>::None,
                    ),
                    &form.subtitle_input,
                    false,
                ))
                .child(app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::ProductsEditorFieldUnit),
                        Option::<SharedString>::None,
                    ),
                    &form.unit_input,
                    false,
                ))
                .child(app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::ProductsEditorFieldPrice),
                        products_editor_invalid_price_key(form, cx).map(app_shared_text),
                    ),
                    &form.price_input,
                    false,
                ))
                .child(app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::ProductsEditorFieldStock),
                        products_editor_invalid_stock_key(form, cx).map(app_shared_text),
                    ),
                    &form.stock_input,
                    false,
                ))
                .child(products_editor_status_section(
                    form.status,
                    on_select_draft,
                    on_select_live,
                    on_select_paused,
                    on_select_archived,
                    cx,
                ))
                .child(products_editor_publish_readiness_section(form, runtime, cx))
                .when(form.save_failed, |this| {
                    this.child(home_body_text(app_shared_text(
                        AppTextKey::ProductsEditorSaveFailed,
                    )))
                })
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(product_display_title(
                                    form.title_input.read(cx).value().as_ref(),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(action_button_compact(
                                    "products-editor-close",
                                    app_shared_text(AppTextKey::ProductsEditorCloseAction),
                                    on_close,
                                    cx,
                                ))
                                .child(if save_ready {
                                    action_button_primary(
                                        "products-editor-save",
                                        app_shared_text(AppTextKey::ProductsEditorSaveAction),
                                        on_save,
                                        cx,
                                    )
                                    .into_any_element()
                                } else {
                                    action_button_primary_disabled(
                                        "products-editor-save",
                                        app_shared_text(AppTextKey::ProductsEditorSaveAction),
                                        cx,
                                    )
                                    .into_any_element()
                                }),
                        ),
                ),
        )),
    )
}

fn products_editor_status_section(
    selected_status: ProductStatus,
    on_select_draft: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_live: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_paused: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_archived: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_field_label(app_shared_text(
            AppTextKey::ProductsEditorFieldStatus,
        )))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(choice_button(
                    "products-editor-status-draft",
                    app_shared_text(AppTextKey::ProductsStatusDraft),
                    selected_status == ProductStatus::Draft,
                    on_select_draft,
                    cx,
                ))
                .child(choice_button(
                    "products-editor-status-live",
                    app_shared_text(AppTextKey::ProductsStatusLive),
                    selected_status == ProductStatus::Published,
                    on_select_live,
                    cx,
                ))
                .child(choice_button(
                    "products-editor-status-paused",
                    app_shared_text(AppTextKey::ProductsStatusPaused),
                    selected_status == ProductStatus::Paused,
                    on_select_paused,
                    cx,
                ))
                .child(choice_button(
                    "products-editor-status-archived",
                    app_shared_text(AppTextKey::ProductsStatusArchived),
                    selected_status == ProductStatus::Archived,
                    on_select_archived,
                    cx,
                )),
        )
}

fn products_editor_publish_readiness_section(
    form: &ProductEditorFormState,
    runtime: &DesktopAppRuntimeSummary,
    cx: &App,
) -> impl IntoElement {
    let blockers = form
        .current_draft(cx)
        .map(|draft| derive_product_publish_blockers(&draft, &runtime.farm_readiness_projection))
        .unwrap_or_default();

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_field_label(app_shared_text(
            AppTextKey::ProductsEditorPublishReadinessTitle,
        )))
        .child(if blockers.is_empty() {
            home_body_text(app_shared_text(AppTextKey::ProductsEditorReady)).into_any_element()
        } else {
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(8.0))
                .children(
                    blockers
                        .into_iter()
                        .map(products_editor_publish_blocker_row)
                        .collect::<Vec<_>>(),
                )
                .into_any_element()
        })
}

fn products_editor_publish_blocker_row(blocker: ProductPublishBlocker) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_start()
        .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
        .child(status_indicator(
            APP_UI_THEME.components.app_status_indicator.attention,
        ))
        .child(home_body_text(app_shared_text(
            products_editor_publish_blocker_key(blocker),
        )))
        .into_any_element()
}

fn products_editor_publish_blocker_key(blocker: ProductPublishBlocker) -> AppTextKey {
    match blocker {
        ProductPublishBlocker::AddProductName => AppTextKey::ProductsEditorBlockerAddProductName,
        ProductPublishBlocker::ChooseUnit => AppTextKey::ProductsEditorBlockerChooseUnit,
        ProductPublishBlocker::SetPrice => AppTextKey::ProductsEditorBlockerSetPrice,
        ProductPublishBlocker::AttachAvailability => {
            AppTextKey::ProductsEditorBlockerAttachAvailability
        }
        ProductPublishBlocker::CompleteFarmProfile => {
            AppTextKey::ProductsEditorBlockerCompleteFarmProfile
        }
        ProductPublishBlocker::AddPickupLocation => {
            AppTextKey::ProductsEditorBlockerAddPickupLocation
        }
        ProductPublishBlocker::AddOperatingRules => {
            AppTextKey::ProductsEditorBlockerAddOperatingRules
        }
        ProductPublishBlocker::AddFulfillmentWindow => {
            AppTextKey::ProductsEditorBlockerAddFulfillmentWindow
        }
        ProductPublishBlocker::ResolveAvailabilityConflicts => {
            AppTextKey::ProductsEditorBlockerResolveAvailabilityConflicts
        }
    }
}

fn products_editor_validation_keys(form: &ProductEditorFormState, cx: &App) -> Vec<AppTextKey> {
    let mut keys = Vec::new();

    if let Some(key) = products_editor_invalid_price_key(form, cx) {
        keys.push(key);
    }

    if let Some(key) = products_editor_invalid_stock_key(form, cx) {
        keys.push(key);
    }

    keys
}

fn products_editor_invalid_price_key(
    form: &ProductEditorFormState,
    cx: &App,
) -> Option<AppTextKey> {
    parse_product_editor_price_input(form.price_input.read(cx).value().as_ref())
        .is_none()
        .then_some(AppTextKey::ProductsEditorInvalidPrice)
}

fn products_editor_invalid_stock_key(
    form: &ProductEditorFormState,
    cx: &App,
) -> Option<AppTextKey> {
    parse_optional_product_editor_stock_input(form.stock_input.read(cx).value().as_ref())
        .is_none()
        .then_some(AppTextKey::ProductsEditorInvalidStock)
}

fn parse_products_stock_quantity(input: &str) -> Option<u32> {
    input.trim().parse().ok()
}

fn parse_product_editor_price_input(input: &str) -> Option<Option<u32>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(None);
    }

    let parse_whole_dollars = |value: &str| -> Option<u32> { value.parse::<u32>().ok() };

    if let Some((dollars, cents)) = trimmed.split_once('.') {
        if trimmed.matches('.').count() != 1 || cents.is_empty() || cents.len() > 2 {
            return None;
        }

        let dollars = if dollars.is_empty() {
            0
        } else {
            parse_whole_dollars(dollars)?
        };
        let cents = match cents.len() {
            1 => cents.parse::<u32>().ok()?.checked_mul(10)?,
            2 => cents.parse::<u32>().ok()?,
            _ => return None,
        };

        return dollars
            .checked_mul(100)
            .and_then(|amount| amount.checked_add(cents))
            .map(Some);
    }

    parse_whole_dollars(trimmed)
        .and_then(|dollars| dollars.checked_mul(100))
        .map(Some)
}

fn parse_optional_product_editor_stock_input(input: &str) -> Option<Option<u32>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Some(None);
    }

    trimmed.parse::<u32>().ok().map(Some)
}

fn product_editor_price_input_value(price_minor_units: Option<u32>) -> String {
    price_minor_units
        .map(|amount_minor_units| {
            format!(
                "{}.{:02}",
                amount_minor_units / 100,
                amount_minor_units % 100
            )
        })
        .unwrap_or_default()
}

fn product_display_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        app_shared_text(AppTextKey::ProductsUntitledDraft).to_string()
    } else {
        trimmed.to_owned()
    }
}

fn home_farm_setup_onboarding_card(
    spec: FarmSetupOnboardingCardSpec,
    on_open_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    home_card(
        app_shared_text(spec.title_key),
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(home_body_text(app_shared_text(spec.body_key)))
            .when_some(spec.action_key, |this, action_key| {
                this.child(div().child(action_button_primary(
                    "home-farm-setup-start",
                    app_shared_text(action_key),
                    on_open_farm_setup,
                    cx,
                )))
            }),
    )
}

fn home_farm_setup_form_card(
    form: &FarmSetupFormState,
    on_pickup_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_delivery_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_shipping_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_finish_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let blockers = form.draft.blockers();
    let finish_ready = blockers.is_empty();

    home_card(
        app_shared_text(AppTextKey::HomeFarmSetupOnboardingTitle),
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(home_body_text(app_shared_text(
                AppTextKey::HomeFarmSetupOnboardingBody,
            )))
            .child(app_form_section(
                app_shared_text(AppTextKey::HomeFarmSetupSectionFarm),
                app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::HomeFarmSetupFieldFarmName),
                        blockers
                            .contains(&FarmSetupBlocker::AddFarmName)
                            .then_some(AppTextKey::HomeFarmSetupBlockerAddFarmName)
                            .map(app_shared_text),
                    ),
                    &form.farm_name_input,
                    false,
                ),
            ))
            .child(app_form_section(
                app_shared_text(AppTextKey::HomeFarmSetupSectionLocation),
                app_form_input_text(
                    AppFormFieldSpec::new(
                        app_shared_text(AppTextKey::HomeFarmSetupFieldLocationOrServiceArea),
                        blockers
                            .contains(&FarmSetupBlocker::AddLocationOrServiceArea)
                            .then_some(AppTextKey::HomeFarmSetupBlockerAddLocationOrServiceArea)
                            .map(app_shared_text),
                    ),
                    &form.location_input,
                    false,
                ),
            ))
            .child(home_farm_setup_order_method_section(
                form,
                blockers
                    .contains(&FarmSetupBlocker::ChooseOrderMethod)
                    .then_some(AppTextKey::HomeFarmSetupBlockerChooseOrderMethod),
                on_pickup_change,
                on_delivery_change,
                on_shipping_change,
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(home_body_text(app_shared_text(farm_setup_save_state_key(
                        form.save_state,
                    ))))
                    .child(div().child(if finish_ready {
                        action_button_primary(
                            "home-farm-setup-finish",
                            app_shared_text(AppTextKey::HomeFarmSetupFinishAction),
                            on_finish_setup,
                            cx,
                        )
                        .into_any_element()
                    } else {
                        action_button_primary_disabled(
                            "home-farm-setup-finish",
                            app_shared_text(AppTextKey::HomeFarmSetupFinishAction),
                            cx,
                        )
                        .into_any_element()
                    })),
            ),
    )
}

fn home_farm_setup_order_method_section(
    form: &FarmSetupFormState,
    blocker_key: Option<AppTextKey>,
    on_pickup_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_delivery_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_shipping_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_form_section(
        app_shared_text(AppTextKey::HomeFarmSetupSectionOrderMethods),
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.0))
            .child(app_checkbox_field(
                AppCheckboxFieldSpec::new(
                    "home-farm-setup-pickup",
                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodPickup),
                    Option::<SharedString>::None,
                ),
                form.draft.order_methods.contains(&FarmOrderMethod::Pickup),
                cx,
                move |checked, window, cx| on_pickup_change(&checked, window, cx),
            ))
            .child(app_checkbox_field(
                AppCheckboxFieldSpec::new(
                    "home-farm-setup-delivery",
                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodDelivery),
                    Option::<SharedString>::None,
                ),
                form.draft
                    .order_methods
                    .contains(&FarmOrderMethod::Delivery),
                cx,
                move |checked, window, cx| on_delivery_change(&checked, window, cx),
            ))
            .child(app_checkbox_field(
                AppCheckboxFieldSpec::new(
                    "home-farm-setup-shipping",
                    app_shared_text(AppTextKey::HomeFarmSetupOrderMethodShipping),
                    Option::<SharedString>::None,
                ),
                form.draft
                    .order_methods
                    .contains(&FarmOrderMethod::Shipping),
                cx,
                move |checked, window, cx| on_shipping_change(&checked, window, cx),
            ))
            .when_some(blocker_key, |this, blocker_key| {
                this.child(home_body_text(app_shared_text(blocker_key)))
            }),
    )
}

fn settings_panel_farm_context(runtime: &DesktopAppRuntimeSummary) -> Option<(String, FarmId)> {
    let account_id = runtime
        .settings_account_projection
        .selected_account
        .as_ref()?
        .account
        .account_id
        .clone();
    let farm_id = runtime
        .settings_account_projection
        .selected_account
        .as_ref()
        .and_then(|account| account.farmer_activation.farm_id)
        .or(runtime
            .farm_setup_projection
            .saved_farm
            .as_ref()
            .map(|farm| farm.farm_id))?;

    Some((account_id, farm_id))
}

fn settings_pickup_location_title(
    index: usize,
    pickup_location: &SettingsPickupLocationFormState,
    cx: &App,
) -> String {
    let label = pickup_location
        .label_input
        .read(cx)
        .value()
        .trim()
        .to_owned();
    if label.is_empty() {
        format!(
            "{} {}",
            app_shared_text(AppTextKey::SettingsPickupLocationsSectionLabel),
            index + 1
        )
    } else {
        label
    }
}

fn settings_pickup_location_card(
    index: usize,
    pickup_location: &SettingsPickupLocationFormState,
    on_make_default: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_remove: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let title = settings_pickup_location_title(index, pickup_location, cx);
    let action_row = div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(if pickup_location.is_default {
            settings_badge_text(app_shared_text(
                AppTextKey::SettingsPickupLocationsDefaultBadge,
            ))
            .into_any_element()
        } else {
            action_button_compact(
                ("settings-farm-default-pickup", index),
                app_shared_text(AppTextKey::SettingsPickupLocationsMakeDefaultAction),
                on_make_default,
                cx,
            )
            .into_any_element()
        })
        .when(pickup_location.can_remove, |this| {
            this.child(
                action_button_compact(
                    ("settings-farm-remove-pickup", index),
                    app_shared_text(AppTextKey::SettingsPickupLocationsRemoveAction),
                    on_remove,
                    cx,
                )
                .into_any_element(),
            )
        });

    app_surface_panel(
        app_stack_v(10.0)
            .w_full()
            .p(px(12.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(title),
                    )
                    .child(action_row),
            )
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsPickupLocationsFieldLabel),
                    Option::<SharedString>::None,
                ),
                &pickup_location.label_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsPickupLocationsFieldAddress),
                    Option::<SharedString>::None,
                ),
                &pickup_location.address_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsPickupLocationsFieldDirections),
                    Option::<SharedString>::None,
                ),
                &pickup_location.directions_input,
                false,
            )),
    )
}

fn settings_fulfillment_window_title(
    index: usize,
    fulfillment_window: &SettingsFulfillmentWindowFormState,
    cx: &App,
) -> String {
    let label = fulfillment_window
        .label_input
        .read(cx)
        .value()
        .trim()
        .to_owned();
    if label.is_empty() {
        format!(
            "{} {}",
            app_shared_text(AppTextKey::SettingsFulfillmentWindowsItemLabel),
            index + 1
        )
    } else {
        label
    }
}

fn settings_blackout_period_title(
    index: usize,
    blackout_period: &SettingsBlackoutPeriodFormState,
    cx: &App,
) -> String {
    let label = blackout_period
        .label_input
        .read(cx)
        .value()
        .trim()
        .to_owned();
    if label.is_empty() {
        format!(
            "{} {}",
            app_shared_text(AppTextKey::SettingsBlackoutPeriodsItemLabel),
            index + 1
        )
    } else {
        label
    }
}

fn settings_fulfillment_window_card(
    index: usize,
    fulfillment_window: &SettingsFulfillmentWindowFormState,
    pickup_location_options: Vec<AnyElement>,
    validation_keys: &[AppTextKey],
    on_remove: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_surface_panel(
        app_stack_v(10.0)
            .w_full()
            .p(px(12.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(settings_fulfillment_window_title(
                                index,
                                fulfillment_window,
                                cx,
                            )),
                    )
                    .child(
                        action_button_compact(
                            ("settings-remove-fulfillment-window", index),
                            app_shared_text(AppTextKey::SettingsFulfillmentWindowsRemoveAction),
                            on_remove,
                            cx,
                        )
                        .into_any_element(),
                    ),
            )
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsFieldLabel),
                    Option::<SharedString>::None,
                ),
                &fulfillment_window.label_input,
                false,
            ))
            .child(app_form_field(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsFieldPickupLocation),
                    Option::<SharedString>::None,
                ),
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .children(pickup_location_options),
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsFieldStartsAt),
                    Option::<SharedString>::None,
                ),
                &fulfillment_window.starts_at_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsFieldEndsAt),
                    Option::<SharedString>::None,
                ),
                &fulfillment_window.ends_at_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsFulfillmentWindowsFieldOrderCutoff),
                    Option::<SharedString>::None,
                ),
                &fulfillment_window.order_cutoff_input,
                false,
            ))
            .children(
                validation_keys
                    .iter()
                    .copied()
                    .map(|key| home_body_text(app_shared_text(key)).into_any_element())
                    .collect::<Vec<_>>(),
            ),
    )
}

fn settings_blackout_period_card(
    index: usize,
    blackout_period: &SettingsBlackoutPeriodFormState,
    validation_keys: &[AppTextKey],
    on_remove: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_surface_panel(
        app_stack_v(10.0)
            .w_full()
            .p(px(12.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(settings_blackout_period_title(index, blackout_period, cx)),
                    )
                    .child(
                        action_button_compact(
                            ("settings-remove-blackout-period", index),
                            app_shared_text(AppTextKey::SettingsBlackoutPeriodsRemoveAction),
                            on_remove,
                            cx,
                        )
                        .into_any_element(),
                    ),
            )
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsBlackoutPeriodsFieldLabel),
                    Option::<SharedString>::None,
                ),
                &blackout_period.label_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsBlackoutPeriodsFieldStartsAt),
                    Option::<SharedString>::None,
                ),
                &blackout_period.starts_at_input,
                false,
            ))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::SettingsBlackoutPeriodsFieldEndsAt),
                    Option::<SharedString>::None,
                ),
                &blackout_period.ends_at_input,
                false,
            ))
            .children(
                validation_keys
                    .iter()
                    .copied()
                    .map(|key| home_body_text(app_shared_text(key)).into_any_element())
                    .collect::<Vec<_>>(),
            ),
    )
}

fn settings_farm_readiness_rows(evaluation: &SettingsFarmRulesEvaluation) -> Vec<AnyElement> {
    let readiness_keys = if evaluation.readiness_keys.is_empty() {
        vec![AppTextKey::SettingsReadinessReady]
    } else {
        evaluation.readiness_keys.clone()
    };

    readiness_keys
        .into_iter()
        .map(|key| {
            app_surface_panel(
                div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .child(home_farm_setup_field_label(app_shared_text(key))),
            )
            .into_any_element()
        })
        .collect()
}

fn settings_readiness_key(blocker: FarmReadinessBlocker) -> AppTextKey {
    match blocker {
        FarmReadinessBlocker::MissingProfileBasics => {
            AppTextKey::SettingsReadinessFieldMissingProfileBasics
        }
        FarmReadinessBlocker::MissingPickupLocation => {
            AppTextKey::SettingsReadinessFieldMissingPickupLocation
        }
        FarmReadinessBlocker::MissingFulfillmentWindow => {
            AppTextKey::SettingsReadinessFieldMissingFulfillmentWindow
        }
        FarmReadinessBlocker::MissingOperatingRules => {
            AppTextKey::SettingsReadinessFieldMissingOperatingRules
        }
    }
}

fn settings_timing_conflict_key(kind: FarmTimingConflictKind) -> AppTextKey {
    match kind {
        FarmTimingConflictKind::FulfillmentWindowEndsBeforeStart => {
            AppTextKey::SettingsReadinessFieldFulfillmentWindowEndsBeforeStart
        }
        FarmTimingConflictKind::FulfillmentWindowCutoffAfterStart => {
            AppTextKey::SettingsReadinessFieldFulfillmentWindowCutoffAfterStart
        }
        FarmTimingConflictKind::BlackoutPeriodEndsBeforeStart => {
            AppTextKey::SettingsReadinessFieldBlackoutPeriodEndsBeforeStart
        }
        FarmTimingConflictKind::BlackoutOverlapsFulfillmentWindow => {
            AppTextKey::SettingsReadinessFieldBlackoutOverlapsFulfillmentWindow
        }
    }
}

#[cfg(test)]
fn settings_inventory_card(spec: SettingsInventorySectionSpec) -> impl IntoElement {
    home_card(
        app_shared_text(spec.title_key),
        app_stack_v(8.0).w_full().children(
            spec.field_keys
                .iter()
                .copied()
                .map(|key| {
                    app_surface_panel(
                        div()
                            .px(px(12.0))
                            .py(px(10.0))
                            .child(home_farm_setup_field_label(app_shared_text(key))),
                    )
                    .into_any_element()
                })
                .collect::<Vec<_>>(),
        ),
    )
}

fn home_saved_farm_summary_card(runtime: &DesktopAppRuntimeSummary) -> Option<AnyElement> {
    let saved_farm = home_saved_farm(runtime)?;
    let location_or_service_area = if runtime
        .farm_setup_projection
        .draft
        .location_or_service_area
        .trim()
        .is_empty()
    {
        app_shared_text(AppTextKey::ValueNone).to_string()
    } else {
        runtime
            .farm_setup_projection
            .draft
            .location_or_service_area
            .clone()
    };

    Some(
        home_card(
            saved_farm.display_name.clone(),
            label_value_list(vec![
                LabelValueRow::new(
                    app_shared_text(AppTextKey::HomeFarmSetupFieldLocationOrServiceArea),
                    location_or_service_area,
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::HomeFarmSetupSectionOrderMethods),
                    home_farm_order_methods_summary(&runtime.farm_setup_projection.draft),
                ),
            ]),
        )
        .into_any_element(),
    )
}

fn home_status_row(status: &HomeStatusPresentation) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
        .child(status_indicator(status.indicator_color))
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(app_shared_text(status.label_key)),
        )
}

fn home_summary_card(summary: &radroots_studio_app_models::TodaySummary) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::HomeTodayTitle),
        div()
            .w_full()
            .flex()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(home_summary_metric(
                AppTextKey::HomeTodayOrdersNeedingAction,
                summary.orders_needing_action,
            ))
            .child(home_summary_metric(
                AppTextKey::HomeTodayLowStock,
                summary.low_stock_products,
            ))
            .child(home_summary_metric(
                AppTextKey::HomeTodayDraftProducts,
                summary.draft_products,
            )),
    )
}

fn home_summary_metric(label_key: AppTextKey, value: u32) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.window_background))
        .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(value.to_string()),
        )
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(app_shared_text(label_key)),
        )
}

fn home_setup_card(
    projection: &TodayAgendaProjection,
    continue_action: Option<AnyElement>,
) -> impl IntoElement {
    home_list_card(
        AppTextKey::HomeTodaySetupChecklist,
        projection
            .setup_checklist
            .iter()
            .map(home_setup_task_row)
            .collect::<Vec<_>>(),
        continue_action,
    )
}

fn home_next_fulfillment_window_card(
    next_window: &FulfillmentWindowSummary,
    action: Option<AnyElement>,
) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::HomeTodayNextFulfillmentWindow),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .child(label_value_list(vec![
                LabelValueRow::new(
                    app_shared_text(AppTextKey::HomeTodayWindowStartsLabel),
                    next_window.starts_at.clone(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::HomeTodayWindowEndsLabel),
                    next_window.ends_at.clone(),
                ),
            ]))
            .when_some(action, |this, action| this.child(div().child(action))),
    )
}

fn home_list_card(
    title_key: AppTextKey,
    rows: Vec<AnyElement>,
    action: Option<AnyElement>,
) -> impl IntoElement {
    home_card(
        app_shared_text(title_key),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
            .children(rows)
            .when_some(action, |this, action| this.child(div().child(action))),
    )
}

fn order_detail_item_row(item: &OrderDetailItemRow) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(item.title.clone()),
        )
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(item.quantity_display.clone()),
        )
        .into_any_element()
}

fn order_optional_text(value: Option<&str>) -> SharedString {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| SharedString::from(value.to_owned()))
        .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone))
}

fn home_order_row(
    index: usize,
    order: &OrderListRow,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(list_row_button(
            ("home-today-order-open", index),
            order.order_number.clone(),
            Some(SharedString::from(order.customer_display_name.clone())),
            false,
            on_open,
            cx,
        ))
        .child(status_indicator(
            APP_UI_THEME.components.app_status_indicator.attention,
        ))
        .into_any_element()
}

fn home_low_stock_row(product: &ProductListRow) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(product_display_title(product.title.as_str())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
                .child(status_indicator(
                    APP_UI_THEME.components.app_status_indicator.attention,
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(app_shared_label_text(AppTextKey::HomeTodayStockCountLabel)),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                .child(product.stock_count.to_string()),
                        ),
                ),
        )
        .into_any_element()
}

fn home_draft_row(product: &ProductListRow) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(product_display_title(product.title.as_str())),
        )
        .child(status_indicator(
            APP_UI_THEME.components.app_status_indicator.offline,
        ))
        .into_any_element()
}

fn home_setup_task_row(task: &radroots_studio_app_models::TodaySetupTask) -> AnyElement {
    let is_complete = task.is_complete;

    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
        .child(status_indicator(if is_complete {
            APP_UI_THEME.components.app_status_indicator.online
        } else {
            APP_UI_THEME.components.app_status_indicator.offline
        }))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(if is_complete {
                    APP_UI_THEME.foundation.text.secondary
                } else {
                    APP_UI_THEME.foundation.text.primary
                }))
                .child(app_shared_text(home_setup_task_label_key(task.kind))),
        )
        .into_any_element()
}

fn home_empty_state_card(title_key: AppTextKey, body_key: AppTextKey) -> impl IntoElement {
    home_card(
        app_shared_text(title_key),
        home_body_text(app_shared_text(body_key)),
    )
}

fn farm_setup_onboarding_card_spec(home_route: HomeRoute) -> Option<FarmSetupOnboardingCardSpec> {
    match home_route {
        HomeRoute::FarmSetupOnboarding => Some(FarmSetupOnboardingCardSpec {
            title_key: AppTextKey::HomeFarmSetupOnboardingTitle,
            body_key: AppTextKey::HomeFarmSetupOnboardingBody,
            action_key: Some(AppTextKey::HomeFarmSetupOnboardingAction),
        }),
        HomeRoute::FarmSetupForm => Some(FarmSetupOnboardingCardSpec {
            title_key: AppTextKey::HomeFarmSetupOnboardingTitle,
            body_key: AppTextKey::HomeFarmSetupOnboardingBody,
            action_key: None,
        }),
        _ => None,
    }
}

fn farm_setup_save_state_key(state: FarmSetupSaveState) -> AppTextKey {
    match state {
        FarmSetupSaveState::AutosavesLocally => AppTextKey::HomeFarmSetupSaveAutosavesLocally,
        FarmSetupSaveState::SavedLocally => AppTextKey::HomeFarmSetupSaveSavedLocally,
        FarmSetupSaveState::SaveFailed => AppTextKey::HomeFarmSetupSaveFailedLocally,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FarmerHomeFarmState {
    NoFarm,
    IncompleteFarm,
    ConfiguredFarm,
}

fn home_saved_farm(runtime: &DesktopAppRuntimeSummary) -> Option<&FarmSummary> {
    runtime
        .today_projection
        .farm
        .as_ref()
        .or(runtime.farm_setup_projection.saved_farm.as_ref())
}

fn farmer_home_farm_state(runtime: &DesktopAppRuntimeSummary) -> FarmerHomeFarmState {
    match runtime.farm_readiness_projection.status {
        FarmWorkspaceStatus::NoFarm => FarmerHomeFarmState::NoFarm,
        FarmWorkspaceStatus::SetupRequired => {
            if home_saved_farm(runtime).is_some() {
                FarmerHomeFarmState::IncompleteFarm
            } else {
                FarmerHomeFarmState::NoFarm
            }
        }
        FarmWorkspaceStatus::Ready => FarmerHomeFarmState::ConfiguredFarm,
    }
}

fn home_farm_order_methods_summary(draft: &FarmSetupDraft) -> String {
    if draft.order_methods.is_empty() {
        return app_shared_text(AppTextKey::ValueNone).to_string();
    }

    draft
        .order_methods
        .iter()
        .copied()
        .map(home_farm_order_method_label_key)
        .map(app_shared_text)
        .map(|label| label.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn home_status_presentation(runtime: &DesktopAppRuntimeSummary) -> HomeStatusPresentation {
    if runtime.startup_issue.is_some() || runtime.startup_gate == AppStartupGate::Blocked {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            label_key: AppTextKey::HomeTodayStatusStartupIssue,
        };
    }

    if runtime.startup_gate == AppStartupGate::SetupRequired {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.offline,
            label_key: AppTextKey::HomeTodayStatusSetup,
        };
    }

    match farmer_home_farm_state(runtime) {
        FarmerHomeFarmState::NoFarm => {
            return HomeStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.offline,
                label_key: AppTextKey::HomeTodayStatusNoFarm,
            };
        }
        FarmerHomeFarmState::IncompleteFarm => {
            return HomeStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.offline,
                label_key: AppTextKey::HomeTodayStatusSetup,
            };
        }
        FarmerHomeFarmState::ConfiguredFarm => {}
    }

    if runtime.today_projection.has_attention_items() {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            label_key: AppTextKey::HomeTodayStatusAttention,
        };
    }

    HomeStatusPresentation {
        indicator_color: APP_UI_THEME.components.app_status_indicator.online,
        label_key: AppTextKey::HomeTodayStatusReady,
    }
}

fn home_setup_task_label_key(kind: TodaySetupTaskKind) -> AppTextKey {
    match kind {
        TodaySetupTaskKind::CompleteFarmProfile => AppTextKey::HomeTodaySetupCompleteFarmProfile,
        TodaySetupTaskKind::AddPickupLocation => AppTextKey::HomeTodaySetupAddPickupLocation,
        TodaySetupTaskKind::AddOperatingRules => AppTextKey::HomeTodaySetupAddOperatingRules,
        TodaySetupTaskKind::AddFulfillmentWindow => AppTextKey::HomeTodaySetupAddFulfillmentWindow,
        TodaySetupTaskKind::ResolveAvailabilityConflicts => {
            AppTextKey::HomeTodaySetupResolveAvailabilityConflicts
        }
        TodaySetupTaskKind::PublishProduct => AppTextKey::HomeTodaySetupPublishProduct,
    }
}

fn home_farm_order_method_label_key(method: FarmOrderMethod) -> AppTextKey {
    match method {
        FarmOrderMethod::Pickup => AppTextKey::HomeFarmSetupOrderMethodPickup,
        FarmOrderMethod::Delivery => AppTextKey::HomeFarmSetupOrderMethodDelivery,
        FarmOrderMethod::Shipping => AppTextKey::HomeFarmSetupOrderMethodShipping,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppTextKey, FarmerHomeFarmState, HomeStage, SETTINGS_FARM_PANEL_SECTIONS,
        SETTINGS_NAVIGATION_ORDER, SETTINGS_OPERATIONS_PANEL_SECTIONS,
        SettingsInventorySectionSpec, SettingsPanelViewKey, StartupHomeSurface,
        StartupSignerConnectState, about_conflict_review_body_key, about_conflict_review_rows,
        about_runtime_rows, about_status_rows, app_text, buyer_orders_status_key,
        farm_setup_onboarding_card_spec, farmer_home_farm_state, farmer_pack_day_available,
        home_content_scroll_id, home_saved_farm, home_sidebar_navigation_sections, home_stage,
        home_window_launch_size_px, home_window_minimum_size_px,
        parse_optional_product_editor_stock_input, parse_product_editor_price_input,
        product_display_title, startup_home_surface, startup_signer_preview_summary,
        startup_signer_preview_summary_for_connect_state, startup_signer_source_input_is_editable,
        startup_signer_status_spec, startup_signer_transport_failure_requires_notice,
    };
    use crate::runtime::{
        DesktopAppRuntimeMetadataSummary, DesktopAppRuntimeSummary, DesktopAppSyncStatusSummary,
    };
    use radroots_studio_app_models::SettingsAccountProjection;
    use radroots_studio_app_models::{
        ActiveSurface, AppStartupGate, BuyerOrderStatus, FarmId, FarmOrderMethod, FarmReadiness,
        FarmSetupDraft, FarmSetupProjection, FarmSummary, FarmerSection, FulfillmentWindowId,
        FulfillmentWindowSummary, LoggedOutStartupPhase, LoggedOutStartupProjection,
        PackDayProjection, PersonalSection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
        RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_state::{
        AppShellProjection, FarmWorkspaceReadinessProjection, FarmWorkspaceStatus, HomeRoute,
    };
    use radroots_studio_app_sync::{
        AppSyncProjection, AppSyncRunStatus, SyncCheckpointStatus, SyncConflictStatus,
    };
    use radroots_identity::RadrootsIdentity;
    use std::path::PathBuf;

    #[test]
    fn farm_setup_onboarding_uses_frozen_copy_and_primary_action() {
        let spec = farm_setup_onboarding_card_spec(HomeRoute::FarmSetupOnboarding).unwrap();

        assert_eq!(spec.title_key, AppTextKey::HomeFarmSetupOnboardingTitle);
        assert_eq!(spec.body_key, AppTextKey::HomeFarmSetupOnboardingBody);
        assert_eq!(
            spec.action_key,
            Some(AppTextKey::HomeFarmSetupOnboardingAction)
        );
    }

    #[test]
    fn farm_setup_form_route_keeps_onboarding_copy_without_no_farm_empty_state() {
        let spec = farm_setup_onboarding_card_spec(HomeRoute::FarmSetupForm).unwrap();

        assert_eq!(spec.title_key, AppTextKey::HomeFarmSetupOnboardingTitle);
        assert_eq!(spec.body_key, AppTextKey::HomeFarmSetupOnboardingBody);
        assert_eq!(spec.action_key, None);
    }

    #[test]
    fn settings_navigation_order_keeps_farm_between_account_and_settings() {
        assert_eq!(
            SETTINGS_NAVIGATION_ORDER,
            &[
                SettingsPanelViewKey::Account,
                SettingsPanelViewKey::Farm,
                SettingsPanelViewKey::Settings,
                SettingsPanelViewKey::About,
            ]
        );
    }

    #[test]
    fn settings_inventory_sections_follow_the_frozen_farm_rules_order() {
        assert_eq!(
            SETTINGS_FARM_PANEL_SECTIONS,
            &[
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::HomeFarmSetupSectionFarm,
                    field_keys: &[
                        AppTextKey::HomeFarmSetupFieldFarmName,
                        AppTextKey::SettingsFarmFieldTimezone,
                        AppTextKey::SettingsFarmFieldCurrency,
                    ],
                },
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::SettingsPickupLocationsSectionLabel,
                    field_keys: &[
                        AppTextKey::SettingsPickupLocationsFieldLabel,
                        AppTextKey::SettingsPickupLocationsFieldAddress,
                        AppTextKey::SettingsPickupLocationsFieldDirections,
                        AppTextKey::SettingsPickupLocationsFieldDefault,
                    ],
                },
            ]
        );
        assert_eq!(
            SETTINGS_OPERATIONS_PANEL_SECTIONS,
            &[
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::SettingsOperatingRulesSectionLabel,
                    field_keys: &[
                        AppTextKey::SettingsOperatingRulesFieldPromiseLeadTime,
                        AppTextKey::SettingsOperatingRulesFieldSubstitutionPolicy,
                        AppTextKey::SettingsOperatingRulesFieldMissedPickupPolicy,
                    ],
                },
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::SettingsFulfillmentWindowsSectionLabel,
                    field_keys: &[
                        AppTextKey::SettingsFulfillmentWindowsFieldLabel,
                        AppTextKey::SettingsFulfillmentWindowsFieldPickupLocation,
                        AppTextKey::SettingsFulfillmentWindowsFieldStartsAt,
                        AppTextKey::SettingsFulfillmentWindowsFieldEndsAt,
                        AppTextKey::SettingsFulfillmentWindowsFieldOrderCutoff,
                    ],
                },
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::SettingsBlackoutPeriodsSectionLabel,
                    field_keys: &[
                        AppTextKey::SettingsBlackoutPeriodsFieldLabel,
                        AppTextKey::SettingsBlackoutPeriodsFieldStartsAt,
                        AppTextKey::SettingsBlackoutPeriodsFieldEndsAt,
                    ],
                },
                SettingsInventorySectionSpec {
                    title_key: AppTextKey::SettingsReadinessSectionLabel,
                    field_keys: &[
                        AppTextKey::SettingsReadinessFieldMissingProfileBasics,
                        AppTextKey::SettingsReadinessFieldMissingPickupLocation,
                        AppTextKey::SettingsReadinessFieldMissingFulfillmentWindow,
                        AppTextKey::SettingsReadinessFieldMissingOperatingRules,
                        AppTextKey::SettingsReadinessFieldInvalidTimingConflicts,
                    ],
                },
            ]
        );
    }

    #[test]
    fn today_route_has_no_setup_onboarding_card() {
        assert!(farm_setup_onboarding_card_spec(HomeRoute::Today).is_none());
    }

    #[test]
    fn home_window_launch_frame_and_minimum_size_are_split() {
        assert_eq!(home_window_launch_size_px(), (1284.0, 795.0));
        assert_eq!(home_window_minimum_size_px(), (1080.0, 720.0));
    }

    #[test]
    fn startup_home_surface_tracks_the_shared_logged_out_phase_contract() {
        let continue_prompt = summary_with_logged_out_phase(LoggedOutStartupPhase::ContinuePrompt);
        let identity_choice = summary_with_logged_out_phase(LoggedOutStartupPhase::IdentityChoice);
        let generate_key_starting =
            summary_with_logged_out_phase(LoggedOutStartupPhase::GenerateKeyStarting);
        let signer_entry = summary_with_logged_out_phase(LoggedOutStartupPhase::SignerEntry);

        assert_eq!(
            startup_home_surface(&continue_prompt),
            StartupHomeSurface::ContinuePrompt
        );
        assert_eq!(
            startup_home_surface(&identity_choice),
            StartupHomeSurface::IdentityChoice
        );
        assert_eq!(
            startup_home_surface(&generate_key_starting),
            StartupHomeSurface::GenerateKeyStarting
        );
        assert_eq!(
            startup_home_surface(&signer_entry),
            StartupHomeSurface::SignerEntry
        );
    }

    #[test]
    fn startup_home_surface_uses_issue_card_when_setup_is_unavailable() {
        let blocked = DesktopAppRuntimeSummary {
            startup_gate: AppStartupGate::Blocked,
            startup_issue: Some("runtime unavailable".to_owned()),
            ..summary_with_logged_out_phase(LoggedOutStartupPhase::IdentityChoice)
        };

        assert_eq!(
            startup_home_surface(&blocked),
            StartupHomeSurface::IssueCard
        );
        assert_eq!(
            startup_home_surface(&summary(
                HomeRoute::Personal,
                TodayAgendaProjection::default(),
                FarmSetupProjection::default(),
            )),
            StartupHomeSurface::IssueCard
        );
    }

    #[test]
    fn home_stage_uses_buyer_workspace_when_guest_enters_marketplace() {
        let mut guest_marketplace = summary(
            HomeRoute::SetupRequired,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        guest_marketplace.startup_gate = AppStartupGate::SetupRequired;
        guest_marketplace.shell_projection = AppShellProjection::new(
            ActiveSurface::Personal,
            ShellSection::Personal(PersonalSection::Browse),
        );

        assert_eq!(home_stage(&guest_marketplace), HomeStage::BuyerWorkspace);
    }

    #[test]
    fn buyer_orders_status_keys_use_buyer_facing_copy() {
        assert_eq!(
            buyer_orders_status_key(BuyerOrderStatus::Placed),
            AppTextKey::PersonalOrdersStatusPlaced
        );
        assert_eq!(
            buyer_orders_status_key(BuyerOrderStatus::Ready),
            AppTextKey::PersonalOrdersStatusReady
        );
    }

    #[test]
    fn farmer_home_farm_state_distinguishes_no_farm_incomplete_and_configured() {
        let farm_id = FarmId::new();
        let incomplete_farm = FarmSummary {
            farm_id,
            display_name: String::new(),
            readiness: FarmReadiness::Incomplete,
        };
        let configured_farm = FarmSummary {
            farm_id: FarmId::new(),
            display_name: String::new(),
            readiness: FarmReadiness::Ready,
        };

        assert_eq!(
            farmer_home_farm_state(&summary(
                HomeRoute::FarmSetupOnboarding,
                TodayAgendaProjection::default(),
                FarmSetupProjection::default(),
            )),
            FarmerHomeFarmState::NoFarm
        );
        assert_eq!(
            farmer_home_farm_state(&summary(
                HomeRoute::Today,
                TodayAgendaProjection {
                    farm: Some(incomplete_farm.clone()),
                    setup_checklist: vec![TodaySetupTask {
                        kind: TodaySetupTaskKind::AddFulfillmentWindow,
                        is_complete: false,
                    }],
                    ..TodayAgendaProjection::default()
                },
                FarmSetupProjection::new(
                    FarmSetupDraft::new(String::new(), String::new(), [FarmOrderMethod::Pickup]),
                    Some(incomplete_farm),
                ),
            )),
            FarmerHomeFarmState::IncompleteFarm
        );
        assert_eq!(
            farmer_home_farm_state(&summary(
                HomeRoute::Today,
                TodayAgendaProjection {
                    farm: Some(configured_farm.clone()),
                    ..TodayAgendaProjection::default()
                },
                FarmSetupProjection::new(
                    FarmSetupDraft::new(
                        String::new(),
                        String::new(),
                        [FarmOrderMethod::Pickup, FarmOrderMethod::Delivery],
                    ),
                    Some(configured_farm),
                ),
            )),
            FarmerHomeFarmState::ConfiguredFarm
        );
    }

    #[test]
    fn pack_day_availability_tracks_the_contextual_window_projection() {
        let farm_id = FarmId::new();
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::from_saved_farm(FarmSummary {
                farm_id,
                display_name: String::new(),
                readiness: FarmReadiness::Ready,
            }),
        );

        assert!(!farmer_pack_day_available(&runtime));
        assert_eq!(
            home_content_scroll_id(FarmerSection::PackDay),
            "home-pack-day-scroll"
        );

        runtime.pack_day_projection.projection = PackDayProjection {
            fulfillment_window: Some(FulfillmentWindowSummary {
                fulfillment_window_id: FulfillmentWindowId::new(),
                farm_id,
                starts_at: String::new(),
                ends_at: String::new(),
            }),
            totals_by_product: Vec::new(),
            pack_list: Vec::new(),
            pickup_roster: Vec::new(),
        };

        assert!(farmer_pack_day_available(&runtime));
    }

    #[test]
    fn sidebar_navigation_keeps_the_active_destination_first() {
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::Today, true, false),
            vec![
                FarmerSection::Today,
                FarmerSection::Products,
                FarmerSection::Orders,
            ]
        );
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::Products, true, false),
            vec![
                FarmerSection::Products,
                FarmerSection::Today,
                FarmerSection::Orders,
            ]
        );
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::Orders, true, false),
            vec![
                FarmerSection::Orders,
                FarmerSection::Today,
                FarmerSection::Products,
            ]
        );
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::PackDay, true, true),
            vec![
                FarmerSection::PackDay,
                FarmerSection::Today,
                FarmerSection::Products,
                FarmerSection::Orders,
            ]
        );
    }

    #[test]
    fn saved_farm_falls_back_to_local_projection_when_today_is_empty() {
        let saved_farm = FarmSummary {
            farm_id: FarmId::new(),
            display_name: String::new(),
            readiness: FarmReadiness::Ready,
        };
        let runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::new(
                FarmSetupDraft::new(String::new(), String::new(), [FarmOrderMethod::Shipping]),
                Some(saved_farm.clone()),
            ),
        );

        assert_eq!(home_saved_farm(&runtime), Some(&saved_farm));
    }

    #[test]
    fn product_editor_price_parser_handles_blank_whole_and_decimal_inputs() {
        assert_eq!(parse_product_editor_price_input(""), Some(None));
        assert_eq!(parse_product_editor_price_input("6"), Some(Some(600)));
        assert_eq!(parse_product_editor_price_input("6.5"), Some(Some(650)));
        assert_eq!(parse_product_editor_price_input("6.50"), Some(Some(650)));
        assert_eq!(parse_product_editor_price_input("6."), None);
        assert_eq!(parse_product_editor_price_input("6.500"), None);
        assert_eq!(parse_product_editor_price_input("abc"), None);
    }

    #[test]
    fn product_editor_stock_parser_accepts_blank_or_whole_numbers_only() {
        assert_eq!(parse_optional_product_editor_stock_input(""), Some(None));
        assert_eq!(
            parse_optional_product_editor_stock_input("14"),
            Some(Some(14))
        );
        assert_eq!(parse_optional_product_editor_stock_input("14.5"), None);
        assert_eq!(parse_optional_product_editor_stock_input("abc"), None);
    }

    #[test]
    fn blank_product_titles_fall_back_to_the_untitled_copy() {
        assert_eq!(product_display_title(""), "Untitled draft");
        assert_eq!(product_display_title("  "), "Untitled draft");
        assert_eq!(product_display_title("Salad mix"), "Salad mix");
    }

    #[test]
    fn startup_signer_preview_summary_surfaces_parsed_signer_details() {
        let preview = startup_signer_preview_summary(
            "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
        )
        .expect("preview");

        assert_eq!(preview.source_label, "bunker uri");
        assert!(preview.signer_npub.starts_with("npub1"));
        assert_eq!(preview.relays_label, "wss://relay.radroots.example");
        assert_eq!(
            preview.permissions_label,
            "sign_event:kind:1, switch_relays"
        );
    }

    #[test]
    fn startup_signer_status_prefers_auth_challenge_until_approval_is_complete() {
        let pending_session = fixture_pending_session();

        assert_eq!(
            startup_signer_status_spec(&StartupSignerConnectState::Connecting),
            Some((AppTextKey::HomeSetupSignerConnectingTitle, None))
        );
        assert_eq!(
            startup_signer_status_spec(&StartupSignerConnectState::PendingApproval {
                pending_session: pending_session.clone(),
                auth_challenge_url: None,
            }),
            Some((AppTextKey::HomeSetupSignerPendingTitle, None))
        );
        assert_eq!(
            startup_signer_status_spec(&StartupSignerConnectState::PendingApproval {
                pending_session: pending_session.clone(),
                auth_challenge_url: Some("https://auth.example/challenge".to_owned()),
            }),
            Some((
                AppTextKey::HomeSetupSignerAuthChallengeTitle,
                Some("https://auth.example/challenge".to_owned()),
            ))
        );
        assert_eq!(
            startup_signer_status_spec(&StartupSignerConnectState::Approved {
                pending_session,
                approved_session: RadrootsAppRemoteSignerApprovedSession {
                    user_identity: fixture_identity(
                        "2222222222222222222222222222222222222222222222222222222222222222",
                    )
                    .to_public(),
                    relays: vec!["wss://relay.radroots.example".to_owned()],
                    approved_permissions: Default::default(),
                },
                auth_challenge_url: None,
            }),
            Some((AppTextKey::HomeSetupSignerApprovedTitle, None))
        );
    }

    #[test]
    fn startup_signer_source_input_is_editable_only_while_idle() {
        let pending_session = fixture_pending_session();

        assert!(startup_signer_source_input_is_editable(
            &StartupSignerConnectState::Idle
        ));
        assert!(!startup_signer_source_input_is_editable(
            &StartupSignerConnectState::Connecting
        ));
        assert!(!startup_signer_source_input_is_editable(
            &StartupSignerConnectState::PendingApproval {
                pending_session: pending_session.clone(),
                auth_challenge_url: None,
            }
        ));
        assert!(!startup_signer_source_input_is_editable(
            &StartupSignerConnectState::Approved {
                pending_session,
                approved_session: RadrootsAppRemoteSignerApprovedSession {
                    user_identity: fixture_identity(
                        "2222222222222222222222222222222222222222222222222222222222222222",
                    )
                    .to_public(),
                    relays: vec!["wss://relay.radroots.example".to_owned()],
                    approved_permissions: Default::default(),
                },
                auth_challenge_url: None,
            }
        ));
    }

    #[test]
    fn startup_signer_preview_summary_prefers_pending_session_details_once_connect_starts() {
        let pending_session = fixture_pending_session();
        let preview = startup_signer_preview_summary_for_connect_state(
            "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
            &StartupSignerConnectState::PendingApproval {
                pending_session: pending_session.clone(),
                auth_challenge_url: None,
            },
        )
        .expect("preview");

        assert_eq!(
            preview.signer_npub,
            pending_session.record.signer_identity.public_key_npub
        );
        assert_eq!(preview.relays_label, "wss://relay.radroots.example");
        assert_eq!(
            preview.permissions_label,
            "sign_event:kind:1, switch_relays"
        );
    }

    #[test]
    fn startup_signer_transport_failure_notice_ignores_the_waiting_timeout_copy() {
        assert!(!startup_signer_transport_failure_requires_notice(
            "remote signer did not respond yet"
        ));
        assert!(startup_signer_transport_failure_requires_notice(
            "remote signer connection failed: relay refused the request"
        ));
    }

    #[test]
    fn about_status_rows_disable_sync_without_a_selected_account() {
        let rows = about_status_rows(&summary(
            HomeRoute::SetupRequired,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        ));

        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSelectedAccount)
                && row.value == app_text(AppTextKey::ValueNone)
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncRunStatus)
                && row.value == app_text(AppTextKey::ValueDisabled)
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncCheckpointState)
                && row.value == app_text(AppTextKey::ValueNone)
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataStartupIssue)
                && row.value == app_text(AppTextKey::ValueNone)
        }));
    }

    #[test]
    fn about_conflict_review_helpers_surface_blocking_attention_truthfully() {
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime.sync_status = DesktopAppSyncStatusSummary {
            account_id: Some(app_text(AppTextKey::AppName)),
            projection: AppSyncProjection {
                run_status: AppSyncRunStatus::Conflicted,
                checkpoint: SyncCheckpointStatus::never_synced(),
                conflict_status: SyncConflictStatus {
                    unresolved_count: 2,
                    blocking_count: 1,
                },
            },
            pending_write_count: 3,
        };

        let rows = about_conflict_review_rows(&runtime.sync_status);

        assert_eq!(
            about_conflict_review_body_key(&runtime.sync_status),
            AppTextKey::SettingsAboutConflictReviewBlocking
        );
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncConflictCount)
                && row.value == 2.to_string()
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncBlockingConflictCount)
                && row.value == 1.to_string()
        }));
    }

    #[test]
    fn about_runtime_rows_append_paths_schema_and_shell_section() {
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        let data_root = PathBuf::from("/tmp/radroots/data/apps/app");
        let logs_root = PathBuf::from("/tmp/radroots/logs/apps/app");
        let database_path = data_root.join("app.sqlite3");
        runtime.shell_projection.selected_section =
            ShellSection::Settings(SettingsPanelViewKey::About);
        runtime.runtime_metadata = DesktopAppRuntimeMetadataSummary {
            data_root: Some(data_root.clone()),
            logs_root: Some(logs_root),
            database_path: Some(database_path),
            database_schema_version: Some(7),
            ..DesktopAppRuntimeMetadataSummary::default()
        };

        let rows = about_runtime_rows(&runtime);

        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataDataRoot)
                && row.value == data_root.display().to_string()
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataDatabaseSchemaVersion)
                && row.value == 7.to_string()
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataShellSection)
                && row.value == ShellSection::Settings(SettingsPanelViewKey::About).storage_key()
        }));
    }

    fn summary(
        home_route: HomeRoute,
        today_projection: TodayAgendaProjection,
        farm_setup_projection: FarmSetupProjection,
    ) -> DesktopAppRuntimeSummary {
        let farm_readiness_projection = match farm_setup_projection.saved_farm.as_ref() {
            Some(saved_farm)
                if saved_farm.readiness == FarmReadiness::Ready
                    && !today_projection.needs_setup() =>
            {
                FarmWorkspaceReadinessProjection {
                    has_saved_farm: true,
                    status: FarmWorkspaceStatus::Ready,
                    ..FarmWorkspaceReadinessProjection::default()
                }
            }
            Some(_) => FarmWorkspaceReadinessProjection {
                has_saved_farm: true,
                status: FarmWorkspaceStatus::SetupRequired,
                ..FarmWorkspaceReadinessProjection::default()
            },
            None => FarmWorkspaceReadinessProjection::default(),
        };

        DesktopAppRuntimeSummary {
            shell_projection: AppShellProjection::default(),
            settings_account_projection: SettingsAccountProjection::default(),
            startup_gate: AppStartupGate::Farmer,
            logged_out_startup: LoggedOutStartupProjection::default(),
            home_route,
            personal_projection: Default::default(),
            farm_readiness_projection,
            farm_setup_projection,
            today_projection,
            products_projection: Default::default(),
            orders_projection: Default::default(),
            pack_day_projection: Default::default(),
            runtime_metadata: DesktopAppRuntimeMetadataSummary::default(),
            sync_status: crate::runtime::DesktopAppSyncStatusSummary::default(),
            startup_issue: None,
        }
    }

    fn summary_with_logged_out_phase(phase: LoggedOutStartupPhase) -> DesktopAppRuntimeSummary {
        DesktopAppRuntimeSummary {
            startup_gate: AppStartupGate::SetupRequired,
            home_route: HomeRoute::SetupRequired,
            logged_out_startup: LoggedOutStartupProjection {
                phase,
                ..LoggedOutStartupProjection::default()
            },
            ..summary(
                HomeRoute::SetupRequired,
                TodayAgendaProjection::default(),
                FarmSetupProjection::default(),
            )
        }
    }

    fn fixture_identity(secret_key_hex: &str) -> RadrootsIdentity {
        RadrootsIdentity::from_secret_key_str(secret_key_hex).expect("identity")
    }

    fn fixture_pending_session() -> RadrootsAppRemoteSignerPendingSession {
        let signer_identity =
            fixture_identity("1111111111111111111111111111111111111111111111111111111111111111");
        let client_identity =
            fixture_identity("3333333333333333333333333333333333333333333333333333333333333333");

        RadrootsAppRemoteSignerPendingSession {
            record: RadrootsAppRemoteSignerSessionRecord::pending(
                client_identity.to_public(),
                signer_identity.to_public(),
                vec!["wss://relay.radroots.example".to_owned()],
            ),
            client_secret_key_hex: client_identity.secret_key_hex(),
        }
    }
}
