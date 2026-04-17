use gpui::{
    App, ClickEvent, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px, relative, rgb, transparent_black,
};
use gpui_component::{
    Icon, IconName, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants},
};
use std::rc::Rc;

use crate::APP_UI_THEME;

pub struct IconSegmentButtonSpec {
    pub id: &'static str,
    pub label: SharedString,
    pub icon: IconName,
}

impl IconSegmentButtonSpec {
    pub fn new(id: &'static str, label: impl Into<SharedString>, icon: IconName) -> Self {
        Self {
            id,
            label: label.into(),
            icon,
        }
    }
}

pub struct AppCheckboxFieldSpec {
    pub id: &'static str,
    pub label: SharedString,
    pub note: Option<SharedString>,
}

impl AppCheckboxFieldSpec {
    pub fn new(
        id: &'static str,
        label: impl Into<SharedString>,
        note: Option<impl Into<SharedString>>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            note: note.map(Into::into),
        }
    }
}

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
        .h_full()
        .max_w(px(APP_UI_THEME.layout.home_card_max_width_px))
        .mx_auto()
        .bg(rgb(APP_UI_THEME.surfaces.card_background))
        .overflow_hidden()
        .child(
            div()
                .size_full()
                .overflow_hidden()
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

pub fn app_checkbox(
    id: &'static str,
    checked: bool,
    cx: &App,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let colors = APP_UI_THEME.controls.checkbox;
    let background = if checked {
        colors.checked_background
    } else {
        colors.unchecked_background
    };
    let border = if checked {
        colors.checked_background
    } else {
        colors.unchecked_border
    };
    let mut button = Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(background).into())
                .foreground(rgb(colors.check_foreground).into())
                .border(rgb(border).into())
                .hover(rgb(background).into())
                .active(rgb(background).into()),
        )
        .rounded(ButtonRounded::Size(px(colors.corner_radius_px)))
        .with_size(Size::Size(px(colors.size_px)))
        .on_click(move |_, window, cx| on_change(!checked, window, cx));

    if checked {
        button = button.icon(
            Icon::new(IconName::Check)
                .with_size(Size::Size(px(colors.icon_size_px)))
                .text_color(rgb(colors.check_foreground)),
        );
    }

    button
}

pub fn app_checkbox_field(
    spec: AppCheckboxFieldSpec,
    checked: bool,
    cx: &App,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let checkbox_id = spec.id;
    let checkbox_label = spec.label;
    let checkbox_note = spec.note;
    let row_text_px = APP_UI_THEME.typography.settings_row_text_px;
    let note_text_px = APP_UI_THEME.typography.utility_title_text_px;
    let note_indent_px =
        APP_UI_THEME.controls.checkbox.size_px + APP_UI_THEME.layout.settings_checkbox_label_gap_px;
    let on_change = Rc::new(on_change);

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            Button::new((checkbox_id, 0usize))
                .custom(
                    ButtonCustomVariant::new(cx)
                        .color(transparent_black().into())
                        .foreground(rgb(APP_UI_THEME.text.primary).into())
                        .border(transparent_black())
                        .hover(transparent_black().into())
                        .active(transparent_black().into()),
                )
                .rounded(ButtonRounded::Size(px(0.0)))
                .w_full()
                .on_click({
                    let on_change = Rc::clone(&on_change);
                    move |_, window, cx| on_change(!checked, window, cx)
                })
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px(APP_UI_THEME.layout.settings_checkbox_label_gap_px))
                        .child(app_checkbox(checkbox_id, checked, cx, {
                            let on_change = Rc::clone(&on_change);
                            move |checked, window, cx| on_change(checked, window, cx)
                        }))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(row_text_px))
                                .line_height(relative(1.1))
                                .text_color(rgb(APP_UI_THEME.text.primary))
                                .child(checkbox_label),
                        ),
                ),
        )
        .when_some(checkbox_note, |this, note| {
            this.child(
                div()
                    .w_full()
                    .pl(px(note_indent_px))
                    .min_w_0()
                    .text_size(px(note_text_px))
                    .text_color(rgb(APP_UI_THEME.text.secondary))
                    .child(note),
            )
        })
}

pub fn icon_segment_button(
    spec: IconSegmentButtonSpec,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let colors = APP_UI_THEME.controls.icon_segment_button.colors;
    let sizing = APP_UI_THEME.controls.icon_segment_button.sizing;
    let background = if is_active {
        colors.active_background
    } else {
        colors.inactive_background
    };
    let foreground = if is_active {
        colors.active_foreground
    } else {
        colors.inactive_foreground
    };

    Button::new(spec.id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(background).into())
                .foreground(rgb(foreground).into())
                .border(transparent_black())
                .hover(rgb(background).into())
                .active(rgb(background).into()),
        )
        .rounded(ButtonRounded::Size(px(sizing.corner_radius_px)))
        .h(px(sizing.height_px))
        .min_w(px(sizing.height_px))
        .on_click(on_click)
        .child(
            div()
                .h_full()
                .flex()
                .flex_col()
                .justify_between()
                .items_center()
                .px(px(sizing.inner_padding_px))
                .py(px(sizing.inner_padding_px))
                .child(
                    Icon::new(spec.icon)
                        .with_size(Size::Size(px(sizing.icon_size_px)))
                        .text_color(rgb(foreground)),
                )
                .child(
                    div()
                        .text_size(px(sizing.label_size_px))
                        .text_color(rgb(foreground))
                        .child(spec.label),
                ),
        )
}

pub fn action_button(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    action_button_label(
        action_button_base(id, on_click, cx),
        label.into(),
        APP_UI_THEME
            .controls
            .action_button
            .sizing
            .horizontal_padding_px,
    )
}

pub fn action_button_compact(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    action_button_label(
        action_button_base(id, on_click, cx),
        label.into(),
        APP_UI_THEME
            .controls
            .action_button
            .sizing
            .compact_horizontal_padding_px,
    )
}

fn action_button_label(
    button: Button,
    label: SharedString,
    horizontal_padding_px: f32,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.controls.action_button.sizing;
    let colors = APP_UI_THEME.controls.action_button.colors;
    button.child(
        div()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(horizontal_padding_px))
            .text_size(px(sizing.label_size_px))
            .text_color(rgb(colors.foreground))
            .child(label),
    )
}

pub fn action_icon_button(
    id: &'static str,
    icon: IconName,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.controls.action_button.sizing;
    let colors = APP_UI_THEME.controls.action_button.colors;

    action_button_base(id, on_click, cx)
        .with_size(Size::Size(px(sizing.square_width_px)))
        .icon(
            Icon::new(icon)
                .with_size(Size::Size(px(sizing.icon_size_px)))
                .text_color(rgb(colors.foreground)),
        )
}

pub fn status_indicator(color: u32) -> impl IntoElement {
    let sizing = APP_UI_THEME.controls.status_indicator;

    div()
        .size(px(sizing.size_px))
        .bg(rgb(color))
        .rounded(px(sizing.size_px / 2.0))
}

fn action_button_base(
    id: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Button {
    let sizing = APP_UI_THEME.controls.action_button.sizing;
    let colors = APP_UI_THEME.controls.action_button.colors;
    let hover_background = if colors.hover_changes_background {
        colors.hover_background
    } else {
        colors.background
    };

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
}

#[cfg(test)]
mod tests {
    use gpui_component::IconName;

    use super::{AppCheckboxFieldSpec, IconSegmentButtonSpec};

    #[test]
    fn icon_segment_spec_preserves_id_and_label() {
        let spec = IconSegmentButtonSpec::new("settings", "Settings", IconName::Settings2);

        assert_eq!(spec.id, "settings");
        assert_eq!(spec.label.as_ref(), "Settings");
    }

    #[test]
    fn checkbox_field_spec_preserves_optional_note() {
        let spec = AppCheckboxFieldSpec::new("launch", "Launch at login", Some("Optional note"));

        assert_eq!(spec.id, "launch");
        assert_eq!(spec.label.as_ref(), "Launch at login");
        assert_eq!(spec.note.as_ref().map(|note| note.as_ref()), Some("Optional note"));
    }
}
