use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, Bounds, ClickEvent, Context, ElementId,
    Entity, Image, ImageFormat, InteractiveElement, IntoElement, ObjectFit, ParentElement, Render,
    SharedString, Styled, StyledImage, Subscription, Timer, Window, WindowBounds, WindowOptions,
    div, img, prelude::FluentBuilder, px, relative, rgb, size,
};
use gpui_component::{IconName, Root, input::InputEvent, input::InputState, menu::PopupMenuItem};
use radroots_studio_app_i18n::{AppTextKey, app_text};
use radroots_studio_app_remote_signer::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingPollOutcome,
    RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerSource,
    radroots_studio_app_remote_signer_connect_pending,
    radroots_studio_app_remote_signer_poll_pending_session_with_progress,
    radroots_studio_app_remote_signer_preview, radroots_studio_app_remote_signer_requested_permissions,
};
use radroots_studio_app_sqlite::{AppSqliteError, derive_farm_rules_readiness};
use radroots_studio_app_state::{
    BuyerOrdersScreenProjection, FarmSetupFlowStage, FarmWorkspaceStatus, HomeRoute,
    PackDayBatchPrintRequest, PackDayExportProjection, PackDayHostHandoffRequest,
    PackDayPrintRequest, derive_product_publish_blockers,
};
use radroots_studio_app_sync::{
    AppOrderReceiptOutcome, AppSyncRunStatus, SyncAggregateRef, SyncCheckpointState, SyncConflict,
    SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity,
};
use radroots_studio_app_ui::{
    APP_UI_THEME, AppCheckboxFieldSpec, AppFormFieldSpec,
    AppSegmentButtonIconSpec as IconSegmentButtonSpec, AppUnderlineTabSpec, LabelValueRow,
    SettingsPreferencesGeneralRowState, app_button_account_selector_row as account_selector_row,
    app_button_card, app_button_choice as choice_button,
    app_button_compact as action_button_compact, app_button_list_row as list_row_button,
    app_button_primary as action_button_primary,
    app_button_primary_disabled as action_button_primary_disabled,
    app_button_secondary as action_button, app_button_secondary_disabled as action_button_disabled,
    app_button_square_dropdown_secondary as action_dropdown_button, app_button_text as text_button,
    app_checkbox_field, app_cluster, app_detail_row, app_divider as section_divider,
    app_focused_detail_view, app_focused_task_view, app_form_field, app_form_input_text,
    app_form_section, app_heading_section, app_heading_view, app_input_text as app_text_input,
    app_scroll_panel, app_segment_button_icon as icon_segment_button, app_shared_label_text,
    app_shared_text, app_split_shell, app_stack_h, app_stack_v,
    app_status_indicator as status_indicator, app_surface_card,
    app_surface_card_section as home_card, app_surface_panel, app_surface_sidebar,
    app_surface_window as app_window_shell, app_text_badge as settings_badge_text,
    app_text_body_subtle as home_body_text, app_text_label,
    app_text_label as home_farm_setup_field_label, app_text_value, app_underline_tabs,
    label_value_list, runtime_metadata_rows, settings_preferences_general_rows, utility_title_row,
};
pub use radroots_studio_app_view::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_view::{
    AccountCustody, AccountSummary, ActiveSurface, AppStartupGate, BlackoutPeriodId,
    BlackoutPeriodRecord, BuyerCartProjection, BuyerCartReplaceConfirmationProjection,
    BuyerListingRow, BuyerOrderDetailProjection, BuyerOrderReviewDraft,
    BuyerOrderReviewSummaryProjection, BuyerOrderStatus, BuyerOrdersListRow,
    BuyerProductDetailProjection, FarmId, FarmOperatingRulesRecord, FarmOrderMethod,
    FarmProfileRecord, FarmReadinessBlocker, FarmRulesProjection, FarmRulesReadiness,
    FarmSetupBlocker, FarmSetupDraft, FarmSummary, FarmTimingConflictKind, FarmerSection,
    FulfillmentWindowId, FulfillmentWindowRecord, FulfillmentWindowSummary, LoggedOutStartupPhase,
    OrderDetailItemRow, OrderDetailProjection, OrderFulfillmentAction, OrderId, OrderListRow,
    OrderPrimaryAction, OrderRecoveryProjection, OrderStatus, OrdersFilter, OrdersListRow,
    PackDayBatchPrintFailureKind, PackDayBatchPrintStatus, PackDayExportBundle,
    PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPackListRow,
    PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintStatus, PackDayProductTotalRow,
    PackDayRosterRow, PersonalEntryState, PersonalSection, PickupLocationId, PickupLocationRecord,
    ProductAttentionState, ProductEditorDraft, ProductId, ProductListRow, ProductPricePresentation,
    ProductPublishBlocker, ProductStatus, ProductsFilter, ProductsListRow, ProductsSort,
    RecoveryKind, RecoveryState, ReminderDeadlineProjection, ReminderDeliveryState, ReminderId,
    ReminderLogEntryProjection, ReminderLogProjection, ReminderSurface, ReminderUrgency,
    RepeatDemandEligibility, RepeatDemandHandoffProjection, SettingsAccountProjection,
    ShellSection, TodayAgendaProjection, TodaySetupTaskKind, TradeAgreementStatus,
    TradeEconomicsProjection, TradeFulfillmentStatus, TradeInventoryStatus,
    TradePaymentDisplayStatus, TradeReceiptProjection, TradeRevisionStatus,
    TradeValidationReceiptProjection, TradeValidationReceiptResult, TradeValidationReceiptType,
    TradeWorkflowProjection, TradeWorkflowSource,
};
use radroots_nostr::prelude::RadrootsNostrClient;
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tracing::error;

use crate::pack_day_host_handoff::{
    PackDayHostHandoffCommandPlan, PackDayHostHandoffError, execute_pack_day_host_handoff_plan,
};
use crate::pack_day_print::{
    PackDayBatchPrintCommandPlan, PackDayBatchPrintError, PackDayPrintCommandPlan,
    PackDayPrintError, execute_pack_day_batch_print_plan, execute_pack_day_print_plan,
};
use crate::runtime::{
    DesktopAppRuntime, DesktopAppRuntimeSummary, DesktopAppSyncConflictSummary,
    DesktopAppSyncStatusSummary,
};

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

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimaryWindowTarget {
    Home,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HomeStage {
    Setup,
    AccountWorkspace,
    BuyerWorkspace,
    FarmerWorkspace,
}

#[cfg(test)]
pub fn primary_window_target(_: &DesktopAppRuntimeSummary) -> PrimaryWindowTarget {
    PrimaryWindowTarget::Home
}

pub fn home_stage(summary: &DesktopAppRuntimeSummary) -> HomeStage {
    if summary.startup_issue.is_some() || summary.startup_gate == AppStartupGate::Blocked {
        HomeStage::Setup
    } else if matches!(
        summary.shell_projection.selected_section,
        ShellSection::Account
    ) {
        HomeStage::AccountWorkspace
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HomeFocusedView {
    FarmSetup,
    ProductEditor,
    FarmerOrderDetail(OrderId),
    BuyerProductDetail(PersonalSection),
    BuyerOrderReview,
    BuyerOrderDetail(OrderId),
    BuyerReceiptIssue(OrderId),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum AccountTab {
    #[default]
    Profile,
    FarmDetails,
    Preferences,
    Security,
}

impl AccountTab {
    const ORDERED: [Self; 4] = [
        Self::Profile,
        Self::FarmDetails,
        Self::Preferences,
        Self::Security,
    ];

    const fn text_key(self) -> AppTextKey {
        match self {
            Self::Profile => AppTextKey::AccountTabProfile,
            Self::FarmDetails => AppTextKey::AccountTabFarmDetails,
            Self::Preferences => AppTextKey::AccountTabPreferences,
            Self::Security => AppTextKey::AccountTabSecurity,
        }
    }

    const fn panel_text_key(self) -> AppTextKey {
        match self {
            Self::Profile | Self::FarmDetails => self.text_key(),
            Self::Preferences | Self::Security => AppTextKey::AccountNotImplemented,
        }
    }

    fn selected_index(self) -> usize {
        Self::ORDERED
            .iter()
            .position(|tab| *tab == self)
            .unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        Self::ORDERED.get(index).copied().unwrap_or_default()
    }
}

fn buyer_order_detail_focus_after_open(
    runtime_changed: bool,
    runtime: &DesktopAppRuntimeSummary,
    order_id: OrderId,
) -> Option<HomeFocusedView> {
    if runtime_changed
        || runtime
            .personal_projection
            .orders
            .detail
            .as_ref()
            .is_some_and(|detail| detail.order_id == order_id)
    {
        Some(HomeFocusedView::BuyerOrderDetail(order_id))
    } else {
        None
    }
}

fn farmer_order_detail_focus_after_open(
    runtime_changed: bool,
    runtime: &DesktopAppRuntimeSummary,
    order_id: OrderId,
) -> Option<HomeFocusedView> {
    if runtime_changed
        || runtime
            .orders_projection
            .detail
            .as_ref()
            .is_some_and(|detail| detail.order_id == order_id)
    {
        Some(HomeFocusedView::FarmerOrderDetail(order_id))
    } else {
        None
    }
}

fn buyer_receipt_issue_focus_after_submit(
    runtime_changed: bool,
    order_id: OrderId,
) -> Option<HomeFocusedView> {
    runtime_changed.then_some(HomeFocusedView::BuyerOrderDetail(order_id))
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
    buyer_order_review_form: Option<BuyerOrderReviewFormState>,
    buyer_receipt_issue_form: Option<BuyerReceiptIssueFormState>,
    products_search: Option<ProductsSearchState>,
    products_stock_editor: Option<ProductsStockEditorState>,
    product_editor_form: Option<ProductEditorFormState>,
    focused_view: Option<HomeFocusedView>,
    selected_account_tab: AccountTab,
    relay_client: Option<RadrootsNostrClient>,
    buyer_workspace_notice: Option<String>,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HomeAutoFocusState {
    has_startup_signer_input: bool,
    startup_signer_input_is_editable: bool,
    has_farm_setup_form: bool,
    has_personal_search_input: bool,
    has_buyer_order_review_form: bool,
    has_buyer_receipt_issue_form: bool,
    has_products_search_input: bool,
    has_products_stock_editor: bool,
    has_product_editor_form: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HomeAutoFocusTarget {
    StartupContinue,
    StartupGenerateKey,
    StartupSignerInput,
    StartupSignerBack,
    BuyerSearchInput,
    BuyerListingOpenFirst,
    BuyerDetailBack,
    BuyerCartOpenOrderReview,
    BuyerOrderReviewNameInput,
    BuyerReceiptIssueInput,
    BuyerOrderOpenFirst,
    BuyerOrderConfirmReplace,
    BuyerOrderRepeatDemand,
    FarmerReminderPrimary,
    FarmerReminderDismiss,
    FarmerSetupStart,
    FarmerSetupContinue,
    FarmerSetupFarmNameInput,
    FarmerTodayReminderChipFirst,
    FarmerTodayOpenPackDay,
    FarmerTodayOpenOrders,
    FarmerTodayOpenProductsLowStock,
    FarmerTodayOpenProductsDrafts,
    ProductsSearchInput,
    ProductsRowOpenFirst,
    ProductsStockInput,
    ProductEditorTitleInput,
    OrdersRowOpenFirst,
    OrdersDetailPublishFulfillmentFirst,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuyerWorkspaceNotice {
    MarketplaceRefreshFailed,
    DetailOpenFailed,
    OrderPlaceFailed,
    OrderCoordinationFailed,
}

impl BuyerWorkspaceNotice {
    fn text_key(self) -> AppTextKey {
        match self {
            Self::MarketplaceRefreshFailed => AppTextKey::PersonalMarketplaceRefreshFailedNotice,
            Self::DetailOpenFailed => AppTextKey::PersonalDetailOpenFailedNotice,
            Self::OrderPlaceFailed => AppTextKey::PersonalOrderPlaceFailedNotice,
            Self::OrderCoordinationFailed => AppTextKey::PersonalOrderCoordinationFailedNotice,
        }
    }

    fn text(self) -> String {
        app_text(self.text_key())
    }
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
            buyer_order_review_form: None,
            buyer_receipt_issue_form: None,
            products_search: None,
            products_stock_editor: None,
            product_editor_form: None,
            focused_view: None,
            selected_account_tab: AccountTab::default(),
            relay_client: None,
            buyer_workspace_notice: None,
        }
    }

    fn auto_focus_state(&self) -> HomeAutoFocusState {
        HomeAutoFocusState {
            has_startup_signer_input: self.startup_signer_entry.is_some(),
            startup_signer_input_is_editable: startup_signer_source_input_is_editable(
                &self.startup_signer_connect_state,
            ),
            has_farm_setup_form: self.farm_setup_form.is_some(),
            has_personal_search_input: self.personal_search.is_some(),
            has_buyer_order_review_form: self.buyer_order_review_form.is_some(),
            has_buyer_receipt_issue_form: self.buyer_receipt_issue_form.is_some(),
            has_products_search_input: self.products_search.is_some(),
            has_products_stock_editor: self.products_stock_editor.is_some(),
            has_product_editor_form: self.product_editor_form.is_some(),
        }
    }

    fn clear_focused_view(&mut self) -> bool {
        self.focused_view.take().is_some()
    }

    fn clear_focused_view_matching(&mut self, view: HomeFocusedView) -> bool {
        if self.focused_view == Some(view) {
            self.focused_view = None;
            true
        } else {
            false
        }
    }

    fn apply_auto_focus(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let desired_target = home_auto_focus_target(runtime, self.auto_focus_state());
        let focus_state = window.use_state(cx, |_, _| Option::<HomeAutoFocusTarget>::None);
        let should_focus = {
            let last_target = focus_state.read(cx);
            last_target.as_ref().copied() != desired_target
        };

        if !should_focus {
            return;
        }

        if let Some(target) = desired_target {
            match target {
                HomeAutoFocusTarget::StartupContinue => {
                    focus_button(window, "home-continue", cx);
                }
                HomeAutoFocusTarget::StartupGenerateKey => {
                    focus_button(window, "home-generate-key", cx);
                }
                HomeAutoFocusTarget::StartupSignerInput => {
                    if let Some(entry) = self.startup_signer_entry.as_ref() {
                        entry.input.update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::StartupSignerBack => {
                    focus_button(window, "home-signer-back", cx);
                }
                HomeAutoFocusTarget::BuyerSearchInput => {
                    if let Some(search) = self.personal_search.as_ref() {
                        search.input.update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::BuyerListingOpenFirst => {
                    focus_button(window, ("buyer-listing-open", 0_usize), cx);
                }
                HomeAutoFocusTarget::BuyerDetailBack => {
                    focus_button(window, "buyer-detail-back", cx);
                }
                HomeAutoFocusTarget::BuyerCartOpenOrderReview => {
                    focus_button(window, "buyer-cart-open-order-review", cx);
                }
                HomeAutoFocusTarget::BuyerOrderReviewNameInput => {
                    if let Some(form) = self.buyer_order_review_form.as_ref() {
                        form.name_input
                            .update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::BuyerReceiptIssueInput => {
                    if let Some(form) = self.buyer_receipt_issue_form.as_ref() {
                        form.issue_input
                            .update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::BuyerOrderOpenFirst => {
                    focus_button(window, ("buyer-order-open", 0_usize), cx);
                }
                HomeAutoFocusTarget::BuyerOrderConfirmReplace => {
                    focus_button(window, "buyer-order-confirm-replace", cx);
                }
                HomeAutoFocusTarget::BuyerOrderRepeatDemand => {
                    focus_button(window, "buyer-order-repeat-demand", cx);
                }
                HomeAutoFocusTarget::FarmerReminderPrimary => {
                    focus_button(window, "reminder-banner-action", cx);
                }
                HomeAutoFocusTarget::FarmerReminderDismiss => {
                    focus_button(window, "reminder-banner-dismiss", cx);
                }
                HomeAutoFocusTarget::FarmerSetupStart => {
                    focus_button(window, "home-farm-setup-start", cx);
                }
                HomeAutoFocusTarget::FarmerSetupContinue => {
                    focus_button(window, "home-farm-setup-continue", cx);
                }
                HomeAutoFocusTarget::FarmerSetupFarmNameInput => {
                    if let Some(form) = self.farm_setup_form.as_ref() {
                        form.farm_name_input
                            .update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::FarmerTodayReminderChipFirst => {
                    focus_button(window, ("today-reminder-chip", 0_usize), cx);
                }
                HomeAutoFocusTarget::FarmerTodayOpenPackDay => {
                    focus_button(window, "home-today-open-pack-day", cx);
                }
                HomeAutoFocusTarget::FarmerTodayOpenOrders => {
                    focus_button(window, "home-today-open-orders", cx);
                }
                HomeAutoFocusTarget::FarmerTodayOpenProductsLowStock => {
                    focus_button(window, "home-today-open-products-low-stock", cx);
                }
                HomeAutoFocusTarget::FarmerTodayOpenProductsDrafts => {
                    focus_button(window, "home-today-open-products-drafts", cx);
                }
                HomeAutoFocusTarget::ProductsSearchInput => {
                    if let Some(search) = self.products_search.as_ref() {
                        search.input.update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::ProductsRowOpenFirst => {
                    focus_button(window, ("products-row-open", 0_usize), cx);
                }
                HomeAutoFocusTarget::ProductsStockInput => {
                    if let Some(editor) = self.products_stock_editor.as_ref() {
                        editor.input.update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::ProductEditorTitleInput => {
                    if let Some(form) = self.product_editor_form.as_ref() {
                        form.title_input
                            .update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                HomeAutoFocusTarget::OrdersRowOpenFirst => {
                    focus_button(window, ("orders-row-open", 0_usize), cx);
                }
                HomeAutoFocusTarget::OrdersDetailPublishFulfillmentFirst => {
                    focus_button(window, "orders-detail-publish-preparing", cx);
                }
            }
        }

        focus_state.update(cx, |last_target, _| *last_target = desired_target);
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
        let relay_urls = self.runtime.nostr_relay_urls();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let startup_task = cx
                .background_executor()
                .spawn(run_startup_app_init(relay_urls));
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
            self.focused_view = Some(HomeFocusedView::FarmSetup);
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
        self.focused_view = Some(HomeFocusedView::FarmSetup);
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
            self.clear_focused_view_matching(HomeFocusedView::FarmSetup);
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

        if runtime_summary.home_route == HomeRoute::FarmSetupForm {
            self.focused_view = Some(HomeFocusedView::FarmSetup);
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

    fn sync_buyer_order_review_form(
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
            self.buyer_order_review_form = None;
            return;
        }

        let workspace_id = personal_workspace_id(runtime_summary);
        let draft = &runtime_summary.personal_projection.cart.order_review.draft;
        let should_reset = self
            .buyer_order_review_form
            .as_ref()
            .map(|form| form.workspace_id != workspace_id)
            .unwrap_or(false);

        if should_reset {
            self.buyer_order_review_form = Some(BuyerOrderReviewFormState::new(
                workspace_id,
                draft,
                window,
                cx,
            ));
            return;
        }

        if let Some(form) = self.buyer_order_review_form.as_mut() {
            form.sync(draft, window, cx);
        }
    }

    fn sync_buyer_receipt_issue_form(&mut self, runtime_summary: &DesktopAppRuntimeSummary) {
        let Some(form) = self.buyer_receipt_issue_form.as_ref() else {
            return;
        };

        if home_stage(runtime_summary) != HomeStage::BuyerWorkspace
            || selected_personal_section(runtime_summary) != PersonalSection::Orders
        {
            self.buyer_receipt_issue_form = None;
            return;
        }

        let Some(detail) = runtime_summary.personal_projection.orders.detail.as_ref() else {
            self.buyer_receipt_issue_form = None;
            return;
        };

        if detail.order_id != form.order_id || !buyer_receipt_actions_available(detail) {
            self.buyer_receipt_issue_form = None;
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
            self.clear_focused_view_matching(HomeFocusedView::ProductEditor);
            return;
        };

        if selected_farmer_section(runtime_summary) != FarmerSection::Products
            || !runtime_summary.farm_setup_projection.has_saved_farm()
        {
            self.product_editor_form = None;
            self.clear_focused_view_matching(HomeFocusedView::ProductEditor);
            return;
        }

        let radroots_studio_app_state::ProductEditorState::Open(session) =
            &runtime_summary.products_projection.editor
        else {
            self.product_editor_form = None;
            self.clear_focused_view_matching(HomeFocusedView::ProductEditor);
            return;
        };
        let Some(product_id) = session.selected_product_id else {
            self.product_editor_form = None;
            self.clear_focused_view_matching(HomeFocusedView::ProductEditor);
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
            self.clear_focused_view();
            if section != FarmerSection::Products {
                self.product_editor_form = None;
            }
            cx.notify();
        }
    }

    fn select_personal_section(&mut self, section: PersonalSection, cx: &mut Context<Self>) {
        if self.select_personal_section_update(section) {
            cx.notify();
        }
    }

    fn select_personal_section_update(&mut self, section: PersonalSection) -> bool {
        match self.runtime.select_personal_section(section) {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.clear_focused_view();
                self.clear_buyer_workspace_notice();
                true
            }
            Ok(false) => self.clear_buyer_workspace_notice(),
            Err(runtime_error) => {
                error!(
                    target: "shell",
                    event = "buyer.section_select_failed",
                    section = ?section,
                    error = %runtime_error,
                    "failed to select buyer section"
                );
                self.set_buyer_workspace_notice(BuyerWorkspaceNotice::MarketplaceRefreshFailed)
            }
        }
    }

    fn switch_to_marketplace(&mut self, cx: &mut Context<Self>) {
        match self
            .runtime
            .select_active_surface(radroots_studio_app_view::ActiveSurface::Personal)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.clear_focused_view();
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
            .select_active_surface(radroots_studio_app_view::ActiveSurface::Farmer)
        {
            Ok(true) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.clear_focused_view();
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
        if self.runtime.select_account() {
            self.products_stock_editor = None;
            self.product_editor_form = None;
            self.clear_focused_view();
            cx.notify();
        }
    }

    fn select_account_tab(&mut self, tab: AccountTab, cx: &mut Context<Self>) {
        if self.selected_account_tab != tab {
            self.selected_account_tab = tab;
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
        if self.set_personal_search_query_update(value.as_str()) {
            cx.notify();
        }
    }

    fn set_personal_search_query_update(&mut self, value: &str) -> bool {
        match self.runtime.set_personal_search_query(value) {
            Ok(changed) => self.clear_buyer_workspace_notice() || changed,
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.search_query_update_failed",
                    error = %runtime_error,
                    "failed to update buyer search query"
                );
                self.set_buyer_workspace_notice(BuyerWorkspaceNotice::MarketplaceRefreshFailed)
            }
        }
    }

    fn handle_buyer_order_review_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(form) = self.buyer_order_review_form.as_ref() else {
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
            .save_personal_order_review_draft(form.current_draft(cx))
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.order_review_save_failed",
                    error = %runtime_error,
                    "failed to save buyer order review draft"
                );
            }
        }
    }

    fn handle_buyer_receipt_issue_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        let Some(form) = self.buyer_receipt_issue_form.as_ref() else {
            return;
        };
        if form.issue_input == *state {
            cx.notify();
        }
    }

    fn toggle_personal_search_fulfillment_method(
        &mut self,
        method: FarmOrderMethod,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.set_personal_search_fulfillment_method_update(method, enabled) {
            cx.notify();
        }
    }

    fn set_personal_search_fulfillment_method_update(
        &mut self,
        method: FarmOrderMethod,
        enabled: bool,
    ) -> bool {
        match self
            .runtime
            .set_personal_search_fulfillment_method(method, enabled)
        {
            Ok(changed) => self.clear_buyer_workspace_notice() || changed,
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.fulfillment_filter_update_failed",
                    error = %runtime_error,
                    method = method.storage_key(),
                    "failed to update buyer fulfillment filter"
                );
                self.set_buyer_workspace_notice(BuyerWorkspaceNotice::MarketplaceRefreshFailed)
            }
        }
    }

    fn open_personal_product_detail(
        &mut self,
        section: PersonalSection,
        product_id: ProductId,
        cx: &mut Context<Self>,
    ) {
        if self.open_personal_product_detail_update(section, product_id) {
            cx.notify();
        }
    }

    fn open_personal_product_detail_update(
        &mut self,
        section: PersonalSection,
        product_id: ProductId,
    ) -> bool {
        match self
            .runtime
            .open_personal_product_detail(section, product_id)
        {
            Ok(true) => {
                self.clear_buyer_workspace_notice();
                self.focused_view = Some(HomeFocusedView::BuyerProductDetail(section));
                true
            }
            Ok(false) => self.clear_buyer_workspace_notice(),
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.detail_open_failed",
                    error = %runtime_error,
                    "failed to open buyer product detail"
                );
                self.set_buyer_workspace_notice(BuyerWorkspaceNotice::DetailOpenFailed)
            }
        }
    }

    fn set_buyer_workspace_notice(&mut self, notice: BuyerWorkspaceNotice) -> bool {
        let notice = notice.text();
        let changed = self.buyer_workspace_notice.as_deref() != Some(notice.as_str());
        self.buyer_workspace_notice = Some(notice);
        changed
    }

    fn clear_buyer_workspace_notice(&mut self) -> bool {
        self.buyer_workspace_notice.take().is_some()
    }

    fn close_personal_product_detail(&mut self, section: PersonalSection, cx: &mut Context<Self>) {
        let runtime_changed = self.runtime.close_personal_product_detail(section);
        let focus_changed =
            self.clear_focused_view_matching(HomeFocusedView::BuyerProductDetail(section));
        if runtime_changed || focus_changed {
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

    fn open_personal_order_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.buyer_order_review_form.is_some() {
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

        self.buyer_order_review_form = Some(BuyerOrderReviewFormState::new(
            personal_workspace_id(&runtime_summary),
            &runtime_summary.personal_projection.cart.order_review.draft,
            window,
            cx,
        ));
        self.focused_view = Some(HomeFocusedView::BuyerOrderReview);
        cx.notify();
    }

    fn close_personal_order_review(&mut self, cx: &mut Context<Self>) {
        let cleared = self.buyer_order_review_form.take().is_some();
        let focus_changed = self.clear_focused_view_matching(HomeFocusedView::BuyerOrderReview);
        if cleared || focus_changed {
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
        if self.place_personal_order_update() {
            cx.notify();
        }
    }

    fn place_personal_order_update(&mut self) -> bool {
        match self.runtime.place_personal_order() {
            Ok(true) => {
                self.buyer_order_review_form = None;
                let _ = self.clear_buyer_workspace_notice();
                true
            }
            Ok(false) => false,
            Err(runtime_error) => {
                let notice = buyer_order_place_failure_notice(&runtime_error);
                if notice == BuyerWorkspaceNotice::OrderCoordinationFailed {
                    self.buyer_order_review_form = None;
                }
                error!(
                    target: "buyer",
                    event = "buyer.order_review_place_failed",
                    error = %runtime_error,
                    "failed to place buyer order"
                );
                let notice_changed = self.set_buyer_workspace_notice(notice);
                buyer_order_coordination_notice_forces_redraw(notice) || notice_changed
            }
        }
    }

    fn retry_pending_personal_order_coordination(&mut self, cx: &mut Context<Self>) {
        if self.retry_pending_personal_order_coordination_update() {
            cx.notify();
        }
    }

    fn retry_pending_personal_order_coordination_update(&mut self) -> bool {
        match self.runtime.retry_pending_personal_order_coordination() {
            Ok(true) => {
                let _ = self.clear_buyer_workspace_notice();
                true
            }
            Ok(false) => false,
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.order_coordination_retry_failed",
                    error = %runtime_error,
                    "failed to retry buyer order coordination"
                );
                let notice = BuyerWorkspaceNotice::OrderCoordinationFailed;
                let notice_changed = self.set_buyer_workspace_notice(notice);
                buyer_order_coordination_notice_forces_redraw(notice) || notice_changed
            }
        }
    }

    fn open_personal_order_detail(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.open_personal_order_detail(order_id) {
            Ok(runtime_changed) => {
                let Some(focused_view) = buyer_order_detail_focus_after_open(
                    runtime_changed,
                    &self.runtime.summary(),
                    order_id,
                ) else {
                    return;
                };
                if self
                    .buyer_receipt_issue_form
                    .as_ref()
                    .is_some_and(|form| form.order_id != order_id)
                {
                    self.buyer_receipt_issue_form = None;
                }
                self.focused_view = Some(focused_view);
                cx.notify();
            }
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

    fn close_personal_order_detail(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        if self.clear_focused_view_matching(HomeFocusedView::BuyerOrderDetail(order_id)) {
            cx.notify();
        }
    }

    fn repeat_personal_order(
        &mut self,
        order_id: OrderId,
        replace_existing: bool,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .repeat_personal_order(order_id, replace_existing)
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "buyer",
                    event = "buyer.repeat_demand_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to reorder buyer order"
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
                self.clear_focused_view();
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
                self.clear_focused_view();
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
                self.clear_focused_view();
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

    fn export_pack_day(&mut self, cx: &mut Context<Self>) {
        match self.runtime.export_pack_day() {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.export_failed",
                    error = %runtime_error,
                    "failed to export pack day"
                );
                cx.notify();
            }
        }
    }

    fn start_pack_day_host_handoff(
        &mut self,
        kind: PackDayHostHandoffKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.prepare_pack_day_host_handoff(kind) {
            Ok(Some((request, plan))) => {
                cx.notify();
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(run_pack_day_host_handoff(plan))
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.finish_pack_day_host_handoff(request, result, cx);
                    });
                })
                .detach();
            }
            Ok(None) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.host_handoff_prepare_failed",
                    kind = %kind.storage_key(),
                    error = %runtime_error,
                    "failed to prepare pack day host handoff"
                );
                cx.notify();
            }
        }
    }

    fn start_pack_day_print(
        &mut self,
        kind: PackDayPrintKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.prepare_pack_day_print(kind) {
            Ok(Some((request, plan))) => {
                cx.notify();
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(run_pack_day_print(plan))
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.finish_pack_day_print(request, result, cx);
                    });
                })
                .detach();
            }
            Ok(None) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.print_prepare_failed",
                    kind = %kind.storage_key(),
                    error = %runtime_error,
                    "failed to prepare pack day print"
                );
                cx.notify();
            }
        }
    }

    fn start_pack_day_batch_print(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.runtime.prepare_pack_day_batch_print() {
            Ok(Some((request, plan))) => {
                cx.notify();
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(run_pack_day_batch_print(plan))
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.finish_pack_day_batch_print(request, result, cx);
                    });
                })
                .detach();
            }
            Ok(None) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.batch_print_prepare_failed",
                    error = %runtime_error,
                    "failed to prepare pack day batch print"
                );
                cx.notify();
            }
        }
    }

    fn finish_pack_day_host_handoff(
        &mut self,
        request: PackDayHostHandoffRequest,
        result: Result<(), PackDayHostHandoffError>,
        cx: &mut Context<Self>,
    ) {
        let kind = request.kind.storage_key();
        match self.runtime.finish_pack_day_host_handoff(request, result) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.host_handoff_failed",
                    kind = %kind,
                    error = %runtime_error,
                    "failed to complete pack day host handoff"
                );
                cx.notify();
            }
        }
    }

    fn finish_pack_day_print(
        &mut self,
        request: PackDayPrintRequest,
        result: Result<(), PackDayPrintError>,
        cx: &mut Context<Self>,
    ) {
        let kind = request.kind.storage_key();
        match self.runtime.finish_pack_day_print(request, result) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.print_failed",
                    kind = %kind,
                    error = %runtime_error,
                    "failed to complete pack day print"
                );
                cx.notify();
            }
        }
    }

    fn finish_pack_day_batch_print(
        &mut self,
        request: PackDayBatchPrintRequest,
        result: Result<(), PackDayBatchPrintError>,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.finish_pack_day_batch_print(request, result) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "pack_day",
                    event = "pack_day.batch_print_failed",
                    error = %runtime_error,
                    "failed to complete pack day batch print"
                );
                cx.notify();
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
                self.clear_focused_view();
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
            Ok(runtime_changed) => {
                let Some(focused_view) = farmer_order_detail_focus_after_open(
                    runtime_changed,
                    &self.runtime.summary(),
                    order_id,
                ) else {
                    return;
                };
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.focused_view = Some(focused_view);
                cx.notify();
            }
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

    fn close_order_detail(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        if self.clear_focused_view_matching(HomeFocusedView::FarmerOrderDetail(order_id)) {
            cx.notify();
        }
    }

    fn dismiss_presented_reminder(&mut self, reminder_id: ReminderId, cx: &mut Context<Self>) {
        match self.runtime.acknowledge_reminder(reminder_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "reminders",
                    event = "reminders.ack_failed",
                    error = %runtime_error,
                    reminder_id = %reminder_id,
                    "failed to acknowledge reminder"
                );
            }
        }
    }

    fn open_presented_order_reminder(
        &mut self,
        reminder_id: ReminderId,
        order_id: OrderId,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.open_order_detail(order_id) {
            Ok(true) | Ok(false) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.focused_view = Some(HomeFocusedView::FarmerOrderDetail(order_id));
                self.dismiss_presented_reminder(reminder_id, cx);
            }
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

    fn open_presented_pack_day_reminder(
        &mut self,
        reminder_id: ReminderId,
        fulfillment_window_id: FulfillmentWindowId,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.open_pack_day(Some(fulfillment_window_id)) {
            Ok(true) | Ok(false) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.dismiss_presented_reminder(reminder_id, cx);
            }
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

    fn open_presented_orders_reminder(&mut self, reminder_id: ReminderId, cx: &mut Context<Self>) {
        match self.runtime.open_orders() {
            Ok(true) | Ok(false) => {
                self.products_stock_editor = None;
                self.product_editor_form = None;
                self.dismiss_presented_reminder(reminder_id, cx);
            }
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

    fn publish_order_fulfillment_update(
        &mut self,
        order_id: OrderId,
        action: OrderFulfillmentAction,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .publish_order_fulfillment_update(order_id, action)
        {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.fulfillment_publish_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    fulfillment_state = action.storage_key(),
                    "failed to publish order fulfillment update"
                );
            }
        }
    }

    fn cancel_buyer_order(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.publish_buyer_order_cancel(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "personal_orders",
                    event = "buyer.order_cancel_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to cancel buyer order"
                );
            }
        }
    }

    fn accept_buyer_order_revision(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.publish_buyer_order_revision_accept(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "personal_orders",
                    event = "buyer.order_revision_accept_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to accept buyer order change"
                );
            }
        }
    }

    fn decline_buyer_order_revision(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self.runtime.publish_buyer_order_revision_decline(order_id) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "personal_orders",
                    event = "buyer.order_revision_decline_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to keep buyer order"
                );
            }
        }
    }

    fn open_buyer_receipt_issue_form(
        &mut self,
        order_id: OrderId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.buyer_receipt_issue_form = Some(BuyerReceiptIssueFormState::new(order_id, window, cx));
        self.focused_view = Some(HomeFocusedView::BuyerReceiptIssue(order_id));
        cx.notify();
    }

    fn close_buyer_receipt_issue_form(&mut self, cx: &mut Context<Self>) {
        let order_id = self
            .buyer_receipt_issue_form
            .as_ref()
            .map(|form| form.order_id);
        let cleared = self.buyer_receipt_issue_form.take().is_some();
        let focus_changed = order_id
            .map(|order_id| {
                self.clear_focused_view_matching(HomeFocusedView::BuyerReceiptIssue(order_id))
            })
            .unwrap_or(false);
        if focus_changed {
            if let Some(order_id) = order_id {
                self.focused_view = Some(HomeFocusedView::BuyerOrderDetail(order_id));
            }
        }
        if cleared || focus_changed {
            cx.notify();
        }
    }

    fn mark_buyer_order_received(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        match self
            .runtime
            .publish_buyer_order_receipt(order_id, AppOrderReceiptOutcome::Received)
        {
            Ok(true) => {
                if self
                    .buyer_receipt_issue_form
                    .as_ref()
                    .is_some_and(|form| form.order_id == order_id)
                {
                    self.buyer_receipt_issue_form = None;
                }
                cx.notify();
            }
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "personal_orders",
                    event = "buyer.order_receipt_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to mark buyer order received"
                );
            }
        }
    }

    fn submit_buyer_order_issue_receipt(&mut self, order_id: OrderId, cx: &mut Context<Self>) {
        let Some(issue) = self
            .buyer_receipt_issue_form
            .as_ref()
            .filter(|form| form.order_id == order_id)
            .and_then(|form| AppOrderReceiptOutcome::issue(form.issue_text(cx)))
        else {
            return;
        };

        match self.runtime.publish_buyer_order_receipt(order_id, issue) {
            Ok(runtime_changed) => {
                if let Some(focused_view) =
                    buyer_receipt_issue_focus_after_submit(runtime_changed, order_id)
                {
                    self.buyer_receipt_issue_form = None;
                    self.focused_view = Some(focused_view);
                    cx.notify();
                }
            }
            Err(runtime_error) => {
                error!(
                    target: "personal_orders",
                    event = "buyer.order_issue_receipt_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    "failed to report buyer order issue"
                );
            }
        }
    }

    fn start_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.start_order_recovery(order_id, kind) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.recovery_start_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    recovery_kind = kind.storage_key(),
                    "failed to start order recovery"
                );
            }
        }
    }

    fn review_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.review_order_recovery(order_id, kind) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.recovery_review_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    recovery_kind = kind.storage_key(),
                    "failed to review order recovery"
                );
            }
        }
    }

    fn reopen_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.reopen_order_recovery(order_id, kind) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.recovery_reopen_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    recovery_kind = kind.storage_key(),
                    "failed to reopen order recovery"
                );
            }
        }
    }

    fn resolve_order_recovery(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.resolve_order_recovery(order_id, kind) {
            Ok(true) => cx.notify(),
            Ok(false) => {}
            Err(runtime_error) => {
                error!(
                    target: "orders",
                    event = "orders.recovery_resolve_failed",
                    error = %runtime_error,
                    order_id = %order_id,
                    recovery_kind = kind.storage_key(),
                    "failed to resolve order recovery"
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
                self.focused_view = Some(HomeFocusedView::ProductEditor);
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
                self.focused_view = Some(HomeFocusedView::ProductEditor);
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
        let focus_changed = self.clear_focused_view_matching(HomeFocusedView::ProductEditor);

        if changed || cleared || focus_changed {
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
            || form.category_input == *state
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

    fn select_product_editor_availability_window(
        &mut self,
        availability_window_id: FulfillmentWindowId,
        cx: &mut Context<Self>,
    ) {
        let Some(form) = self.product_editor_form.as_mut() else {
            return;
        };

        if form.selected_availability_window_id == Some(availability_window_id) {
            return;
        }

        form.selected_availability_window_id = Some(availability_window_id);
        form.save_failed = false;
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

        if let Some(spec) = setup_onboarding {
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

        if !projection.reminders.is_empty() {
            sections.push(self.render_today_reminder_strip(&projection.reminders.items, cx));
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
        let main_content = self
            .render_buyer_focused_view(runtime, cx)
            .unwrap_or_else(|| match selected_personal_section {
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
            });

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
                .when_some(self.buyer_workspace_notice.as_deref(), |this, notice| {
                    this.child(buyer_workspace_notice_card(notice.to_owned()))
                })
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

    fn render_buyer_focused_view(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        match self.focused_view? {
            HomeFocusedView::BuyerProductDetail(section) => {
                let detail = match section {
                    PersonalSection::Browse => runtime.personal_projection.browse.detail.as_ref(),
                    PersonalSection::Search => runtime.personal_projection.search.detail.as_ref(),
                    PersonalSection::Cart | PersonalSection::Orders => None,
                }?;
                Some(
                    buyer_product_detail_card(
                        detail,
                        runtime
                            .personal_projection
                            .cart
                            .cart
                            .replace_confirmation
                            .as_ref(),
                        cx.listener(move |this, _, _, cx| {
                            this.close_personal_product_detail(section, cx)
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.decrease_personal_product_quantity(section, cx)
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.increase_personal_product_quantity(section, cx)
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.add_personal_product_to_cart(section, false, cx)
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.add_personal_product_to_cart(section, true, cx)
                        }),
                        cx.listener(|this, _, _, cx| {
                            this.clear_personal_cart_replace_confirmation(cx)
                        }),
                        cx,
                    )
                    .into_any_element(),
                )
            }
            HomeFocusedView::BuyerOrderReview => {
                let form = self.buyer_order_review_form.as_ref()?;
                Some(
                    buyer_order_review_card(
                        form,
                        &runtime.personal_projection.cart.order_review,
                        cx.listener(|this, _, _, cx| this.close_personal_order_review(cx)),
                        cx.listener(|this, _, _, cx| this.place_personal_order(cx)),
                        cx,
                    )
                    .into_any_element(),
                )
            }
            HomeFocusedView::BuyerOrderDetail(order_id) => {
                let detail = runtime
                    .personal_projection
                    .orders
                    .detail
                    .as_ref()
                    .filter(|detail| detail.order_id == order_id)?;
                Some(
                    buyer_order_detail_card(
                        detail,
                        None,
                        runtime
                            .personal_projection
                            .cart
                            .cart
                            .replace_confirmation
                            .as_ref(),
                        cx.listener(move |this, _, _, cx| {
                            this.close_personal_order_detail(order_id, cx)
                        }),
                        cx,
                    )
                    .into_any_element(),
                )
            }
            HomeFocusedView::BuyerReceiptIssue(order_id) => {
                let detail = runtime
                    .personal_projection
                    .orders
                    .detail
                    .as_ref()
                    .filter(|detail| detail.order_id == order_id)?;
                let issue_form = self
                    .buyer_receipt_issue_form
                    .as_ref()
                    .filter(|form| form.order_id == order_id)?;
                Some(buyer_receipt_issue_focused_view(detail, issue_form, cx))
            }
            HomeFocusedView::FarmSetup
            | HomeFocusedView::ProductEditor
            | HomeFocusedView::FarmerOrderDetail(_) => None,
        }
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
        let order_review = &runtime.personal_projection.cart.order_review;

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
                        &order_review.summary,
                        self.buyer_order_review_form.is_some(),
                        cx,
                    ))
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
            .when(buyer_orders_retry_action_visible(orders), |this| {
                this.child(buyer_orders_retry_card(cx))
            })
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
        let main_content = self
            .render_farmer_focused_view(runtime, cx)
            .unwrap_or_else(|| match selected_farmer_section {
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
            });

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
                .when_some(presented_farmer_reminder(runtime), |this, reminder| {
                    this.child(
                        div()
                            .w_full()
                            .px(px(APP_UI_THEME.shells.home_window_padding_px))
                            .child(
                                div()
                                    .w_full()
                                    .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
                                    .mx_auto()
                                    .child(self.render_presented_reminder_banner(reminder, cx)),
                            ),
                    )
                })
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

    fn render_farmer_focused_view(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        match self.focused_view? {
            HomeFocusedView::FarmSetup => {
                let form = self.farm_setup_form.as_ref()?;
                Some(
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
                    .into_any_element(),
                )
            }
            HomeFocusedView::ProductEditor => {
                let form = self.product_editor_form.as_ref()?;
                Some(products_editor_surface(form, runtime, cx).into_any_element())
            }
            HomeFocusedView::FarmerOrderDetail(order_id) => {
                let detail = runtime
                    .orders_projection
                    .detail
                    .as_ref()
                    .filter(|detail| detail.order_id == order_id)?;
                Some(self.render_order_detail_card(
                    detail,
                    cx.listener(move |this, _, _, cx| this.close_order_detail(order_id, cx)),
                    cx,
                ))
            }
            HomeFocusedView::BuyerProductDetail(_)
            | HomeFocusedView::BuyerOrderReview
            | HomeFocusedView::BuyerOrderDetail(_)
            | HomeFocusedView::BuyerReceiptIssue(_) => None,
        }
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
                        AppTextKey::OrdersStatusInHandoff,
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
                        app_shared_text(AppTextKey::OrdersStatusInHandoff),
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
            .when(!projection.reminders.is_empty(), |this| {
                this.child(self.render_reminder_feed_card(
                    "orders-reminders",
                    AppTextKey::OrdersRemindersTitle,
                    &projection.reminders.items,
                    cx,
                ))
            })
            .child(self.render_orders_reminder_log_card(&runtime.reminder_log))
            .child(if projection.list.is_empty() {
                orders_empty_state_card(projection.query.filter).into_any_element()
            } else {
                self.render_orders_table_card(
                    &projection.list.rows,
                    projection.detail.as_ref().map(|detail| detail.order_id),
                    cx,
                )
            })
            .into_any_element()
    }

    fn render_presented_reminder_banner(
        &mut self,
        reminder: &ReminderDeadlineProjection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let primary_action = self.render_presented_reminder_primary_action(reminder, cx);

        home_card(
            app_shared_text(AppTextKey::ReminderPresentationTitle),
            app_stack_v(APP_UI_THEME.foundation.spacing.medium_px)
                .w_full()
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .justify_between()
                        .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                                .child(status_indicator(reminder_urgency_color(reminder.urgency)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(APP_UI_THEME
                                            .foundation
                                            .typography
                                            .body_text_px))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .line_height(relative(1.2))
                                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                        .child(reminder.title.clone()),
                                ),
                        )
                        .child(
                            app_cluster(APP_UI_THEME.foundation.spacing.tight_px)
                                .child(reminder_urgency_badge(reminder.urgency))
                                .child(reminder_delivery_state_badge(reminder.delivery_state)),
                        ),
                )
                .when(!reminder.detail.trim().is_empty(), |this| {
                    this.child(home_body_text(reminder.detail.clone()))
                })
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(APP_UI_THEME.foundation.spacing.medium_px))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(reminder_deadline_text(reminder)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                                .when_some(primary_action, |this, action| this.child(action))
                                .child(text_button(
                                    "reminder-banner-dismiss",
                                    app_shared_text(AppTextKey::ReminderPresentationDismissAction),
                                    cx.listener({
                                        let reminder_id = reminder.reminder_id;
                                        move |this, _, _, cx| {
                                            this.dismiss_presented_reminder(reminder_id, cx)
                                        }
                                    }),
                                    cx,
                                )),
                        ),
                ),
        )
        .into_any_element()
    }

    fn render_presented_reminder_primary_action(
        &mut self,
        reminder: &ReminderDeadlineProjection,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let label = reminder.action_label.clone()?;

        match reminder_action_target(reminder) {
            Some(ReminderActionTarget::OrderDetail(order_id)) => Some(
                action_button_primary(
                    "reminder-banner-action",
                    SharedString::from(label),
                    cx.listener({
                        let reminder_id = reminder.reminder_id;
                        move |this, _, _, cx| {
                            this.open_presented_order_reminder(reminder_id, order_id, cx)
                        }
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            Some(ReminderActionTarget::PackDay(fulfillment_window_id)) => Some(
                action_button_primary(
                    "reminder-banner-action",
                    SharedString::from(label),
                    cx.listener({
                        let reminder_id = reminder.reminder_id;
                        move |this, _, _, cx| {
                            this.open_presented_pack_day_reminder(
                                reminder_id,
                                fulfillment_window_id,
                                cx,
                            )
                        }
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            None if reminder.surface == ReminderSurface::Orders => Some(
                action_button_primary(
                    "reminder-banner-action",
                    SharedString::from(label),
                    cx.listener({
                        let reminder_id = reminder.reminder_id;
                        move |this, _, _, cx| this.open_presented_orders_reminder(reminder_id, cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            None => None,
        }
    }

    fn render_pack_day_content(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
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
            .child(pack_day_export_card(
                runtime,
                cx.listener(|this, _, _, cx| this.export_pack_day(cx)),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_host_handoff(
                        PackDayHostHandoffKind::RevealBundle,
                        window,
                        cx,
                    )
                }),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_host_handoff(
                        PackDayHostHandoffKind::OpenPackSheet,
                        window,
                        cx,
                    )
                }),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_host_handoff(
                        PackDayHostHandoffKind::OpenPickupRoster,
                        window,
                        cx,
                    )
                }),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_host_handoff(
                        PackDayHostHandoffKind::OpenCustomerLabels,
                        window,
                        cx,
                    )
                }),
                cx.listener(|this, _, window, cx| this.start_pack_day_batch_print(window, cx)),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_print(PackDayPrintKind::PrintPackSheet, window, cx)
                }),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_print(PackDayPrintKind::PrintPickupRoster, window, cx)
                }),
                cx.listener(|this, _, window, cx| {
                    this.start_pack_day_print(PackDayPrintKind::PrintCustomerLabels, window, cx)
                }),
                cx,
            ))
            .when(!projection.reminders.is_empty(), |this| {
                this.child(self.render_reminder_feed_card(
                    "pack-day-reminders",
                    AppTextKey::PackDayRemindersTitle,
                    &projection.reminders.items,
                    cx,
                ))
            })
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

    fn render_today_reminder_strip(
        &mut self,
        reminders: &[ReminderDeadlineProjection],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        app_surface_card(
            app_stack_v(APP_UI_THEME.foundation.spacing.tight_px)
                .w_full()
                .child(app_text_label(app_shared_text(
                    AppTextKey::HomeTodayRemindersTitle,
                )))
                .child(
                    app_cluster(APP_UI_THEME.foundation.spacing.tight_px)
                        .w_full()
                        .items_start()
                        .children(
                            reminders
                                .iter()
                                .enumerate()
                                .map(|(index, reminder)| {
                                    self.render_today_reminder_chip(index, reminder, cx)
                                })
                                .collect::<Vec<_>>(),
                        ),
                ),
        )
        .into_any_element()
    }

    fn render_today_reminder_chip(
        &mut self,
        index: usize,
        reminder: &ReminderDeadlineProjection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let content = div()
            .w_full()
            .min_w_0()
            .p(px(APP_UI_THEME.shells.home_card_padding_px))
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                            .child(status_indicator(reminder_urgency_color(reminder.urgency)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                    .child(reminder.title.clone()),
                            ),
                    )
                    .child(reminder_urgency_badge(reminder.urgency)),
            )
            .child(
                div()
                    .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                    .child(reminder_deadline_text(reminder)),
            );
        let shell = div().min_w(px(244.0)).max_w(px(296.0)).flex_1();

        match reminder_action_target(reminder) {
            Some(ReminderActionTarget::OrderDetail(order_id)) => shell
                .child(app_button_card(
                    ("today-reminder-chip", index),
                    false,
                    cx.listener(move |this, _, _, cx| this.open_order_detail(order_id, cx)),
                    cx,
                    content,
                ))
                .into_any_element(),
            Some(ReminderActionTarget::PackDay(fulfillment_window_id)) => shell
                .child(app_button_card(
                    ("today-reminder-chip", index),
                    false,
                    cx.listener(move |this, _, _, cx| {
                        this.open_pack_day(Some(fulfillment_window_id), cx)
                    }),
                    cx,
                    content,
                ))
                .into_any_element(),
            None => shell
                .child(
                    div()
                        .w_full()
                        .bg(rgb(APP_UI_THEME.foundation.surfaces.card_background))
                        .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
                        .child(content),
                )
                .into_any_element(),
        }
    }

    fn render_reminder_feed_card(
        &mut self,
        scope: &'static str,
        title_key: AppTextKey,
        reminders: &[ReminderDeadlineProjection],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut rows = Vec::with_capacity(reminders.len().saturating_mul(2));
        for (index, reminder) in reminders.iter().enumerate() {
            rows.push(self.render_reminder_feed_row(scope, index, reminder, cx));
            if index + 1 < reminders.len() {
                rows.push(section_divider().into_any_element());
            }
        }

        home_card(
            app_shared_text(title_key),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.foundation.spacing.medium_px))
                .children(rows),
        )
        .into_any_element()
    }

    fn render_orders_reminder_log_card(&self, reminder_log: &ReminderLogProjection) -> AnyElement {
        let body = if reminder_log.entries.is_empty() {
            home_body_text(app_shared_text(AppTextKey::OrdersReminderLogEmptyBody))
                .into_any_element()
        } else {
            let mut rows = Vec::with_capacity(reminder_log.entries.len().saturating_mul(2));
            for (index, entry) in reminder_log.entries.iter().enumerate() {
                rows.push(self.render_orders_reminder_log_row(entry));
                if index + 1 < reminder_log.entries.len() {
                    rows.push(section_divider().into_any_element());
                }
            }

            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.foundation.spacing.medium_px))
                .children(rows)
                .into_any_element()
        };

        home_card(app_shared_text(AppTextKey::OrdersReminderLogTitle), body).into_any_element()
    }

    fn render_orders_reminder_log_row(&self, entry: &ReminderLogEntryProjection) -> AnyElement {
        app_stack_v(APP_UI_THEME.foundation.spacing.tight_px)
            .w_full()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .line_height(relative(1.2))
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(entry.title.clone()),
                    )
                    .child(reminder_delivery_state_badge(entry.delivery_state)),
            )
            .when_some(
                entry
                    .detail
                    .as_ref()
                    .map(|detail| detail.trim())
                    .filter(|detail| !detail.is_empty()),
                |this, detail| this.child(home_body_text(detail.to_owned())),
            )
            .child(
                div()
                    .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                    .child(entry.recorded_at.clone()),
            )
            .into_any_element()
    }

    fn render_reminder_feed_row(
        &mut self,
        scope: &'static str,
        index: usize,
        reminder: &ReminderDeadlineProjection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let action = self.render_reminder_action(scope, index, reminder, cx);

        app_stack_v(APP_UI_THEME.foundation.spacing.tight_px)
            .w_full()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                            .child(status_indicator(reminder_urgency_color(reminder.urgency)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .line_height(relative(1.2))
                                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                    .child(reminder.title.clone()),
                            ),
                    )
                    .child(reminder_urgency_badge(reminder.urgency)),
            )
            .child(home_body_text(reminder.detail.clone()))
            .child(
                div()
                    .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                    .child(reminder_deadline_text(reminder)),
            )
            .when_some(action, |this, action| this.child(div().child(action)))
            .into_any_element()
    }

    fn render_reminder_action(
        &mut self,
        scope: &'static str,
        index: usize,
        reminder: &ReminderDeadlineProjection,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let label = reminder.action_label.clone()?;

        match reminder_action_target(reminder) {
            Some(ReminderActionTarget::OrderDetail(order_id)) => Some(
                action_button_compact(
                    (scope, index),
                    SharedString::from(label),
                    cx.listener(move |this, _, _, cx| this.open_order_detail(order_id, cx)),
                    cx,
                )
                .into_any_element(),
            ),
            Some(ReminderActionTarget::PackDay(fulfillment_window_id)) => Some(
                action_button_compact(
                    (scope, index),
                    SharedString::from(label),
                    cx.listener(move |this, _, _, cx| {
                        this.open_pack_day(Some(fulfillment_window_id), cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            None => None,
        }
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
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let fulfillment_actions = (!detail.fulfillment_actions.is_empty()).then(|| {
            app_form_section(
                app_shared_text(AppTextKey::TradeWorkflowAxisFulfillment),
                app_cluster(APP_UI_THEME.foundation.spacing.tight_px).children(
                    detail
                        .fulfillment_actions
                        .iter()
                        .copied()
                        .map(|action| {
                            action_button_compact(
                                order_detail_fulfillment_action_id(action),
                                app_shared_text(order_fulfillment_action_label_key(action)),
                                cx.listener({
                                    let order_id = detail.order_id;
                                    move |this, _, _, cx| {
                                        this.publish_order_fulfillment_update(order_id, action, cx)
                                    }
                                }),
                                cx,
                            )
                            .into_any_element()
                        })
                        .collect::<Vec<_>>(),
                ),
            )
        });

        app_focused_detail_view(
            app_shared_text(AppTextKey::OrdersDetailTitle),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .w_full()
                .child(app_heading_section(detail.order_number.clone()))
                .child(home_body_text(detail.customer_display_name.clone()))
                .child(trade_workflow_detail_badge_strip(&detail.workflow))
                .child(label_value_list([
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailCustomerLabel),
                        detail.customer_display_name.clone(),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailWindowLabel),
                        order_optional_text(detail.fulfillment_window_label.as_deref()),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailPickupLabel),
                        order_optional_text(detail.pickup_location_label.as_deref()),
                    ),
                    LabelValueRow::new(
                        app_shared_text(AppTextKey::OrdersDetailTotalLabel),
                        trade_economics_total_text(&detail.workflow.economics),
                    ),
                ]))
                .when(!detail.validation_receipts.is_empty(), |this| {
                    this.child(validation_receipts_summary_section(
                        &detail.validation_receipts,
                    ))
                })
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
                .child(self.render_order_recovery_section(detail, cx))
                .when_some(fulfillment_actions, |this, fulfillment_actions| {
                    this.child(fulfillment_actions)
                }),
            text_button(
                "orders-detail-back",
                app_shared_text(AppTextKey::PersonalDetailBackAction),
                on_close,
                cx,
            ),
        )
    }

    fn render_order_recovery_section(
        &mut self,
        detail: &OrderDetailProjection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        app_form_section(
            app_shared_text(AppTextKey::OrdersRecoverySectionTitle),
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.foundation.spacing.medium_px))
                .child(
                    self.render_order_recovery_card(
                        detail.order_id,
                        RecoveryKind::MissedPickup,
                        detail
                            .recoveries
                            .iter()
                            .find(|record| record.kind == RecoveryKind::MissedPickup),
                        cx,
                    ),
                )
                .child(
                    self.render_order_recovery_card(
                        detail.order_id,
                        RecoveryKind::RefundFollowUp,
                        detail
                            .recoveries
                            .iter()
                            .find(|record| record.kind == RecoveryKind::RefundFollowUp),
                        cx,
                    ),
                ),
        )
        .into_any_element()
    }

    fn render_order_recovery_card(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        recovery: Option<&OrderRecoveryProjection>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title_key = order_recovery_title_key(kind);
        let body = recovery.map_or_else(
            || {
                home_body_text(app_shared_text(order_recovery_empty_body_key(kind)))
                    .into_any_element()
            },
            |record| {
                app_stack_v(APP_UI_THEME.foundation.spacing.tight_px)
                    .w_full()
                    .child(home_body_text(record.summary.clone()))
                    .when_some(
                        record
                            .note
                            .as_ref()
                            .map(|note| note.trim())
                            .filter(|note| !note.is_empty()),
                        |this, note| this.child(home_body_text(note.to_owned())),
                    )
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                            .child(format!(
                                "{}: {}",
                                app_text(AppTextKey::OrdersRecoveryLastUpdatedLabel),
                                record.last_updated_at
                            )),
                    )
                    .into_any_element()
            },
        );

        app_surface_card(
            app_stack_v(APP_UI_THEME.foundation.spacing.medium_px)
                .w_full()
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .items_start()
                        .justify_between()
                        .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .line_height(relative(1.2))
                                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                .child(app_shared_text(title_key)),
                        )
                        .when_some(
                            recovery.map(|record| order_recovery_state_badge(record.state)),
                            |this, badge| this.child(badge),
                        ),
                )
                .child(body)
                .when_some(
                    self.render_order_recovery_actions(order_id, kind, recovery, cx),
                    |this, actions| {
                        this.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                                .child(actions),
                        )
                    },
                ),
        )
        .into_any_element()
    }

    fn render_order_recovery_actions(
        &mut self,
        order_id: OrderId,
        kind: RecoveryKind,
        recovery: Option<&OrderRecoveryProjection>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let index = order_recovery_kind_index(kind);

        match recovery.map(|record| record.state) {
            None => Some(
                action_button_primary(
                    ("orders-recovery-open", index),
                    app_shared_text(AppTextKey::OrdersRecoveryActionOpenFollowUp),
                    cx.listener(move |this, _, _, cx| {
                        this.start_order_recovery(order_id, kind, cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
            Some(RecoveryState::Open) => Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .child(action_button_compact(
                        ("orders-recovery-review", index),
                        app_shared_text(AppTextKey::OrdersRecoveryActionStartReview),
                        cx.listener(move |this, _, _, cx| {
                            this.review_order_recovery(order_id, kind, cx)
                        }),
                        cx,
                    ))
                    .child(action_button_primary(
                        ("orders-recovery-resolve", index),
                        app_shared_text(AppTextKey::OrdersRecoveryActionResolve),
                        cx.listener(move |this, _, _, cx| {
                            this.resolve_order_recovery(order_id, kind, cx)
                        }),
                        cx,
                    ))
                    .into_any_element(),
            ),
            Some(RecoveryState::InReview) => Some(
                div()
                    .flex()
                    .items_center()
                    .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
                    .child(action_button_compact(
                        ("orders-recovery-reopen", index),
                        app_shared_text(AppTextKey::OrdersRecoveryActionMarkOpen),
                        cx.listener(move |this, _, _, cx| {
                            this.reopen_order_recovery(order_id, kind, cx)
                        }),
                        cx,
                    ))
                    .child(action_button_primary(
                        ("orders-recovery-resolve", index),
                        app_shared_text(AppTextKey::OrdersRecoveryActionResolve),
                        cx.listener(move |this, _, _, cx| {
                            this.resolve_order_recovery(order_id, kind, cx)
                        }),
                        cx,
                    ))
                    .into_any_element(),
            ),
            Some(RecoveryState::Resolved) => Some(
                action_button_compact(
                    ("orders-recovery-reopen", index),
                    app_shared_text(AppTextKey::OrdersRecoveryActionMarkOpen),
                    cx.listener(move |this, _, _, cx| {
                        this.reopen_order_recovery(order_id, kind, cx)
                    }),
                    cx,
                )
                .into_any_element(),
            ),
        }
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
                let action = row
                    .primary_action
                    .and_then(OrderPrimaryAction::fulfillment_action);
                move |this, _, _, cx| {
                    if let Some(action) = action {
                        this.publish_order_fulfillment_update(order_id, action, cx);
                    }
                }
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
        self.sync_buyer_order_review_form(&runtime_summary, window, cx);
        self.sync_buyer_receipt_issue_form(&runtime_summary);
        self.sync_products_search(&runtime_summary, window, cx);
        self.sync_products_stock_editor(&runtime_summary);
        self.sync_product_editor_form(&runtime_summary, window, cx);
        self.apply_auto_focus(&runtime_summary, window, cx);
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
            HomeStage::AccountWorkspace => self.render_account_workspace(&runtime_summary, cx),
            HomeStage::BuyerWorkspace => self.render_buyer_workspace(&runtime_summary, cx),
            HomeStage::FarmerWorkspace => self.render_farmer_workspace(&runtime_summary, cx),
        }
    }
}

impl HomeView {
    fn render_account_workspace(
        &mut self,
        runtime: &DesktopAppRuntimeSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sidebar = if runtime.shell_projection.active_surface == ActiveSurface::Farmer {
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
            .into_any_element()
        } else {
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
            .into_any_element()
        };

        app_split_shell(
            sidebar,
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .size_full()
                .child(shared_shell_header(
                    runtime,
                    cx.listener(|this, _, _, cx| this.switch_to_marketplace(cx)),
                    cx.listener(|this, _, _, cx| this.switch_to_farmer_workspace(cx)),
                    cx.listener(|this, _, _, cx| this.open_account_entry(cx)),
                    cx,
                ))
                .child(app_scroll_panel(
                    "account-scroll",
                    0.0,
                    None,
                    self.render_account_content(cx),
                ))
                .into_any_element(),
        )
        .into_any_element()
    }

    fn render_account_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let selected_tab = self.selected_account_tab;
        let tabs = AccountTab::ORDERED
            .into_iter()
            .map(|tab| AppUnderlineTabSpec::new(app_shared_text(tab.text_key())));

        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .max_w(px(APP_UI_THEME.shells.home_card_max_width_px))
            .mx_auto()
            .child(app_text_value(app_shared_text(AppTextKey::AccountTitle)))
            .child(
                app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                    .w_full()
                    .child(app_underline_tabs(
                        "account-tabs",
                        tabs,
                        selected_tab.selected_index(),
                        cx.listener(|this, index: &usize, _, cx| {
                            this.select_account_tab(AccountTab::from_index(*index), cx)
                        }),
                    ))
                    .child(account_placeholder_panel(selected_tab.panel_text_key())),
            )
            .into_any_element()
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

struct BuyerOrderReviewFormState {
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

impl BuyerOrderReviewFormState {
    fn new(
        workspace_id: String,
        draft: &BuyerOrderReviewDraft,
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
            HomeView::handle_buyer_order_review_input_event,
        );
        let email_subscription = cx.subscribe_in(
            &email_input,
            window,
            HomeView::handle_buyer_order_review_input_event,
        );
        let phone_subscription = cx.subscribe_in(
            &phone_input,
            window,
            HomeView::handle_buyer_order_review_input_event,
        );
        let order_note_subscription = cx.subscribe_in(
            &order_note_input,
            window,
            HomeView::handle_buyer_order_review_input_event,
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
        draft: &BuyerOrderReviewDraft,
        window: &mut Window,
        cx: &mut Context<HomeView>,
    ) {
        sync_order_review_input(&self.name_input, draft.name.as_str(), window, cx);
        sync_order_review_input(&self.email_input, draft.email.as_str(), window, cx);
        sync_order_review_input(&self.phone_input, draft.phone.as_str(), window, cx);
        sync_order_review_input(
            &self.order_note_input,
            draft.order_note.as_str(),
            window,
            cx,
        );
    }

    fn current_draft(&self, cx: &App) -> BuyerOrderReviewDraft {
        BuyerOrderReviewDraft {
            name: self.name_input.read(cx).value().to_string(),
            email: self.email_input.read(cx).value().to_string(),
            phone: self.phone_input.read(cx).value().to_string(),
            order_note: self.order_note_input.read(cx).value().to_string(),
        }
    }
}

fn sync_order_review_input(
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

struct BuyerReceiptIssueFormState {
    order_id: OrderId,
    issue_input: Entity<InputState>,
    _issue_subscription: Subscription,
}

impl BuyerReceiptIssueFormState {
    fn new(order_id: OrderId, window: &mut Window, cx: &mut Context<HomeView>) -> Self {
        let issue_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(app_shared_text(
                    AppTextKey::PersonalOrdersReceiptIssuePlaceholder,
                ))
                .default_value(String::new())
        });
        let issue_subscription = cx.subscribe_in(
            &issue_input,
            window,
            HomeView::handle_buyer_receipt_issue_input_event,
        );

        Self {
            order_id,
            issue_input,
            _issue_subscription: issue_subscription,
        }
    }

    fn issue_text(&self, cx: &App) -> String {
        self.issue_input.read(cx).value().trim().to_owned()
    }

    fn can_submit(&self, cx: &App) -> bool {
        !self.issue_text(cx).is_empty()
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
    selected_availability_window_id: Option<FulfillmentWindowId>,
    title_input: Entity<InputState>,
    subtitle_input: Entity<InputState>,
    category_input: Entity<InputState>,
    unit_input: Entity<InputState>,
    price_input: Entity<InputState>,
    stock_input: Entity<InputState>,
    _title_subscription: Subscription,
    _subtitle_subscription: Subscription,
    _category_subscription: Subscription,
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
        let selected_availability_window_id = draft.availability_window_id;
        let title_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.title.clone()));
        let subtitle_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.subtitle.clone()));
        let category_input =
            cx.new(|cx| InputState::new(window, cx).default_value(draft.category.clone()));
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
        let category_subscription = cx.subscribe_in(
            &category_input,
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
            selected_availability_window_id,
            initial_draft: draft,
            title_input,
            subtitle_input,
            category_input,
            unit_input,
            price_input,
            stock_input,
            _title_subscription: title_subscription,
            _subtitle_subscription: subtitle_subscription,
            _category_subscription: category_subscription,
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
            category: self.category_input.read(cx).value().to_string(),
            unit_label: self.unit_input.read(cx).value().to_string(),
            price_minor_units: parse_product_editor_price_input(
                self.price_input.read(cx).value().as_ref(),
            )?,
            price_currency: "USD".to_owned(),
            stock_quantity: parse_optional_product_editor_stock_input(
                self.stock_input.read(cx).value().as_ref(),
            )?,
            availability_window_id: self.selected_availability_window_id,
            status: self.status,
        })
    }

    fn has_changes(&self, cx: &App) -> bool {
        self.current_draft(cx)
            .map(|draft| draft != self.initial_draft)
            .unwrap_or(false)
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
    about_panel_notice: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsAutoFocusTarget {
    Navigation(SettingsPanelViewKey),
    AccountAdd,
    FarmNameInput,
    AboutRefresh,
}

fn settings_preferences_general_row_state(
    runtime: &DesktopAppRuntimeSummary,
) -> SettingsPreferencesGeneralRowState {
    let general = &runtime.shell_projection.settings.general;
    SettingsPreferencesGeneralRowState {
        allow_relay_connections: general.allow_relay_connections,
        use_media_servers: general.use_media_servers,
        use_nip05: general.use_nip05,
        launch_at_login: general.launch_at_login,
    }
}

impl SettingsWindowView {
    pub fn new(runtime: DesktopAppRuntime, initial_view: SettingsPanelViewKey) -> Self {
        let _ = initial_view;
        Self {
            runtime,
            farm_panel_state: None,
            farm_panel_error: None,
            about_panel_notice: None,
        }
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        self.about_panel_notice = None;
        if self.runtime.select_settings_section(view) {
            cx.notify();
        }
    }

    fn selected_view(&self) -> SettingsPanelViewKey {
        self.runtime.selected_settings_section()
    }

    fn select_account(&mut self, account_id: String, cx: &mut Context<Self>) {
        match self.runtime.select_local_account(account_id.as_str()) {
            Ok(changed) => {
                if changed {
                    cx.refresh_windows();
                }
                cx.notify();
            }
            Err(runtime_error) => {
                error!(
                    target: "settings",
                    event = "settings.account.select_failed",
                    error = %runtime_error,
                    "failed to select account from settings panel"
                );
            }
        }
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

    fn refresh_about_sync(&mut self, cx: &mut Context<Self>) {
        match self.runtime.sync_on_manual_refresh() {
            Ok(changed) => {
                if changed {
                    self.about_panel_notice = None;
                    cx.refresh_windows();
                } else {
                    self.about_panel_notice = Some(app_text(about_conflict_review_body_key(
                        &self.runtime.summary().sync_status,
                    )));
                }
                cx.notify();
            }
            Err(runtime_error) => {
                error!(
                    target: "settings",
                    event = "settings.about.sync_refresh_failed",
                    error = %runtime_error,
                    "failed to refresh sync from the about panel"
                );
                self.about_panel_notice = Some(runtime_error.to_string());
                cx.notify();
            }
        }
    }

    fn resolve_about_conflict(
        &mut self,
        conflict_id: String,
        resolution: SyncConflictResolutionStatus,
        cx: &mut Context<Self>,
    ) {
        match self
            .runtime
            .resolve_sync_conflict(conflict_id.as_str(), resolution)
        {
            Ok(changed) => {
                if changed {
                    self.about_panel_notice = None;
                    cx.refresh_windows();
                } else {
                    self.about_panel_notice = Some(app_text(about_conflict_review_body_key(
                        &self.runtime.summary().sync_status,
                    )));
                }
                cx.notify();
            }
            Err(runtime_error) => {
                error!(
                    target: "settings",
                    event = "settings.about.conflict_resolution_failed",
                    conflict_id = %conflict_id,
                    error = %runtime_error,
                    "failed to resolve sync conflict from the about panel"
                );
                self.about_panel_notice = Some(runtime_error.to_string());
                cx.notify();
            }
        }
    }

    fn about_conflict_card(
        &mut self,
        conflict_index: usize,
        conflict: &DesktopAppSyncConflictSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let action_specs = about_conflict_action_specs(&conflict.conflict);

        app_surface_panel(
            app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                .w_full()
                .p(px(APP_UI_THEME.shells.home_card_padding_px))
                .child(app_text_value(about_conflict_aggregate_text(
                    &conflict.conflict,
                )))
                .child(label_value_list(about_conflict_detail_rows(conflict)))
                .when(!action_specs.is_empty(), |this| {
                    this.child(
                        app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                            .w_full()
                            .children(
                                action_specs
                                    .into_iter()
                                    .enumerate()
                                    .map(|(action_index, (key, resolution))| {
                                        action_button_compact(
                                            (
                                                gpui::ElementId::from((
                                                    "settings-about-conflict-action",
                                                    conflict_index,
                                                )),
                                                action_index.to_string(),
                                            ),
                                            app_shared_text(key),
                                            cx.listener({
                                                let conflict_id = conflict.conflict_id.clone();
                                                move |this, _, _, cx| {
                                                    this.resolve_about_conflict(
                                                        conflict_id.clone(),
                                                        resolution,
                                                        cx,
                                                    )
                                                }
                                            }),
                                            cx,
                                        )
                                        .into_any_element()
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                    )
                }),
        )
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
        let runtime = self.runtime.summary();
        let projection = &runtime.settings_account_projection;
        let detail_text_px = APP_UI_THEME
            .foundation
            .typography
            .settings_account_detail_text_px;
        let detail_account = settings_account_detail_account(projection);
        let selected_account_id = projection
            .selected_account
            .as_ref()
            .map(|account| account.account.account_id.as_str());
        let account_rows = projection
            .roster
            .iter()
            .enumerate()
            .map(|(index, account)| {
                let account_id = account.account_id.clone();
                let is_selected = selected_account_id
                    .is_some_and(|selected_account_id| selected_account_id == account.account_id);

                account_selector_row(
                    ("settings-account-row", index),
                    account_display_name(account),
                    SharedString::from(abbreviated_npub(account.npub.as_str())),
                    is_selected,
                    cx.listener(move |this, _, _, cx| this.select_account(account_id.clone(), cx)),
                    cx,
                )
                .into_any_element()
            })
            .collect::<Vec<_>>();

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
                        app_stack_v(APP_UI_THEME.foundation.spacing.tight_px)
                            .w_full()
                            .rounded(px(APP_UI_THEME
                                .shells
                                .settings_account_sidebar_button_corner_radius_px))
                            .children(account_rows)
                            .when(projection.roster.is_empty(), |this| {
                                this.child(
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
                                                .text_color(rgb(
                                                    APP_UI_THEME.foundation.text.primary,
                                                ))
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
                                                .text_color(rgb(
                                                    APP_UI_THEME.foundation.text.secondary,
                                                ))
                                                .line_height(relative(1.2))
                                                .child(app_shared_text(
                                                    AppTextKey::SettingsAccountNoSelectionBody,
                                                )),
                                        ),
                                )
                            }),
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
                                        cx.listener(|_, _, _, _| {}),
                                        cx,
                                    ))
                                    .child(settings_account_more_actions_button(cx)),
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
                                            .child(
                                                detail_account
                                                    .map(account_display_name)
                                                    .unwrap_or_else(|| {
                                                        app_shared_text(
                                                            AppTextKey::SettingsAccountNoSelectionTitle,
                                                        )
                                                        .to_string()
                                                    }),
                                            ),
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
                                            .child(
                                                detail_account
                                                    .map(account_display_name)
                                                    .unwrap_or_else(|| {
                                                        app_shared_text(AppTextKey::ValueNone)
                                                            .to_string()
                                                    }),
                                            ),
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
                                            .child(status_indicator(settings_account_status_color(
                                                detail_account,
                                                selected_account_id,
                                            )))
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .text_color(rgb(APP_UI_THEME
                                                        .foundation
                                                        .text
                                                        .primary))
                                                    .child(app_shared_text(
                                                        settings_account_status_key(
                                                            detail_account,
                                                            selected_account_id,
                                                        ),
                                                    )),
                                            ),
                                    ))
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
                                                cx.listener(|_, _, _, _| {}),
                                                cx,
                                            )))
                                            .child(div().child(action_button(
                                                "account-open-workspace",
                                                app_shared_text(
                                                    AppTextKey::SettingsAccountOpenWorkspaceAction,
                                                ),
                                                cx.listener(|_, _, _, _| {}),
                                                cx,
                                            ))),
                                    )
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountCustodyLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(
                                                detail_account
                                                    .map(|account| {
                                                        account_custody_key(account.custody)
                                                    })
                                                    .unwrap_or(AppTextKey::ValueNone),
                                            )),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountSurfaceLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(settings_account_surface_key(
                                                projection,
                                                detail_account,
                                            ))),
                                    ))
                                    .child(app_detail_row(
                                        app_shared_label_text(
                                            AppTextKey::SettingsAccountActivationLabel,
                                        ),
                                        div()
                                            .text_size(px(detail_text_px))
                                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                            .child(app_shared_text(
                                                settings_account_activation_key(
                                                    projection,
                                                    detail_account,
                                                ),
                                            )),
                                    ))
                                    .when(detail_account.is_none(), |this| {
                                        this.child(home_body_text(app_shared_text(
                                            AppTextKey::SettingsAccountNoSelectionBody,
                                        )))
                                    }),
                            ),
                    ),
            )
    }

    fn settings_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_farm_panel_state(window, cx);
        let runtime = self.runtime.summary();

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
                app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .child(label_value_list(settings_preferences_general_rows(
                        settings_preferences_general_row_state(&runtime),
                    ))),
            )
            .into_any_element(),
        );

        app_scroll_panel(
            "settings-panel-scroll",
            APP_UI_THEME.shells.settings_content_padding_px,
            Some(APP_UI_THEME.shells.settings_panel_content_max_width_px),
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
                Some(APP_UI_THEME.shells.settings_panel_content_max_width_px),
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
                Some(APP_UI_THEME.shells.settings_panel_content_max_width_px),
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
            Some(APP_UI_THEME.shells.settings_panel_content_max_width_px),
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .w_full()
                .child(home_body_text(app_shared_text(
                    AppTextKey::SettingsFarmPanelBody,
                )))
                .children(cards),
        )
    }

    fn about_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime = self.runtime.summary();
        let status_rows = about_status_rows(&runtime);
        let runtime_rows = about_runtime_rows(&runtime);
        let manual_refresh_enabled = about_manual_refresh_enabled(&runtime.sync_status);
        let conflict_cards = runtime
            .sync_status
            .conflicts
            .iter()
            .enumerate()
            .map(|(conflict_index, conflict)| {
                self.about_conflict_card(conflict_index, conflict, cx)
                    .into_any_element()
            })
            .collect::<Vec<_>>();

        app_scroll_panel(
            "settings-panel-scroll",
            APP_UI_THEME.shells.settings_content_padding_px,
            None,
            app_stack_v(APP_UI_THEME.shells.settings_account_main_stack_gap_px)
                .size_full()
                .py_12()
                .child(settings_about_product_section(cx))
                .child(app_surface_card(
                    app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                        .w_full()
                        .child(app_heading_section(app_shared_text(
                            AppTextKey::SettingsAboutStatusSectionLabel,
                        )))
                        .child(label_value_list(status_rows))
                        .child(if manual_refresh_enabled {
                            action_button_primary(
                                "settings-about-refresh-sync",
                                app_shared_text(AppTextKey::SettingsAboutRefreshAction),
                                cx.listener(|this, _, _, cx| this.refresh_about_sync(cx)),
                                cx,
                            )
                            .into_any_element()
                        } else {
                            action_button_primary_disabled(
                                "settings-about-refresh-sync-disabled",
                                app_shared_text(AppTextKey::SettingsAboutRefreshAction),
                                cx,
                            )
                            .into_any_element()
                        }),
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
                        .when_some(self.about_panel_notice.as_deref(), |this, notice| {
                            this.child(home_body_text(notice.to_owned()))
                        })
                        .children(conflict_cards),
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
            SettingsPanelViewKey::About => self.about_panel(cx).into_any_element(),
        }
    }

    fn apply_auto_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let runtime = self.runtime.summary();
        let desired_target = settings_auto_focus_target(
            self.selected_view(),
            self.farm_panel_state.as_ref(),
            &runtime,
        );
        let focus_state = window.use_state(cx, |_, _| Option::<SettingsAutoFocusTarget>::None);
        let should_focus = {
            let last_target = focus_state.read(cx);
            last_target.as_ref().copied() != desired_target
        };

        if !should_focus {
            return;
        }

        if let Some(target) = desired_target {
            match target {
                SettingsAutoFocusTarget::Navigation(view) => {
                    let (navigation_id, _) = settings_panel_spec(view);
                    focus_button(window, navigation_id, cx);
                }
                SettingsAutoFocusTarget::AccountAdd => {
                    focus_button(window, "account-add", cx);
                }
                SettingsAutoFocusTarget::FarmNameInput => {
                    if let Some(form) = self.farm_panel_state.as_ref() {
                        form.farm_name_input
                            .update(cx, |input, cx| input.focus(window, cx));
                    }
                }
                SettingsAutoFocusTarget::AboutRefresh => {
                    focus_button(window, "settings-about-refresh-sync", cx);
                }
            }
        }

        focus_state.update(cx, |last_target, _| *last_target = desired_target);
    }
}

fn settings_account_more_actions_button(cx: &App) -> impl IntoElement {
    action_dropdown_button(
        "account-more",
        |menu, _, _| {
            menu.item(
                PopupMenuItem::new(app_text(AppTextKey::SettingsAccountImportFileAction))
                    .on_click(|_, _, _| {}),
            )
            .item(
                PopupMenuItem::new(app_text(AppTextKey::SettingsAccountImportDatabaseAction))
                    .on_click(|_, _, _| {}),
            )
            .item(
                PopupMenuItem::new(app_text(
                    AppTextKey::SettingsAccountConnectRemoteBunkerAction,
                ))
                .on_click(|_, _, _| {}),
            )
        },
        cx,
    )
}

fn settings_about_product_section(cx: &mut Context<SettingsWindowView>) -> impl IntoElement {
    let app_icon = Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../../../platforms/macos/App/Resources/AppIconSource.png").to_vec(),
    ));
    let version = format!(
        "{} {}",
        app_text(AppTextKey::SettingsAboutVersionLabel),
        env!("CARGO_PKG_VERSION")
    );

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .justify_center()
                .gap(px(APP_UI_THEME.shells.settings_account_main_padding_px))
                .child(
                    img(app_icon)
                        .w(px(128.0))
                        .h(px(128.0))
                        .object_fit(ObjectFit::Contain)
                        .flex_shrink_0(),
                )
                .child(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .min_w_0()
                        .child(
                            div()
                                .text_size(
                                    px(APP_UI_THEME.foundation.typography.body_text_px * 1.7),
                                )
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                .child(app_shared_text(AppTextKey::AppName)),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(version),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(app_shared_text(AppTextKey::SettingsAboutVariantLabel)),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(app_shared_text(AppTextKey::SettingsAboutCompanyName)),
                        )
                        .child(text_button(
                            "settings-about-acknowledgements",
                            app_shared_text(AppTextKey::SettingsAboutAcknowledgementsAction),
                            cx.listener(|_, _, _, _| {}),
                            cx,
                        ))
                        .child(text_button(
                            "settings-about-privacy-policy",
                            app_shared_text(AppTextKey::SettingsAboutPrivacyPolicyAction),
                            cx.listener(|_, _, _, _| {}),
                            cx,
                        ))
                        .child(text_button(
                            "settings-about-terms",
                            app_shared_text(AppTextKey::SettingsAboutTermsAction),
                            cx.listener(|_, _, _, _| {}),
                            cx,
                        ))
                        .child(action_button(
                            "settings-about-report-issue",
                            app_shared_text(AppTextKey::SettingsAboutReportIssueAction),
                            cx.listener(|_, _, _, _| {}),
                            cx,
                        )),
                ),
        )
        .child(
            app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                .items_center()
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                        .child(app_shared_text(AppTextKey::SettingsAboutCopyrightNotice)),
                )
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                        .child(app_shared_text(AppTextKey::SettingsAboutTrademarkNotice)),
                ),
        )
}

fn settings_account_detail_account(
    projection: &SettingsAccountProjection,
) -> Option<&AccountSummary> {
    projection
        .selected_account
        .as_ref()
        .map(|selected_account| &selected_account.account)
        .or_else(|| projection.roster.first())
}

fn account_display_name(account: &AccountSummary) -> String {
    account
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| abbreviated_npub(account.npub.as_str()))
}

fn abbreviated_npub(npub: &str) -> String {
    let trimmed = npub.trim();
    if trimmed.chars().count() <= 20 {
        return trimmed.to_owned();
    }

    let prefix = trimmed.chars().take(10).collect::<String>();
    let suffix = trimmed
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn settings_account_status_color(
    account: Option<&AccountSummary>,
    selected_account_id: Option<&str>,
) -> u32 {
    if settings_account_is_selected(account, selected_account_id) {
        APP_UI_THEME.components.app_status_indicator.online
    } else {
        APP_UI_THEME.components.app_status_indicator.offline
    }
}

fn settings_account_status_key(
    account: Option<&AccountSummary>,
    selected_account_id: Option<&str>,
) -> AppTextKey {
    if settings_account_is_selected(account, selected_account_id) {
        AppTextKey::SettingsAccountStatusLoggedIn
    } else {
        AppTextKey::SettingsAccountStatusLoggedOut
    }
}

fn settings_account_is_selected(
    account: Option<&AccountSummary>,
    selected_account_id: Option<&str>,
) -> bool {
    account
        .zip(selected_account_id)
        .is_some_and(|(account, selected_account_id)| account.account_id == selected_account_id)
}

fn account_custody_key(custody: AccountCustody) -> AppTextKey {
    match custody {
        AccountCustody::LocalManaged => AppTextKey::SettingsAccountCustodyLocalManaged,
        AccountCustody::BrowserSigner => AppTextKey::SettingsAccountCustodyBrowserSigner,
        AccountCustody::RemoteSigner => AppTextKey::SettingsAccountCustodyRemoteSigner,
    }
}

fn settings_account_surface_key(
    projection: &SettingsAccountProjection,
    account: Option<&AccountSummary>,
) -> AppTextKey {
    projection
        .selected_account
        .as_ref()
        .filter(|selected_account| {
            account.is_some_and(|account| account.account_id == selected_account.account.account_id)
        })
        .map(|selected_account| active_surface_settings_key(selected_account.active_surface()))
        .unwrap_or(AppTextKey::ValueNone)
}

fn active_surface_settings_key(surface: ActiveSurface) -> AppTextKey {
    match surface {
        ActiveSurface::Personal => AppTextKey::SettingsAccountSurfacePersonal,
        ActiveSurface::Farmer => AppTextKey::SettingsAccountSurfaceFarmer,
    }
}

fn settings_account_activation_key(
    projection: &SettingsAccountProjection,
    account: Option<&AccountSummary>,
) -> AppTextKey {
    if projection
        .selected_account
        .as_ref()
        .filter(|selected_account| {
            account.is_some_and(|account| account.account_id == selected_account.account.account_id)
        })
        .is_some_and(|selected_account| selected_account.farmer_activation.is_active())
    {
        AppTextKey::SettingsAccountActivationActive
    } else {
        AppTextKey::SettingsAccountActivationInactive
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
            .as_deref()
            .map(startup_issue_summary_text)
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

fn about_manual_refresh_enabled(sync_status: &DesktopAppSyncStatusSummary) -> bool {
    sync_status.is_enabled()
        && !sync_status
            .projection
            .conflict_status
            .has_blocking_conflicts()
}

fn about_conflict_action_specs(
    conflict: &SyncConflict,
) -> Vec<(AppTextKey, SyncConflictResolutionStatus)> {
    if !conflict.is_unresolved() {
        return Vec::new();
    }

    let mut actions = vec![
        (
            AppTextKey::SettingsAboutConflictAcceptLocalAction,
            SyncConflictResolutionStatus::AcceptedLocal,
        ),
        (
            AppTextKey::SettingsAboutConflictAcceptRemoteAction,
            SyncConflictResolutionStatus::AcceptedRemote,
        ),
    ];
    if !matches!(
        conflict.severity,
        radroots_studio_app_sync::SyncConflictSeverity::Blocking
    ) {
        actions.push((
            AppTextKey::SettingsAboutConflictDismissAction,
            SyncConflictResolutionStatus::Dismissed,
        ));
    }

    actions
}

fn about_conflict_detail_rows(conflict: &DesktopAppSyncConflictSummary) -> Vec<LabelValueRow> {
    vec![
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictAggregate),
            about_conflict_aggregate_text(&conflict.conflict),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictKind),
            about_conflict_kind_text(&conflict.conflict),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictSeverity),
            about_conflict_severity_text(&conflict.conflict),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictDetectedAt),
            conflict.conflict.detected_at.clone(),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::MetadataSyncConflictResolution),
            about_conflict_resolution_text(&conflict.conflict),
        ),
    ]
}

fn about_conflict_aggregate_text(conflict: &SyncConflict) -> String {
    let (aggregate_kind_key, aggregate_id) = match &conflict.aggregate {
        SyncAggregateRef::Farm(farm_id) => (
            AppTextKey::ValueSyncConflictAggregateFarm,
            farm_id.to_string(),
        ),
        SyncAggregateRef::FulfillmentWindow(fulfillment_window_id) => (
            AppTextKey::ValueSyncConflictAggregateFulfillmentWindow,
            fulfillment_window_id.to_string(),
        ),
        SyncAggregateRef::Product(product_id) => (
            AppTextKey::ValueSyncConflictAggregateProduct,
            product_id.to_string(),
        ),
        SyncAggregateRef::Order(order_id) => (
            AppTextKey::ValueSyncConflictAggregateOrder,
            order_id.to_string(),
        ),
    };

    format!("{}: {}", app_text(aggregate_kind_key), aggregate_id)
}

fn about_conflict_kind_text(conflict: &SyncConflict) -> String {
    app_text(match conflict.kind {
        SyncConflictKind::RevisionMismatch => AppTextKey::ValueSyncConflictKindRevisionMismatch,
        SyncConflictKind::RemoteDelete => AppTextKey::ValueSyncConflictKindRemoteDelete,
        SyncConflictKind::RemoteValidationReject => {
            AppTextKey::ValueSyncConflictKindRemoteValidationReject
        }
    })
}

fn about_conflict_severity_text(conflict: &SyncConflict) -> String {
    match conflict.severity {
        SyncConflictSeverity::ReviewRequired => {
            app_text(AppTextKey::ValueSyncConflictSeverityReviewRequired)
        }
        SyncConflictSeverity::Blocking => app_text(AppTextKey::ValueSyncConflictSeverityBlocking),
    }
}

fn about_conflict_resolution_text(conflict: &SyncConflict) -> String {
    match conflict.resolution {
        SyncConflictResolutionStatus::Unresolved => {
            app_text(AppTextKey::ValueSyncConflictResolutionUnresolved)
        }
        SyncConflictResolutionStatus::AcceptedLocal => {
            app_text(AppTextKey::ValueSyncConflictResolutionAcceptedLocal)
        }
        SyncConflictResolutionStatus::AcceptedRemote => {
            app_text(AppTextKey::ValueSyncConflictResolutionAcceptedRemote)
        }
        SyncConflictResolutionStatus::Dismissed => {
            app_text(AppTextKey::ValueSyncConflictResolutionDismissed)
        }
    }
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

fn focus_button<V>(window: &mut Window, id: impl Into<ElementId>, cx: &mut Context<V>) {
    let focus_handle = window
        .use_keyed_state(id, cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    focus_handle.focus(window);
}

fn home_auto_focus_target(
    runtime: &DesktopAppRuntimeSummary,
    state: HomeAutoFocusState,
) -> Option<HomeAutoFocusTarget> {
    match home_stage(runtime) {
        HomeStage::Setup => startup_auto_focus_target(runtime, state),
        HomeStage::AccountWorkspace => None,
        HomeStage::BuyerWorkspace => buyer_auto_focus_target(runtime, state),
        HomeStage::FarmerWorkspace => farmer_auto_focus_target(runtime, state),
    }
}

fn startup_auto_focus_target(
    runtime: &DesktopAppRuntimeSummary,
    state: HomeAutoFocusState,
) -> Option<HomeAutoFocusTarget> {
    match startup_home_surface(runtime) {
        StartupHomeSurface::ContinuePrompt => Some(HomeAutoFocusTarget::StartupContinue),
        StartupHomeSurface::IdentityChoice => Some(HomeAutoFocusTarget::StartupGenerateKey),
        StartupHomeSurface::GenerateKeyStarting | StartupHomeSurface::IssueCard => None,
        StartupHomeSurface::SignerEntry => {
            if state.has_startup_signer_input && state.startup_signer_input_is_editable {
                Some(HomeAutoFocusTarget::StartupSignerInput)
            } else {
                Some(HomeAutoFocusTarget::StartupSignerBack)
            }
        }
    }
}

fn buyer_auto_focus_target(
    runtime: &DesktopAppRuntimeSummary,
    state: HomeAutoFocusState,
) -> Option<HomeAutoFocusTarget> {
    match selected_personal_section(runtime) {
        PersonalSection::Browse => {
            if runtime.personal_projection.browse.detail.is_some() {
                Some(HomeAutoFocusTarget::BuyerDetailBack)
            } else if !runtime.personal_projection.browse.listings.rows.is_empty() {
                Some(HomeAutoFocusTarget::BuyerListingOpenFirst)
            } else {
                None
            }
        }
        PersonalSection::Search => {
            if runtime.personal_projection.search.detail.is_some() {
                Some(HomeAutoFocusTarget::BuyerDetailBack)
            } else if state.has_personal_search_input {
                Some(HomeAutoFocusTarget::BuyerSearchInput)
            } else if !runtime.personal_projection.search.listings.rows.is_empty() {
                Some(HomeAutoFocusTarget::BuyerListingOpenFirst)
            } else {
                None
            }
        }
        PersonalSection::Cart => {
            if state.has_buyer_order_review_form {
                Some(HomeAutoFocusTarget::BuyerOrderReviewNameInput)
            } else if !runtime.personal_projection.cart.cart.lines.is_empty() {
                Some(HomeAutoFocusTarget::BuyerCartOpenOrderReview)
            } else {
                None
            }
        }
        PersonalSection::Orders => {
            if let Some(detail) = runtime.personal_projection.orders.detail.as_ref() {
                let replace_confirmation = runtime
                    .personal_projection
                    .cart
                    .cart
                    .replace_confirmation
                    .as_ref()
                    .is_some_and(|confirmation| {
                        confirmation.incoming_farm_display_name == detail.farm_display_name
                    });
                if state.has_buyer_receipt_issue_form {
                    Some(HomeAutoFocusTarget::BuyerReceiptIssueInput)
                } else if replace_confirmation {
                    Some(HomeAutoFocusTarget::BuyerOrderConfirmReplace)
                } else if detail.repeat_demand.as_ref().is_some_and(|repeat_demand| {
                    repeat_demand.eligibility != RepeatDemandEligibility::Unavailable
                }) {
                    Some(HomeAutoFocusTarget::BuyerOrderRepeatDemand)
                } else if !runtime.personal_projection.orders.list.rows.is_empty() {
                    Some(HomeAutoFocusTarget::BuyerOrderOpenFirst)
                } else {
                    None
                }
            } else if !runtime.personal_projection.orders.list.rows.is_empty() {
                Some(HomeAutoFocusTarget::BuyerOrderOpenFirst)
            } else {
                None
            }
        }
    }
}

fn farmer_auto_focus_target(
    runtime: &DesktopAppRuntimeSummary,
    state: HomeAutoFocusState,
) -> Option<HomeAutoFocusTarget> {
    if let Some(reminder) = presented_farmer_reminder(runtime) {
        if reminder.action_label.is_some() {
            return Some(HomeAutoFocusTarget::FarmerReminderPrimary);
        }
        return Some(HomeAutoFocusTarget::FarmerReminderDismiss);
    }

    match selected_farmer_section(runtime) {
        FarmerSection::Today | FarmerSection::Farm => today_auto_focus_target(runtime, state),
        FarmerSection::Products if farmer_products_available(runtime) => {
            if state.has_product_editor_form {
                Some(HomeAutoFocusTarget::ProductEditorTitleInput)
            } else if state.has_products_stock_editor {
                Some(HomeAutoFocusTarget::ProductsStockInput)
            } else if state.has_products_search_input {
                Some(HomeAutoFocusTarget::ProductsSearchInput)
            } else if !runtime.products_projection.list.rows.is_empty() {
                Some(HomeAutoFocusTarget::ProductsRowOpenFirst)
            } else {
                None
            }
        }
        FarmerSection::Orders if farmer_products_available(runtime) => {
            if let Some(detail) = runtime.orders_projection.detail.as_ref() {
                if !detail.fulfillment_actions.is_empty() {
                    Some(HomeAutoFocusTarget::OrdersDetailPublishFulfillmentFirst)
                } else if !runtime.orders_projection.list.rows.is_empty() {
                    Some(HomeAutoFocusTarget::OrdersRowOpenFirst)
                } else {
                    None
                }
            } else if !runtime.orders_projection.list.rows.is_empty() {
                Some(HomeAutoFocusTarget::OrdersRowOpenFirst)
            } else {
                None
            }
        }
        FarmerSection::PackDay if farmer_pack_day_available(runtime) => None,
        FarmerSection::Products | FarmerSection::Orders | FarmerSection::PackDay => {
            today_auto_focus_target(runtime, state)
        }
    }
}

fn today_auto_focus_target(
    runtime: &DesktopAppRuntimeSummary,
    state: HomeAutoFocusState,
) -> Option<HomeAutoFocusTarget> {
    let projection = &runtime.today_projection;

    if state.has_farm_setup_form {
        return Some(HomeAutoFocusTarget::FarmerSetupFarmNameInput);
    }

    if let Some(spec) = farm_setup_onboarding_card_spec(runtime.home_route) {
        if spec.action_key.is_some() {
            return Some(HomeAutoFocusTarget::FarmerSetupStart);
        }
    } else if projection.needs_setup()
        && farmer_home_farm_state(runtime) == FarmerHomeFarmState::IncompleteFarm
    {
        return Some(HomeAutoFocusTarget::FarmerSetupContinue);
    }

    if projection
        .reminders
        .items
        .iter()
        .any(|reminder| reminder_action_target(reminder).is_some())
    {
        return Some(HomeAutoFocusTarget::FarmerTodayReminderChipFirst);
    }
    if projection.next_fulfillment_window.is_some() {
        return Some(HomeAutoFocusTarget::FarmerTodayOpenPackDay);
    }
    if !projection.orders_needing_action.is_empty() {
        return Some(HomeAutoFocusTarget::FarmerTodayOpenOrders);
    }
    if !projection.low_stock_products.is_empty() {
        return Some(HomeAutoFocusTarget::FarmerTodayOpenProductsLowStock);
    }
    if !projection.draft_products.is_empty() {
        return Some(HomeAutoFocusTarget::FarmerTodayOpenProductsDrafts);
    }

    None
}

fn settings_auto_focus_target(
    selected_view: SettingsPanelViewKey,
    farm_panel_state: Option<&SettingsFarmPanelState>,
    runtime: &DesktopAppRuntimeSummary,
) -> Option<SettingsAutoFocusTarget> {
    match selected_view {
        SettingsPanelViewKey::Account => Some(SettingsAutoFocusTarget::AccountAdd),
        SettingsPanelViewKey::Farm => farm_panel_state
            .map(|_| SettingsAutoFocusTarget::FarmNameInput)
            .or(Some(SettingsAutoFocusTarget::Navigation(
                SettingsPanelViewKey::Farm,
            ))),
        SettingsPanelViewKey::Settings => Some(SettingsAutoFocusTarget::Navigation(
            SettingsPanelViewKey::Settings,
        )),
        SettingsPanelViewKey::About => {
            if about_manual_refresh_enabled(&runtime.sync_status) {
                Some(SettingsAutoFocusTarget::AboutRefresh)
            } else {
                Some(SettingsAutoFocusTarget::Navigation(
                    SettingsPanelViewKey::About,
                ))
            }
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
        let panel_content = self.settings_panel_content(window, cx);
        self.apply_auto_focus(window, cx);

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
                .child(div().flex_1().overflow_hidden().child(panel_content)),
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
        runtime.shell_projection.active_surface != radroots_studio_app_view::ActiveSurface::Farmer;
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
    choice_button(id, app_shared_text(key), is_active, on_click, cx).into_any_element()
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
        action_button_compact("shell-account-entry", account_label, on_open_account, cx)
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
                .or_else(|| Some(app_shared_text(AppTextKey::HomeHeaderAccountLabel).to_string()))
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

fn account_placeholder_panel(text_key: AppTextKey) -> impl IntoElement {
    div()
        .w_full()
        .min_h(px(320.0))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
        .child(app_shared_text(text_key))
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
    app_focused_detail_view(
        product_display_title(detail.listing.title.as_str()),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(settings_badge_text(
                detail.listing.farm_display_name.clone(),
            ))
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
        text_button(
            "buyer-detail-back",
            app_shared_text(AppTextKey::PersonalDetailBackAction),
            on_close,
            cx,
        ),
    )
}

fn buyer_cart_card(
    cart: &BuyerCartProjection,
    summary: &BuyerOrderReviewSummaryProjection,
    order_review_open: bool,
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
            .when(!order_review_open, |this| {
                this.child(action_button_primary(
                    "buyer-cart-open-order-review",
                    app_shared_text(AppTextKey::PersonalCartReviewOrderAction),
                    cx.listener(|this, _, window, cx| this.open_personal_order_review(window, cx)),
                    cx,
                ))
            }),
    )
}

fn buyer_cart_line_card(
    index: usize,
    line: &radroots_studio_app_view::BuyerCartLineProjection,
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

fn buyer_order_review_card(
    form: &BuyerOrderReviewFormState,
    order_review: &radroots_studio_app_view::BuyerOrderReviewProjection,
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
                        AppTextKey::PersonalOrderReviewTitle,
                    )))
                    .child(text_button(
                        "buyer-order-review-back",
                        app_shared_text(AppTextKey::PersonalOrderReviewBackAction),
                        on_close,
                        cx,
                    )),
            )
            .child(home_body_text(app_shared_text(
                AppTextKey::PersonalOrderReviewLocalOnlyBody,
            )))
            .child(app_surface_panel(
                app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .p(px(APP_UI_THEME.shells.home_card_padding_px))
                    .child(app_text_label(app_shared_text(
                        AppTextKey::PersonalOrderSummaryTitle,
                    )))
                    .child(label_value_list(buyer_order_summary_rows(
                        &order_review.summary,
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
                        order_review
                            .summary
                            .fulfillment_summary
                            .clone()
                            .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone).to_string()),
                    )),
            ))
            .child(app_form_section(
                app_shared_text(AppTextKey::PersonalOrderReviewContactTitle),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalOrderReviewFieldName),
                            Option::<SharedString>::None,
                        ),
                        &form.name_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalOrderReviewFieldEmail),
                            Option::<SharedString>::None,
                        ),
                        &form.email_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalOrderReviewFieldPhone),
                            Option::<SharedString>::None,
                        ),
                        &form.phone_input,
                        false,
                    ))
                    .child(app_form_input_text(
                        AppFormFieldSpec::new(
                            app_shared_text(AppTextKey::PersonalOrderReviewFieldOrderNote),
                            Option::<SharedString>::None,
                        ),
                        &form.order_note_input,
                        false,
                    )),
            ))
            .child(if order_review.can_place_order {
                action_button_primary(
                    "buyer-order-review-place-order",
                    app_shared_text(AppTextKey::PersonalOrderReviewPlaceOrderAction),
                    on_place_order,
                    cx,
                )
                .into_any_element()
            } else {
                action_button_primary_disabled(
                    "buyer-order-review-place-order",
                    app_shared_text(AppTextKey::PersonalOrderReviewPlaceOrderAction),
                    cx,
                )
                .into_any_element()
            }),
    )
}

fn buyer_order_summary_rows(summary: &BuyerOrderReviewSummaryProjection) -> Vec<LabelValueRow> {
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

fn trade_economics_total_text(economics: &TradeEconomicsProjection) -> String {
    economics
        .total_minor_units
        .zip(economics.currency_code.as_deref())
        .map(|(amount, currency_code)| buyer_money_text(amount, currency_code))
        .unwrap_or_else(|| app_shared_text(AppTextKey::ValueNone).to_string())
}

fn trade_workflow_detail_badge_strip(workflow: &TradeWorkflowProjection) -> AnyElement {
    let mut badges = vec![
        trade_workflow_labeled_key_badge(
            AppTextKey::TradeWorkflowAxisAgreement,
            trade_agreement_status_key(workflow.agreement),
        ),
        trade_workflow_labeled_key_badge(
            AppTextKey::TradeWorkflowAxisRevision,
            trade_revision_status_key(workflow.revision),
        ),
    ];

    if let Some(fulfillment) = workflow.fulfillment {
        badges.push(trade_workflow_labeled_key_badge(
            AppTextKey::TradeWorkflowAxisFulfillment,
            trade_fulfillment_status_key(fulfillment),
        ));
    }
    if let Some(receipt) = workflow.receipt.as_ref() {
        badges.push(trade_workflow_labeled_key_badge(
            AppTextKey::TradeWorkflowAxisReceipt,
            buyer_receipt_status_key(receipt),
        ));
    }

    badges.push(trade_workflow_labeled_key_badge(
        AppTextKey::TradeWorkflowAxisInventory,
        trade_inventory_status_key(workflow.inventory),
    ));
    badges.push(trade_workflow_labeled_key_badge(
        AppTextKey::TradeWorkflowAxisPayment,
        trade_payment_display_status_key(workflow.payment),
    ));
    if workflow.provenance.primary_source != TradeWorkflowSource::Unknown {
        badges.push(trade_workflow_labeled_key_badge(
            AppTextKey::TradeWorkflowAxisSource,
            trade_workflow_source_key(workflow.provenance.primary_source),
        ));
    }

    app_cluster(APP_UI_THEME.foundation.spacing.small_px)
        .w_full()
        .children(badges)
        .into_any_element()
}

fn trade_workflow_list_badge_strip(workflow: &TradeWorkflowProjection) -> AnyElement {
    let mut badges = vec![trade_workflow_value_badge(trade_agreement_status_key(
        workflow.agreement,
    ))];

    if workflow.revision != TradeRevisionStatus::None {
        badges.push(trade_workflow_value_badge(trade_revision_status_key(
            workflow.revision,
        )));
    }

    if let Some(fulfillment) = workflow.fulfillment {
        badges.push(trade_workflow_value_badge(trade_fulfillment_status_key(
            fulfillment,
        )));
    }
    if let Some(receipt) = workflow.receipt.as_ref() {
        badges.push(trade_workflow_value_badge(buyer_receipt_status_key(
            receipt,
        )));
    }

    badges.push(trade_workflow_labeled_key_badge(
        AppTextKey::TradeWorkflowAxisPayment,
        trade_payment_display_status_key(workflow.payment),
    ));

    app_cluster(APP_UI_THEME.foundation.spacing.tight_px)
        .w_full()
        .children(badges)
        .into_any_element()
}

fn trade_workflow_status_stack(workflow: &TradeWorkflowProjection) -> AnyElement {
    app_stack_v(2.0)
        .min_w_0()
        .child(trade_workflow_value_badge(trade_agreement_status_key(
            workflow.agreement,
        )))
        .when_some(workflow.fulfillment, |this, fulfillment| {
            this.child(trade_workflow_value_badge(trade_fulfillment_status_key(
                fulfillment,
            )))
        })
        .when_some(workflow.receipt.as_ref(), |this, receipt| {
            this.child(trade_workflow_value_badge(buyer_receipt_status_key(
                receipt,
            )))
        })
        .into_any_element()
}

fn trade_workflow_labeled_key_badge(label_key: AppTextKey, value_key: AppTextKey) -> AnyElement {
    settings_badge_text(format!("{}: {}", app_text(label_key), app_text(value_key)))
        .into_any_element()
}

fn trade_workflow_value_badge(value_key: AppTextKey) -> AnyElement {
    settings_badge_text(app_shared_text(value_key)).into_any_element()
}

fn trade_agreement_status_key(status: TradeAgreementStatus) -> AppTextKey {
    match status {
        TradeAgreementStatus::Ordered => AppTextKey::TradeWorkflowAgreementOrdered,
        TradeAgreementStatus::Confirmed => AppTextKey::TradeWorkflowAgreementConfirmed,
        TradeAgreementStatus::Declined => AppTextKey::TradeWorkflowAgreementDeclined,
        TradeAgreementStatus::Cancelled => AppTextKey::TradeWorkflowAgreementCancelled,
        TradeAgreementStatus::Completed => AppTextKey::TradeWorkflowAgreementCompleted,
        TradeAgreementStatus::NeedsReview => AppTextKey::TradeWorkflowAgreementNeedsReview,
    }
}

fn trade_revision_status_key(status: TradeRevisionStatus) -> AppTextKey {
    match status {
        TradeRevisionStatus::None => AppTextKey::TradeWorkflowRevisionNone,
        TradeRevisionStatus::ChangeProposed => AppTextKey::TradeWorkflowRevisionChangeProposed,
        TradeRevisionStatus::Updated => AppTextKey::TradeWorkflowRevisionUpdated,
        TradeRevisionStatus::KeptAsPlaced => AppTextKey::TradeWorkflowRevisionKeptAsPlaced,
    }
}

fn trade_fulfillment_status_key(status: TradeFulfillmentStatus) -> AppTextKey {
    match status {
        TradeFulfillmentStatus::Confirmed => AppTextKey::TradeWorkflowFulfillmentConfirmed,
        TradeFulfillmentStatus::Preparing => AppTextKey::TradeWorkflowFulfillmentPreparing,
        TradeFulfillmentStatus::ReadyForPickup => {
            AppTextKey::TradeWorkflowFulfillmentReadyForPickup
        }
        TradeFulfillmentStatus::OutForDelivery => {
            AppTextKey::TradeWorkflowFulfillmentOutForDelivery
        }
        TradeFulfillmentStatus::Delivered => AppTextKey::TradeWorkflowFulfillmentDelivered,
        TradeFulfillmentStatus::Cancelled => AppTextKey::TradeWorkflowFulfillmentCancelled,
    }
}

fn trade_inventory_status_key(status: TradeInventoryStatus) -> AppTextKey {
    match status {
        TradeInventoryStatus::Available => AppTextKey::TradeWorkflowInventoryAvailable,
        TradeInventoryStatus::Reserved => AppTextKey::TradeWorkflowInventoryReserved,
        TradeInventoryStatus::SoldOut => AppTextKey::TradeWorkflowInventorySoldOut,
        TradeInventoryStatus::NeedsReview => AppTextKey::TradeWorkflowInventoryNeedsReview,
    }
}

fn trade_payment_display_status_key(status: TradePaymentDisplayStatus) -> AppTextKey {
    match status {
        TradePaymentDisplayStatus::NotRecorded => AppTextKey::TradeWorkflowPaymentNotRecorded,
        TradePaymentDisplayStatus::Pending => AppTextKey::TradeWorkflowPaymentPending,
        TradePaymentDisplayStatus::Recorded => AppTextKey::TradeWorkflowPaymentRecorded,
        TradePaymentDisplayStatus::Settled => AppTextKey::TradeWorkflowPaymentSettled,
        TradePaymentDisplayStatus::NeedsReview => AppTextKey::TradeWorkflowPaymentNeedsReview,
    }
}

fn trade_workflow_source_key(source: TradeWorkflowSource) -> AppTextKey {
    match source {
        TradeWorkflowSource::App => AppTextKey::TradeWorkflowProvenanceApp,
        TradeWorkflowSource::Cli => AppTextKey::TradeWorkflowProvenanceCli,
        TradeWorkflowSource::Relay => AppTextKey::TradeWorkflowProvenanceRelay,
        TradeWorkflowSource::LocalEvents => AppTextKey::TradeWorkflowProvenanceLocalEvents,
        TradeWorkflowSource::Unknown => AppTextKey::TradeWorkflowProvenanceUnknown,
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

fn buyer_orders_retry_action_visible(orders: &BuyerOrdersScreenProjection) -> bool {
    orders.has_recoverable_coordination
}

fn buyer_orders_retry_card(cx: &mut Context<HomeView>) -> AnyElement {
    home_card(
        app_shared_text(AppTextKey::PersonalOrdersCoordinationRetryTitle),
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .child(home_body_text(app_shared_text(
                AppTextKey::PersonalOrdersCoordinationRetryBody,
            )))
            .child(action_button_primary(
                "buyer-orders-retry-coordination",
                app_shared_text(AppTextKey::PersonalOrdersCoordinationRetryAction),
                cx.listener(|this, _, _, cx| this.retry_pending_personal_order_coordination(cx)),
                cx,
            )),
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
                            .child(settings_badge_text(row.farm_display_name.clone()))
                            .child(settings_badge_text(trade_economics_total_text(
                                &row.workflow.economics,
                            ))),
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
                                    .child(app_shared_text(trade_agreement_status_key(
                                        row.workflow.agreement,
                                    ))),
                            ),
                    ),
            )
            .child(trade_workflow_list_badge_strip(&row.workflow))
            .child(buyer_listing_chip(row.fulfillment_summary.clone())),
    )
    .into_any_element()
}

fn buyer_order_detail_card(
    detail: &BuyerOrderDetailProjection,
    issue_form: Option<&BuyerReceiptIssueFormState>,
    replace_confirmation: Option<&BuyerCartReplaceConfirmationProjection>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    let repeat_confirmation = replace_confirmation
        .filter(|confirmation| confirmation.incoming_farm_display_name == detail.farm_display_name);

    app_focused_detail_view(
        app_shared_text(AppTextKey::PersonalOrdersDetailTitle),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(app_heading_section(detail.order_number.clone()))
            .child(settings_badge_text(detail.farm_display_name.clone()))
            .child(trade_workflow_detail_badge_strip(&detail.workflow))
            .child(label_value_list([
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailFarmLabel),
                    detail.farm_display_name.clone(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailFulfillmentLabel),
                    detail.fulfillment_summary.clone(),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailTotalLabel),
                    trade_economics_total_text(&detail.workflow.economics),
                ),
                LabelValueRow::new(
                    app_shared_text(AppTextKey::PersonalOrdersDetailNoteLabel),
                    order_optional_text(detail.order_note.as_deref()),
                ),
            ]))
            .when_some(detail.workflow.receipt.as_ref(), |this, receipt| {
                this.child(buyer_receipt_summary_section(receipt))
            })
            .when(!detail.validation_receipts.is_empty(), |this| {
                this.child(validation_receipts_summary_section(
                    &detail.validation_receipts,
                ))
            })
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
            ))
            .when(
                detail.status == BuyerOrderStatus::Scheduled
                    && detail.workflow.revision == TradeRevisionStatus::ChangeProposed,
                |this| {
                    this.child(
                        app_stack_h(APP_UI_THEME.foundation.spacing.small_px)
                            .w_full()
                            .child(action_button_primary(
                                "buyer-order-accept-change",
                                app_shared_text(AppTextKey::PersonalOrdersActionAcceptChange),
                                cx.listener({
                                    let order_id = detail.order_id;
                                    move |this, _, _, cx| {
                                        this.accept_buyer_order_revision(order_id, cx)
                                    }
                                }),
                                cx,
                            ))
                            .child(action_button_compact(
                                "buyer-order-keep-order",
                                app_shared_text(AppTextKey::PersonalOrdersActionKeepOrder),
                                cx.listener({
                                    let order_id = detail.order_id;
                                    move |this, _, _, cx| {
                                        this.decline_buyer_order_revision(order_id, cx)
                                    }
                                }),
                                cx,
                            )),
                    )
                },
            )
            .when(
                matches!(
                    detail.status,
                    BuyerOrderStatus::Placed | BuyerOrderStatus::Scheduled
                ),
                |this| {
                    this.child(action_button_compact(
                        "buyer-order-cancel",
                        app_shared_text(AppTextKey::PersonalOrdersActionCancel),
                        cx.listener({
                            let order_id = detail.order_id;
                            move |this, _, _, cx| this.cancel_buyer_order(order_id, cx)
                        }),
                        cx,
                    ))
                },
            )
            .when(buyer_receipt_actions_available(detail), |this| {
                this.child(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .child(
                            app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                                .w_full()
                                .child(action_button_primary(
                                    "buyer-order-mark-received",
                                    app_shared_text(AppTextKey::PersonalOrdersActionMarkReceived),
                                    cx.listener({
                                        let order_id = detail.order_id;
                                        move |this, _, _, cx| {
                                            this.mark_buyer_order_received(order_id, cx)
                                        }
                                    }),
                                    cx,
                                ))
                                .child(action_button_compact(
                                    "buyer-order-report-issue",
                                    app_shared_text(AppTextKey::PersonalOrdersActionReportIssue),
                                    cx.listener({
                                        let order_id = detail.order_id;
                                        move |this, _, window, cx| {
                                            this.open_buyer_receipt_issue_form(order_id, window, cx)
                                        }
                                    }),
                                    cx,
                                )),
                        )
                        .when_some(issue_form, |this, form| {
                            this.child(buyer_receipt_issue_form_section(form, cx))
                        }),
                )
            })
            .when_some(detail.repeat_demand.as_ref(), |this, repeat_demand| {
                this.child(app_form_section(
                    app_shared_text(AppTextKey::PersonalOrdersRepeatDemandTitle),
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .when_some(buyer_repeat_demand_note(repeat_demand), |this, note| {
                            this.child(home_body_text(note))
                        })
                        .when_some(repeat_confirmation, |this, replace_confirmation| {
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
                                        app_shared_text(AppTextKey::PersonalDetailReplaceCartBody,),
                                        replace_confirmation.incoming_farm_display_name,
                                    )))
                                    .child(
                                        app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                                            .w_full()
                                            .child(action_button_primary(
                                                "buyer-order-confirm-replace",
                                                app_shared_text(
                                                    AppTextKey::PersonalDetailReplaceCartAction,
                                                ),
                                                cx.listener({
                                                    let order_id = detail.order_id;
                                                    move |this, _, _, cx| {
                                                        this.repeat_personal_order(
                                                            order_id, true, cx,
                                                        )
                                                    }
                                                }),
                                                cx,
                                            ))
                                            .child(action_button_compact(
                                                "buyer-order-keep-current",
                                                app_shared_text(
                                                    AppTextKey::PersonalDetailKeepCurrentCartAction,
                                                ),
                                                cx.listener(|this, _, _, cx| {
                                                    this.clear_personal_cart_replace_confirmation(
                                                        cx,
                                                    )
                                                }),
                                                cx,
                                            )),
                                    ),
                            ))
                        })
                        .when(
                            repeat_confirmation.is_none()
                                && repeat_demand.eligibility
                                    != RepeatDemandEligibility::Unavailable,
                            |this| {
                                this.child(action_button_primary(
                                    "buyer-order-repeat-demand",
                                    buyer_repeat_demand_action_label(repeat_demand),
                                    cx.listener({
                                        let order_id = detail.order_id;
                                        move |this, _, _, cx| {
                                            this.repeat_personal_order(order_id, false, cx)
                                        }
                                    }),
                                    cx,
                                ))
                            },
                        ),
                ))
            }),
        text_button(
            "buyer-order-detail-back",
            app_shared_text(AppTextKey::PersonalDetailBackAction),
            on_close,
            cx,
        ),
    )
}

fn buyer_receipt_summary_section(receipt: &TradeReceiptProjection) -> AnyElement {
    app_form_section(
        app_shared_text(AppTextKey::PersonalOrdersDetailReceiptLabel),
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .child(trade_workflow_value_badge(buyer_receipt_status_key(
                receipt,
            )))
            .when_some(receipt.issue.as_ref(), |this, issue| {
                this.child(home_body_text(issue.clone()))
            }),
    )
    .into_any_element()
}

fn validation_receipts_summary_section(
    receipts: &[TradeValidationReceiptProjection],
) -> AnyElement {
    app_form_section(
        app_shared_text(AppTextKey::TradeValidationReceiptSectionLabel),
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .children(
                receipts
                    .iter()
                    .map(validation_receipt_summary_panel)
                    .collect::<Vec<_>>(),
            ),
    )
    .into_any_element()
}

fn validation_receipt_summary_panel(receipt: &TradeValidationReceiptProjection) -> AnyElement {
    app_surface_panel(
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .p(px(APP_UI_THEME.shells.home_card_padding_px))
            .child(
                app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .child(trade_workflow_value_badge(validation_receipt_result_key(
                        receipt.result,
                    )))
                    .child(trade_workflow_value_badge(validation_receipt_type_key(
                        receipt.receipt_type,
                    ))),
            )
            .child(home_body_text(format!(
                "{} {}",
                app_shared_text(AppTextKey::TradeValidationReceiptRecordedAtLabel),
                receipt.recorded_at
            ))),
    )
    .into_any_element()
}

fn buyer_receipt_issue_form_section(
    form: &BuyerReceiptIssueFormState,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    let order_id = form.order_id;
    let submit_action = if form.can_submit(cx) {
        action_button_primary(
            "buyer-order-send-issue",
            app_shared_text(AppTextKey::PersonalOrdersActionSendReceiptIssue),
            cx.listener(move |this, _, _, cx| this.submit_buyer_order_issue_receipt(order_id, cx)),
            cx,
        )
        .into_any_element()
    } else {
        action_button_primary_disabled(
            "buyer-order-send-issue",
            app_shared_text(AppTextKey::PersonalOrdersActionSendReceiptIssue),
            cx,
        )
        .into_any_element()
    };

    app_form_section(
        app_shared_text(AppTextKey::PersonalOrdersDetailReceiptLabel),
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::PersonalOrdersReceiptIssueLabel),
                    Option::<SharedString>::None,
                ),
                &form.issue_input,
                false,
            ))
            .child(
                app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                    .w_full()
                    .child(submit_action)
                    .child(action_button_compact(
                        "buyer-order-close-issue",
                        app_shared_text(AppTextKey::PersonalOrdersActionCloseReceiptIssue),
                        cx.listener(|this, _, _, cx| this.close_buyer_receipt_issue_form(cx)),
                        cx,
                    )),
            ),
    )
    .into_any_element()
}

fn buyer_receipt_issue_focused_view(
    detail: &BuyerOrderDetailProjection,
    form: &BuyerReceiptIssueFormState,
    cx: &mut Context<HomeView>,
) -> AnyElement {
    let order_id = form.order_id;
    let submit_action = if form.can_submit(cx) {
        action_button_primary(
            "buyer-order-send-issue",
            app_shared_text(AppTextKey::PersonalOrdersActionSendReceiptIssue),
            cx.listener(move |this, _, _, cx| this.submit_buyer_order_issue_receipt(order_id, cx)),
            cx,
        )
        .into_any_element()
    } else {
        action_button_primary_disabled(
            "buyer-order-send-issue",
            app_shared_text(AppTextKey::PersonalOrdersActionSendReceiptIssue),
            cx,
        )
        .into_any_element()
    };

    app_focused_task_view(
        app_shared_text(AppTextKey::PersonalOrdersActionReportIssue),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
            .child(app_heading_section(detail.order_number.clone()))
            .child(settings_badge_text(detail.farm_display_name.clone()))
            .child(home_body_text(detail.fulfillment_summary.clone()))
            .child(app_form_input_text(
                AppFormFieldSpec::new(
                    app_shared_text(AppTextKey::PersonalOrdersReceiptIssueLabel),
                    Option::<SharedString>::None,
                ),
                &form.issue_input,
                false,
            ))
            .child(submit_action),
        text_button(
            "buyer-order-close-issue",
            app_shared_text(AppTextKey::PersonalDetailBackAction),
            cx.listener(|this, _, _, cx| this.close_buyer_receipt_issue_form(cx)),
            cx,
        ),
    )
}

fn buyer_receipt_actions_available(detail: &BuyerOrderDetailProjection) -> bool {
    detail.workflow.receipt.is_none()
        && detail.workflow.agreement == TradeAgreementStatus::Confirmed
        && matches!(
            detail.workflow.fulfillment,
            Some(TradeFulfillmentStatus::ReadyForPickup | TradeFulfillmentStatus::Delivered)
        )
}

fn buyer_receipt_status_key(receipt: &TradeReceiptProjection) -> AppTextKey {
    if receipt.received {
        AppTextKey::TradeWorkflowReceiptReceived
    } else {
        AppTextKey::TradeWorkflowReceiptNeedsReview
    }
}

fn validation_receipt_result_key(result: TradeValidationReceiptResult) -> AppTextKey {
    match result {
        TradeValidationReceiptResult::Valid => AppTextKey::TradeValidationReceiptResultValid,
        TradeValidationReceiptResult::NeedsReview => {
            AppTextKey::TradeValidationReceiptResultNeedsReview
        }
    }
}

fn validation_receipt_type_key(receipt_type: TradeValidationReceiptType) -> AppTextKey {
    match receipt_type {
        TradeValidationReceiptType::ListingValidation => {
            AppTextKey::TradeValidationReceiptTypeListingValidation
        }
        TradeValidationReceiptType::TradeTransition => {
            AppTextKey::TradeValidationReceiptTypeTradeTransition
        }
        TradeValidationReceiptType::InventoryState => {
            AppTextKey::TradeValidationReceiptTypeInventoryState
        }
        TradeValidationReceiptType::StateCheckpoint => {
            AppTextKey::TradeValidationReceiptTypeStateCheckpoint
        }
    }
}

fn buyer_repeat_demand_action_label(repeat_demand: &RepeatDemandHandoffProjection) -> SharedString {
    match repeat_demand.eligibility {
        RepeatDemandEligibility::Eligible => {
            app_shared_text(AppTextKey::PersonalOrdersRepeatDemandActionEligible)
        }
        RepeatDemandEligibility::Partial => {
            app_shared_text(AppTextKey::PersonalOrdersRepeatDemandActionPartial)
        }
        RepeatDemandEligibility::Unavailable => {
            app_shared_text(AppTextKey::PersonalOrdersRepeatDemandActionEligible)
        }
    }
}

fn buyer_repeat_demand_note(repeat_demand: &RepeatDemandHandoffProjection) -> Option<SharedString> {
    match repeat_demand.eligibility {
        RepeatDemandEligibility::Eligible => None,
        RepeatDemandEligibility::Partial if repeat_demand.unavailable_item_count == 1 => Some(
            app_shared_text(AppTextKey::PersonalOrdersRepeatDemandNotePartialSingle),
        ),
        RepeatDemandEligibility::Partial => Some(app_shared_text(
            AppTextKey::PersonalOrdersRepeatDemandNotePartialMultiple,
        )),
        RepeatDemandEligibility::Unavailable => Some(app_shared_text(
            AppTextKey::PersonalOrdersRepeatDemandNoteUnavailable,
        )),
    }
}

fn buyer_orders_status_color(status: BuyerOrderStatus) -> u32 {
    match status {
        BuyerOrderStatus::Placed => APP_UI_THEME.components.app_status_indicator.attention,
        BuyerOrderStatus::Scheduled | BuyerOrderStatus::Ready => {
            APP_UI_THEME.components.app_status_indicator.online
        }
        BuyerOrderStatus::Completed
        | BuyerOrderStatus::Declined
        | BuyerOrderStatus::Refunded
        | BuyerOrderStatus::NeedsReview => APP_UI_THEME.components.app_status_indicator.offline,
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
    let startup_notice = startup_notice.map(startup_notice_text);

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
                                        .when_some(startup_notice, |this, error: String| {
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
                                                .when_some(startup_notice, |this, error: String| {
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
    startup_notice: Option<String>,
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
        preview
            .as_ref()
            .err()
            .map(|error| startup_notice_text(error))
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
        .when_some(startup_notice, |this, notice: String| {
            this.child(div().w_full().text_center().child(home_body_text(notice)))
        })
}

fn startup_signer_preview_summary(input: &str) -> Result<StartupSignerPreviewSummary, String> {
    let target = radroots_studio_app_remote_signer_preview(input).map_err(|error| error.to_string())?;

    Ok(StartupSignerPreviewSummary {
        source_label: startup_signer_source_text(target.source),
        signer_npub: target.signer_identity.public_key_npub.clone(),
        relays_label: startup_signer_csv_or_none(target.relays.as_slice()),
        permissions_label: startup_signer_permissions_label(target.requested_permission_labels()),
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
        return app_text(AppTextKey::ValueNone);
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
    if permissions.is_empty() {
        return app_text(AppTextKey::ValueNone);
    }

    permissions
        .into_iter()
        .map(|permission| startup_signer_permission_text(permission.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
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

fn startup_issue_summary_text(_startup_issue: &str) -> String {
    app_text(AppTextKey::HomeSetupIssueUnavailableBody)
}

fn startup_signer_source_text(source: RadrootsAppRemoteSignerSource) -> String {
    app_text(match source {
        RadrootsAppRemoteSignerSource::BunkerUri => AppTextKey::HomeSetupSignerSourceValueBunkerUri,
        RadrootsAppRemoteSignerSource::DiscoveryUrl => {
            AppTextKey::HomeSetupSignerSourceValueDiscoveryUrl
        }
    })
}

fn startup_signer_permission_text(permission: &str) -> String {
    app_text(match permission {
        "sign_event:kind:1" => AppTextKey::HomeSetupSignerPermissionSignEventKind1,
        "switch_relays" => AppTextKey::HomeSetupSignerPermissionSwitchRelays,
        _ => AppTextKey::HomeSetupSignerPermissionAdditional,
    })
}

fn startup_notice_text(message: &str) -> String {
    app_text(match message {
        "enter a bunker or discovery url to continue" => {
            AppTextKey::HomeSetupSignerErrorEnterSource
        }
        "discovery url does not contain a remote signer uri" => {
            AppTextKey::HomeSetupSignerErrorMissingDiscoveryUri
        }
        "a remote signer connection is already pending approval" => {
            AppTextKey::HomeSetupSignerErrorPendingApprovalExists
        }
        _ if message.contains("raw nostrconnect client uris are signer-side only") => {
            AppTextKey::HomeSetupSignerErrorUseSignerUri
        }
        _ if message.starts_with("invalid discovery url:") => {
            AppTextKey::HomeSetupSignerErrorInvalidDiscoveryUrl
        }
        _ if message.starts_with("invalid remote signer uri:") => {
            AppTextKey::HomeSetupSignerErrorInvalidRemoteSignerUri
        }
        _ if message.contains("remote signer") => AppTextKey::HomeSetupSignerErrorConnectionFailed,
        _ => AppTextKey::HomeSetupErrorStartupFailed,
    })
}

fn startup_home_body(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let body = runtime.startup_issue.as_deref().map_or_else(
        || app_shared_text(AppTextKey::HomeTodayEmptySetupBody).to_string(),
        startup_issue_summary_text,
    );

    div().w_full().text_center().child(home_body_text(body))
}

async fn connect_configured_relays(relay_urls: Vec<String>) -> Result<RadrootsNostrClient, String> {
    let client = RadrootsNostrClient::new_signerless();
    for relay_url in relay_urls {
        client
            .add_relay(relay_url.as_str())
            .await
            .map_err(|error| format!("failed to add relay `{relay_url}`: {error}"))?;
    }
    client.connect().await;
    Ok(client)
}

struct StartupAppInitResult {
    relay_client: RadrootsNostrClient,
}

async fn run_startup_app_init(relay_urls: Vec<String>) -> Result<StartupAppInitResult, String> {
    let relay_client = connect_configured_relays(relay_urls).await?;
    Ok(StartupAppInitResult { relay_client })
}

async fn run_startup_signer_connect(
    source_input: String,
) -> Result<RadrootsAppRemoteSignerPendingSession, String> {
    radroots_studio_app_remote_signer_connect_pending(source_input.as_str())
        .map_err(|error| error.to_string())
}

async fn run_pack_day_host_handoff(
    plan: PackDayHostHandoffCommandPlan,
) -> Result<(), PackDayHostHandoffError> {
    execute_pack_day_host_handoff_plan(&plan)
}

async fn run_pack_day_print(plan: PackDayPrintCommandPlan) -> Result<(), PackDayPrintError> {
    execute_pack_day_print_plan(&plan)
}

async fn run_pack_day_batch_print(
    plan: PackDayBatchPrintCommandPlan,
) -> Result<(), PackDayBatchPrintError> {
    execute_pack_day_batch_print_plan(&plan)
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
    choice_button(id, app_shared_text(key), is_active, on_click, cx).into_any_element()
}

fn home_sidebar_navigation_sections(
    _selected_section: FarmerSection,
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

    sections
}

fn selected_farmer_section(runtime: &DesktopAppRuntimeSummary) -> FarmerSection {
    match runtime.shell_projection.selected_section {
        ShellSection::Farmer(section) => section,
        ShellSection::Home
        | ShellSection::Account
        | ShellSection::Personal(_)
        | ShellSection::Settings(_) => FarmerSection::Today,
    }
}

fn selected_personal_section(runtime: &DesktopAppRuntimeSummary) -> PersonalSection {
    match runtime.shell_projection.selected_section {
        ShellSection::Personal(section) => section,
        ShellSection::Home
        | ShellSection::Account
        | ShellSection::Farmer(_)
        | ShellSection::Settings(_) => PersonalSection::Browse,
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

    choice_button(id, app_shared_text(key), is_active, on_click, cx).into_any_element()
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
            Some(144.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersDetailTotalLabel,
            Some(112.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::TradeWorkflowAxisPayment,
            Some(128.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnWindow,
            Some(160.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnPickup,
            Some(160.0),
            false,
        ))
        .child(products_table_header_column(
            AppTextKey::OrdersColumnAction,
            Some(120.0),
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
                .w(px(144.0))
                .flex()
                .items_start()
                .gap(px(6.0))
                .child(status_indicator(orders_status_color(row.status)))
                .child(trade_workflow_status_stack(&row.workflow)),
        )
        .child(
            div()
                .w(px(112.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(trade_economics_total_text(&row.workflow.economics)),
        )
        .child(div().w(px(128.0)).child(trade_workflow_value_badge(
            trade_payment_display_status_key(row.workflow.payment),
        )))
        .child(
            div()
                .w(px(160.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(order_optional_text(row.fulfillment_window_label.as_deref())),
        )
        .child(
            div()
                .w(px(160.0))
                .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                .line_height(relative(1.2))
                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                .child(order_optional_text(row.pickup_location_label.as_deref())),
        )
        .child(div().w(px(120.0)).flex().justify_end().child(action))
}

fn orders_table_action(
    index: usize,
    row: &OrdersListRow,
    on_review: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_publish_fulfillment: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
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
        Some(
            OrderPrimaryAction::PublishPreparing
            | OrderPrimaryAction::PublishReadyForPickup
            | OrderPrimaryAction::PublishOutForDelivery
            | OrderPrimaryAction::PublishDelivered
            | OrderPrimaryAction::PublishSellerCancelled,
        ) => action_button_compact(
            ("orders-row-action-publish-fulfillment", index),
            app_shared_text(AppTextKey::OrdersActionUpdateFulfillment),
            on_publish_fulfillment,
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

fn order_detail_fulfillment_action_id(action: OrderFulfillmentAction) -> &'static str {
    match action {
        OrderFulfillmentAction::Preparing => "orders-detail-publish-preparing",
        OrderFulfillmentAction::ReadyForPickup => "orders-detail-publish-ready-for-pickup",
        OrderFulfillmentAction::OutForDelivery => "orders-detail-publish-out-for-delivery",
        OrderFulfillmentAction::Delivered => "orders-detail-publish-delivered",
        OrderFulfillmentAction::SellerCancelled => "orders-detail-publish-seller-cancelled",
    }
}

fn order_fulfillment_action_label_key(action: OrderFulfillmentAction) -> AppTextKey {
    match action {
        OrderFulfillmentAction::Preparing => AppTextKey::OrdersActionPreparing,
        OrderFulfillmentAction::ReadyForPickup => AppTextKey::OrdersActionReadyForPickup,
        OrderFulfillmentAction::OutForDelivery => AppTextKey::OrdersActionOutForDelivery,
        OrderFulfillmentAction::Delivered => AppTextKey::OrdersActionMarkDelivered,
        OrderFulfillmentAction::SellerCancelled => AppTextKey::OrdersActionCancelFulfillment,
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

fn orders_status_color(status: OrderStatus) -> u32 {
    match status {
        OrderStatus::NeedsAction => APP_UI_THEME.components.app_status_indicator.attention,
        OrderStatus::Scheduled | OrderStatus::Packed => {
            APP_UI_THEME.components.app_status_indicator.online
        }
        OrderStatus::Completed
        | OrderStatus::Declined
        | OrderStatus::Refunded
        | OrderStatus::NeedsReview => APP_UI_THEME.components.app_status_indicator.offline,
    }
}

fn order_recovery_title_key(kind: RecoveryKind) -> AppTextKey {
    match kind {
        RecoveryKind::MissedPickup => AppTextKey::OrdersRecoveryMissedPickupTitle,
        RecoveryKind::RefundFollowUp => AppTextKey::OrdersRecoveryRefundFollowUpTitle,
    }
}

fn order_recovery_empty_body_key(kind: RecoveryKind) -> AppTextKey {
    match kind {
        RecoveryKind::MissedPickup => AppTextKey::OrdersRecoveryMissedPickupBody,
        RecoveryKind::RefundFollowUp => AppTextKey::OrdersRecoveryRefundFollowUpBody,
    }
}

fn order_recovery_state_key(state: RecoveryState) -> AppTextKey {
    match state {
        RecoveryState::Open => AppTextKey::OrdersRecoveryStateOpen,
        RecoveryState::InReview => AppTextKey::OrdersRecoveryStateInReview,
        RecoveryState::Resolved => AppTextKey::OrdersRecoveryStateResolved,
    }
}

fn order_recovery_state_color(state: RecoveryState) -> u32 {
    match state {
        RecoveryState::Open => APP_UI_THEME.components.app_status_indicator.attention,
        RecoveryState::InReview => APP_UI_THEME.foundation.text.accent,
        RecoveryState::Resolved => APP_UI_THEME.components.app_status_indicator.online,
    }
}

fn order_recovery_state_badge(state: RecoveryState) -> AnyElement {
    div()
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(order_recovery_state_color(state)))
        .child(app_shared_text(order_recovery_state_key(state)))
        .into_any_element()
}

fn order_recovery_kind_index(kind: RecoveryKind) -> usize {
    match kind {
        RecoveryKind::MissedPickup => 0,
        RecoveryKind::RefundFollowUp => 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayExportStatusPresentation {
    indicator_color: u32,
    title_key: AppTextKey,
    body_key: AppTextKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayHostHandoffActionPresentation {
    kind: PackDayHostHandoffKind,
    label_key: AppTextKey,
    enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayHostHandoffStatusPresentation {
    indicator_color: u32,
    title_key: AppTextKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayPrintActionPresentation {
    kind: PackDayPrintKind,
    label_key: AppTextKey,
    enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayPrintStatusPresentation {
    indicator_color: u32,
    title_key: AppTextKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayBatchPrintActionPresentation {
    label_key: AppTextKey,
    enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDayBatchPrintStatusPresentation {
    indicator_color: u32,
    title_key: AppTextKey,
}

fn pack_day_export_card(
    runtime: &DesktopAppRuntimeSummary,
    on_export: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_reveal_bundle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_pack_sheet: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_pickup_roster: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_customer_labels: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_print_all: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_print_pack_sheet: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_print_pickup_roster: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_print_customer_labels: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let export = &runtime.pack_day_projection.export;
    let status = pack_day_export_status_presentation(runtime);
    let detail_rows = pack_day_export_detail_rows(export);
    let host_handoff_actions = pack_day_host_handoff_action_presentations(runtime);
    let host_handoff_status = pack_day_host_handoff_status_presentation(runtime);
    let batch_print_action = pack_day_batch_print_action_presentation(runtime);
    let batch_print_status = pack_day_batch_print_status_presentation(runtime);
    let print_actions = pack_day_print_action_presentations(runtime);
    let print_status = pack_day_print_status_presentation(runtime);
    let host_handoff_error_message = runtime
        .pack_day_projection
        .host_handoff
        .error_message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_owned);
    let action = if pack_day_export_action_enabled(runtime) {
        action_button_primary(
            "pack-day-export",
            app_shared_text(AppTextKey::PackDayExportAction),
            on_export,
            cx,
        )
        .into_any_element()
    } else {
        action_button_primary_disabled(
            "pack-day-export",
            app_shared_text(pack_day_export_action_label_key(export)),
            cx,
        )
        .into_any_element()
    };

    home_card(
        app_shared_text(AppTextKey::PackDayExportTitle),
        app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
            .w_full()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
                    .child(status_indicator(status.indicator_color))
                    .child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(app_shared_text(status.title_key)),
                    ),
            )
            .child(home_body_text(app_shared_text(status.body_key)))
            .when(!detail_rows.is_empty(), |this| {
                this.child(label_value_list(detail_rows))
            })
            .child(div().child(action))
            .when(!host_handoff_actions.is_empty(), |this| {
                let on_reveal_bundle = Arc::new(on_reveal_bundle);
                let on_open_pack_sheet = Arc::new(on_open_pack_sheet);
                let on_open_pickup_roster = Arc::new(on_open_pickup_roster);
                let on_open_customer_labels = Arc::new(on_open_customer_labels);
                this.child(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .child(
                            app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                                .items_center()
                                .children(host_handoff_actions.into_iter().map(move |action| {
                                    let button = match action.kind {
                                        PackDayHostHandoffKind::RevealBundle if action.enabled => {
                                            action_button(
                                                "pack-day-reveal-bundle",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_reveal_bundle =
                                                        Arc::clone(&on_reveal_bundle);
                                                    move |event, window, cx| {
                                                        (on_reveal_bundle)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenPackSheet if action.enabled => {
                                            action_button(
                                                "pack-day-open-pack-sheet",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_open_pack_sheet =
                                                        Arc::clone(&on_open_pack_sheet);
                                                    move |event, window, cx| {
                                                        (on_open_pack_sheet)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenPickupRoster
                                            if action.enabled =>
                                        {
                                            action_button(
                                                "pack-day-open-pickup-roster",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_open_pickup_roster =
                                                        Arc::clone(&on_open_pickup_roster);
                                                    move |event, window, cx| {
                                                        (on_open_pickup_roster)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenCustomerLabels
                                            if action.enabled =>
                                        {
                                            action_button(
                                                "pack-day-open-customer-labels",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_open_customer_labels =
                                                        Arc::clone(&on_open_customer_labels);
                                                    move |event, window, cx| {
                                                        (on_open_customer_labels)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::RevealBundle => {
                                            action_button_disabled(
                                                "pack-day-reveal-bundle",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenPackSheet => {
                                            action_button_disabled(
                                                "pack-day-open-pack-sheet",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenPickupRoster => {
                                            action_button_disabled(
                                                "pack-day-open-pickup-roster",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayHostHandoffKind::OpenCustomerLabels => {
                                            action_button_disabled(
                                                "pack-day-open-customer-labels",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                    };
                                    button
                                })),
                        )
                        .when_some(host_handoff_status, |this, status| {
                            this.child(pack_day_host_handoff_status_note(
                                status,
                                host_handoff_error_message.clone(),
                            ))
                        }),
                )
            })
            .when_some(batch_print_action, |this, action| {
                let button = if action.enabled {
                    action_button(
                        "pack-day-print-all",
                        app_shared_text(action.label_key),
                        on_print_all,
                        cx,
                    )
                    .into_any_element()
                } else {
                    action_button_disabled(
                        "pack-day-print-all",
                        app_shared_text(action.label_key),
                        cx,
                    )
                    .into_any_element()
                };
                this.child(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .child(button)
                        .when_some(batch_print_status, |this, status| {
                            this.child(pack_day_batch_print_status_note(status))
                        }),
                )
            })
            .when(!print_actions.is_empty(), |this| {
                let on_print_pack_sheet = Arc::new(on_print_pack_sheet);
                let on_print_pickup_roster = Arc::new(on_print_pickup_roster);
                let on_print_customer_labels = Arc::new(on_print_customer_labels);
                this.child(
                    app_stack_v(APP_UI_THEME.foundation.spacing.small_px)
                        .w_full()
                        .child(
                            app_cluster(APP_UI_THEME.foundation.spacing.small_px)
                                .items_center()
                                .children(print_actions.into_iter().map(move |action| {
                                    match action.kind {
                                        PackDayPrintKind::PrintPackSheet if action.enabled => {
                                            action_button(
                                                "pack-day-print-pack-sheet",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_print_pack_sheet =
                                                        Arc::clone(&on_print_pack_sheet);
                                                    move |event, window, cx| {
                                                        (on_print_pack_sheet)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayPrintKind::PrintPickupRoster if action.enabled => {
                                            action_button(
                                                "pack-day-print-pickup-roster",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_print_pickup_roster =
                                                        Arc::clone(&on_print_pickup_roster);
                                                    move |event, window, cx| {
                                                        (on_print_pickup_roster)(event, window, cx)
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayPrintKind::PrintCustomerLabels if action.enabled => {
                                            action_button(
                                                "pack-day-print-customer-labels",
                                                app_shared_text(action.label_key),
                                                {
                                                    let on_print_customer_labels =
                                                        Arc::clone(&on_print_customer_labels);
                                                    move |event, window, cx| {
                                                        (on_print_customer_labels)(
                                                            event, window, cx,
                                                        )
                                                    }
                                                },
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayPrintKind::PrintPackSheet => action_button_disabled(
                                            "pack-day-print-pack-sheet",
                                            app_shared_text(action.label_key),
                                            cx,
                                        )
                                        .into_any_element(),
                                        PackDayPrintKind::PrintPickupRoster => {
                                            action_button_disabled(
                                                "pack-day-print-pickup-roster",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                        PackDayPrintKind::PrintCustomerLabels => {
                                            action_button_disabled(
                                                "pack-day-print-customer-labels",
                                                app_shared_text(action.label_key),
                                                cx,
                                            )
                                            .into_any_element()
                                        }
                                    }
                                })),
                        )
                        .when_some(print_status, |this, status| {
                            this.child(pack_day_print_status_note(status))
                        }),
                )
            }),
    )
}

fn pack_day_host_handoff_action_presentations(
    runtime: &DesktopAppRuntimeSummary,
) -> Vec<PackDayHostHandoffActionPresentation> {
    let Some(bundle) = pack_day_export_succeeded_bundle(runtime) else {
        return Vec::new();
    };

    let host_handoff = &runtime.pack_day_projection.host_handoff;
    let running_kind = (host_handoff.status == PackDayHostHandoffStatus::Running)
        .then(|| host_handoff.request.as_ref().map(|request| request.kind))
        .flatten();
    let print_running = runtime.pack_day_projection.print.status == PackDayPrintStatus::Running;
    let batch_print_running =
        runtime.pack_day_projection.batch_print.status == PackDayBatchPrintStatus::Running;

    PackDayHostHandoffKind::all_v1()
        .into_iter()
        .map(|kind| PackDayHostHandoffActionPresentation {
            kind,
            label_key: pack_day_host_handoff_action_label_key(kind, running_kind),
            enabled: running_kind.is_none()
                && !print_running
                && !batch_print_running
                && pack_day_host_handoff_action_is_available(bundle, kind),
        })
        .collect()
}

fn pack_day_host_handoff_action_is_available(
    bundle: &PackDayExportBundle,
    kind: PackDayHostHandoffKind,
) -> bool {
    match kind.artifact_kind() {
        None => Path::new(&bundle.bundle_directory).is_dir(),
        Some(artifact_kind) => bundle
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == artifact_kind)
            .and_then(|artifact| pack_day_export_artifact_path(bundle, &artifact.relative_path))
            .is_some_and(|path| path.is_file()),
    }
}

fn pack_day_export_artifact_path(
    bundle: &PackDayExportBundle,
    relative_path: &str,
) -> Option<PathBuf> {
    let relative_path = Path::new(relative_path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }

    Some(PathBuf::from(&bundle.bundle_directory).join(relative_path))
}

fn pack_day_batch_print_action_presentation(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<PackDayBatchPrintActionPresentation> {
    let bundle = pack_day_export_succeeded_bundle(runtime)?;
    let batch_print = &runtime.pack_day_projection.batch_print;
    let batch_running = batch_print.status == PackDayBatchPrintStatus::Running;
    let print_running = runtime.pack_day_projection.print.status == PackDayPrintStatus::Running;
    let host_handoff_running =
        runtime.pack_day_projection.host_handoff.status == PackDayHostHandoffStatus::Running;
    let all_artifacts_available = PackDayPrintKind::all_v1()
        .into_iter()
        .all(|kind| pack_day_print_action_is_available(bundle, kind));

    Some(PackDayBatchPrintActionPresentation {
        label_key: if batch_running {
            AppTextKey::PackDayBatchPrintActionRunning
        } else {
            AppTextKey::PackDayBatchPrintAction
        },
        enabled: !batch_running
            && !print_running
            && !host_handoff_running
            && all_artifacts_available,
    })
}

fn pack_day_batch_print_status_presentation(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<PackDayBatchPrintStatusPresentation> {
    let batch_print = &runtime.pack_day_projection.batch_print;

    let status = match (
        batch_print.status,
        batch_print.failed_artifact,
        batch_print.failure,
    ) {
        (PackDayBatchPrintStatus::Idle, _, _) => return None,
        (PackDayBatchPrintStatus::Running, _, _) => PackDayBatchPrintStatusPresentation {
            indicator_color: APP_UI_THEME.foundation.text.accent,
            title_key: AppTextKey::PackDayBatchPrintQueuedTitle,
        },
        (PackDayBatchPrintStatus::Succeeded, _, _) => PackDayBatchPrintStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.online,
            title_key: AppTextKey::PackDayBatchPrintSucceededTitle,
        },
        (
            PackDayBatchPrintStatus::Failed,
            _,
            Some(PackDayBatchPrintFailureKind::CustomerLabelsAvery5160Overflow),
        ) => PackDayBatchPrintStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            title_key: AppTextKey::PackDayBatchPrintCustomerLabelsAvery5160OverflowFailedTitle,
        },
        (PackDayBatchPrintStatus::Failed, Some(failed_artifact), _) => {
            PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: pack_day_print_failed_title_key(failed_artifact.print_kind),
            }
        }
        (PackDayBatchPrintStatus::Failed, None, Some(PackDayBatchPrintFailureKind::Preflight)) => {
            PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayBatchPrintFailedPreflightTitle,
            }
        }
        (
            PackDayBatchPrintStatus::Failed,
            None,
            Some(PackDayBatchPrintFailureKind::QueueLaunch),
        ) => PackDayBatchPrintStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            title_key: AppTextKey::PackDayBatchPrintFailedQueueLaunchTitle,
        },
        (PackDayBatchPrintStatus::Failed, None, Some(PackDayBatchPrintFailureKind::QueueExit)) => {
            PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayBatchPrintFailedQueueExitTitle,
            }
        }
        (PackDayBatchPrintStatus::Failed, None, _) => PackDayBatchPrintStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            title_key: AppTextKey::PackDayBatchPrintFailedTitle,
        },
    };

    Some(status)
}

fn pack_day_print_action_presentations(
    runtime: &DesktopAppRuntimeSummary,
) -> Vec<PackDayPrintActionPresentation> {
    let Some(bundle) = pack_day_export_succeeded_bundle(runtime) else {
        return Vec::new();
    };

    let print = &runtime.pack_day_projection.print;
    let running_kind = (print.status == PackDayPrintStatus::Running)
        .then(|| print.request.as_ref().map(|request| request.kind))
        .flatten();
    let host_handoff_running =
        runtime.pack_day_projection.host_handoff.status == PackDayHostHandoffStatus::Running;
    let batch_print_running =
        runtime.pack_day_projection.batch_print.status == PackDayBatchPrintStatus::Running;

    PackDayPrintKind::all_v1()
        .into_iter()
        .map(|kind| PackDayPrintActionPresentation {
            kind,
            label_key: pack_day_print_action_label_key(kind, running_kind),
            enabled: running_kind.is_none()
                && !host_handoff_running
                && !batch_print_running
                && pack_day_print_action_is_available(bundle, kind),
        })
        .collect()
}

fn pack_day_print_action_is_available(
    bundle: &PackDayExportBundle,
    kind: PackDayPrintKind,
) -> bool {
    bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == kind.artifact_kind())
        .and_then(|artifact| pack_day_export_artifact_path(bundle, &artifact.relative_path))
        .is_some_and(|path| path.is_file())
}

fn pack_day_print_action_label_key(
    kind: PackDayPrintKind,
    running_kind: Option<PackDayPrintKind>,
) -> AppTextKey {
    match (kind, running_kind == Some(kind)) {
        (PackDayPrintKind::PrintPackSheet, true) => AppTextKey::PackDayPrintPackSheetActionRunning,
        (PackDayPrintKind::PrintPackSheet, false) => AppTextKey::PackDayPrintPackSheetAction,
        (PackDayPrintKind::PrintPickupRoster, true) => {
            AppTextKey::PackDayPrintPickupRosterActionRunning
        }
        (PackDayPrintKind::PrintPickupRoster, false) => AppTextKey::PackDayPrintPickupRosterAction,
        (PackDayPrintKind::PrintCustomerLabels, true) => {
            AppTextKey::PackDayPrintCustomerLabelsActionRunning
        }
        (PackDayPrintKind::PrintCustomerLabels, false) => {
            AppTextKey::PackDayPrintCustomerLabelsAction
        }
    }
}

fn pack_day_print_failed_title_key(kind: PackDayPrintKind) -> AppTextKey {
    match kind {
        PackDayPrintKind::PrintPackSheet => AppTextKey::PackDayPrintPackSheetFailedTitle,
        PackDayPrintKind::PrintPickupRoster => AppTextKey::PackDayPrintPickupRosterFailedTitle,
        PackDayPrintKind::PrintCustomerLabels => AppTextKey::PackDayPrintCustomerLabelsFailedTitle,
    }
}

fn pack_day_print_status_presentation(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<PackDayPrintStatusPresentation> {
    let print = &runtime.pack_day_projection.print;
    let kind = print.request.as_ref()?.kind;
    let failure = print.failure;

    let status = match (print.status, kind, failure) {
        (PackDayPrintStatus::Idle, _, _) => return None,
        (PackDayPrintStatus::Running, PackDayPrintKind::PrintPackSheet, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayPrintPackSheetQueuedTitle,
            }
        }
        (PackDayPrintStatus::Running, PackDayPrintKind::PrintPickupRoster, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayPrintPickupRosterQueuedTitle,
            }
        }
        (PackDayPrintStatus::Running, PackDayPrintKind::PrintCustomerLabels, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayPrintCustomerLabelsQueuedTitle,
            }
        }
        (PackDayPrintStatus::Succeeded, PackDayPrintKind::PrintPackSheet, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayPrintPackSheetSubmittedTitle,
            }
        }
        (PackDayPrintStatus::Succeeded, PackDayPrintKind::PrintPickupRoster, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayPrintPickupRosterSubmittedTitle,
            }
        }
        (PackDayPrintStatus::Succeeded, PackDayPrintKind::PrintCustomerLabels, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayPrintCustomerLabelsSubmittedTitle,
            }
        }
        (PackDayPrintStatus::Failed, PackDayPrintKind::PrintPackSheet, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintPackSheetFailedTitle,
            }
        }
        (PackDayPrintStatus::Failed, PackDayPrintKind::PrintPickupRoster, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintPickupRosterFailedTitle,
            }
        }
        (
            PackDayPrintStatus::Failed,
            PackDayPrintKind::PrintCustomerLabels,
            Some(PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow),
        ) => PackDayPrintStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            title_key: AppTextKey::PackDayPrintCustomerLabelsAvery5160OverflowFailedTitle,
        },
        (PackDayPrintStatus::Failed, PackDayPrintKind::PrintCustomerLabels, _) => {
            PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintCustomerLabelsFailedTitle,
            }
        }
    };

    Some(status)
}

fn pack_day_host_handoff_action_label_key(
    kind: PackDayHostHandoffKind,
    running_kind: Option<PackDayHostHandoffKind>,
) -> AppTextKey {
    match (kind, running_kind == Some(kind)) {
        (PackDayHostHandoffKind::RevealBundle, true) => {
            AppTextKey::PackDayHostHandoffRevealActionRunning
        }
        (PackDayHostHandoffKind::RevealBundle, false) => AppTextKey::PackDayHostHandoffRevealAction,
        (PackDayHostHandoffKind::OpenPackSheet, true) => {
            AppTextKey::PackDayHostHandoffOpenPackSheetActionRunning
        }
        (PackDayHostHandoffKind::OpenPackSheet, false) => {
            AppTextKey::PackDayHostHandoffOpenPackSheetAction
        }
        (PackDayHostHandoffKind::OpenPickupRoster, true) => {
            AppTextKey::PackDayHostHandoffOpenPickupRosterActionRunning
        }
        (PackDayHostHandoffKind::OpenPickupRoster, false) => {
            AppTextKey::PackDayHostHandoffOpenPickupRosterAction
        }
        (PackDayHostHandoffKind::OpenCustomerLabels, true) => {
            AppTextKey::PackDayHostHandoffOpenCustomerLabelsActionRunning
        }
        (PackDayHostHandoffKind::OpenCustomerLabels, false) => {
            AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction
        }
    }
}

fn pack_day_host_handoff_status_presentation(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<PackDayHostHandoffStatusPresentation> {
    let host_handoff = &runtime.pack_day_projection.host_handoff;
    let kind = host_handoff.request.as_ref()?.kind;

    let status = match (host_handoff.status, kind) {
        (PackDayHostHandoffStatus::Idle, _) => return None,
        (PackDayHostHandoffStatus::Running, PackDayHostHandoffKind::RevealBundle) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayHostHandoffRevealRunningTitle,
            }
        }
        (PackDayHostHandoffStatus::Running, PackDayHostHandoffKind::OpenPackSheet) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayHostHandoffOpenPackSheetRunningTitle,
            }
        }
        (PackDayHostHandoffStatus::Running, PackDayHostHandoffKind::OpenPickupRoster) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayHostHandoffOpenPickupRosterRunningTitle,
            }
        }
        (PackDayHostHandoffStatus::Running, PackDayHostHandoffKind::OpenCustomerLabels) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsRunningTitle,
            }
        }
        (PackDayHostHandoffStatus::Succeeded, PackDayHostHandoffKind::RevealBundle) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayHostHandoffRevealSucceededTitle,
            }
        }
        (PackDayHostHandoffStatus::Succeeded, PackDayHostHandoffKind::OpenPackSheet) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayHostHandoffOpenPackSheetSucceededTitle,
            }
        }
        (PackDayHostHandoffStatus::Succeeded, PackDayHostHandoffKind::OpenPickupRoster) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayHostHandoffOpenPickupRosterSucceededTitle,
            }
        }
        (PackDayHostHandoffStatus::Succeeded, PackDayHostHandoffKind::OpenCustomerLabels) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsSucceededTitle,
            }
        }
        (PackDayHostHandoffStatus::Failed, PackDayHostHandoffKind::RevealBundle) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayHostHandoffRevealFailedTitle,
            }
        }
        (PackDayHostHandoffStatus::Failed, PackDayHostHandoffKind::OpenPackSheet) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayHostHandoffOpenPackSheetFailedTitle,
            }
        }
        (PackDayHostHandoffStatus::Failed, PackDayHostHandoffKind::OpenPickupRoster) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayHostHandoffOpenPickupRosterFailedTitle,
            }
        }
        (PackDayHostHandoffStatus::Failed, PackDayHostHandoffKind::OpenCustomerLabels) => {
            PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsFailedTitle,
            }
        }
    };

    Some(status)
}

fn pack_day_host_handoff_status_note(
    status: PackDayHostHandoffStatusPresentation,
    error_message: Option<String>,
) -> impl IntoElement {
    app_stack_v(4.0)
        .w_full()
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
                .child(status_indicator(status.indicator_color))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(app_shared_text(status.title_key)),
                ),
        )
        .when_some(error_message, |this, error_message| {
            this.child(home_body_text(error_message))
        })
}

fn pack_day_print_status_note(status: PackDayPrintStatusPresentation) -> impl IntoElement {
    app_stack_v(4.0).w_full().child(
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
            .child(status_indicator(status.indicator_color))
            .child(
                div()
                    .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                    .child(app_shared_text(status.title_key)),
            ),
    )
}

fn pack_day_batch_print_status_note(
    status: PackDayBatchPrintStatusPresentation,
) -> impl IntoElement {
    app_stack_v(4.0).w_full().child(
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(APP_UI_THEME.shells.settings_account_status_gap_px))
            .child(status_indicator(status.indicator_color))
            .child(
                div()
                    .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                    .child(app_shared_text(status.title_key)),
            ),
    )
}

fn pack_day_export_succeeded_bundle(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<&PackDayExportBundle> {
    (runtime.pack_day_projection.export.status == PackDayExportStatus::Succeeded)
        .then_some(runtime.pack_day_projection.export.bundle.as_ref())
        .flatten()
}

fn pack_day_export_has_exportable_context(runtime: &DesktopAppRuntimeSummary) -> bool {
    let projection = &runtime.pack_day_projection.projection;
    projection.fulfillment_window.is_some() && !projection.is_empty()
}

fn pack_day_export_action_enabled(runtime: &DesktopAppRuntimeSummary) -> bool {
    pack_day_export_has_exportable_context(runtime)
        && runtime.pack_day_projection.export.status != PackDayExportStatus::Running
}

fn pack_day_export_action_label_key(export: &PackDayExportProjection) -> AppTextKey {
    match export.status {
        PackDayExportStatus::Running => AppTextKey::PackDayExportActionRunning,
        PackDayExportStatus::Idle
        | PackDayExportStatus::Succeeded
        | PackDayExportStatus::Failed => AppTextKey::PackDayExportAction,
    }
}

fn pack_day_export_status_presentation(
    runtime: &DesktopAppRuntimeSummary,
) -> PackDayExportStatusPresentation {
    match runtime.pack_day_projection.export.status {
        PackDayExportStatus::Running => PackDayExportStatusPresentation {
            indicator_color: APP_UI_THEME.foundation.text.accent,
            title_key: AppTextKey::PackDayExportRunningTitle,
            body_key: AppTextKey::PackDayExportRunningBody,
        },
        PackDayExportStatus::Succeeded => PackDayExportStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.online,
            title_key: AppTextKey::PackDayExportSucceededTitle,
            body_key: AppTextKey::PackDayExportSucceededBody,
        },
        PackDayExportStatus::Failed => PackDayExportStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
            title_key: AppTextKey::PackDayExportFailedTitle,
            body_key: AppTextKey::PackDayExportFailedBody,
        },
        PackDayExportStatus::Idle if pack_day_export_has_exportable_context(runtime) => {
            PackDayExportStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayExportReadyTitle,
                body_key: AppTextKey::PackDayExportReadyBody,
            }
        }
        PackDayExportStatus::Idle => PackDayExportStatusPresentation {
            indicator_color: APP_UI_THEME.components.app_status_indicator.offline,
            title_key: AppTextKey::PackDayExportUnavailableTitle,
            body_key: AppTextKey::PackDayExportUnavailableBody,
        },
    }
}

fn pack_day_export_detail_rows(export: &PackDayExportProjection) -> Vec<LabelValueRow> {
    match export.status {
        PackDayExportStatus::Succeeded => export
            .bundle
            .as_ref()
            .map(pack_day_export_bundle_rows)
            .unwrap_or_default(),
        PackDayExportStatus::Failed => export
            .error_message
            .as_deref()
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .map(|message| {
                vec![LabelValueRow::new(
                    app_shared_text(AppTextKey::PackDayExportErrorLabel),
                    message.to_owned(),
                )]
            })
            .unwrap_or_default(),
        PackDayExportStatus::Idle | PackDayExportStatus::Running => Vec::new(),
    }
}

fn pack_day_export_bundle_rows(bundle: &PackDayExportBundle) -> Vec<LabelValueRow> {
    vec![
        LabelValueRow::new(
            app_shared_text(AppTextKey::PackDayExportFolderLabel),
            bundle.bundle_directory.clone(),
        ),
        LabelValueRow::new(
            app_shared_text(AppTextKey::PackDayExportFilesLabel),
            pack_day_export_artifact_names(bundle),
        ),
    ]
}

fn pack_day_export_artifact_names(bundle: &PackDayExportBundle) -> String {
    bundle
        .artifacts
        .iter()
        .map(|artifact| artifact.kind.file_name())
        .collect::<Vec<_>>()
        .join(", ")
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReminderActionTarget {
    OrderDetail(OrderId),
    PackDay(FulfillmentWindowId),
}

fn reminder_action_target(reminder: &ReminderDeadlineProjection) -> Option<ReminderActionTarget> {
    reminder
        .order_id
        .map(ReminderActionTarget::OrderDetail)
        .or_else(|| {
            reminder
                .fulfillment_window_id
                .map(ReminderActionTarget::PackDay)
        })
}

fn reminder_urgency_key(urgency: ReminderUrgency) -> AppTextKey {
    match urgency {
        ReminderUrgency::Upcoming => AppTextKey::ReminderUrgencyUpcoming,
        ReminderUrgency::DueSoon => AppTextKey::ReminderUrgencyDueSoon,
        ReminderUrgency::Overdue => AppTextKey::ReminderUrgencyOverdue,
        ReminderUrgency::Blocking => AppTextKey::ReminderUrgencyBlocking,
    }
}

fn reminder_urgency_color(urgency: ReminderUrgency) -> u32 {
    match urgency {
        ReminderUrgency::Upcoming => APP_UI_THEME.components.app_status_indicator.offline,
        ReminderUrgency::DueSoon => APP_UI_THEME.foundation.text.accent,
        ReminderUrgency::Overdue | ReminderUrgency::Blocking => {
            APP_UI_THEME.components.app_status_indicator.attention
        }
    }
}

fn reminder_urgency_badge(urgency: ReminderUrgency) -> AnyElement {
    div()
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(reminder_urgency_color(urgency)))
        .child(app_shared_text(reminder_urgency_key(urgency)))
        .into_any_element()
}

fn reminder_delivery_state_key(delivery_state: ReminderDeliveryState) -> AppTextKey {
    match delivery_state {
        ReminderDeliveryState::Scheduled => AppTextKey::ReminderDeliveryStateScheduled,
        ReminderDeliveryState::Presented => AppTextKey::ReminderDeliveryStatePresented,
        ReminderDeliveryState::Acknowledged => AppTextKey::ReminderDeliveryStateAcknowledged,
        ReminderDeliveryState::Resolved => AppTextKey::ReminderDeliveryStateResolved,
    }
}

fn reminder_delivery_state_color(delivery_state: ReminderDeliveryState) -> u32 {
    match delivery_state {
        ReminderDeliveryState::Scheduled => APP_UI_THEME.components.app_status_indicator.offline,
        ReminderDeliveryState::Presented => APP_UI_THEME.foundation.text.accent,
        ReminderDeliveryState::Acknowledged => APP_UI_THEME.foundation.text.secondary,
        ReminderDeliveryState::Resolved => APP_UI_THEME.components.app_status_indicator.online,
    }
}

fn reminder_delivery_state_badge(delivery_state: ReminderDeliveryState) -> AnyElement {
    div()
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(reminder_delivery_state_color(delivery_state)))
        .child(app_shared_text(reminder_delivery_state_key(delivery_state)))
        .into_any_element()
}

fn presented_farmer_reminder(
    runtime: &DesktopAppRuntimeSummary,
) -> Option<&ReminderDeadlineProjection> {
    runtime
        .today_projection
        .reminders
        .items
        .iter()
        .chain(runtime.orders_projection.reminders.items.iter())
        .chain(
            runtime
                .pack_day_projection
                .projection
                .reminders
                .items
                .iter(),
        )
        .filter(|reminder| reminder.delivery_state == ReminderDeliveryState::Presented)
        .min_by(|left, right| {
            reminder_urgency_priority(left.urgency)
                .cmp(&reminder_urgency_priority(right.urgency))
                .then_with(|| left.deadline_at.cmp(&right.deadline_at))
                .then_with(|| left.reminder_id.cmp(&right.reminder_id))
        })
}

fn reminder_urgency_priority(urgency: ReminderUrgency) -> u8 {
    match urgency {
        ReminderUrgency::Blocking => 0,
        ReminderUrgency::Overdue => 1,
        ReminderUrgency::DueSoon => 2,
        ReminderUrgency::Upcoming => 3,
    }
}

fn reminder_deadline_text(reminder: &ReminderDeadlineProjection) -> String {
    format!(
        "{}: {}",
        app_text(AppTextKey::ReminderDeadlineLabel),
        reminder.deadline_at
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
    cx: &mut Context<HomeView>,
) -> AnyElement {
    let validation_keys = products_editor_validation_keys(form, cx);
    let save_ready = form.has_changes(cx) && validation_keys.is_empty();

    let save_action = if save_ready {
        action_button_primary(
            "products-editor-save",
            app_shared_text(AppTextKey::ProductsEditorSaveAction),
            cx.listener(|this, _, _, cx| this.save_product_editor(cx)),
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
    };

    app_focused_task_view(
        app_shared_text(AppTextKey::ProductsEditorTitle),
        app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
            .w_full()
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
                    app_shared_text(AppTextKey::ProductsEditorFieldCategory),
                    Option::<SharedString>::None,
                ),
                &form.category_input,
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
            .child(products_editor_availability_section(
                form,
                &runtime.farm_rules_projection.fulfillment_windows,
                cx,
            ))
            .child(products_editor_status_section(
                form.status,
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
                            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                            .child(product_display_title(
                                form.title_input.read(cx).value().as_ref(),
                            )),
                    )
                    .child(save_action),
            ),
        text_button(
            "products-editor-close",
            app_shared_text(AppTextKey::ProductsEditorCloseAction),
            cx.listener(|this, _, _, cx| this.close_product_editor(cx)),
            cx,
        ),
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

fn products_editor_availability_section(
    form: &ProductEditorFormState,
    fulfillment_windows: &[FulfillmentWindowRecord],
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let choices = fulfillment_windows
        .iter()
        .enumerate()
        .map(|(index, fulfillment_window)| {
            let fulfillment_window_id = fulfillment_window.fulfillment_window_id;
            choice_button(
                ("products-editor-availability", index),
                SharedString::from(fulfillment_window.label.clone()),
                form.selected_availability_window_id == Some(fulfillment_window_id),
                cx.listener(move |this, _, _, cx| {
                    this.select_product_editor_availability_window(fulfillment_window_id, cx)
                }),
                cx,
            )
            .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(APP_UI_THEME.foundation.spacing.small_px))
        .child(home_farm_setup_field_label(app_shared_text(
            AppTextKey::ProductsEditorFieldAvailability,
        )))
        .child(if choices.is_empty() {
            home_body_text(app_shared_text(AppTextKey::ProductsEditorAvailabilityEmpty))
                .into_any_element()
        } else {
            app_cluster(APP_UI_THEME.foundation.spacing.tight_px)
                .w_full()
                .children(choices)
                .into_any_element()
        })
}

fn products_editor_publish_readiness_section(
    form: &ProductEditorFormState,
    runtime: &DesktopAppRuntimeSummary,
    cx: &App,
) -> impl IntoElement {
    let blockers = form
        .current_draft(cx)
        .map(|draft| {
            derive_product_publish_blockers(
                &draft,
                &runtime.farm_readiness_projection,
                &runtime.farm_rules_projection,
            )
        })
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
        ProductPublishBlocker::ChooseCategory => AppTextKey::ProductsEditorBlockerChooseCategory,
        ProductPublishBlocker::ChooseUnit => AppTextKey::ProductsEditorBlockerChooseUnit,
        ProductPublishBlocker::SetPrice => AppTextKey::ProductsEditorBlockerSetPrice,
        ProductPublishBlocker::SetStock => AppTextKey::ProductsEditorBlockerSetStock,
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

fn home_summary_card(summary: &radroots_studio_app_view::TodaySummary) -> impl IntoElement {
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
    let unit_price = item.unit_price.as_ref().map(buyer_listing_price_text);
    let line_total = item.unit_price.as_ref().and_then(|unit_price| {
        item.line_total_minor_units
            .map(|amount| buyer_money_text(amount, unit_price.currency_code.as_str()))
    });

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
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .line_height(relative(1.2))
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(item.title.clone()),
                )
                .when_some(unit_price, |this, unit_price| {
                    this.child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                            .child(unit_price),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                        .child(item.quantity_display.clone()),
                )
                .when_some(line_total, |this, line_total| {
                    this.child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                            .child(line_total),
                    )
                }),
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

fn home_setup_task_row(task: &radroots_studio_app_view::TodaySetupTask) -> AnyElement {
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

fn buyer_order_place_failure_notice(error: &AppSqliteError) -> BuyerWorkspaceNotice {
    match error {
        AppSqliteError::LocalEventsSql { .. } | AppSqliteError::LocalEvents { .. } => {
            BuyerWorkspaceNotice::OrderCoordinationFailed
        }
        _ => BuyerWorkspaceNotice::OrderPlaceFailed,
    }
}

fn buyer_order_coordination_notice_forces_redraw(notice: BuyerWorkspaceNotice) -> bool {
    notice == BuyerWorkspaceNotice::OrderCoordinationFailed
}

fn buyer_workspace_notice_card(notice: String) -> impl IntoElement {
    app_surface_card(home_body_text(notice))
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
        APP_UI_THEME, AppTextKey, BuyerWorkspaceNotice, FarmerHomeFarmState, HomeAutoFocusState,
        HomeAutoFocusTarget, HomeFocusedView, HomeStage, HomeView, LabelValueRow,
        PackDayBatchPrintActionPresentation, PackDayBatchPrintStatusPresentation,
        PackDayExportStatusPresentation, PackDayHostHandoffActionPresentation,
        PackDayHostHandoffStatusPresentation, PackDayPrintActionPresentation,
        PackDayPrintStatusPresentation, ReminderActionTarget, SETTINGS_FARM_PANEL_SECTIONS,
        SETTINGS_NAVIGATION_ORDER, SETTINGS_OPERATIONS_PANEL_SECTIONS, SettingsAutoFocusTarget,
        SettingsInventorySectionSpec, SettingsPanelViewKey, StartupHomeSurface,
        StartupSignerConnectState, abbreviated_npub, about_conflict_action_specs,
        about_conflict_aggregate_text, about_conflict_detail_rows, about_conflict_review_body_key,
        about_manual_refresh_enabled, about_runtime_rows, about_status_rows, account_display_name,
        app_text, buyer_order_coordination_notice_forces_redraw,
        buyer_order_detail_focus_after_open, buyer_orders_retry_action_visible,
        buyer_receipt_issue_focus_after_submit, buyer_receipt_status_key,
        farm_setup_onboarding_card_spec, farmer_home_farm_state,
        farmer_order_detail_focus_after_open, farmer_pack_day_available, home_auto_focus_target,
        home_content_scroll_id, home_saved_farm, home_sidebar_navigation_sections, home_stage,
        home_window_launch_size_px, home_window_minimum_size_px,
        pack_day_batch_print_action_presentation, pack_day_batch_print_status_presentation,
        pack_day_export_action_enabled, pack_day_export_action_label_key,
        pack_day_export_artifact_names, pack_day_export_detail_rows,
        pack_day_export_status_presentation, pack_day_host_handoff_action_presentations,
        pack_day_host_handoff_status_presentation, pack_day_print_action_presentations,
        pack_day_print_status_presentation, parse_optional_product_editor_stock_input,
        parse_product_editor_price_input, presented_farmer_reminder, product_display_title,
        reminder_action_target, reminder_deadline_text, reminder_delivery_state_key,
        reminder_urgency_color, reminder_urgency_key, settings_auto_focus_target,
        settings_preferences_general_row_state, startup_home_surface, startup_issue_summary_text,
        startup_notice_text, startup_signer_preview_summary,
        startup_signer_preview_summary_for_connect_state, startup_signer_source_input_is_editable,
        startup_signer_status_spec, startup_signer_transport_failure_requires_notice,
        trade_agreement_status_key, trade_fulfillment_status_key, trade_inventory_status_key,
        trade_payment_display_status_key, trade_revision_status_key, trade_workflow_source_key,
    };
    use crate::runtime::{
        DesktopAppRuntimeMetadataSummary, DesktopAppRuntimeSummary, DesktopAppSyncConflictSummary,
        DesktopAppSyncStatusSummary,
    };
    use radroots_studio_app_core::{
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePlatform,
    };
    use radroots_studio_app_remote_signer::{
        RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingSession,
        RadrootsAppRemoteSignerSessionRecord,
    };
    use radroots_studio_app_state::{
        AppShellProjection, BuyerOrdersScreenProjection, FarmWorkspaceReadinessProjection,
        FarmWorkspaceStatus, HomeRoute, PackDayBatchPrintProjection, PackDayBatchPrintRequest,
        PackDayExportProjection, PackDayHostHandoffProjection, PackDayHostHandoffRequest,
        PackDayPrintProjection, PackDayPrintRequest,
    };
    use radroots_studio_app_sync::{
        AppSyncProjection, AppSyncRunStatus, SyncAggregateRef, SyncCheckpointStatus, SyncConflict,
        SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity, SyncConflictStatus,
    };
    use radroots_studio_app_view::SettingsAccountProjection;
    use radroots_studio_app_view::{
        AccountCustody, AccountSummary, ActiveSurface, AppStartupGate, BuyerOrderDetailProjection,
        BuyerOrderStatus, BuyerOrdersListRow, FarmId, FarmOrderMethod, FarmReadiness,
        FarmSetupDraft, FarmSetupProjection, FarmSummary, FarmerSection, FulfillmentWindowId,
        FulfillmentWindowSummary, LoggedOutStartupPhase, LoggedOutStartupProjection,
        OrderDetailProjection, OrderFulfillmentAction, OrderId, OrderPrimaryAction, OrderStatus,
        OrdersListRow, PackDayBatchPrintArtifact, PackDayBatchPrintFailureKind,
        PackDayExportArtifact, PackDayExportArtifactKind, PackDayExportBundle,
        PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPrintFailureKind,
        PackDayPrintKind, PackDayPrintStatus, PackDayProductTotalRow, PackDayProjection,
        PersonalSection, ProductId, ReminderDeadlineProjection, ReminderDeliveryState, ReminderId,
        ReminderKind, ReminderSurface, ReminderUrgency, RepeatDemandEligibility,
        RepeatDemandHandoffProjection, ShellSection, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TradeAgreementStatus, TradeEconomicsProjection, TradeFulfillmentStatus,
        TradeInventoryStatus, TradePaymentDisplayStatus, TradeReceiptProjection,
        TradeRevisionStatus, TradeWorkflowProjection, TradeWorkflowSource,
    };
    use radroots_identity::RadrootsIdentity;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(FulfillmentWindowId::new().to_string());
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_artifact(bundle_directory: &PathBuf, file_name: &str) -> PathBuf {
        let path = bundle_directory.join(file_name);
        fs::write(&path, file_name).unwrap();
        path
    }

    fn test_home_view(label: &str) -> (HomeView, AppDesktopRuntimePaths, PathBuf) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let home_dir = std::env::temp_dir().join(format!("radroots_home_view_{label}_{suffix}"));
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(home_dir.clone()),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("desktop runtime paths should resolve");
        let runtime = crate::runtime::DesktopAppRuntime::bootstrap_with_paths(
            paths.clone(),
            vec!["wss://relay.example".to_owned()],
        );

        (HomeView::new(runtime), paths, home_dir)
    }

    fn block_shared_local_events_database(paths: &AppDesktopRuntimePaths) {
        let database_path = paths.shared_local_events_database_path().unwrap();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if database_path.is_file() {
            fs::remove_file(&database_path).unwrap();
        } else if database_path.is_dir() {
            fs::remove_dir_all(&database_path).unwrap();
        }
        fs::create_dir(&database_path).unwrap();
    }

    #[test]
    fn buyer_workspace_notice_tracks_visible_buyer_runtime_errors() {
        let (mut view, _, home_dir) = test_home_view("buyer_notice");

        assert!(view.set_buyer_workspace_notice(BuyerWorkspaceNotice::MarketplaceRefreshFailed));
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalMarketplaceRefreshFailedNotice).as_str())
        );
        assert!(!view.set_buyer_workspace_notice(BuyerWorkspaceNotice::MarketplaceRefreshFailed));
        assert!(view.set_buyer_workspace_notice(BuyerWorkspaceNotice::OrderPlaceFailed));
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalOrderPlaceFailedNotice).as_str())
        );
        assert!(view.set_buyer_workspace_notice(BuyerWorkspaceNotice::OrderCoordinationFailed));
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalOrderCoordinationFailedNotice).as_str())
        );
        assert!(view.clear_buyer_workspace_notice());
        assert_eq!(view.buyer_workspace_notice, None);

        let _ = fs::remove_dir_all(home_dir);
    }

    #[test]
    fn buyer_order_place_failure_uses_typed_visible_notice() {
        let (mut view, _, home_dir) = test_home_view("buyer_notice");

        assert!(view.place_personal_order_update());
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalOrderPlaceFailedNotice).as_str())
        );

        let _ = fs::remove_dir_all(home_dir);
    }

    #[test]
    fn buyer_order_coordination_failure_forces_redraw_when_notice_is_unchanged() {
        assert!(buyer_order_coordination_notice_forces_redraw(
            BuyerWorkspaceNotice::OrderCoordinationFailed
        ));
        assert!(!buyer_order_coordination_notice_forces_redraw(
            BuyerWorkspaceNotice::OrderPlaceFailed
        ));
    }

    #[test]
    fn buyer_orders_retry_action_tracks_recoverable_coordination() {
        let mut orders = BuyerOrdersScreenProjection::default();
        assert!(!buyer_orders_retry_action_visible(&orders));

        orders.has_recoverable_coordination = true;
        assert!(buyer_orders_retry_action_visible(&orders));
    }

    #[test]
    fn buyer_order_detail_focus_reopens_same_selected_detail() {
        let order_id = OrderId::new();
        let farm_id = FarmId::new();
        let mut runtime = summary(
            HomeRoute::Personal,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert_eq!(
            buyer_order_detail_focus_after_open(false, &runtime, order_id),
            None
        );

        runtime.personal_projection.orders.detail = Some(BuyerOrderDetailProjection {
            order_id,
            farm_id,
            order_number: String::new(),
            farm_display_name: String::new(),
            fulfillment_summary: String::new(),
            status: BuyerOrderStatus::Placed,
            items: Vec::new(),
            economics: TradeEconomicsProjection::default(),
            payment: TradePaymentDisplayStatus::NotRecorded,
            workflow: TradeWorkflowProjection::from_buyer_order_status(
                order_id,
                BuyerOrderStatus::Placed,
            ),
            validation_receipts: Vec::new(),
            order_note: None,
            repeat_demand: None,
        });

        assert_eq!(
            buyer_order_detail_focus_after_open(false, &runtime, order_id),
            Some(HomeFocusedView::BuyerOrderDetail(order_id))
        );
        assert_eq!(
            buyer_order_detail_focus_after_open(false, &runtime, OrderId::new()),
            None
        );
    }

    #[test]
    fn farmer_order_detail_focus_reopens_same_selected_detail() {
        let order_id = OrderId::new();
        let farm_id = FarmId::new();
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert_eq!(
            farmer_order_detail_focus_after_open(false, &runtime, order_id),
            None
        );

        runtime.orders_projection.detail = Some(OrderDetailProjection {
            order_id,
            farm_id,
            order_number: String::new(),
            customer_display_name: String::new(),
            status: OrderStatus::Scheduled,
            fulfillment_window_id: None,
            fulfillment_window_label: None,
            pickup_location_label: None,
            items: Vec::new(),
            economics: TradeEconomicsProjection::default(),
            payment: TradePaymentDisplayStatus::NotRecorded,
            workflow: TradeWorkflowProjection::from_order_status(order_id, OrderStatus::Scheduled),
            validation_receipts: Vec::new(),
            primary_action: Some(OrderPrimaryAction::PublishPreparing),
            fulfillment_actions: OrderFulfillmentAction::ALL.to_vec(),
            recoveries: Vec::new(),
        });

        assert_eq!(
            farmer_order_detail_focus_after_open(false, &runtime, order_id),
            Some(HomeFocusedView::FarmerOrderDetail(order_id))
        );
        assert_eq!(
            farmer_order_detail_focus_after_open(false, &runtime, OrderId::new()),
            None
        );
    }

    #[test]
    fn buyer_receipt_issue_submit_returns_to_order_detail() {
        let order_id = OrderId::new();

        assert_eq!(
            buyer_receipt_issue_focus_after_submit(true, order_id),
            Some(HomeFocusedView::BuyerOrderDetail(order_id))
        );
        assert_eq!(
            buyer_receipt_issue_focus_after_submit(false, order_id),
            None
        );
    }

    #[test]
    fn buyer_browse_refresh_failure_uses_typed_visible_notice() {
        let (mut view, paths, home_dir) = test_home_view("buyer_notice");
        block_shared_local_events_database(&paths);

        assert!(view.select_personal_section_update(PersonalSection::Browse));
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalMarketplaceRefreshFailedNotice).as_str())
        );

        let _ = fs::remove_dir_all(home_dir);
    }

    #[test]
    fn buyer_search_refresh_failure_uses_typed_visible_notice() {
        let (mut view, paths, home_dir) = test_home_view("buyer_notice");
        block_shared_local_events_database(&paths);

        assert!(view.set_personal_search_query_update("eggs"));
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalMarketplaceRefreshFailedNotice).as_str())
        );

        let _ = fs::remove_dir_all(home_dir);
    }

    #[test]
    fn buyer_detail_open_failure_uses_typed_visible_notice() {
        let (mut view, paths, home_dir) = test_home_view("buyer_notice");
        block_shared_local_events_database(&paths);

        assert!(
            view.open_personal_product_detail_update(PersonalSection::Browse, ProductId::new())
        );
        assert_eq!(
            view.buyer_workspace_notice.as_deref(),
            Some(app_text(AppTextKey::PersonalDetailOpenFailedNotice).as_str())
        );

        let _ = fs::remove_dir_all(home_dir);
    }

    fn sample_pack_day_bundle(bundle_directory: &PathBuf) -> PackDayExportBundle {
        PackDayExportBundle {
            fulfillment_window_id: FulfillmentWindowId::new(),
            export_instance_id: radroots_studio_app_view::PackDayExportInstanceId::new(),
            generated_at_utc: "2026-04-23T15:00:00Z".to_owned(),
            bundle_directory: bundle_directory.to_string_lossy().into_owned(),
            artifacts: vec![
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PackSheet,
                    relative_path: "pack_sheet.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PickupRoster,
                    relative_path: "pickup_roster.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::CustomerLabels,
                    relative_path: "customer_labels.txt".to_owned(),
                },
            ],
        }
    }

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
    fn settings_account_display_uses_label_before_npub_fallback() {
        let labeled = AccountSummary {
            account_id: "account_1".to_owned(),
            npub: "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq".to_owned(),
            label: Some("  Farm Profile  ".to_owned()),
            custody: AccountCustody::LocalManaged,
        };
        let unlabeled = AccountSummary {
            label: None,
            ..labeled.clone()
        };

        assert_eq!(account_display_name(&labeled), "Farm Profile");
        assert_eq!(account_display_name(&unlabeled), "npub1qqqqq...qqqqqq");
    }

    #[test]
    fn settings_account_npub_fallback_stays_compact() {
        assert_eq!(
            abbreviated_npub("npub1sxczrq2dp4jtehcm8mtemj975u5ytf2d7mc6dpuuq3rzkjzr76ls5lkheq"),
            "npub1sxczr...5lkheq"
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
    fn trade_workflow_badge_keys_cover_refactored_status_axes() {
        for (status, key) in [
            (
                TradeAgreementStatus::Ordered,
                AppTextKey::TradeWorkflowAgreementOrdered,
            ),
            (
                TradeAgreementStatus::Confirmed,
                AppTextKey::TradeWorkflowAgreementConfirmed,
            ),
            (
                TradeAgreementStatus::Declined,
                AppTextKey::TradeWorkflowAgreementDeclined,
            ),
            (
                TradeAgreementStatus::Cancelled,
                AppTextKey::TradeWorkflowAgreementCancelled,
            ),
            (
                TradeAgreementStatus::Completed,
                AppTextKey::TradeWorkflowAgreementCompleted,
            ),
            (
                TradeAgreementStatus::NeedsReview,
                AppTextKey::TradeWorkflowAgreementNeedsReview,
            ),
        ] {
            assert_eq!(trade_agreement_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }

        for (status, key) in [
            (
                TradeRevisionStatus::None,
                AppTextKey::TradeWorkflowRevisionNone,
            ),
            (
                TradeRevisionStatus::ChangeProposed,
                AppTextKey::TradeWorkflowRevisionChangeProposed,
            ),
            (
                TradeRevisionStatus::Updated,
                AppTextKey::TradeWorkflowRevisionUpdated,
            ),
            (
                TradeRevisionStatus::KeptAsPlaced,
                AppTextKey::TradeWorkflowRevisionKeptAsPlaced,
            ),
        ] {
            assert_eq!(trade_revision_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }

        for (status, key) in [
            (
                TradeFulfillmentStatus::Confirmed,
                AppTextKey::TradeWorkflowFulfillmentConfirmed,
            ),
            (
                TradeFulfillmentStatus::Preparing,
                AppTextKey::TradeWorkflowFulfillmentPreparing,
            ),
            (
                TradeFulfillmentStatus::ReadyForPickup,
                AppTextKey::TradeWorkflowFulfillmentReadyForPickup,
            ),
            (
                TradeFulfillmentStatus::OutForDelivery,
                AppTextKey::TradeWorkflowFulfillmentOutForDelivery,
            ),
            (
                TradeFulfillmentStatus::Delivered,
                AppTextKey::TradeWorkflowFulfillmentDelivered,
            ),
            (
                TradeFulfillmentStatus::Cancelled,
                AppTextKey::TradeWorkflowFulfillmentCancelled,
            ),
        ] {
            assert_eq!(trade_fulfillment_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }

        for (status, key) in [
            (
                TradeInventoryStatus::Available,
                AppTextKey::TradeWorkflowInventoryAvailable,
            ),
            (
                TradeInventoryStatus::Reserved,
                AppTextKey::TradeWorkflowInventoryReserved,
            ),
            (
                TradeInventoryStatus::SoldOut,
                AppTextKey::TradeWorkflowInventorySoldOut,
            ),
            (
                TradeInventoryStatus::NeedsReview,
                AppTextKey::TradeWorkflowInventoryNeedsReview,
            ),
        ] {
            assert_eq!(trade_inventory_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }

        for (status, key) in [
            (
                TradePaymentDisplayStatus::NotRecorded,
                AppTextKey::TradeWorkflowPaymentNotRecorded,
            ),
            (
                TradePaymentDisplayStatus::Pending,
                AppTextKey::TradeWorkflowPaymentPending,
            ),
            (
                TradePaymentDisplayStatus::Recorded,
                AppTextKey::TradeWorkflowPaymentRecorded,
            ),
            (
                TradePaymentDisplayStatus::Settled,
                AppTextKey::TradeWorkflowPaymentSettled,
            ),
            (
                TradePaymentDisplayStatus::NeedsReview,
                AppTextKey::TradeWorkflowPaymentNeedsReview,
            ),
        ] {
            assert_eq!(trade_payment_display_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }

        for (receipt, key) in [
            (
                TradeReceiptProjection {
                    event_id: "receipt-clean".to_owned(),
                    received: true,
                    issue: None,
                    received_at: 1_774_000_030,
                },
                AppTextKey::TradeWorkflowReceiptReceived,
            ),
            (
                TradeReceiptProjection {
                    event_id: "receipt-issue".to_owned(),
                    received: false,
                    issue: Some("items need review".to_owned()),
                    received_at: 1_774_000_031,
                },
                AppTextKey::TradeWorkflowReceiptNeedsReview,
            ),
        ] {
            assert_eq!(buyer_receipt_status_key(&receipt), key);
            assert!(!app_text(key).is_empty());
        }

        for (source, key) in [
            (
                TradeWorkflowSource::App,
                AppTextKey::TradeWorkflowProvenanceApp,
            ),
            (
                TradeWorkflowSource::Cli,
                AppTextKey::TradeWorkflowProvenanceCli,
            ),
            (
                TradeWorkflowSource::Relay,
                AppTextKey::TradeWorkflowProvenanceRelay,
            ),
            (
                TradeWorkflowSource::LocalEvents,
                AppTextKey::TradeWorkflowProvenanceLocalEvents,
            ),
            (
                TradeWorkflowSource::Unknown,
                AppTextKey::TradeWorkflowProvenanceUnknown,
            ),
        ] {
            assert_eq!(trade_workflow_source_key(source), key);
            assert!(!app_text(key).is_empty());
        }
    }

    #[test]
    fn trade_payment_display_status_keys_cover_passive_states() {
        for (status, key) in [
            (
                TradePaymentDisplayStatus::NotRecorded,
                AppTextKey::TradeWorkflowPaymentNotRecorded,
            ),
            (
                TradePaymentDisplayStatus::Pending,
                AppTextKey::TradeWorkflowPaymentPending,
            ),
            (
                TradePaymentDisplayStatus::Recorded,
                AppTextKey::TradeWorkflowPaymentRecorded,
            ),
            (
                TradePaymentDisplayStatus::Settled,
                AppTextKey::TradeWorkflowPaymentSettled,
            ),
            (
                TradePaymentDisplayStatus::NeedsReview,
                AppTextKey::TradeWorkflowPaymentNeedsReview,
            ),
        ] {
            assert_eq!(trade_payment_display_status_key(status), key);
            assert!(!app_text(key).is_empty());
        }
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
    fn home_auto_focus_target_tracks_startup_surface_contract() {
        assert_eq!(
            home_auto_focus_target(
                &summary_with_logged_out_phase(LoggedOutStartupPhase::ContinuePrompt),
                HomeAutoFocusState::default(),
            ),
            Some(HomeAutoFocusTarget::StartupContinue)
        );
        assert_eq!(
            home_auto_focus_target(
                &summary_with_logged_out_phase(LoggedOutStartupPhase::IdentityChoice),
                HomeAutoFocusState::default(),
            ),
            Some(HomeAutoFocusTarget::StartupGenerateKey)
        );
        assert_eq!(
            home_auto_focus_target(
                &summary_with_logged_out_phase(LoggedOutStartupPhase::SignerEntry),
                HomeAutoFocusState {
                    has_startup_signer_input: true,
                    startup_signer_input_is_editable: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::StartupSignerInput)
        );
        assert_eq!(
            home_auto_focus_target(
                &summary_with_logged_out_phase(LoggedOutStartupPhase::SignerEntry),
                HomeAutoFocusState {
                    has_startup_signer_input: true,
                    startup_signer_input_is_editable: false,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::StartupSignerBack)
        );
    }

    #[test]
    fn home_auto_focus_target_tracks_buyer_surface_contract() {
        let mut buyer_search = summary(
            HomeRoute::Personal,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        buyer_search.startup_gate = AppStartupGate::Personal;
        buyer_search.shell_projection = AppShellProjection::new(
            ActiveSurface::Personal,
            ShellSection::Personal(PersonalSection::Search),
        );
        assert_eq!(
            home_auto_focus_target(
                &buyer_search,
                HomeAutoFocusState {
                    has_personal_search_input: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::BuyerSearchInput)
        );

        let mut buyer_cart_order_review = buyer_search.clone();
        buyer_cart_order_review.shell_projection = AppShellProjection::new(
            ActiveSurface::Personal,
            ShellSection::Personal(PersonalSection::Cart),
        );
        assert_eq!(
            home_auto_focus_target(
                &buyer_cart_order_review,
                HomeAutoFocusState {
                    has_buyer_order_review_form: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::BuyerOrderReviewNameInput)
        );

        let order_id = OrderId::new();
        let farm_id = FarmId::new();
        let mut buyer_orders = buyer_search.clone();
        buyer_orders.shell_projection = AppShellProjection::new(
            ActiveSurface::Personal,
            ShellSection::Personal(PersonalSection::Orders),
        );
        buyer_orders.personal_projection.orders.list.rows = vec![BuyerOrdersListRow {
            order_id,
            farm_id,
            order_number: String::new(),
            farm_display_name: String::new(),
            fulfillment_summary: String::new(),
            status: BuyerOrderStatus::Placed,
            workflow: TradeWorkflowProjection::from_buyer_order_status(
                order_id,
                BuyerOrderStatus::Placed,
            ),
            repeat_demand: None,
        }];
        buyer_orders.personal_projection.orders.detail = Some(BuyerOrderDetailProjection {
            order_id,
            farm_id,
            order_number: String::new(),
            farm_display_name: String::new(),
            fulfillment_summary: String::new(),
            status: BuyerOrderStatus::Placed,
            items: Vec::new(),
            economics: TradeEconomicsProjection::default(),
            payment: TradePaymentDisplayStatus::NotRecorded,
            workflow: TradeWorkflowProjection::from_buyer_order_status(
                order_id,
                BuyerOrderStatus::Placed,
            ),
            validation_receipts: Vec::new(),
            order_note: None,
            repeat_demand: Some(RepeatDemandHandoffProjection {
                order_id,
                farm_id,
                eligibility: RepeatDemandEligibility::Eligible,
                available_item_count: 1,
                unavailable_item_count: 0,
            }),
        });
        assert_eq!(
            home_auto_focus_target(&buyer_orders, HomeAutoFocusState::default()),
            Some(HomeAutoFocusTarget::BuyerOrderRepeatDemand)
        );
        assert_eq!(
            home_auto_focus_target(
                &buyer_orders,
                HomeAutoFocusState {
                    has_buyer_receipt_issue_form: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::BuyerReceiptIssueInput)
        );
    }

    #[test]
    fn home_auto_focus_target_tracks_farmer_surface_contract() {
        let mut onboarding = summary(
            HomeRoute::FarmSetupOnboarding,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        onboarding.startup_gate = AppStartupGate::Farmer;
        onboarding.shell_projection = AppShellProjection::new(
            ActiveSurface::Farmer,
            ShellSection::Farmer(FarmerSection::Today),
        );
        assert_eq!(
            home_auto_focus_target(&onboarding, HomeAutoFocusState::default()),
            Some(HomeAutoFocusTarget::FarmerSetupStart)
        );

        let farm_id = FarmId::new();
        let incomplete_farm = FarmSummary {
            farm_id,
            display_name: String::new(),
            readiness: FarmReadiness::Incomplete,
        };
        let incomplete_today = summary(
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
        );
        assert_eq!(
            home_auto_focus_target(&incomplete_today, HomeAutoFocusState::default()),
            Some(HomeAutoFocusTarget::FarmerSetupContinue)
        );

        let saved_farm = FarmSummary {
            farm_id: FarmId::new(),
            display_name: String::new(),
            readiness: FarmReadiness::Ready,
        };
        let mut products = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::from_saved_farm(saved_farm.clone()),
        );
        products.startup_gate = AppStartupGate::Farmer;
        products.shell_projection = AppShellProjection::new(
            ActiveSurface::Farmer,
            ShellSection::Farmer(FarmerSection::Products),
        );
        assert_eq!(
            home_auto_focus_target(
                &products,
                HomeAutoFocusState {
                    has_products_search_input: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::ProductsSearchInput)
        );
        assert_eq!(
            home_auto_focus_target(
                &products,
                HomeAutoFocusState {
                    has_product_editor_form: true,
                    ..HomeAutoFocusState::default()
                },
            ),
            Some(HomeAutoFocusTarget::ProductEditorTitleInput)
        );

        let mut orders = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::from_saved_farm(saved_farm),
        );
        orders.startup_gate = AppStartupGate::Farmer;
        orders.shell_projection = AppShellProjection::new(
            ActiveSurface::Farmer,
            ShellSection::Farmer(FarmerSection::Orders),
        );
        let farmer_order_id = OrderId::new();
        let farmer_order_farm_id = FarmId::new();
        orders.orders_projection.list.rows = vec![OrdersListRow {
            order_id: farmer_order_id,
            farm_id: farmer_order_farm_id,
            fulfillment_window_id: None,
            order_number: String::new(),
            customer_display_name: String::new(),
            fulfillment_window_label: None,
            pickup_location_label: None,
            status: OrderStatus::Scheduled,
            workflow: TradeWorkflowProjection::from_order_status(
                farmer_order_id,
                OrderStatus::Scheduled,
            ),
            primary_action: Some(OrderPrimaryAction::PublishPreparing),
            fulfillment_actions: OrderFulfillmentAction::ALL.to_vec(),
        }];
        orders.orders_projection.detail = Some(OrderDetailProjection {
            order_id: farmer_order_id,
            farm_id: farmer_order_farm_id,
            order_number: String::new(),
            customer_display_name: String::new(),
            status: OrderStatus::Scheduled,
            fulfillment_window_id: None,
            fulfillment_window_label: None,
            pickup_location_label: None,
            items: Vec::new(),
            economics: TradeEconomicsProjection::default(),
            payment: TradePaymentDisplayStatus::NotRecorded,
            workflow: TradeWorkflowProjection::from_order_status(
                farmer_order_id,
                OrderStatus::Scheduled,
            ),
            validation_receipts: Vec::new(),
            primary_action: Some(OrderPrimaryAction::PublishPreparing),
            fulfillment_actions: OrderFulfillmentAction::ALL.to_vec(),
            recoveries: Vec::new(),
        });
        assert_eq!(
            home_auto_focus_target(&orders, HomeAutoFocusState::default()),
            Some(HomeAutoFocusTarget::OrdersDetailPublishFulfillmentFirst)
        );
    }

    #[test]
    fn settings_auto_focus_target_tracks_panel_contract() {
        let runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        assert_eq!(
            settings_auto_focus_target(SettingsPanelViewKey::Account, None, &runtime),
            Some(SettingsAutoFocusTarget::AccountAdd)
        );
        assert_eq!(
            settings_auto_focus_target(SettingsPanelViewKey::Farm, None, &runtime),
            Some(SettingsAutoFocusTarget::Navigation(
                SettingsPanelViewKey::Farm
            ))
        );
        assert_eq!(
            settings_auto_focus_target(SettingsPanelViewKey::Settings, None, &runtime),
            Some(SettingsAutoFocusTarget::Navigation(
                SettingsPanelViewKey::Settings
            ))
        );

        let mut about_enabled = runtime.clone();
        about_enabled.sync_status.account_id = Some("guest".to_owned());
        assert_eq!(
            settings_auto_focus_target(SettingsPanelViewKey::About, None, &about_enabled),
            Some(SettingsAutoFocusTarget::AboutRefresh)
        );
        assert_eq!(
            settings_auto_focus_target(SettingsPanelViewKey::About, None, &runtime),
            Some(SettingsAutoFocusTarget::Navigation(
                SettingsPanelViewKey::About
            ))
        );
    }

    #[test]
    fn settings_general_rows_read_runtime_projection_values() {
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime
            .shell_projection
            .settings
            .general
            .allow_relay_connections = false;
        runtime.shell_projection.settings.general.use_media_servers = true;
        runtime.shell_projection.settings.general.use_nip05 = false;
        runtime.shell_projection.settings.general.launch_at_login = true;

        let state = settings_preferences_general_row_state(&runtime);

        assert!(!state.allow_relay_connections);
        assert!(state.use_media_servers);
        assert!(!state.use_nip05);
        assert!(state.launch_at_login);
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
            reminders: Default::default(),
            totals_by_product: Vec::new(),
            pack_list: Vec::new(),
            pickup_roster: Vec::new(),
        };

        assert!(farmer_pack_day_available(&runtime));
    }

    #[test]
    fn pack_day_export_action_enabled_requires_a_window_and_exportable_rows() {
        let farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert!(!pack_day_export_action_enabled(&runtime));
        assert_eq!(
            pack_day_export_status_presentation(&runtime),
            PackDayExportStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.offline,
                title_key: AppTextKey::PackDayExportUnavailableTitle,
                body_key: AppTextKey::PackDayExportUnavailableBody,
            }
        );

        runtime.pack_day_projection.projection = PackDayProjection {
            fulfillment_window: Some(FulfillmentWindowSummary {
                fulfillment_window_id,
                farm_id,
                starts_at: String::new(),
                ends_at: String::new(),
            }),
            reminders: Default::default(),
            totals_by_product: Vec::new(),
            pack_list: Vec::new(),
            pickup_roster: Vec::new(),
        };

        assert!(!pack_day_export_action_enabled(&runtime));

        runtime.pack_day_projection.projection.totals_by_product = vec![PackDayProductTotalRow {
            title: "Salad mix".to_owned(),
            quantity_display: "2 bags".to_owned(),
        }];

        assert!(pack_day_export_action_enabled(&runtime));
        assert_eq!(
            pack_day_export_status_presentation(&runtime),
            PackDayExportStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayExportReadyTitle,
                body_key: AppTextKey::PackDayExportReadyBody,
            }
        );

        runtime.pack_day_projection.export = PackDayExportProjection::running(
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id),
        );
        assert!(!pack_day_export_action_enabled(&runtime));
        assert_eq!(
            pack_day_export_action_label_key(&runtime.pack_day_projection.export),
            AppTextKey::PackDayExportActionRunning
        );
        assert_eq!(
            pack_day_export_status_presentation(&runtime),
            PackDayExportStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayExportRunningTitle,
                body_key: AppTextKey::PackDayExportRunningBody,
            }
        );
    }

    #[test]
    fn pack_day_export_detail_rows_surface_bundle_and_failure_details() {
        let fulfillment_window_id = FulfillmentWindowId::new();
        let bundle = PackDayExportBundle {
            fulfillment_window_id,
            export_instance_id: radroots_studio_app_view::PackDayExportInstanceId::new(),
            generated_at_utc: "2026-04-23T15:00:00Z".to_owned(),
            bundle_directory: "exports/pack_day/window-1/20260423T150000Z".to_owned(),
            artifacts: vec![
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PackSheet,
                    relative_path: "pack_sheet.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PickupRoster,
                    relative_path: "pickup_roster.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::CustomerLabels,
                    relative_path: "customer_labels.txt".to_owned(),
                },
            ],
        };
        let request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);

        let rows = pack_day_export_detail_rows(&PackDayExportProjection::succeeded(
            request.clone(),
            bundle.clone(),
        ));
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            LabelValueRow::new(
                app_text(AppTextKey::PackDayExportFolderLabel),
                "exports/pack_day/window-1/20260423T150000Z"
            )
        );
        assert_eq!(
            rows[1],
            LabelValueRow::new(
                app_text(AppTextKey::PackDayExportFilesLabel),
                "pack_sheet.txt, pickup_roster.txt, customer_labels.txt"
            )
        );
        assert_eq!(
            pack_day_export_artifact_names(&bundle),
            "pack_sheet.txt, pickup_roster.txt, customer_labels.txt"
        );

        let failed = PackDayExportProjection::failed(request, "disk unavailable");
        assert_eq!(
            pack_day_export_detail_rows(&failed),
            vec![LabelValueRow::new(
                app_text(AppTextKey::PackDayExportErrorLabel),
                "disk unavailable"
            )]
        );
        assert_eq!(
            pack_day_export_status_presentation(&DesktopAppRuntimeSummary {
                pack_day_projection: radroots_studio_app_state::PackDayScreenProjection {
                    export: failed,
                    ..Default::default()
                },
                ..summary(
                    HomeRoute::Today,
                    TodayAgendaProjection::default(),
                    FarmSetupProjection::default(),
                )
            }),
            PackDayExportStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayExportFailedTitle,
                body_key: AppTextKey::PackDayExportFailedBody,
            }
        );
    }

    #[test]
    fn pack_day_host_handoff_actions_only_surface_after_a_successful_export() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert!(pack_day_host_handoff_action_presentations(&runtime).is_empty());

        runtime.pack_day_projection.export = PackDayExportProjection::succeeded(
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id),
            bundle,
        );

        assert_eq!(
            pack_day_host_handoff_action_presentations(&runtime),
            vec![
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::RevealBundle,
                    label_key: AppTextKey::PackDayHostHandoffRevealAction,
                    enabled: true,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPackSheet,
                    label_key: AppTextKey::PackDayHostHandoffOpenPackSheetAction,
                    enabled: true,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPickupRoster,
                    label_key: AppTextKey::PackDayHostHandoffOpenPickupRosterAction,
                    enabled: true,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenCustomerLabels,
                    label_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction,
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn pack_day_host_handoff_running_and_failure_postures_track_the_active_request() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let export_request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);
        let reveal_request =
            PackDayHostHandoffRequest::for_bundle(PackDayHostHandoffKind::RevealBundle, &bundle);
        let open_request = PackDayHostHandoffRequest::for_bundle(
            PackDayHostHandoffKind::OpenCustomerLabels,
            &bundle,
        );
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime.pack_day_projection.export =
            PackDayExportProjection::succeeded(export_request, bundle);

        runtime.pack_day_projection.host_handoff =
            PackDayHostHandoffProjection::running(reveal_request);
        assert_eq!(
            pack_day_host_handoff_action_presentations(&runtime),
            vec![
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::RevealBundle,
                    label_key: AppTextKey::PackDayHostHandoffRevealActionRunning,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPackSheet,
                    label_key: AppTextKey::PackDayHostHandoffOpenPackSheetAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPickupRoster,
                    label_key: AppTextKey::PackDayHostHandoffOpenPickupRosterAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenCustomerLabels,
                    label_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );
        assert_eq!(
            pack_day_host_handoff_status_presentation(&runtime),
            Some(PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayHostHandoffRevealRunningTitle,
            })
        );

        runtime.pack_day_projection.host_handoff =
            PackDayHostHandoffProjection::failed(open_request, "finder unavailable");
        assert_eq!(
            runtime.pack_day_projection.host_handoff.status,
            PackDayHostHandoffStatus::Failed
        );
        assert_eq!(
            pack_day_host_handoff_status_presentation(&runtime),
            Some(PackDayHostHandoffStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsFailedTitle,
            })
        );
    }

    #[test]
    fn pack_day_host_handoff_actions_disable_missing_artifacts_even_after_export_success() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        runtime.pack_day_projection.export = PackDayExportProjection::succeeded(
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id),
            bundle,
        );

        assert_eq!(
            pack_day_host_handoff_action_presentations(&runtime),
            vec![
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::RevealBundle,
                    label_key: AppTextKey::PackDayHostHandoffRevealAction,
                    enabled: true,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPackSheet,
                    label_key: AppTextKey::PackDayHostHandoffOpenPackSheetAction,
                    enabled: true,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPickupRoster,
                    label_key: AppTextKey::PackDayHostHandoffOpenPickupRosterAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenCustomerLabels,
                    label_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn pack_day_print_actions_only_surface_after_a_successful_export() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert!(pack_day_print_action_presentations(&runtime).is_empty());

        runtime.pack_day_projection.export = PackDayExportProjection::succeeded(
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id),
            bundle,
        );

        assert_eq!(
            pack_day_print_action_presentations(&runtime),
            vec![
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPackSheet,
                    label_key: AppTextKey::PackDayPrintPackSheetAction,
                    enabled: true,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPickupRoster,
                    label_key: AppTextKey::PackDayPrintPickupRosterAction,
                    enabled: true,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintCustomerLabels,
                    label_key: AppTextKey::PackDayPrintCustomerLabelsAction,
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn pack_day_batch_workflow_action_only_surfaces_after_a_successful_export() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        assert_eq!(pack_day_batch_print_action_presentation(&runtime), None);

        runtime.pack_day_projection.export = PackDayExportProjection::succeeded(
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id),
            bundle,
        );

        assert_eq!(
            pack_day_batch_print_action_presentation(&runtime),
            Some(PackDayBatchPrintActionPresentation {
                label_key: AppTextKey::PackDayBatchPrintAction,
                enabled: true,
            })
        );
    }

    #[test]
    fn pack_day_batch_print_running_disables_conflicting_pack_day_actions() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let export_request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);
        let batch_request = PackDayBatchPrintRequest::for_bundle(&bundle);
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime.pack_day_projection.export =
            PackDayExportProjection::succeeded(export_request, bundle);
        runtime.pack_day_projection.batch_print =
            PackDayBatchPrintProjection::running(batch_request);

        assert_eq!(
            pack_day_batch_print_action_presentation(&runtime),
            Some(PackDayBatchPrintActionPresentation {
                label_key: AppTextKey::PackDayBatchPrintActionRunning,
                enabled: false,
            })
        );
        assert!(
            pack_day_print_action_presentations(&runtime)
                .into_iter()
                .all(|action| !action.enabled)
        );
        assert!(
            pack_day_host_handoff_action_presentations(&runtime)
                .into_iter()
                .all(|action| !action.enabled)
        );
    }

    #[test]
    fn pack_day_batch_print_status_tracks_outcomes_and_failed_artifacts() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let export_request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);
        let batch_request = PackDayBatchPrintRequest::for_bundle(&bundle);
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime.pack_day_projection.export =
            PackDayExportProjection::succeeded(export_request, bundle);

        runtime.pack_day_projection.batch_print =
            PackDayBatchPrintProjection::running(batch_request.clone());
        assert_eq!(
            pack_day_batch_print_status_presentation(&runtime),
            Some(PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayBatchPrintQueuedTitle,
            })
        );

        runtime.pack_day_projection.batch_print =
            PackDayBatchPrintProjection::succeeded(batch_request.clone());
        assert_eq!(
            pack_day_batch_print_status_presentation(&runtime),
            Some(PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.online,
                title_key: AppTextKey::PackDayBatchPrintSucceededTitle,
            })
        );

        runtime.pack_day_projection.batch_print = PackDayBatchPrintProjection::failed(
            batch_request.clone(),
            Some(PackDayBatchPrintArtifact::from_print_kind(
                PackDayPrintKind::PrintPickupRoster,
            )),
            PackDayBatchPrintFailureKind::QueueExit,
        );
        assert_eq!(
            pack_day_batch_print_status_presentation(&runtime),
            Some(PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintPickupRosterFailedTitle,
            })
        );

        runtime.pack_day_projection.batch_print = PackDayBatchPrintProjection::failed(
            batch_request.clone(),
            None,
            PackDayBatchPrintFailureKind::Preflight,
        );
        assert_eq!(
            pack_day_batch_print_status_presentation(&runtime),
            Some(PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayBatchPrintFailedPreflightTitle,
            })
        );

        runtime.pack_day_projection.batch_print = PackDayBatchPrintProjection::failed(
            batch_request,
            Some(PackDayBatchPrintArtifact::from_print_kind(
                PackDayPrintKind::PrintCustomerLabels,
            )),
            PackDayBatchPrintFailureKind::CustomerLabelsAvery5160Overflow,
        );
        assert_eq!(
            pack_day_batch_print_status_presentation(&runtime),
            Some(PackDayBatchPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayBatchPrintCustomerLabelsAvery5160OverflowFailedTitle,
            })
        );
    }

    #[test]
    fn pack_day_print_running_and_failure_postures_track_the_active_request() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let export_request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);
        let print_request =
            PackDayPrintRequest::for_bundle(PackDayPrintKind::PrintPackSheet, &bundle);
        let failed_request =
            PackDayPrintRequest::for_bundle(PackDayPrintKind::PrintCustomerLabels, &bundle);
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        runtime.pack_day_projection.export =
            PackDayExportProjection::succeeded(export_request, bundle.clone());

        runtime.pack_day_projection.print = PackDayPrintProjection::running(print_request);
        assert_eq!(
            pack_day_print_action_presentations(&runtime),
            vec![
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPackSheet,
                    label_key: AppTextKey::PackDayPrintPackSheetActionRunning,
                    enabled: false,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPickupRoster,
                    label_key: AppTextKey::PackDayPrintPickupRosterAction,
                    enabled: false,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintCustomerLabels,
                    label_key: AppTextKey::PackDayPrintCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );
        assert_eq!(
            pack_day_print_status_presentation(&runtime),
            Some(PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.foundation.text.accent,
                title_key: AppTextKey::PackDayPrintPackSheetQueuedTitle,
            })
        );
        assert_eq!(
            pack_day_host_handoff_action_presentations(&runtime),
            vec![
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::RevealBundle,
                    label_key: AppTextKey::PackDayHostHandoffRevealAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPackSheet,
                    label_key: AppTextKey::PackDayHostHandoffOpenPackSheetAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenPickupRoster,
                    label_key: AppTextKey::PackDayHostHandoffOpenPickupRosterAction,
                    enabled: false,
                },
                PackDayHostHandoffActionPresentation {
                    kind: PackDayHostHandoffKind::OpenCustomerLabels,
                    label_key: AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );

        runtime.pack_day_projection.print = PackDayPrintProjection::failed(failed_request);
        assert_eq!(
            runtime.pack_day_projection.print.status,
            PackDayPrintStatus::Failed
        );
        assert_eq!(
            pack_day_print_status_presentation(&runtime),
            Some(PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintCustomerLabelsFailedTitle,
            })
        );

        let overflow_request =
            PackDayPrintRequest::for_bundle(PackDayPrintKind::PrintCustomerLabels, &bundle);
        runtime.pack_day_projection.print = PackDayPrintProjection::failed_with_kind(
            overflow_request,
            PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow,
        );
        assert_eq!(
            pack_day_print_status_presentation(&runtime),
            Some(PackDayPrintStatusPresentation {
                indicator_color: APP_UI_THEME.components.app_status_indicator.attention,
                title_key: AppTextKey::PackDayPrintCustomerLabelsAvery5160OverflowFailedTitle,
            })
        );
    }

    #[test]
    fn pack_day_print_actions_disable_missing_artifacts_and_host_handoff_runs() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        let bundle = sample_pack_day_bundle(temp_dir.path());
        let fulfillment_window_id = bundle.fulfillment_window_id;
        let export_request =
            radroots_studio_app_state::PackDayExportRequest::for_fulfillment_window(fulfillment_window_id);
        let host_handoff_request =
            PackDayHostHandoffRequest::for_bundle(PackDayHostHandoffKind::RevealBundle, &bundle);
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );

        runtime.pack_day_projection.export =
            PackDayExportProjection::succeeded(export_request, bundle.clone());
        assert_eq!(
            pack_day_print_action_presentations(&runtime),
            vec![
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPackSheet,
                    label_key: AppTextKey::PackDayPrintPackSheetAction,
                    enabled: true,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPickupRoster,
                    label_key: AppTextKey::PackDayPrintPickupRosterAction,
                    enabled: false,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintCustomerLabels,
                    label_key: AppTextKey::PackDayPrintCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );

        runtime.pack_day_projection.host_handoff =
            PackDayHostHandoffProjection::running(host_handoff_request);
        assert_eq!(
            pack_day_print_action_presentations(&runtime),
            vec![
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPackSheet,
                    label_key: AppTextKey::PackDayPrintPackSheetAction,
                    enabled: false,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintPickupRoster,
                    label_key: AppTextKey::PackDayPrintPickupRosterAction,
                    enabled: false,
                },
                PackDayPrintActionPresentation {
                    kind: PackDayPrintKind::PrintCustomerLabels,
                    label_key: AppTextKey::PackDayPrintCustomerLabelsAction,
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn sidebar_navigation_keeps_destinations_stable() {
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
                FarmerSection::Today,
                FarmerSection::Products,
                FarmerSection::Orders,
            ]
        );
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::Orders, true, false),
            vec![
                FarmerSection::Today,
                FarmerSection::Products,
                FarmerSection::Orders,
            ]
        );
        assert_eq!(
            home_sidebar_navigation_sections(FarmerSection::PackDay, true, true),
            vec![
                FarmerSection::Today,
                FarmerSection::Products,
                FarmerSection::Orders,
                FarmerSection::PackDay,
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
        assert_eq!(
            product_display_title(""),
            app_text(AppTextKey::ProductsUntitledDraft)
        );
        assert_eq!(
            product_display_title("  "),
            app_text(AppTextKey::ProductsUntitledDraft)
        );
        assert_eq!(product_display_title("Salad mix"), "Salad mix");
    }

    #[test]
    fn startup_signer_preview_summary_surfaces_parsed_signer_details() {
        let preview = startup_signer_preview_summary(
            "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
        )
        .expect("preview");

        assert_eq!(
            preview.source_label,
            app_text(AppTextKey::HomeSetupSignerSourceValueBunkerUri)
        );
        assert!(preview.signer_npub.starts_with("npub1"));
        assert_eq!(preview.relays_label, "wss://relay.radroots.example");
        assert_eq!(
            preview.permissions_label,
            format!(
                "{}, {}",
                app_text(AppTextKey::HomeSetupSignerPermissionSignEventKind1),
                app_text(AppTextKey::HomeSetupSignerPermissionSwitchRelays)
            )
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
            format!(
                "{}, {}",
                app_text(AppTextKey::HomeSetupSignerPermissionSignEventKind1),
                app_text(AppTextKey::HomeSetupSignerPermissionSwitchRelays)
            )
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
    fn startup_signer_notice_copy_maps_known_signer_failures() {
        assert_eq!(
            startup_notice_text("enter a bunker or discovery url to continue"),
            app_text(AppTextKey::HomeSetupSignerErrorEnterSource)
        );
        assert_eq!(
            startup_notice_text(
                "enter a bunker or discovery url from the signer; raw nostrconnect client uris are signer-side only"
            ),
            app_text(AppTextKey::HomeSetupSignerErrorUseSignerUri)
        );
        assert_eq!(
            startup_notice_text("discovery url does not contain a remote signer uri"),
            app_text(AppTextKey::HomeSetupSignerErrorMissingDiscoveryUri)
        );
        assert_eq!(
            startup_notice_text("invalid discovery url: relative URL without a base"),
            app_text(AppTextKey::HomeSetupSignerErrorInvalidDiscoveryUrl)
        );
        assert_eq!(
            startup_notice_text("invalid remote signer uri: invalid public key"),
            app_text(AppTextKey::HomeSetupSignerErrorInvalidRemoteSignerUri)
        );
        assert_eq!(
            startup_notice_text("a remote signer connection is already pending approval"),
            app_text(AppTextKey::HomeSetupSignerErrorPendingApprovalExists)
        );
        assert_eq!(
            startup_notice_text("remote signer connection failed: relay refused the request"),
            app_text(AppTextKey::HomeSetupSignerErrorConnectionFailed)
        );
        assert_eq!(
            startup_notice_text("failed to add relay `{relay_url}`: {error}"),
            app_text(AppTextKey::HomeSetupErrorStartupFailed)
        );
    }

    #[test]
    fn startup_issue_copy_fails_closed_to_a_localized_summary() {
        assert_eq!(
            startup_issue_summary_text("runtime unavailable"),
            app_text(AppTextKey::HomeSetupIssueUnavailableBody)
        );
        assert_eq!(
            startup_issue_summary_text("desktop runtime roots require HOME for macos"),
            app_text(AppTextKey::HomeSetupIssueUnavailableBody)
        );
    }

    #[test]
    fn reminder_action_target_prefers_order_detail_before_pack_day() {
        let order_id = radroots_studio_app_view::OrderId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();

        assert_eq!(
            reminder_action_target(&fixture_reminder(
                Some(order_id),
                Some(fulfillment_window_id),
                ReminderKind::OrderAction,
                ReminderUrgency::DueSoon,
            )),
            Some(ReminderActionTarget::OrderDetail(order_id))
        );
        assert_eq!(
            reminder_action_target(&fixture_reminder(
                None,
                Some(fulfillment_window_id),
                ReminderKind::FulfillmentWindow,
                ReminderUrgency::Upcoming,
            )),
            Some(ReminderActionTarget::PackDay(fulfillment_window_id))
        );
        assert_eq!(
            reminder_action_target(&fixture_reminder(
                None,
                None,
                ReminderKind::SyncImpact,
                ReminderUrgency::Blocking,
            )),
            None
        );
    }

    #[test]
    fn reminder_urgency_helpers_follow_the_surface_contract() {
        assert_eq!(
            reminder_urgency_key(ReminderUrgency::Upcoming),
            AppTextKey::ReminderUrgencyUpcoming
        );
        assert_eq!(
            reminder_urgency_key(ReminderUrgency::DueSoon),
            AppTextKey::ReminderUrgencyDueSoon
        );
        assert_eq!(
            reminder_urgency_color(ReminderUrgency::Upcoming),
            APP_UI_THEME.components.app_status_indicator.offline
        );
        assert_eq!(
            reminder_urgency_color(ReminderUrgency::DueSoon),
            APP_UI_THEME.foundation.text.accent
        );
        assert_eq!(
            reminder_urgency_color(ReminderUrgency::Blocking),
            APP_UI_THEME.components.app_status_indicator.attention
        );
    }

    #[test]
    fn reminder_deadline_text_uses_the_typed_due_label() {
        let reminder = fixture_reminder(
            None,
            Some(FulfillmentWindowId::new()),
            ReminderKind::FulfillmentWindow,
            ReminderUrgency::Upcoming,
        );

        assert_eq!(
            reminder_deadline_text(&reminder),
            format!("{}: {}", app_text(AppTextKey::ReminderDeadlineLabel), "0")
        );
    }

    #[test]
    fn reminder_delivery_state_key_matches_the_local_presentation_contract() {
        assert_eq!(
            reminder_delivery_state_key(ReminderDeliveryState::Scheduled),
            AppTextKey::ReminderDeliveryStateScheduled
        );
        assert_eq!(
            reminder_delivery_state_key(ReminderDeliveryState::Presented),
            AppTextKey::ReminderDeliveryStatePresented
        );
        assert_eq!(
            reminder_delivery_state_key(ReminderDeliveryState::Acknowledged),
            AppTextKey::ReminderDeliveryStateAcknowledged
        );
        assert_eq!(
            reminder_delivery_state_key(ReminderDeliveryState::Resolved),
            AppTextKey::ReminderDeliveryStateResolved
        );
    }

    #[test]
    fn presented_farmer_reminder_prefers_the_highest_priority_presented_item() {
        let mut runtime = summary(
            HomeRoute::Today,
            TodayAgendaProjection::default(),
            FarmSetupProjection::default(),
        );
        let due_soon = fixture_reminder(
            None,
            Some(FulfillmentWindowId::new()),
            ReminderKind::FulfillmentWindow,
            ReminderUrgency::DueSoon,
        );
        let blocking = fixture_reminder(
            None,
            None,
            ReminderKind::SyncImpact,
            ReminderUrgency::Blocking,
        );

        runtime
            .today_projection
            .reminders
            .items
            .push(ReminderDeadlineProjection {
                delivery_state: ReminderDeliveryState::Presented,
                ..due_soon
            });
        runtime
            .orders_projection
            .reminders
            .items
            .push(ReminderDeadlineProjection {
                delivery_state: ReminderDeliveryState::Presented,
                ..blocking.clone()
            });

        assert_eq!(
            presented_farmer_reminder(&runtime)
                .expect("presented reminder")
                .reminder_id,
            blocking.reminder_id
        );
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
    fn about_conflict_review_helpers_surface_actions_and_details_truthfully() {
        let blocking_conflict = DesktopAppSyncConflictSummary {
            conflict_id: String::new(),
            conflict: SyncConflict {
                aggregate: SyncAggregateRef::Farm(FarmId::new()),
                kind: SyncConflictKind::RevisionMismatch,
                severity: SyncConflictSeverity::Blocking,
                resolution: SyncConflictResolutionStatus::Unresolved,
                local_payload_json: String::new(),
                remote_payload_json: Some(String::new()),
                detected_at: "0".to_owned(),
                resolved_at: None,
            },
        };
        let review_conflict = DesktopAppSyncConflictSummary {
            conflict_id: String::new(),
            conflict: SyncConflict {
                aggregate: SyncAggregateRef::Order(radroots_studio_app_view::OrderId::new()),
                kind: SyncConflictKind::RemoteValidationReject,
                severity: SyncConflictSeverity::ReviewRequired,
                resolution: SyncConflictResolutionStatus::Unresolved,
                local_payload_json: String::new(),
                remote_payload_json: Some(String::new()),
                detected_at: "0".to_owned(),
                resolved_at: None,
            },
        };
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
            conflicts: vec![blocking_conflict.clone(), review_conflict.clone()],
        };

        assert_eq!(
            about_conflict_review_body_key(&runtime.sync_status),
            AppTextKey::SettingsAboutConflictReviewBlocking
        );
        assert!(!about_manual_refresh_enabled(&runtime.sync_status));

        let blocking_actions = about_conflict_action_specs(&blocking_conflict.conflict);
        assert_eq!(
            blocking_actions,
            vec![
                (
                    AppTextKey::SettingsAboutConflictAcceptLocalAction,
                    SyncConflictResolutionStatus::AcceptedLocal,
                ),
                (
                    AppTextKey::SettingsAboutConflictAcceptRemoteAction,
                    SyncConflictResolutionStatus::AcceptedRemote,
                ),
            ]
        );

        let review_actions = about_conflict_action_specs(&review_conflict.conflict);
        assert_eq!(
            review_actions,
            vec![
                (
                    AppTextKey::SettingsAboutConflictAcceptLocalAction,
                    SyncConflictResolutionStatus::AcceptedLocal,
                ),
                (
                    AppTextKey::SettingsAboutConflictAcceptRemoteAction,
                    SyncConflictResolutionStatus::AcceptedRemote,
                ),
                (
                    AppTextKey::SettingsAboutConflictDismissAction,
                    SyncConflictResolutionStatus::Dismissed,
                ),
            ]
        );

        let rows = about_conflict_detail_rows(&blocking_conflict);
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncConflictAggregate)
                && row.value == about_conflict_aggregate_text(&blocking_conflict.conflict)
        }));
        assert!(rows.iter().any(|row| {
            row.label == app_text(AppTextKey::MetadataSyncConflictResolution)
                && row.value == app_text(AppTextKey::ValueSyncConflictResolutionUnresolved)
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
            farm_rules_projection: Default::default(),
            farm_readiness_projection,
            farm_setup_projection,
            today_projection,
            products_projection: Default::default(),
            orders_projection: Default::default(),
            pack_day_projection: Default::default(),
            reminder_log: Default::default(),
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

    fn fixture_reminder(
        order_id: Option<radroots_studio_app_view::OrderId>,
        fulfillment_window_id: Option<FulfillmentWindowId>,
        kind: ReminderKind,
        urgency: ReminderUrgency,
    ) -> ReminderDeadlineProjection {
        ReminderDeadlineProjection {
            reminder_id: ReminderId::new(),
            farm_id: FarmId::new(),
            order_id,
            fulfillment_window_id,
            kind,
            surface: ReminderSurface::Orders,
            urgency,
            title: String::new(),
            detail: String::new(),
            deadline_at: "0".to_owned(),
            action_label: None,
            delivery_state: ReminderDeliveryState::Scheduled,
        }
    }
}
