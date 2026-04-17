use gpui::{
    AnyElement, App, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions, div,
    prelude::FluentBuilder, px, rgb, size,
};
use gpui_component::IconName;
use radroots_studio_app_core::AppRuntimeSnapshot;
use radroots_studio_app_i18n::AppTextKey;
pub use radroots_studio_app_models::SettingsSection as SettingsPanelViewKey;
use radroots_studio_app_state::{
    AppShellCommand, AppShellProjection, AppStateStore, InMemoryAppStateRepository,
    SettingsPreference,
};
use radroots_studio_app_ui::{
    APP_UI_THEME, AppCheckboxFieldSpec, IconSegmentButtonSpec, LabelValueRow, action_button,
    action_button_compact, action_icon_button, app_checkbox_field, app_shared_label_text,
    app_shared_text, app_window_shell, icon_segment_button, label_value_list,
    runtime_metadata_rows, section_divider, status_indicator, utility_title_row,
};

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

pub fn open_settings_window(cx: &mut App, initial_view: SettingsPanelViewKey) {
    let bounds = Bounds::centered(
        None,
        size(
            px(APP_UI_THEME.windows.settings_width_px),
            px(APP_UI_THEME.windows.settings_height_px),
        ),
        cx,
    );

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(
                px(APP_UI_THEME.windows.settings_width_px),
                px(APP_UI_THEME.windows.settings_height_px),
            )),
            titlebar: Some(settings_titlebar_options()),
            ..Default::default()
        },
        |_, cx| cx.new(|_| SettingsWindowView::new(initial_view)),
    )
    .expect("settings window should open");
}

pub struct HomeView {
    metadata_rows: Vec<LabelValueRow>,
}

impl HomeView {
    pub fn new(snapshot: AppRuntimeSnapshot) -> Self {
        let metadata_rows = runtime_metadata_rows(&snapshot);

        Self { metadata_rows }
    }
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
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
                        .child(
                            div().child(
                                div()
                                    .text_size(px(APP_UI_THEME.typography.body_text_px))
                                    .text_color(rgb(APP_UI_THEME.text.primary))
                                    .child(app_shared_text(AppTextKey::HomeBrand)),
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
                                        .id("home-metadata-scroll")
                                        .size_full()
                                        .overflow_y_scroll()
                                        .child(label_value_list(self.metadata_rows.clone())),
                                ),
                        ),
                ),
        )
    }
}

pub struct SettingsWindowView {
    store: AppStateStore<InMemoryAppStateRepository>,
}

impl SettingsWindowView {
    pub fn new(initial_view: SettingsPanelViewKey) -> Self {
        Self {
            store: AppStateStore::in_memory(AppShellProjection::for_settings(initial_view)),
        }
    }

    fn selected_view(&self) -> SettingsPanelViewKey {
        self.store.projection().settings.selected_section
    }

    fn select_view(&mut self, view: SettingsPanelViewKey, cx: &mut Context<Self>) {
        if self
            .store
            .apply_in_memory(AppShellCommand::select_settings_section(view))
        {
            cx.notify();
        }
    }

    fn set_settings_preference(
        &mut self,
        preference: SettingsPreference,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self
            .store
            .apply_in_memory(AppShellCommand::SetSettingsPreference {
                preference,
                enabled,
            })
        {
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
        let general_allow_relay_connections = self
            .store
            .projection()
            .settings
            .general
            .allow_relay_connections;
        let general_use_media_servers = self.store.projection().settings.general.use_media_servers;
        let general_use_nip05 = self.store.projection().settings.general.use_nip05;
        let general_launch_at_login = self.store.projection().settings.general.launch_at_login;

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
