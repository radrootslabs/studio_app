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
    AppStartupGate, FarmOrderMethod, FarmReadiness, FarmSetupBlocker, FarmSetupDraft, FarmSummary,
    FarmerSection, FulfillmentWindowSummary, LoggedOutStartupPhase, OrderListRow,
    ProductAttentionState, ProductEditorDraft, ProductId, ProductListRow, ProductPublishBlocker,
    ProductStatus, ProductsFilter, ProductsListRow, ProductsSort, ShellSection,
    TodayAgendaProjection, TodaySetupTaskKind,
};
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
    logged_in_view: LoggedInHomeView,
    farm_setup_form: Option<FarmSetupFormState>,
    products_search: Option<ProductsSearchState>,
    products_stock_editor: Option<ProductsStockEditorState>,
    product_editor_form: Option<ProductEditorFormState>,
    relay_client: Option<RadrootsNostrClient>,
}

impl HomeView {
    pub fn new(runtime: DesktopAppRuntime) -> Self {
        Self {
            runtime,
            startup_view: StartupHomeView::new(),
            startup_signer_entry: None,
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

    fn show_startup_identity_choice(&mut self, cx: &mut Context<Self>) {
        self.startup_view.clear_error();
        if self.runtime.show_startup_identity_choice() {
            cx.notify();
        }
    }

    fn show_startup_signer_entry(&mut self, cx: &mut Context<Self>) {
        self.startup_view.clear_error();
        if self.runtime.show_startup_signer_entry() {
            cx.notify();
        }
    }

    fn start_generate_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.runtime.begin_generate_key_startup() {
            return;
        }

        self.startup_view.clear_error();
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
                self.startup_view.clear_error();
                if !self.generate_local_account(cx) {
                    self.show_startup_identity_choice(cx);
                }
            }
            Err(error) => {
                self.runtime.show_startup_identity_choice();
                self.startup_view.fail_starting(error);
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

        let value = state.read(cx).value().to_string();
        if self.runtime.set_startup_signer_source_input(value.as_str()) {
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
                    cx.listener(|this, _, _, cx| this.show_startup_identity_choice(cx)),
                    cx.listener(|this, _, window, cx| this.start_generate_key(window, cx)),
                    cx.listener(|this, _, _, cx| this.show_startup_signer_entry(cx)),
                    cx.listener(|_, _, _, _| {}),
                    cx.listener(|this, _, _, cx| this.show_startup_identity_choice(cx)),
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
    relay_error: Option<String>,
}

impl StartupHomeView {
    fn new() -> Self {
        Self { relay_error: None }
    }

    fn fail_starting(&mut self, error: String) {
        self.relay_error = Some(error);
    }

    fn clear_error(&mut self) {
        self.relay_error = None;
    }

    fn render(
        &self,
        runtime: &DesktopAppRuntimeSummary,
        signer_entry: Option<&StartupSignerEntryState>,
        on_continue: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_generate_key: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_connect_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> impl IntoElement {
        startup_home_shell(
            runtime,
            self.relay_error.as_deref(),
            signer_entry,
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

pub struct SettingsWindowView {
    runtime: DesktopAppRuntime,
}

impl SettingsWindowView {
    pub fn new(runtime: DesktopAppRuntime, initial_view: SettingsPanelViewKey) -> Self {
        let _ = initial_view;
        Self { runtime }
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        if self.runtime.select_settings_section(view) {
            cx.notify();
        }
    }

    fn selected_view(&self) -> SettingsPanelViewKey {
        self.runtime.selected_settings_section()
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

    fn settings_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_label_width_px = 72.0;
        let form_max_width_px = 420.0;
        let runtime_summary = self.runtime.summary();
        let general_settings = runtime_summary.shell_projection.settings.general;
        let general_allow_relay_connections = general_settings.allow_relay_connections;
        let general_use_media_servers = general_settings.use_media_servers;
        let general_use_nip05 = general_settings.use_nip05;
        let general_launch_at_login = general_settings.launch_at_login;

        div()
            .size_full()
            .p(px(APP_UI_THEME.layout.settings_content_padding_px))
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .h_full()
                    .w_full()
                    .max_w(px(form_max_width_px))
                    .flex()
                    .items_start()
                    .gap(px(APP_UI_THEME.layout.settings_section_gap_px))
                    .child(
                        div()
                            .w(px(section_label_width_px))
                            .text_size(px(APP_UI_THEME.typography.body_text_px))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(rgb(APP_UI_THEME.text.secondary))
                            .child(app_shared_label_text(
                                AppTextKey::SettingsGeneralSectionLabel,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
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
                    ),
            )
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

    fn settings_panel_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.selected_view() {
            SettingsPanelViewKey::Account => self.account_panel(cx).into_any_element(),
            SettingsPanelViewKey::Settings => self.settings_panel(cx).into_any_element(),
            SettingsPanelViewKey::About => self.about_panel().into_any_element(),
        }
    }
}

impl Render for SettingsWindowView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                .child(self.navigation_button(SettingsPanelViewKey::Account, cx))
                                .child(self.navigation_button(SettingsPanelViewKey::Settings, cx))
                                .child(self.navigation_button(SettingsPanelViewKey::About, cx)),
                        ),
                )
                .child(section_divider())
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .child(self.settings_panel_content(cx)),
                ),
        )
    }
}

fn settings_panel_label_key(view: SettingsPanelViewKey) -> AppTextKey {
    match view {
        SettingsPanelViewKey::Account => AppTextKey::SettingsNavAccounts,
        SettingsPanelViewKey::Settings => AppTextKey::SettingsNavSettings,
        SettingsPanelViewKey::About => AppTextKey::SettingsNavAbout,
    }
}

fn settings_panel_spec(view: SettingsPanelViewKey) -> (&'static str, IconName) {
    match view {
        SettingsPanelViewKey::Account => ("settings-nav-accounts", IconName::CircleUser),
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
    relay_error: Option<&str>,
    signer_entry: Option<&StartupSignerEntryState>,
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
                                            .when_some(relay_error, |this, error| {
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
                                            .when_some(relay_error, |this, error| {
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
                                                relay_error,
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
    relay_error: Option<&str>,
    on_submit_signer: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
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
                            .w_full(),
                    ),
            )
        })
        .child(action_button_primary(
            "home-connect-signer-submit",
            app_shared_text(AppTextKey::HomeSetupSignerConnectAction),
            on_submit_signer,
            cx,
        ))
        .child(startup_text_button(
            "home-signer-back",
            AppTextKey::HomeSetupBackAction,
            on_back,
            cx,
        ))
        .when_some(relay_error, |this, error| {
            this.child(startup_home_support_text(error.to_owned()))
        })
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
        AppTextKey, FarmerHomeFarmState, StartupHomeSurface, farm_setup_onboarding_card_spec,
        farmer_home_farm_state, home_saved_farm, home_window_launch_size_px,
        home_window_minimum_size_px, parse_optional_product_editor_stock_input,
        parse_product_editor_price_input, product_display_title, startup_home_surface,
    };
    use crate::runtime::DesktopAppRuntimeSummary;
    use radroots_studio_app_models::SettingsAccountProjection;
    use radroots_studio_app_models::{
        AppStartupGate, FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft,
        FarmSetupProjection, FarmSummary, LoggedOutStartupPhase, LoggedOutStartupProjection,
        TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
    };
    use radroots_studio_app_state::AppShellProjection;
    use radroots_studio_app_state::HomeRoute;

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
}
