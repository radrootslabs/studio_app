use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, Bounds, ClickEvent, Context, Entity,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, Window, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, div, prelude::FluentBuilder, px, relative, rgb, size,
    transparent_black,
};
use gpui_component::{
    IconName, Root, Sizable, Size as ComponentSize,
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use radroots_studio_app_i18n::AppTextKey;
pub use radroots_studio_app_models::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_models::{
    AppStartupGate, BlackoutPeriodId, BlackoutPeriodRecord, FarmId, FarmOperatingRulesRecord,
    FarmOrderMethod, FarmProfileRecord, FarmReadiness, FarmReadinessBlocker, FarmRulesProjection,
    FarmRulesReadiness, FarmSetupBlocker, FarmSetupDraft, FarmSummary, FarmTimingConflictKind,
    FarmerSection, FulfillmentWindowId, FulfillmentWindowRecord, FulfillmentWindowSummary,
    LoggedOutStartupPhase, OrderListRow, PickupLocationId, PickupLocationRecord,
    ProductAttentionState, ProductEditorDraft, ProductId, ProductListRow, ProductPublishBlocker,
    ProductStatus, ProductsFilter, ProductsListRow, ProductsSort, ShellSection,
    TodayAgendaProjection, TodaySetupTaskKind,
};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingPollOutcome,
    RadrootsAppRemoteSignerPendingSession, radroots_studio_app_remote_signer_connect_pending,
    radroots_studio_app_remote_signer_poll_pending_session_with_progress,
    radroots_studio_app_remote_signer_preview, radroots_studio_app_remote_signer_requested_permissions,
};
use radroots_studio_app_sqlite::derive_farm_rules_readiness;
use radroots_studio_app_state::{FarmSetupFlowStage, HomeRoute};
use radroots_studio_app_ui::{
    APP_UI_THEME, AppCheckboxFieldSpec, IconSegmentButtonSpec, LabelValueRow, action_button,
    action_button_compact, action_button_primary, action_button_primary_disabled,
    action_icon_button, app_checkbox_field, app_shared_label_text, app_shared_text,
    app_window_shell, icon_segment_button, label_value_list, section_divider, status_indicator,
    utility_title_row,
};
use radroots_nostr::prelude::RadrootsNostrClient;
use std::time::Duration;
use tracing::error;

use crate::runtime::{DesktopAppRuntime, DesktopAppRuntimeSummary};

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
    PersonalHolding,
    FarmerWorkspace,
}

pub fn primary_window_target(_: &DesktopAppRuntimeSummary) -> PrimaryWindowTarget {
    PrimaryWindowTarget::Home
}

pub fn home_stage(summary: &DesktopAppRuntimeSummary) -> HomeStage {
    if summary.startup_issue.is_some()
        || matches!(
            summary.startup_gate,
            AppStartupGate::Blocked | AppStartupGate::SetupRequired
        )
    {
        HomeStage::Setup
    } else if summary.startup_gate == AppStartupGate::Farmer {
        HomeStage::FarmerWorkspace
    } else {
        HomeStage::PersonalHolding
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
        APP_UI_THEME.windows.home_min_width_px,
        APP_UI_THEME.windows.home_min_height_px,
    )
}

fn home_window_minimum_size_px() -> (f32, f32) {
    (HOME_WINDOW_MIN_WIDTH_PX, HOME_WINDOW_MIN_HEIGHT_PX)
}

pub fn settings_window_options(cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(
        None,
        size(
            px(APP_UI_THEME.windows.settings_width_px),
            px(APP_UI_THEME.windows.settings_height_px),
        ),
        cx,
    );

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(
            px(APP_UI_THEME.windows.settings_width_px),
            px(APP_UI_THEME.windows.settings_height_px),
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
    logged_in_view: LoggedInHomeView,
    farm_setup_form: Option<FarmSetupFormState>,
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
            logged_in_view: LoggedInHomeView::new(),
            farm_setup_form: None,
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

        if runtime_summary.farm_setup_projection.has_saved_farm() {
            let Some(account_id) = runtime_summary
                .settings_account_projection
                .selected_account
                .as_ref()
                .map(|account| account.account.account_id.clone())
            else {
                return;
            };

            self.farm_setup_form = Some(FarmSetupFormState::new(
                account_id,
                runtime_summary.farm_setup_projection.draft,
                window,
                cx,
            ));
            cx.notify();
            return;
        }

        if self
            .runtime
            .select_farm_setup_flow_stage(FarmSetupFlowStage::Editing)
        {
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
            FarmerSection::Today
            | FarmerSection::Products
            | FarmerSection::Orders
            | FarmerSection::PackDay
            | FarmerSection::Farm => home_today_content(
                runtime,
                self.farm_setup_form.as_ref().map(|form| {
                    home_farm_setup_form_card(
                        form,
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
                    .into_any_element()
                }),
                cx.listener(|this, _, window, cx| this.open_farm_setup(window, cx)),
                cx.listener(|this, _, window, cx| this.open_farm_setup(window, cx)),
                cx.listener(|this, _, _, cx| {
                    this.open_products_filter(ProductsFilter::NeedAttention, cx)
                }),
                cx.listener(|this, _, _, cx| this.open_products_filter(ProductsFilter::Drafts, cx)),
                cx,
            )
            .into_any_element(),
        };

        home_shell_frame(
            home_sidebar(
                runtime,
                cx.listener(|this, _, _, cx| this.select_farmer_section(FarmerSection::Today, cx)),
                cx.listener(|this, _, _, cx| {
                    this.select_farmer_section(FarmerSection::Products, cx)
                }),
                cx,
            )
            .into_any_element(),
            div()
                .id(home_content_scroll_id(selected_farmer_section))
                .size_full()
                .overflow_y_scroll()
                .child(main_content)
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

        div()
            .w_full()
            .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
        let product = products_row_open_button(
            ("products-row-open", index),
            row,
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
            products_row_action_button(
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
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime_summary = self.runtime.summary();
        self.sync_startup_signer_entry(&runtime_summary, window, cx);
        self.sync_farm_setup_form(&runtime_summary, window, cx);
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
                    cx.listener(|this, _, window, cx| this.start_generate_key(window, cx)),
                    cx.listener(|this, _, _, cx| this.show_startup_signer_entry(cx)),
                    cx.listener(|this, _, window, cx| this.submit_startup_signer(window, cx)),
                    cx.listener(|this, _, _, cx| this.back_out_of_startup_signer_entry(cx)),
                    cx,
                )
                .into_any_element(),
            HomeStage::PersonalHolding => self
                .logged_in_view
                .render_holding(&runtime_summary)
                .into_any_element(),
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
            on_generate_key,
            on_connect_signer,
            on_submit_signer,
            on_back,
            cx,
        )
    }
}

struct LoggedInHomeView;

impl LoggedInHomeView {
    fn new() -> Self {
        Self
    }

    fn render_holding(&self, runtime: &DesktopAppRuntimeSummary) -> AnyElement {
        holding_home_shell(runtime).into_any_element()
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
        let detail_text_px = APP_UI_THEME.typography.settings_account_detail_text_px;
        let account_status_color = APP_UI_THEME.controls.status_indicator.offline;

        div()
            .size_full()
            .flex()
            .child(
                div()
                    .h_full()
                    .w(px(APP_UI_THEME.layout.settings_account_sidebar_width_px))
                    .p(px(APP_UI_THEME.layout.settings_account_sidebar_padding_px))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .w_full()
                            .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
                            .rounded(px(
                                APP_UI_THEME
                                    .layout
                                    .settings_account_sidebar_button_corner_radius_px,
                            ))
                            .p(px(
                                APP_UI_THEME.layout.settings_account_sidebar_button_padding_px,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(
                                                APP_UI_THEME
                                                    .typography
                                                    .settings_account_identity_text_px,
                                            ))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(APP_UI_THEME.text.primary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountNoSelectionTitle,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(
                                                APP_UI_THEME
                                                    .typography
                                                    .settings_account_identity_text_px,
                                            ))
                                            .text_color(rgb(APP_UI_THEME.text.secondary))
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
                            .pt(px(
                                APP_UI_THEME
                                    .layout
                                    .settings_account_sidebar_footer_padding_top_px,
                            ))
                            .flex()
                            .flex_col()
                            .gap(px(
                                APP_UI_THEME.layout.settings_account_sidebar_footer_row_gap_px,
                            ))
                            .child(section_divider())
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(
                                        APP_UI_THEME
                                            .layout
                                            .settings_account_sidebar_footer_button_gap_px,
                                    ))
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
                    .w(px(APP_UI_THEME.layout.divider_thickness_px))
                    .bg(rgb(APP_UI_THEME.surfaces.divider)),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .p(px(APP_UI_THEME.layout.settings_account_main_padding_px))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_start()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(APP_UI_THEME.layout.settings_account_content_max_width_px))
                            .flex()
                            .flex_col()
                            .items_start()
                            .gap(px(APP_UI_THEME.layout.settings_account_main_stack_gap_px))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(APP_UI_THEME.layout.settings_account_main_stack_gap_px))
                                    .child(
                                        div()
                                            .size(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_profile_avatar_size_px,
                                            ))
                                            .bg(rgb(APP_UI_THEME.surfaces.card_background))
                                            .rounded(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_profile_avatar_size_px
                                                    / 2.0,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(detail_text_px))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(APP_UI_THEME.text.primary))
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
                                    .gap(px(APP_UI_THEME.layout.settings_account_detail_row_gap_px))
                                    .child(self.settings_account_detail_row(
                                        AppTextKey::SettingsAccountProfileLabel,
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(self.settings_account_detail_row(
                                        AppTextKey::SettingsAccountStatusLabel,
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_status_gap_px,
                                            ))
                                            .child(status_indicator(account_status_color))
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .text_color(rgb(APP_UI_THEME.text.primary))
                                                    .child(app_shared_text(
                                                        AppTextKey::SettingsAccountStatusLoggedOut,
                                                    )),
                                            ),
                                    ))
                                    .child(self.settings_account_detail_row(
                                        AppTextKey::SettingsAccountCustodyLabel,
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(self.settings_account_detail_row(
                                        AppTextKey::SettingsAccountSurfaceLabel,
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.text.primary))
                                            .child(app_shared_text(AppTextKey::ValueNone)),
                                    ))
                                    .child(self.settings_account_detail_row(
                                        AppTextKey::SettingsAccountActivationLabel,
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.text.primary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountActivationInactive,
                                            )),
                                    ))
                                    .child(
                                        div()
                                            .w_full()
                                            .text_size(px(detail_text_px))
                                            .line_height(relative(1.2))
                                            .text_color(rgb(APP_UI_THEME.text.secondary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountNoSelectionBody,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .min_w_0()
                                            .items_center()
                                            .gap(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_action_row_gap_px,
                                            ))
                                            .child(
                                                div().child(action_button(
                                                    "account-log-out",
                                                    app_shared_text(
                                                        AppTextKey::SettingsAccountLogOutAction,
                                                    ),
                                                    |_, _, _| {},
                                                    cx,
                                                )),
                                            )
                                            .child(
                                                div().child(action_button(
                                                    "account-open-workspace",
                                                    app_shared_text(
                                                        AppTextKey::SettingsAccountOpenWorkspaceAction,
                                                    ),
                                                    |_, _, _| {},
                                                    cx,
                                                )),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn settings_account_detail_row(
        &self,
        label_key: AppTextKey,
        value: impl IntoElement,
    ) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(APP_UI_THEME.layout.settings_account_detail_value_gap_px))
            .child(
                div()
                    .text_size(px(APP_UI_THEME.typography.settings_account_detail_text_px))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(APP_UI_THEME.text.secondary))
                    .child(app_shared_label_text(label_key)),
            )
            .child(value)
    }

    fn settings_checkbox_row(
        &mut self,
        id: &'static str,
        checked: bool,
        label_key: AppTextKey,
        trailing_button_id: Option<&'static str>,
        trailing_button_key: Option<AppTextKey>,
        note_key: Option<AppTextKey>,
        on_toggle: impl Fn(&bool, &mut Window, &mut gpui::App) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let note_text = note_key.map(app_shared_text);

        div().w_full().child(
            div()
                .w_full()
                .flex()
                .items_start()
                .gap(px(APP_UI_THEME.layout.settings_account_detail_value_gap_px))
                .child(app_checkbox_field(
                    AppCheckboxFieldSpec::new(id, app_shared_text(label_key), note_text),
                    checked,
                    cx,
                    move |checked, window, cx| on_toggle(&checked, window, cx),
                ))
                .when_some(
                    trailing_button_id.zip(trailing_button_key),
                    |this, (button_id, button_key)| {
                        this.child(div().flex_none().child(action_button_compact(
                            button_id,
                            app_shared_text(button_key),
                            |_, _, _| {},
                            cx,
                        )))
                    },
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
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(settings_text_field(
                            AppTextKey::SettingsOperatingRulesFieldPromiseLeadTime,
                            &form.operating_rules.promise_lead_hours_input,
                        ))
                        .child(settings_text_field(
                            AppTextKey::SettingsOperatingRulesFieldSubstitutionPolicy,
                            &form.operating_rules.substitution_policy_input,
                        ))
                        .child(settings_text_field(
                            AppTextKey::SettingsOperatingRulesFieldMissedPickupPolicy,
                            &form.operating_rules.missed_pickup_policy_input,
                        ))
                        .children(settings_validation_rows(
                            &evaluation.operating_rules_validation_keys,
                        )),
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
                                            settings_dynamic_action_button(
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
                            settings_dynamic_action_button(
                                "settings-add-fulfillment-window",
                                app_shared_text(AppTextKey::SettingsFulfillmentWindowsAddAction),
                                false,
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
                            settings_dynamic_action_button(
                                "settings-add-blackout-period",
                                app_shared_text(AppTextKey::SettingsBlackoutPeriodsAddAction),
                                false,
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
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(self.settings_checkbox_row(
                        "settings-allow-relay-connections",
                        general_allow_relay_connections,
                        AppTextKey::SettingsGeneralAllowRelayConnections,
                        None,
                        None,
                        None,
                        |_, _, _| {},
                        cx,
                    ))
                    .child(self.settings_checkbox_row(
                        "settings-use-media-servers",
                        general_use_media_servers,
                        AppTextKey::SettingsGeneralUseMediaServers,
                        Some("settings-manage-media-servers"),
                        Some(AppTextKey::SettingsGeneralManageAction),
                        None,
                        |_, _, _| {},
                        cx,
                    ))
                    .child(self.settings_checkbox_row(
                        "settings-use-nip05",
                        general_use_nip05,
                        AppTextKey::SettingsGeneralUseNip05,
                        None,
                        None,
                        Some(AppTextKey::SettingsGeneralUseNip05Note),
                        |_, _, _| {},
                        cx,
                    ))
                    .child(self.settings_checkbox_row(
                        "settings-launch-at-login",
                        general_launch_at_login,
                        AppTextKey::SettingsGeneralLaunchAtLogin,
                        None,
                        None,
                        None,
                        |_, _, _| {},
                        cx,
                    )),
            )
            .into_any_element(),
        );

        settings_inventory_panel(AppTextKey::SettingsSettingsPanelBody, cards)
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
            return settings_inventory_panel(AppTextKey::SettingsFarmPanelBody, cards);
        }

        let Some(form) = self.farm_panel_state.as_ref() else {
            cards.push(
                home_card(
                    app_shared_text(AppTextKey::SettingsNavFarm),
                    home_body_text(app_shared_text(AppTextKey::SettingsFarmUnavailableBody)),
                )
                .into_any_element(),
            );
            return settings_inventory_panel(AppTextKey::SettingsFarmPanelBody, cards);
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
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(settings_text_field(
                        AppTextKey::HomeFarmSetupFieldFarmName,
                        &form.farm_name_input,
                    ))
                    .child(settings_text_field(
                        AppTextKey::SettingsFarmFieldTimezone,
                        &form.timezone_input,
                    ))
                    .child(settings_text_field(
                        AppTextKey::SettingsFarmFieldCurrency,
                        &form.currency_input,
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
                        settings_dynamic_action_button(
                            "settings-farm-add-pickup",
                            app_shared_text(AppTextKey::SettingsPickupLocationsAddAction),
                            false,
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

        settings_inventory_panel(AppTextKey::SettingsFarmPanelBody, cards)
    }

    fn about_panel(&self) -> impl IntoElement {
        div()
            .id("settings-panel-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(
                div()
                    .p(px(APP_UI_THEME.layout.settings_content_padding_px))
                    .size_full()
                    .flex()
                    .flex_col()
                    .py_12()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .gap(px(APP_UI_THEME.layout.settings_account_main_stack_gap_px))
                            .text_size(px(APP_UI_THEME.typography.body_text_px))
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(app_shared_text(
                                AppTextKey::SettingsAboutPlaceholderTopPrimary,
                            ))
                            .child(app_shared_text(
                                AppTextKey::SettingsAboutPlaceholderTopSecondary,
                            ))
                            .child(app_shared_text(
                                AppTextKey::SettingsAboutPlaceholderTopTertiary,
                            )),
                    )
                    .child(section_divider())
                    .child(
                        div()
                            .w_full()
                            .py_12()
                            .text_size(px(APP_UI_THEME.typography.body_text_px))
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(app_shared_text(AppTextKey::SettingsAboutPlaceholderMiddle)),
                    )
                    .child(section_divider())
                    .child(
                        div()
                            .w_full()
                            .py_12()
                            .text_size(px(APP_UI_THEME.typography.body_text_px))
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(app_shared_text(AppTextKey::SettingsAboutPlaceholderBottom)),
                    ),
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

impl Render for SettingsWindowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let navigation_buttons = SETTINGS_NAVIGATION_ORDER
            .iter()
            .copied()
            .map(|view| self.navigation_button(view, cx).into_any_element())
            .collect::<Vec<_>>();

        app_window_shell(
            APP_UI_THEME.surfaces.panel_background,
            div()
                .size_full()
                .bg(rgb(APP_UI_THEME.surfaces.panel_background))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(APP_UI_THEME.layout.settings_chrome_height_px))
                        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
                        .flex()
                        .flex_col()
                        .child(utility_title_row(app_shared_text(
                            AppTextKey::SettingsTitle,
                        )))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .justify_center()
                                .pt(px(APP_UI_THEME.layout.settings_navigation_row_padding_px))
                                .pb(px(APP_UI_THEME.layout.settings_navigation_row_padding_px))
                                .gap(px(APP_UI_THEME.layout.settings_navigation_row_gap_px))
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

fn holding_home_shell(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let home_status = home_status_presentation(runtime);
    let (title_key, body_key) = match home_stage(runtime) {
        HomeStage::Setup => (
            AppTextKey::HomeTodayEmptySetupTitle,
            AppTextKey::HomeTodayEmptySetupBody,
        ),
        HomeStage::PersonalHolding => (
            AppTextKey::HomeTodayEmptyNoFarmTitle,
            AppTextKey::HomeTodayEmptyNoFarmBody,
        ),
        HomeStage::FarmerWorkspace => (
            AppTextKey::HomeTodayEmptyQuietTitle,
            AppTextKey::HomeTodayEmptyQuietBody,
        ),
    };
    let mut sections = vec![home_empty_state_card(title_key, body_key).into_any_element()];

    if let Some(issue) = runtime.startup_issue.as_ref() {
        sections.push(
            home_card(
                app_shared_text(AppTextKey::MetadataStartupIssue),
                home_body_text(issue.clone()),
            )
            .into_any_element(),
        );
    }

    home_shell_frame(
        holding_home_sidebar(runtime).into_any_element(),
        div()
            .size_full()
            .child(
                div()
                    .w_full()
                    .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                    .child(home_status_row(&home_status))
                    .children(sections),
            )
            .into_any_element(),
    )
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
    on_generate_key: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_connect_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let surface = startup_home_surface(runtime);

    app_window_shell(
        APP_UI_THEME.surfaces.window_background,
        div()
            .size_full()
            .bg(rgb(APP_UI_THEME.surfaces.window_background))
            .child(
                div()
                    .size_full()
                    .p(px(APP_UI_THEME.layout.home_window_padding_px))
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w_full()
                                    .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
                                    .mx_auto()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
                                    .child(startup_home_title(surface))
                                    .child(startup_home_tagline())
                                    .child(match surface {
                                        StartupHomeSurface::ContinuePrompt => div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
                                            .child(action_button_primary(
                                                "home-continue",
                                                app_shared_text(
                                                    AppTextKey::HomeSetupContinueAction,
                                                ),
                                                on_continue,
                                                cx,
                                            ))
                                            .when_some(startup_notice, |this, error| {
                                                this.child(startup_home_support_text(
                                                    error.to_owned(),
                                                ))
                                            })
                                            .into_any_element(),
                                        StartupHomeSurface::IdentityChoice => div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
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
                                                this.child(startup_home_support_text(
                                                    error.to_owned(),
                                                ))
                                            })
                                            .into_any_element(),
                                        StartupHomeSurface::GenerateKeyStarting => div()
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
                                            .child(action_button_primary_disabled(
                                                "home-generate-key",
                                                app_shared_text(
                                                    AppTextKey::HomeSetupGenerateKeyAction,
                                                ),
                                                cx,
                                            ))
                                            .into_any_element(),
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
                                        StartupHomeSurface::IssueCard => startup_home_card(
                                            app_shared_text(AppTextKey::MetadataStartupIssue),
                                            startup_home_body(runtime),
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
        .text_size(px(APP_UI_THEME.typography.startup_title_text_px))
        .font_weight(gpui::FontWeight::NORMAL)
        .text_color(rgb(APP_UI_THEME.text.primary))
        .text_center()
        .child(app_shared_text(title_key))
        .with_animation(
            animation_id,
            Animation::new(Duration::from_millis(180)),
            |this, delta| this.opacity(delta),
        )
}

fn startup_home_tagline() -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.startup_tagline_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.text.primary))
        .text_center()
        .child(app_shared_text(AppTextKey::HomeSetupTagline))
}

fn startup_home_support_text(body: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.body_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.text.secondary))
        .text_center()
        .child(body.into())
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

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
        .when_some(signer_entry, |this, signer_entry| {
            this.child(
                div()
                    .w_full()
                    .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
                    .id("home-signer-source-input")
                    .child(
                        Input::new(&signer_entry.input)
                            .with_size(ComponentSize::Large)
                            .disabled(!source_input_is_editable)
                            .w_full(),
                    ),
            )
        })
        .when_some(preview.as_ref().ok(), |this, preview| {
            this.child(startup_home_card(
                app_shared_text(AppTextKey::HomeSetupSignerReviewTitle),
                label_value_list([
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
                ]),
            ))
        })
        .when_some(startup_signer_status_spec(connect_state), |this, status| {
            this.child(startup_home_card(
                app_shared_text(status.0),
                status
                    .1
                    .map(|body| startup_home_support_text(body).into_any_element())
                    .unwrap_or_else(|| div().into_any_element()),
            ))
        })
        .when_some(parse_error, |this, error| {
            this.child(startup_home_support_text(error))
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
        .child(startup_text_button(
            "home-signer-back",
            AppTextKey::HomeSetupBackAction,
            on_back,
            cx,
        ))
        .when_some(startup_notice, |this, notice| {
            this.child(startup_home_support_text(notice.to_owned()))
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

fn startup_text_button(
    id: &'static str,
    key: AppTextKey,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(transparent_black().into())
                .foreground(rgb(APP_UI_THEME.text.secondary).into())
                .border(transparent_black())
                .hover(transparent_black().into())
                .active(transparent_black().into()),
        )
        .rounded(ButtonRounded::Size(px(0.0)))
        .on_click(on_click)
        .child(
            div()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(rgb(APP_UI_THEME.text.secondary))
                .child(app_shared_text(key)),
        )
}

fn startup_home_card(title: impl Into<SharedString>, body: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .child(
            div()
                .w_full()
                .p(px(APP_UI_THEME.layout.home_card_padding_px))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(title.into()),
                )
                .child(body),
        )
}

fn startup_home_body(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let body = runtime
        .startup_issue
        .clone()
        .unwrap_or_else(|| app_shared_text(AppTextKey::HomeTodayEmptySetupBody).to_string());

    div()
        .w_full()
        .text_size(px(APP_UI_THEME.typography.body_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.text.secondary))
        .text_center()
        .child(body)
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

fn home_shell_frame(sidebar: AnyElement, main_content: AnyElement) -> impl IntoElement {
    app_window_shell(
        APP_UI_THEME.surfaces.window_background,
        div()
            .size_full()
            .overflow_hidden()
            .flex()
            .child(sidebar)
            .child(
                div()
                    .h_full()
                    .w(px(APP_UI_THEME.layout.divider_thickness_px))
                    .bg(rgb(APP_UI_THEME.surfaces.divider)),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(rgb(APP_UI_THEME.surfaces.window_background))
                    .overflow_hidden()
                    .child(
                        div()
                            .size_full()
                            .p(px(APP_UI_THEME.layout.home_window_padding_px))
                            .child(main_content),
                    ),
            ),
    )
}

fn home_sidebar(
    runtime: &DesktopAppRuntimeSummary,
    on_select_today: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_select_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let home_status = home_status_presentation(runtime);
    let selected_section = selected_farmer_section(runtime);
    let products_available = farmer_products_available(runtime);

    div()
        .h_full()
        .w(px(APP_UI_THEME.layout.home_sidebar_width_px))
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .p(px(APP_UI_THEME.layout.home_window_padding_px))
        .flex()
        .flex_col()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px * 2.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(AppTextKey::AppName)),
                )
                .child(home_status_row(&home_status)),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_start()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(home_sidebar_nav_button(
                    "home-nav-today",
                    AppTextKey::HomeNavToday,
                    selected_section == FarmerSection::Today,
                    on_select_today,
                    cx,
                ))
                .when(products_available, |this| {
                    this.child(home_sidebar_nav_button(
                        "home-nav-products",
                        AppTextKey::HomeNavProducts,
                        selected_section == FarmerSection::Products,
                        on_select_products,
                        cx,
                    ))
                }),
        )
        .child(
            div().child(
                div()
                    .text_size(px(APP_UI_THEME.typography.body_text_px))
                    .line_height(relative(1.2))
                    .text_color(rgb(APP_UI_THEME.text.secondary))
                    .when_some(home_saved_farm(runtime), |this, farm| {
                        this.child(farm.display_name.clone())
                    }),
            ),
        )
}

fn holding_home_sidebar(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let home_status = home_status_presentation(runtime);

    div()
        .h_full()
        .w(px(APP_UI_THEME.layout.home_sidebar_width_px))
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .p(px(APP_UI_THEME.layout.home_window_padding_px))
        .flex()
        .flex_col()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px * 2.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(AppTextKey::AppName)),
                )
                .child(home_status_row(&home_status)),
        )
        .child(
            div().child(
                div()
                    .text_size(px(APP_UI_THEME.typography.body_text_px))
                    .line_height(relative(1.2))
                    .text_color(rgb(APP_UI_THEME.text.secondary))
                    .when_some(home_saved_farm(runtime), |this, farm| {
                        this.child(farm.display_name.clone())
                    }),
            ),
        )
}

fn home_today_content(
    runtime: &DesktopAppRuntimeSummary,
    farm_setup_form: Option<AnyElement>,
    on_start_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_continue_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_low_stock_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_draft_products: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let projection = &runtime.today_projection;
    let home_status = home_status_presentation(runtime);
    let setup_onboarding = farm_setup_onboarding_card_spec(runtime.home_route);
    let farm_state = farmer_home_farm_state(runtime);
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

    if runtime.home_route == HomeRoute::FarmSetupForm {
        if let Some(farm_setup_form) = farm_setup_form {
            sections.push(farm_setup_form);
        }
    } else if let Some(spec) = setup_onboarding {
        sections.push(
            home_farm_setup_onboarding_card(spec, on_start_farm_setup, cx).into_any_element(),
        );
    } else if projection.needs_setup() {
        sections.push(
            home_setup_card(
                projection,
                matches!(farm_state, FarmerHomeFarmState::IncompleteFarm).then_some(
                    action_button_primary(
                        "home-farm-setup-continue",
                        app_shared_text(AppTextKey::HomeFarmSetupContinueAction),
                        on_continue_farm_setup,
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
        sections.push(home_next_fulfillment_window_card(next_window).into_any_element());
    }

    if !projection.orders_needing_action.is_empty() {
        sections.push(
            home_list_card(
                AppTextKey::HomeTodayOrdersNeedingAction,
                projection
                    .orders_needing_action
                    .iter()
                    .map(home_order_row)
                    .collect::<Vec<_>>(),
                None,
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
                        on_open_low_stock_products,
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
                        on_open_draft_products,
                        cx,
                    )
                    .into_any_element(),
                ),
            )
            .into_any_element(),
        );
    }

    if runtime.startup_issue.is_none() && runtime.startup_gate == AppStartupGate::SetupRequired {
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
        .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
        .mx_auto()
        .flex()
        .flex_col()
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px * 2.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(AppTextKey::HomeTodayTitle)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.text.primary))
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
}

fn selected_farmer_section(runtime: &DesktopAppRuntimeSummary) -> FarmerSection {
    match runtime.shell_projection.selected_section {
        ShellSection::Farmer(section) => section,
        ShellSection::Home | ShellSection::Settings(_) => FarmerSection::Today,
    }
}

fn farmer_products_available(runtime: &DesktopAppRuntimeSummary) -> bool {
    runtime.farm_setup_projection.has_saved_farm()
}

fn home_content_scroll_id(section: FarmerSection) -> &'static str {
    match section {
        FarmerSection::Products => "home-products-scroll",
        FarmerSection::Today
        | FarmerSection::Orders
        | FarmerSection::PackDay
        | FarmerSection::Farm => "home-today-scroll",
    }
}

fn home_sidebar_nav_button(
    id: &'static str,
    key: AppTextKey,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    if is_active {
        action_button_primary(id, app_shared_text(key), on_click, cx).into_any_element()
    } else {
        action_button(id, app_shared_text(key), on_click, cx).into_any_element()
    }
}

fn products_title_row(
    runtime: &DesktopAppRuntimeSummary,
    add_product_action: AnyElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px * 2.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(AppTextKey::ProductsTitle)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.text.primary))
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
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
            .when_some(products_search, |this, products_search| {
                this.child(
                    Input::new(&products_search.input)
                        .with_size(ComponentSize::Large)
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
                    .child(products_filter_button(
                        "products-filter-all",
                        AppTextKey::ProductsFilterAll,
                        selected_filter == ProductsFilter::All,
                        on_select_all_products,
                        cx,
                    ))
                    .child(products_filter_button(
                        "products-filter-live",
                        AppTextKey::ProductsFilterLive,
                        selected_filter == ProductsFilter::Live,
                        on_select_live_products,
                        cx,
                    ))
                    .child(products_filter_button(
                        "products-filter-drafts",
                        AppTextKey::ProductsFilterDrafts,
                        selected_filter == ProductsFilter::Drafts,
                        on_select_draft_products,
                        cx,
                    ))
                    .child(products_filter_button(
                        "products-filter-need-attention",
                        AppTextKey::ProductsFilterNeedAttention,
                        selected_filter == ProductsFilter::NeedAttention,
                        on_select_products_needing_attention,
                        cx,
                    ))
                    .child(products_filter_button(
                        "products-filter-paused",
                        AppTextKey::ProductsFilterPaused,
                        selected_filter == ProductsFilter::Paused,
                        on_select_paused_products,
                        cx,
                    ))
                    .child(products_filter_button(
                        "products-filter-archived",
                        AppTextKey::ProductsFilterArchived,
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(app_shared_text(AppTextKey::ProductsSortTitle)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(products_filter_button(
                                "products-sort-updated",
                                AppTextKey::ProductsSortUpdated,
                                selected_sort == ProductsSort::Updated,
                                on_sort_products_by_updated,
                                cx,
                            ))
                            .child(products_filter_button(
                                "products-sort-name",
                                AppTextKey::ProductsSortName,
                                selected_sort == ProductsSort::Name,
                                on_sort_products_by_name,
                                cx,
                            ))
                            .child(products_filter_button(
                                "products-sort-availability",
                                AppTextKey::ProductsSortAvailability,
                                selected_sort == ProductsSort::Availability,
                                on_sort_products_by_availability,
                                cx,
                            ))
                            .child(products_filter_button(
                                "products-sort-stock",
                                AppTextKey::ProductsSortStock,
                                selected_sort == ProductsSort::Stock,
                                on_sort_products_by_stock,
                                cx,
                            ))
                            .child(products_filter_button(
                                "products-sort-price",
                                AppTextKey::ProductsSortPrice,
                                selected_sort == ProductsSort::Price,
                                on_sort_products_by_price,
                                cx,
                            )),
                    ),
            ),
    )
}

fn products_filter_button(
    id: &'static str,
    key: AppTextKey,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    if is_active {
        action_button_primary(id, app_shared_text(key), on_click, cx).into_any_element()
    } else {
        action_button_compact(id, app_shared_text(key), on_click, cx).into_any_element()
    }
}

fn products_table_header() -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.text.secondary))
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
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
                        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(products_status_key(row.status))),
                ),
        )
        .child(
            div()
                .w(px(192.0))
                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(row.availability.label.clone()),
        )
        .child(
            div()
                .w(px(128.0))
                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(products_stock_text(row)),
        )
        .child(
            div()
                .w(px(128.0))
                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(products_price_text(row)),
        )
        .child(
            div()
                .w(px(164.0))
                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.text.secondary))
                .child(row.updated_at.clone()),
        )
        .child(div().w(px(120.0)).flex().justify_end().child(action))
}

fn products_row_open_button(
    id: (&'static str, usize),
    row: &ProductsListRow,
    is_open: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let selected_background = rgb(APP_UI_THEME.surfaces.window_background);

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(if is_open {
                    selected_background.into()
                } else {
                    transparent_black().into()
                })
                .foreground(rgb(APP_UI_THEME.text.primary).into())
                .border(transparent_black())
                .hover(selected_background.into())
                .active(selected_background.into()),
        )
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px)))
        .flex_1()
        .min_w_0()
        .on_click(on_click)
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(4.0))
                .px(px(8.0))
                .py(px(6.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(product_display_title(row.title.as_str())),
                )
                .when_some(row.subtitle.as_ref(), |this, subtitle| {
                    this.child(
                        div()
                            .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                            .line_height(relative(1.2))
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(subtitle.clone()),
                    )
                }),
        )
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
        APP_UI_THEME.controls.status_indicator.attention
    } else {
        match row.status {
            ProductStatus::Published => APP_UI_THEME.controls.status_indicator.online,
            ProductStatus::Draft | ProductStatus::Paused | ProductStatus::Archived => {
                APP_UI_THEME.controls.status_indicator.offline
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

fn products_row_action_button(
    id: (&'static str, usize),
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.controls.action_button.sizing;
    let colors = APP_UI_THEME.controls.action_button.colors;

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(colors.background).into())
                .foreground(rgb(colors.foreground).into())
                .border(transparent_black())
                .hover(rgb(colors.background).into())
                .active(rgb(colors.active_background).into()),
        )
        .rounded(ButtonRounded::Size(px(sizing.corner_radius_px)))
        .h(px(sizing.height_px))
        .on_click(on_click)
        .child(
            div()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .px(px(sizing.compact_horizontal_padding_px))
                .text_size(px(sizing.label_size_px))
                .text_color(rgb(colors.foreground))
                .child(label.into()),
        )
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
        .bg(rgb(APP_UI_THEME.surfaces.window_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
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
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(app_shared_text(AppTextKey::ProductsStockEditorTitle)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.text.secondary))
                        .child(product_display_title(row.title.as_str())),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_end()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(APP_UI_THEME.text.secondary))
                                .child(app_shared_text(AppTextKey::ProductsStockEditorFieldLabel)),
                        )
                        .child(
                            Input::new(&editor.input)
                                .with_size(ComponentSize::Large)
                                .w_full(),
                        )
                        .when_some(validation_key, |this, key| {
                            this.child(
                                div()
                                    .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.text.secondary))
                                    .child(app_shared_text(key)),
                            )
                        })
                        .when(editor.save_failed, |this| {
                            this.child(
                                div()
                                    .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.text.secondary))
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
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(home_body_text(app_shared_text(
                    AppTextKey::ProductsEditorBody,
                )))
                .child(products_editor_text_field(
                    AppTextKey::ProductsEditorFieldTitle,
                    &form.title_input,
                    None,
                ))
                .child(products_editor_text_field(
                    AppTextKey::ProductsEditorFieldSubtitle,
                    &form.subtitle_input,
                    None,
                ))
                .child(products_editor_text_field(
                    AppTextKey::ProductsEditorFieldUnit,
                    &form.unit_input,
                    None,
                ))
                .child(products_editor_text_field(
                    AppTextKey::ProductsEditorFieldPrice,
                    &form.price_input,
                    products_editor_invalid_price_key(form, cx),
                ))
                .child(products_editor_text_field(
                    AppTextKey::ProductsEditorFieldStock,
                    &form.stock_input,
                    products_editor_invalid_stock_key(form, cx),
                ))
                .child(products_editor_status_section(
                    form.status,
                    on_select_draft,
                    on_select_live,
                    on_select_paused,
                    on_select_archived,
                    cx,
                ))
                .child(products_editor_publish_readiness_section(form, cx))
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
                        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.text.secondary))
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

fn products_editor_text_field(
    field_label_key: AppTextKey,
    input: &Entity<InputState>,
    validation_key: Option<AppTextKey>,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_field_label(field_label_key))
        .child(Input::new(input).with_size(ComponentSize::Large).w_full())
        .when_some(validation_key, |this, validation_key| {
            this.child(home_body_text(app_shared_text(validation_key)))
        })
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
        .child(home_farm_setup_field_label(
            AppTextKey::ProductsEditorFieldStatus,
        ))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(products_filter_button(
                    "products-editor-status-draft",
                    AppTextKey::ProductsStatusDraft,
                    selected_status == ProductStatus::Draft,
                    on_select_draft,
                    cx,
                ))
                .child(products_filter_button(
                    "products-editor-status-live",
                    AppTextKey::ProductsStatusLive,
                    selected_status == ProductStatus::Published,
                    on_select_live,
                    cx,
                ))
                .child(products_filter_button(
                    "products-editor-status-paused",
                    AppTextKey::ProductsStatusPaused,
                    selected_status == ProductStatus::Paused,
                    on_select_paused,
                    cx,
                ))
                .child(products_filter_button(
                    "products-editor-status-archived",
                    AppTextKey::ProductsStatusArchived,
                    selected_status == ProductStatus::Archived,
                    on_select_archived,
                    cx,
                )),
        )
}

fn products_editor_publish_readiness_section(
    form: &ProductEditorFormState,
    cx: &App,
) -> impl IntoElement {
    let blockers = form.publish_blockers(cx);

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_field_label(
            AppTextKey::ProductsEditorPublishReadinessTitle,
        ))
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
        .gap(px(APP_UI_THEME.layout.settings_account_status_gap_px))
        .child(status_indicator(
            APP_UI_THEME.controls.status_indicator.attention,
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
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
            .child(home_body_text(app_shared_text(
                AppTextKey::HomeFarmSetupOnboardingBody,
            )))
            .child(home_farm_setup_text_field(
                AppTextKey::HomeFarmSetupSectionFarm,
                AppTextKey::HomeFarmSetupFieldFarmName,
                &form.farm_name_input,
                blockers
                    .contains(&FarmSetupBlocker::AddFarmName)
                    .then_some(AppTextKey::HomeFarmSetupBlockerAddFarmName),
            ))
            .child(home_farm_setup_text_field(
                AppTextKey::HomeFarmSetupSectionLocation,
                AppTextKey::HomeFarmSetupFieldLocationOrServiceArea,
                &form.location_input,
                blockers
                    .contains(&FarmSetupBlocker::AddLocationOrServiceArea)
                    .then_some(AppTextKey::HomeFarmSetupBlockerAddLocationOrServiceArea),
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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

fn home_farm_setup_text_field(
    section_key: AppTextKey,
    field_label_key: AppTextKey,
    input: &Entity<InputState>,
    blocker_key: Option<AppTextKey>,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_section_label(section_key))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(6.0))
                .child(home_farm_setup_field_label(field_label_key))
                .child(
                    Input::new(input)
                        .with_size(ComponentSize::Large)
                        .w_full()
                        .into_any_element(),
                )
                .when_some(blocker_key, |this, blocker_key| {
                    this.child(home_farm_setup_blocker(blocker_key))
                }),
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
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.0))
        .child(home_farm_setup_section_label(
            AppTextKey::HomeFarmSetupSectionOrderMethods,
        ))
        .child(
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
                    this.child(home_farm_setup_blocker(blocker_key))
                }),
        )
}

fn home_farm_setup_section_label(key: AppTextKey) -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.text.secondary))
        .child(app_shared_text(key))
}

fn home_farm_setup_field_label(key: AppTextKey) -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.body_text_px))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(APP_UI_THEME.text.primary))
        .child(app_shared_text(key))
}

fn home_farm_setup_blocker(key: AppTextKey) -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.text.secondary))
        .child(app_shared_text(key))
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

fn settings_text_field(label_key: AppTextKey, input: &Entity<InputState>) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(home_farm_setup_field_label(label_key))
        .child(
            Input::new(input)
                .with_size(ComponentSize::Large)
                .w_full()
                .into_any_element(),
        )
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
            settings_badge_text(AppTextKey::SettingsPickupLocationsDefaultBadge).into_any_element()
        } else {
            settings_dynamic_action_button(
                ("settings-farm-default-pickup", index),
                app_shared_text(AppTextKey::SettingsPickupLocationsMakeDefaultAction),
                false,
                on_make_default,
                cx,
            )
            .into_any_element()
        })
        .when(pickup_location.can_remove, |this| {
            this.child(
                settings_dynamic_action_button(
                    ("settings-farm-remove-pickup", index),
                    app_shared_text(AppTextKey::SettingsPickupLocationsRemoveAction),
                    false,
                    on_remove,
                    cx,
                )
                .into_any_element(),
            )
        });

    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(title),
                )
                .child(action_row),
        )
        .child(settings_text_field(
            AppTextKey::SettingsPickupLocationsFieldLabel,
            &pickup_location.label_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsPickupLocationsFieldAddress,
            &pickup_location.address_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsPickupLocationsFieldDirections,
            &pickup_location.directions_input,
        ))
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

fn settings_validation_rows(keys: &[AppTextKey]) -> Vec<AnyElement> {
    keys.iter()
        .copied()
        .map(home_farm_setup_blocker)
        .map(IntoElement::into_any_element)
        .collect()
}

fn settings_fulfillment_window_card(
    index: usize,
    fulfillment_window: &SettingsFulfillmentWindowFormState,
    pickup_location_options: Vec<AnyElement>,
    validation_keys: &[AppTextKey],
    on_remove: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(settings_fulfillment_window_title(
                            index,
                            fulfillment_window,
                            cx,
                        )),
                )
                .child(
                    settings_dynamic_action_button(
                        ("settings-remove-fulfillment-window", index),
                        app_shared_text(AppTextKey::SettingsFulfillmentWindowsRemoveAction),
                        false,
                        on_remove,
                        cx,
                    )
                    .into_any_element(),
                ),
        )
        .child(settings_text_field(
            AppTextKey::SettingsFulfillmentWindowsFieldLabel,
            &fulfillment_window.label_input,
        ))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(home_farm_setup_field_label(
                    AppTextKey::SettingsFulfillmentWindowsFieldPickupLocation,
                ))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .gap(px(8.0))
                        .children(pickup_location_options),
                ),
        )
        .child(settings_text_field(
            AppTextKey::SettingsFulfillmentWindowsFieldStartsAt,
            &fulfillment_window.starts_at_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsFulfillmentWindowsFieldEndsAt,
            &fulfillment_window.ends_at_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsFulfillmentWindowsFieldOrderCutoff,
            &fulfillment_window.order_cutoff_input,
        ))
        .children(settings_validation_rows(validation_keys))
}

fn settings_blackout_period_card(
    index: usize,
    blackout_period: &SettingsBlackoutPeriodFormState,
    validation_keys: &[AppTextKey],
    on_remove: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(settings_blackout_period_title(index, blackout_period, cx)),
                )
                .child(
                    settings_dynamic_action_button(
                        ("settings-remove-blackout-period", index),
                        app_shared_text(AppTextKey::SettingsBlackoutPeriodsRemoveAction),
                        false,
                        on_remove,
                        cx,
                    )
                    .into_any_element(),
                ),
        )
        .child(settings_text_field(
            AppTextKey::SettingsBlackoutPeriodsFieldLabel,
            &blackout_period.label_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsBlackoutPeriodsFieldStartsAt,
            &blackout_period.starts_at_input,
        ))
        .child(settings_text_field(
            AppTextKey::SettingsBlackoutPeriodsFieldEndsAt,
            &blackout_period.ends_at_input,
        ))
        .children(settings_validation_rows(validation_keys))
}

fn settings_farm_readiness_rows(evaluation: &SettingsFarmRulesEvaluation) -> Vec<AnyElement> {
    let readiness_keys = if evaluation.readiness_keys.is_empty() {
        vec![AppTextKey::SettingsReadinessReady]
    } else {
        evaluation.readiness_keys.clone()
    };

    readiness_keys
        .into_iter()
        .map(settings_inventory_field_row)
        .map(IntoElement::into_any_element)
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

fn settings_badge_text(key: AppTextKey) -> impl IntoElement {
    div()
        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.text.accent))
        .child(app_shared_text(key))
}

fn settings_dynamic_action_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    is_primary: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.controls.action_button.sizing;
    let colors = if is_primary {
        APP_UI_THEME.controls.action_button.primary_colors
    } else {
        APP_UI_THEME.controls.action_button.colors
    };
    let hover_background = if colors.hover_changes_background {
        colors.hover_background
    } else {
        colors.background
    };
    let label = label.into();

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(colors.background).into())
                .foreground(rgb(colors.foreground).into())
                .border(transparent_black())
                .hover(rgb(hover_background).into())
                .active(rgb(colors.active_background).into()),
        )
        .rounded(ButtonRounded::Size(px(sizing.corner_radius_px)))
        .h(px(sizing.height_px))
        .on_click(on_click)
        .child(
            div()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .px(px(sizing.compact_horizontal_padding_px))
                .text_size(px(sizing.label_size_px))
                .text_color(rgb(colors.foreground))
                .child(label),
        )
}

fn settings_inventory_panel(intro_key: AppTextKey, cards: Vec<AnyElement>) -> impl IntoElement {
    let content_max_width_px = 560.0;

    div()
        .id("settings-panel-scroll")
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .w_full()
                .p(px(APP_UI_THEME.layout.settings_content_padding_px))
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(content_max_width_px))
                        .flex()
                        .flex_col()
                        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                        .child(home_body_text(app_shared_text(intro_key)))
                        .children(cards),
                ),
        )
}

#[cfg(test)]
fn settings_inventory_card(spec: SettingsInventorySectionSpec) -> impl IntoElement {
    home_card(
        app_shared_text(spec.title_key),
        div().w_full().flex().flex_col().gap(px(8.0)).children(
            spec.field_keys
                .iter()
                .copied()
                .map(settings_inventory_field_row)
                .map(IntoElement::into_any_element)
                .collect::<Vec<_>>(),
        ),
    )
}

fn settings_inventory_field_row(key: AppTextKey) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .px(px(12.0))
        .py(px(10.0))
        .child(home_farm_setup_field_label(key))
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

fn home_card(title: impl Into<SharedString>, body: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .child(
            div()
                .w_full()
                .p(px(APP_UI_THEME.layout.home_card_padding_px))
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(title.into()),
                )
                .child(body),
        )
}

fn home_body_text(body: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.typography.body_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.text.secondary))
        .child(body.into())
}

fn home_status_row(status: &HomeStatusPresentation) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.layout.settings_account_status_gap_px))
        .child(status_indicator(status.indicator_color))
        .child(
            div()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .text_color(rgb(APP_UI_THEME.text.secondary))
                .child(app_shared_text(status.label_key)),
        )
}

fn home_summary_card(summary: &radroots_studio_app_models::TodaySummary) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::HomeTodayTitle),
        div()
            .w_full()
            .flex()
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
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
        .bg(rgb(APP_UI_THEME.surfaces.window_background))
        .rounded(px(APP_UI_THEME
            .controls
            .action_button
            .sizing
            .corner_radius_px))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(APP_UI_THEME.typography.body_text_px * 2.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(value.to_string()),
        )
        .child(
            div()
                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.text.secondary))
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

fn home_next_fulfillment_window_card(next_window: &FulfillmentWindowSummary) -> impl IntoElement {
    home_card(
        app_shared_text(AppTextKey::HomeTodayNextFulfillmentWindow),
        label_value_list(vec![
            LabelValueRow::new(
                app_shared_text(AppTextKey::HomeTodayWindowStartsLabel),
                next_window.starts_at.clone(),
            ),
            LabelValueRow::new(
                app_shared_text(AppTextKey::HomeTodayWindowEndsLabel),
                next_window.ends_at.clone(),
            ),
        ]),
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
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
            .children(rows)
            .when_some(action, |this, action| this.child(div().child(action))),
    )
}

fn home_order_row(order: &OrderListRow) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.text.primary))
                        .child(order.order_number.clone()),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.text.secondary))
                        .child(order.customer_display_name.clone()),
                ),
        )
        .child(status_indicator(
            APP_UI_THEME.controls.status_indicator.attention,
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
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(product_display_title(product.title.as_str())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(APP_UI_THEME.layout.settings_account_status_gap_px))
                .child(status_indicator(
                    APP_UI_THEME.controls.status_indicator.attention,
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.text.secondary))
                                .child(app_shared_label_text(AppTextKey::HomeTodayStockCountLabel)),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.typography.body_text_px))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(APP_UI_THEME.text.primary))
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
        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
        .child(
            div()
                .min_w_0()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(APP_UI_THEME.text.primary))
                .child(product_display_title(product.title.as_str())),
        )
        .child(status_indicator(
            APP_UI_THEME.controls.status_indicator.offline,
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
        .gap(px(APP_UI_THEME.layout.settings_account_status_gap_px))
        .child(status_indicator(if is_complete {
            APP_UI_THEME.controls.status_indicator.online
        } else {
            APP_UI_THEME.controls.status_indicator.offline
        }))
        .child(
            div()
                .min_w_0()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .line_height(relative(1.2))
                .text_color(rgb(if is_complete {
                    APP_UI_THEME.text.secondary
                } else {
                    APP_UI_THEME.text.primary
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
    let Some(saved_farm) = home_saved_farm(runtime) else {
        return FarmerHomeFarmState::NoFarm;
    };

    if runtime.today_projection.needs_setup() || saved_farm.readiness == FarmReadiness::Incomplete {
        FarmerHomeFarmState::IncompleteFarm
    } else {
        FarmerHomeFarmState::ConfiguredFarm
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
            indicator_color: APP_UI_THEME.controls.status_indicator.attention,
            label_key: AppTextKey::HomeTodayStatusStartupIssue,
        };
    }

    if runtime.startup_gate == AppStartupGate::SetupRequired {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.controls.status_indicator.offline,
            label_key: AppTextKey::HomeTodayStatusSetup,
        };
    }

    match farmer_home_farm_state(runtime) {
        FarmerHomeFarmState::NoFarm => {
            return HomeStatusPresentation {
                indicator_color: APP_UI_THEME.controls.status_indicator.offline,
                label_key: AppTextKey::HomeTodayStatusNoFarm,
            };
        }
        FarmerHomeFarmState::IncompleteFarm => {
            return HomeStatusPresentation {
                indicator_color: APP_UI_THEME.controls.status_indicator.offline,
                label_key: AppTextKey::HomeTodayStatusSetup,
            };
        }
        FarmerHomeFarmState::ConfiguredFarm => {}
    }

    if runtime.today_projection.has_attention_items() {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.controls.status_indicator.attention,
            label_key: AppTextKey::HomeTodayStatusAttention,
        };
    }

    HomeStatusPresentation {
        indicator_color: APP_UI_THEME.controls.status_indicator.online,
        label_key: AppTextKey::HomeTodayStatusReady,
    }
}

fn home_setup_task_label_key(kind: TodaySetupTaskKind) -> AppTextKey {
    match kind {
        TodaySetupTaskKind::AddFulfillmentWindow => AppTextKey::HomeTodaySetupAddFulfillmentWindow,
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
        AppTextKey, FarmerHomeFarmState, SETTINGS_FARM_PANEL_SECTIONS, SETTINGS_NAVIGATION_ORDER,
        SETTINGS_OPERATIONS_PANEL_SECTIONS, SettingsInventorySectionSpec, SettingsPanelViewKey,
        StartupHomeSurface, StartupSignerConnectState, farm_setup_onboarding_card_spec,
        farmer_home_farm_state, home_saved_farm, home_window_launch_size_px,
        home_window_minimum_size_px, parse_optional_product_editor_stock_input,
        parse_product_editor_price_input, product_display_title, startup_home_surface,
        startup_signer_preview_summary, startup_signer_preview_summary_for_connect_state,
        startup_signer_source_input_is_editable, startup_signer_status_spec,
        startup_signer_transport_failure_requires_notice,
    };
    use crate::runtime::DesktopAppRuntimeSummary;
    use radroots_studio_app_models::SettingsAccountProjection;
    use radroots_studio_app_models::{
        AppStartupGate, FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft,
        FarmSetupProjection, FarmSummary, LoggedOutStartupPhase, LoggedOutStartupProjection,
        TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
        RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_state::AppShellProjection;
    use radroots_studio_app_state::HomeRoute;
    use radroots_identity::RadrootsIdentity;

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

    fn summary(
        home_route: HomeRoute,
        today_projection: TodayAgendaProjection,
        farm_setup_projection: FarmSetupProjection,
    ) -> DesktopAppRuntimeSummary {
        DesktopAppRuntimeSummary {
            shell_projection: AppShellProjection::default(),
            settings_account_projection: SettingsAccountProjection::default(),
            startup_gate: AppStartupGate::Farmer,
            logged_out_startup: LoggedOutStartupProjection::default(),
            home_route,
            farm_setup_projection,
            today_projection,
            products_projection: Default::default(),
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
