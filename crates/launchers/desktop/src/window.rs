use gpui::{
    Animation, AnimationExt, AnyElement, App, AppContext, Bounds, ClickEvent, Context,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Timer, Window, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, div, prelude::FluentBuilder, px, relative, rgb, size,
};
use gpui_component::{IconName, Root};
use radroots_studio_app_i18n::AppTextKey;
pub use radroots_studio_app_models::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_models::{
    AppStartupGate, FulfillmentWindowSummary, OrderListRow, ProductListRow, TodayAgendaProjection,
    TodaySetupTaskKind,
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

use crate::runtime::{DesktopAppRuntime, DesktopAppRuntimeSummary};

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
    let bounds = Bounds::centered(
        None,
        size(
            px(APP_UI_THEME.windows.home_min_width_px),
            px(APP_UI_THEME.windows.home_min_height_px),
        ),
        cx,
    );

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(
            px(APP_UI_THEME.windows.home_min_width_px),
            px(APP_UI_THEME.windows.home_min_height_px),
        )),
        titlebar: Some(home_titlebar_options()),
        ..Default::default()
    }
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
    let _ = runtime.record_settings_opened(initial_view);
    let view = cx.new(|_| SettingsWindowView::new(runtime, initial_view));
    cx.new(|cx| Root::new(view, window, cx))
}

pub struct HomeView {
    runtime: DesktopAppRuntime,
    startup_view: StartupHomeView,
    logged_in_view: LoggedInHomeView,
    relay_client: Option<RadrootsNostrClient>,
}

impl HomeView {
    pub fn new(runtime: DesktopAppRuntime) -> Self {
        Self {
            runtime,
            startup_view: StartupHomeView::new(),
            logged_in_view: LoggedInHomeView::new(),
            relay_client: None,
        }
    }

    fn generate_local_account(&mut self, cx: &mut Context<Self>) {
        if self.runtime.generate_local_account(None).unwrap_or(false) {
            cx.refresh_windows();
            cx.notify();
        }
    }

    fn start_create_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.startup_view.begin_starting() {
            return;
        }

        let relay_url = self.runtime.default_nostr_relay_url();
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let startup_task = cx
                .background_executor()
                .spawn(run_startup_app_init(relay_url));
            Timer::after(Duration::from_secs(1)).await;
            let startup_result = startup_task.await;
            let _ = this.update(cx, |this, cx| {
                this.finish_create_account(startup_result, cx);
            });
        })
        .detach();
    }

    fn finish_create_account(
        &mut self,
        startup_result: Result<StartupAppInitResult, String>,
        cx: &mut Context<Self>,
    ) {
        match startup_result {
            Ok(result) => {
                self.relay_client = Some(result.relay_client);
                self.startup_view.clear_error();
                self.startup_view.finish_starting();
                self.generate_local_account(cx);
            }
            Err(error) => {
                self.startup_view.fail_starting(error);
                cx.notify();
            }
        }
    }

    fn open_farm_setup(&mut self, cx: &mut Context<Self>) {
        if self
            .runtime
            .select_farm_setup_flow_stage(FarmSetupFlowStage::Editing)
        {
            cx.notify();
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime_summary = self.runtime.summary();
        match home_stage(&runtime_summary) {
            HomeStage::Setup => self
                .startup_view
                .render(
                    &runtime_summary,
                    runtime_summary.startup_issue.is_none()
                        && runtime_summary.startup_gate == AppStartupGate::SetupRequired,
                    cx.listener(|this, _, window, cx| this.start_create_account(window, cx)),
                    cx,
                )
                .into_any_element(),
            HomeStage::PersonalHolding => self
                .logged_in_view
                .render_holding(&runtime_summary)
                .into_any_element(),
            HomeStage::FarmerWorkspace => self
                .logged_in_view
                .render_farmer(
                    &runtime_summary,
                    cx.listener(|this, _, _, cx| this.open_farm_setup(cx)),
                    cx,
                )
                .into_any_element(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StartupPhase {
    Idle,
    Starting,
}

struct StartupHomeView {
    phase: StartupPhase,
    relay_error: Option<String>,
}

impl StartupHomeView {
    fn new() -> Self {
        Self {
            phase: StartupPhase::Idle,
            relay_error: None,
        }
    }

    fn begin_starting(&mut self) -> bool {
        if self.phase == StartupPhase::Starting {
            return false;
        }

        self.phase = StartupPhase::Starting;
        self.relay_error = None;
        true
    }

    fn finish_starting(&mut self) {
        self.phase = StartupPhase::Idle;
    }

    fn fail_starting(&mut self, error: String) {
        self.phase = StartupPhase::Idle;
        self.relay_error = Some(error);
    }

    fn clear_error(&mut self) {
        self.relay_error = None;
    }

    fn render(
        &self,
        runtime: &DesktopAppRuntimeSummary,
        allow_create_account: bool,
        on_create_account: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> impl IntoElement {
        startup_home_shell(
            runtime,
            self.phase == StartupPhase::Starting,
            self.relay_error.as_deref(),
            allow_create_account,
            on_create_account,
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

    fn render_farmer(
        &self,
        runtime: &DesktopAppRuntimeSummary,
        on_open_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> AnyElement {
        farmer_home_shell(runtime, on_open_farm_setup, cx).into_any_element()
    }
}

pub struct SettingsWindowView {
    runtime: DesktopAppRuntime,
    selected_view: SettingsPanelViewKey,
}

impl SettingsWindowView {
    pub fn new(runtime: DesktopAppRuntime, initial_view: SettingsPanelViewKey) -> Self {
        Self {
            runtime,
            selected_view: initial_view,
        }
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        if self.selected_view != view {
            self.selected_view = view;
            cx.notify();
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
            self.selected_view == view,
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
        match self.selected_view {
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

fn farmer_home_shell(
    runtime: &DesktopAppRuntimeSummary,
    on_open_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    home_shell_frame(
        runtime,
        div()
            .id("home-today-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(home_view_content(runtime, on_open_farm_setup, cx))
            .into_any_element(),
    )
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
        runtime,
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

fn startup_home_shell(
    runtime: &DesktopAppRuntimeSummary,
    is_starting: bool,
    relay_error: Option<&str>,
    allow_create_account: bool,
    on_create_account: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
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
                                    .child(startup_home_title(is_starting))
                                    .child(startup_home_tagline())
                                    .when(allow_create_account, |this| {
                                        this.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .gap(px(APP_UI_THEME.layout.startup_stack_gap_px))
                                                .child(if is_starting {
                                                    action_button_primary_disabled(
                                                    "home-create-account",
                                                    app_shared_text(
                                                        AppTextKey::HomeSetupCreateAccountAction,
                                                    ),
                                                    cx,
                                                )
                                                .into_any_element()
                                                } else {
                                                    action_button_primary(
                                                    "home-create-account",
                                                    app_shared_text(
                                                        AppTextKey::HomeSetupCreateAccountAction,
                                                    ),
                                                    on_create_account,
                                                    cx,
                                                )
                                                .into_any_element()
                                                })
                                                .when_some(relay_error, |this, error| {
                                                    this.child(startup_home_support_text(
                                                        error.to_owned(),
                                                    ))
                                                }),
                                        )
                                    })
                                    .when(!allow_create_account, |this| {
                                        this.child(startup_home_card(
                                            app_shared_text(AppTextKey::MetadataStartupIssue),
                                            startup_home_body(runtime),
                                        ))
                                    }),
                            ),
                    ),
            ),
    )
}

fn startup_home_title(is_starting: bool) -> impl IntoElement {
    let (animation_id, title_key) = if is_starting {
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

fn home_shell_frame(
    runtime: &DesktopAppRuntimeSummary,
    main_content: AnyElement,
) -> impl IntoElement {
    app_window_shell(
        APP_UI_THEME.surfaces.window_background,
        div()
            .size_full()
            .overflow_hidden()
            .flex()
            .child(home_sidebar(runtime))
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

fn home_sidebar(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
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
                        .child(app_shared_text(AppTextKey::HomeTodayTitle)),
                )
                .child(home_status_row(&home_status)),
        )
        .child(
            div().child(
                div()
                    .text_size(px(APP_UI_THEME.typography.body_text_px))
                    .line_height(relative(1.2))
                    .text_color(rgb(APP_UI_THEME.text.secondary))
                    .when_some(runtime.today_projection.farm.as_ref(), |this, farm| {
                        this.child(farm.display_name.clone())
                    }),
            ),
        )
}

fn home_view_content(
    runtime: &DesktopAppRuntimeSummary,
    on_open_farm_setup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let projection = &runtime.today_projection;
    let home_status = home_status_presentation(runtime);
    let setup_onboarding = farm_setup_onboarding_card_spec(runtime.home_route);
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
        sections
            .push(home_farm_setup_onboarding_card(spec, on_open_farm_setup, cx).into_any_element());
    } else if projection.needs_setup() {
        sections.push(home_setup_card(projection).into_any_element());
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
        && projection.farm.is_none()
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
        && projection.farm.is_some()
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
                        .when_some(projection.farm.as_ref(), |this, farm| {
                            this.child(farm.display_name.clone())
                        })
                        .when(projection.farm.is_none(), |this| {
                            this.child(app_shared_text(home_status.label_key))
                        }),
                )
                .child(home_status_row(&home_status)),
        )
        .children(sections)
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

fn home_setup_card(projection: &TodayAgendaProjection) -> impl IntoElement {
    home_list_card(
        AppTextKey::HomeTodaySetupChecklist,
        projection
            .setup_checklist
            .iter()
            .map(home_setup_task_row)
            .collect::<Vec<_>>(),
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

fn home_list_card(title_key: AppTextKey, rows: Vec<AnyElement>) -> impl IntoElement {
    home_card(
        app_shared_text(title_key),
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
            .children(rows),
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
                .child(product.title.clone()),
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
                .child(product.title.clone()),
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

    if matches!(
        runtime.home_route,
        HomeRoute::FarmSetupOnboarding | HomeRoute::FarmSetupForm
    ) || runtime.today_projection.farm.is_none()
    {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.controls.status_indicator.offline,
            label_key: AppTextKey::HomeTodayStatusNoFarm,
        };
    }

    if runtime.today_projection.needs_setup() {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.controls.status_indicator.offline,
            label_key: AppTextKey::HomeTodayStatusSetup,
        };
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

#[cfg(test)]
mod tests {
    use super::{AppTextKey, farm_setup_onboarding_card_spec};
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
}
