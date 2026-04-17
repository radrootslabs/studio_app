use gpui::{
    AnyElement, App, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions, div, px, rgb,
    size,
};
use gpui_component::IconName;
use radroots_studio_app_core::AppRuntimeSnapshot;
use radroots_studio_app_i18n::AppTextKey;
use radroots_studio_app_ui::{
    APP_UI_THEME, IconSegmentButtonSpec, LabelValueRow, app_card, app_shared_text,
    app_window_shell, icon_segment_button, label_value_list, runtime_metadata_rows,
    section_divider, settings_about_status_rows, settings_account_profile_rows,
    settings_preferences_general_rows, utility_title_row,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsPanelViewKey {
    #[default]
    Account,
    Settings,
    About,
}

impl SettingsPanelViewKey {
    fn label_key(self) -> AppTextKey {
        match self {
            Self::Account => AppTextKey::SettingsNavAccounts,
            Self::Settings => AppTextKey::SettingsNavSettings,
            Self::About => AppTextKey::SettingsNavAbout,
        }
    }

    fn spec(self) -> (&'static str, IconName) {
        match self {
            Self::Account => ("settings-nav-accounts", IconName::CircleUser),
            Self::Settings => ("settings-nav-settings", IconName::Settings2),
            Self::About => ("settings-nav-about", IconName::Info),
        }
    }
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
    selected_view: SettingsPanelViewKey,
}

impl SettingsWindowView {
    pub fn new(initial_view: SettingsPanelViewKey) -> Self {
        Self {
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
        let (navigation_id, navigation_icon) = view.spec();
        icon_segment_button(
            IconSegmentButtonSpec::new(
                navigation_id,
                app_shared_text(view.label_key()),
                navigation_icon,
            ),
            self.selected_view == view,
            cx.listener(move |this, _, _, cx| this.select_view(view, cx)),
            cx,
        )
    }

    fn detail_card(&self, title: AppTextKey, rows: Vec<LabelValueRow>) -> impl IntoElement {
        app_card(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                .child(utility_title_row(app_shared_text(title)))
                .child(section_divider())
                .child(label_value_list(rows)),
        )
    }

    fn accounts_panel(&self) -> impl IntoElement {
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                    .child(self.detail_card(
                        AppTextKey::SettingsNavAccounts,
                        settings_account_profile_rows(),
                    )),
            )
    }

    fn settings_panel(&self) -> impl IntoElement {
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                    .child(self.detail_card(
                        AppTextKey::SettingsGeneralSectionLabel,
                        settings_preferences_general_rows(),
                    )),
            )
    }

    fn about_panel(&self) -> impl IntoElement {
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
                    .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                    .child(
                        self.detail_card(
                            AppTextKey::SettingsNavAbout,
                            settings_about_status_rows(),
                        ),
                    ),
            )
    }

    fn settings_panel_content(&self) -> AnyElement {
        match self.selected_view {
            SettingsPanelViewKey::Account => self.accounts_panel().into_any_element(),
            SettingsPanelViewKey::Settings => self.settings_panel().into_any_element(),
            SettingsPanelViewKey::About => self.about_panel().into_any_element(),
        }
    }
}

impl Render for SettingsWindowView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        app_window_shell(
            APP_UI_THEME.surfaces.window_background,
            div()
                .size_full()
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(APP_UI_THEME.layout.settings_chrome_height_px))
                        .bg(rgb(APP_UI_THEME.surfaces.chrome_background))
                        .p(px(APP_UI_THEME.layout.settings_content_padding_px))
                        .flex()
                        .flex_col()
                        .gap(px(APP_UI_THEME.layout.settings_section_gap_px))
                        .child(utility_title_row(app_shared_text(
                            AppTextKey::SettingsTitle,
                        )))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .justify_center()
                                .items_center()
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
                        .child(self.settings_panel_content()),
                ),
        )
    }
}
