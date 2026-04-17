use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};
use radroots_studio_app_core::AppRuntimeSnapshot;
use radroots_studio_app_i18n::AppTextKey;
use radroots_studio_app_ui::{
    APP_UI_THEME, LabelValueRow, app_card, app_shared_text, app_window_shell, label_value_list,
    runtime_metadata_rows, section_divider, utility_title_row,
};

pub fn home_titlebar_options() -> gpui::TitlebarOptions {
    gpui::TitlebarOptions {
        title: None,
        appears_transparent: true,
        ..Default::default()
    }
}

pub struct HomeView {
    snapshot: AppRuntimeSnapshot,
    metadata_rows: Vec<LabelValueRow>,
}

impl HomeView {
    pub fn new(snapshot: AppRuntimeSnapshot) -> Self {
        let metadata_rows = runtime_metadata_rows(&snapshot);

        Self {
            snapshot,
            metadata_rows,
        }
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
                        .bg(rgb(APP_UI_THEME.surfaces.panel_background))
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
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(rgb(APP_UI_THEME.text.primary))
                                        .child(app_shared_text(AppTextKey::HomeBrand)),
                                )
                                .child(
                                    div()
                                        .text_size(px(APP_UI_THEME.typography.body_text_px))
                                        .text_color(rgb(APP_UI_THEME.text.secondary))
                                        .child(app_shared_text(AppTextKey::HomeTitle)),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
                                .text_color(rgb(APP_UI_THEME.text.secondary))
                                .child(format!("v{}", self.snapshot.host.app_version)),
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
                                .id("home-shell-scroll")
                                .size_full()
                                .overflow_y_scroll()
                                .p(px(APP_UI_THEME.layout.home_window_padding_px))
                                .child(app_card(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .gap(px(APP_UI_THEME.layout.home_stack_gap_px))
                                        .child(utility_title_row(app_shared_text(
                                            AppTextKey::HomeMetadataTitle,
                                        )))
                                        .child(section_divider())
                                        .child(label_value_list(self.metadata_rows.clone())),
                                )),
                        ),
                ),
        )
    }
}
