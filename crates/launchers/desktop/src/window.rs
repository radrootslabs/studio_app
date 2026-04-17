use gpui::{
    AnyElement, App, AppContext, Context, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
    relative, rgb,
};
use gpui_component::{IconName, Root};
use radroots_studio_app_i18n::AppTextKey;
pub use radroots_studio_app_models::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_models::{
    FulfillmentWindowSummary, OrderListRow, ProductListRow, TodayAgendaProjection,
    TodaySetupTaskKind,
};
use radroots_studio_app_state::SettingsPreference;
use radroots_studio_app_ui::{
    APP_UI_THEME, AppCheckboxFieldSpec, IconSegmentButtonSpec, LabelValueRow, action_button,
    action_button_compact, action_icon_button, app_checkbox_field, app_shared_label_text,
    app_shared_text, app_window_shell, icon_segment_button, label_value_list, section_divider,
    status_indicator, utility_title_row,
};

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

pub fn open_home_window(
    window: &mut Window,
    cx: &mut App,
    runtime: DesktopAppRuntime,
) -> gpui::Entity<Root> {
    let view = cx.new(|_| HomeView::new(runtime));
    cx.new(|cx| Root::new(view, window, cx))
}

pub fn open_settings_window(
    window: &mut Window,
    cx: &mut App,
    runtime: DesktopAppRuntime,
    initial_view: SettingsPanelViewKey,
) -> gpui::Entity<Root> {
    let _ = runtime.select_settings_section(initial_view);
    let view = cx.new(|_| SettingsWindowView::new(runtime));
    cx.new(|cx| Root::new(view, window, cx))
}

pub struct HomeView {
    runtime: DesktopAppRuntime,
}

impl HomeView {
    pub fn new(runtime: DesktopAppRuntime) -> Self {
        Self { runtime }
    }
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let runtime_summary = self.runtime.summary();
        let home_status = home_status_presentation(&runtime_summary);

        app_window_shell(
            APP_UI_THEME.surfaces.window_background,
            div()
                .size_full()
                .overflow_hidden()
                .flex()
                .child(
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
                                        .text_size(px(APP_UI_THEME.typography.brand_text_px))
                                        .text_color(rgb(APP_UI_THEME.text.primary))
                                        .child(app_shared_text(AppTextKey::HomeBrand)),
                                )
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
                                    .when_some(
                                        runtime_summary.today_projection.farm.as_ref(),
                                        |this, farm| this.child(farm.display_name.clone()),
                                    ),
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
                        .bg(rgb(APP_UI_THEME.surfaces.window_background))
                        .overflow_hidden()
                        .child(
                            div()
                                .size_full()
                                .p(px(APP_UI_THEME.layout.home_window_padding_px))
                                .child(
                                    div()
                                        .id("home-today-scroll")
                                        .size_full()
                                        .overflow_y_scroll()
                                        .child(home_view_content(&runtime_summary)),
                                ),
                        ),
                ),
        )
    }
}

pub struct SettingsWindowView {
    runtime: DesktopAppRuntime,
}

impl SettingsWindowView {
    pub fn new(runtime: DesktopAppRuntime) -> Self {
        Self { runtime }
    }

    fn selected_view(&self) -> SettingsPanelViewKey {
        self.runtime.selected_settings_section()
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        if self.runtime.select_settings_section(view) {
            cx.notify();
        }
    }

    fn set_settings_preference(
        &mut self,
        preference: SettingsPreference,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.set_settings_preference(preference, enabled) {
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
            self.selected_view() == view,
            cx.listener(move |this, _, _, cx| this.select_view(view, cx)),
            cx,
        )
    }

    fn account_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let detail_text_px = APP_UI_THEME.typography.settings_account_detail_text_px;
        let account_status_color = APP_UI_THEME.controls.status_indicator.online;

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
                            .h(px(APP_UI_THEME.layout.settings_account_sidebar_button_height_px))
                            .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
                            .rounded(px(
                                APP_UI_THEME
                                    .layout
                                    .settings_account_sidebar_button_corner_radius_px,
                            ))
                            .p(px(
                                APP_UI_THEME.layout.settings_account_sidebar_button_padding_px,
                            ))
                            .flex()
                            .flex_row()
                            .justify_start()
                            .items_center()
                            .gap(px(APP_UI_THEME.layout.settings_account_sidebar_button_gap_px))
                            .child(
                                div()
                                    .size(px(APP_UI_THEME.layout.settings_account_sidebar_avatar_size_px))
                                    .bg(rgb(APP_UI_THEME.surfaces.card_background))
                                    .rounded(px(
                                        APP_UI_THEME.layout.settings_account_sidebar_avatar_size_px
                                            / 2.0,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(APP_UI_THEME.layout.settings_account_identity_text_gap_px))
                                    .justify_center()
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
                                                AppTextKey::SettingsAccountPlaceholderName,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(
                                                APP_UI_THEME
                                                    .typography
                                                    .settings_account_identity_text_px,
                                            ))
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(rgb(APP_UI_THEME.text.secondary))
                                            .child(app_shared_text(
                                                AppTextKey::SettingsAccountPlaceholderHandle,
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
                                                AppTextKey::SettingsAccountPlaceholderName,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .gap(px(APP_UI_THEME.layout.settings_account_detail_row_gap_px))
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .items_center()
                                            .gap(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_detail_value_gap_px,
                                            ))
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(rgb(APP_UI_THEME.text.secondary))
                                                    .child(app_shared_label_text(
                                                        AppTextKey::SettingsAccountProfileLabel,
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .text_color(rgb(APP_UI_THEME.text.primary))
                                                    .child(app_shared_text(
                                                        AppTextKey::SettingsAccountPlaceholderHandle,
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .items_center()
                                            .gap(px(
                                                APP_UI_THEME
                                                    .layout
                                                    .settings_account_detail_value_gap_px,
                                            ))
                                            .child(
                                                div()
                                                    .text_size(px(detail_text_px))
                                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                                    .text_color(rgb(APP_UI_THEME.text.secondary))
                                                    .child(app_shared_label_text(
                                                        AppTextKey::SettingsAccountStatusLabel,
                                                    )),
                                            )
                                            .child(
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
                                                                AppTextKey::SettingsAccountStatusLoggedIn,
                                                            )),
                                                    ),
                                            ),
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
                                                    "account-admin-console",
                                                    app_shared_text(
                                                        AppTextKey::SettingsAccountAdminConsoleAction,
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
                                cx.listener(|this, checked: &bool, _, cx| {
                                    this.set_settings_preference(
                                        SettingsPreference::AllowRelayConnections,
                                        *checked,
                                        cx,
                                    );
                                }),
                                cx,
                            ))
                            .child(self.settings_checkbox_row(
                                "settings-use-media-servers",
                                general_use_media_servers,
                                AppTextKey::SettingsGeneralUseMediaServers,
                                Some("settings-manage-media-servers"),
                                Some(AppTextKey::SettingsGeneralManageAction),
                                None,
                                cx.listener(|this, checked: &bool, _, cx| {
                                    this.set_settings_preference(
                                        SettingsPreference::UseMediaServers,
                                        *checked,
                                        cx,
                                    );
                                }),
                                cx,
                            ))
                            .child(self.settings_checkbox_row(
                                "settings-use-nip05",
                                general_use_nip05,
                                AppTextKey::SettingsGeneralUseNip05,
                                None,
                                None,
                                Some(AppTextKey::SettingsGeneralUseNip05Note),
                                cx.listener(|this, checked: &bool, _, cx| {
                                    this.set_settings_preference(
                                        SettingsPreference::UseNip05,
                                        *checked,
                                        cx,
                                    );
                                }),
                                cx,
                            ))
                            .child(self.settings_checkbox_row(
                                "settings-launch-at-login",
                                general_launch_at_login,
                                AppTextKey::SettingsGeneralLaunchAtLogin,
                                None,
                                None,
                                None,
                                cx.listener(|this, checked: &bool, _, cx| {
                                    this.set_settings_preference(
                                        SettingsPreference::LaunchAtLogin,
                                        *checked,
                                        cx,
                                    );
                                }),
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

fn home_view_content(runtime: &DesktopAppRuntimeSummary) -> impl IntoElement {
    let projection = &runtime.today_projection;
    let home_status = home_status_presentation(runtime);
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

    if projection.needs_setup() {
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

    if runtime.startup_issue.is_none() && projection.farm.is_none() {
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

fn home_status_presentation(runtime: &DesktopAppRuntimeSummary) -> HomeStatusPresentation {
    if runtime.startup_issue.is_some() {
        return HomeStatusPresentation {
            indicator_color: APP_UI_THEME.controls.status_indicator.attention,
            label_key: AppTextKey::HomeTodayStatusStartupIssue,
        };
    }

    if runtime.today_projection.farm.is_none() {
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
