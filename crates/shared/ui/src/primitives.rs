use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px, rgb};

use crate::APP_UI_THEME;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelValueRow {
    pub label: SharedString,
    pub value: SharedString,
}

impl LabelValueRow {
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

pub fn app_window_shell(background: u32, content: impl IntoElement) -> impl IntoElement {
    div()
        .size_full()
        .overflow_hidden()
        .bg(rgb(background))
        .text_color(rgb(APP_UI_THEME.text.primary))
        .child(content)
}

pub fn app_center_stage(content: impl IntoElement) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(APP_UI_THEME.layout.home_window_padding_px))
        .child(content)
}

pub fn app_card(content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
        .mx_auto()
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .overflow_hidden()
        .child(
            div()
                .w_full()
                .p(px(APP_UI_THEME.layout.home_card_padding_px))
                .child(content),
        )
}

pub fn section_divider() -> impl IntoElement {
    div()
        .w_full()
        .h(px(APP_UI_THEME.layout.divider_thickness_px))
        .bg(rgb(APP_UI_THEME.surfaces.divider))
}

pub fn utility_title_row(title: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .h(px(APP_UI_THEME.layout.utility_title_row_height_px))
        .flex()
        .justify_center()
        .items_center()
        .text_size(px(APP_UI_THEME.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(APP_UI_THEME.text.primary))
        .child(title.into())
}

pub fn label_value_list(rows: impl IntoIterator<Item = LabelValueRow>) -> impl IntoElement {
    let rows = rows
        .into_iter()
        .map(|row| {
            let line = format!("{}: {}", row.label, row.value);
            div()
                .w_full()
                .text_size(px(APP_UI_THEME.typography.body_text_px))
                .text_color(rgb(APP_UI_THEME.text.secondary))
                .child(line)
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(APP_UI_THEME.layout.metadata_row_gap_px))
        .children(rows)
}
