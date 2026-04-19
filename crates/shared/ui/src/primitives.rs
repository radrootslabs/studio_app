use gpui::{
    App, ClickEvent, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px, relative, rgb, transparent_black,
};
use gpui_component::{
    Icon, IconName, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants},
    input::{Input, InputState},
};
use std::rc::Rc;

use crate::APP_UI_THEME;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppButtonVariant {
    Secondary,
    Primary,
}

pub struct AppSegmentButtonIconSpec {
    pub id: &'static str,
    pub label: SharedString,
    pub icon: IconName,
}

impl AppSegmentButtonIconSpec {
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

pub struct AppFormFieldSpec {
    pub label: SharedString,
    pub note: Option<SharedString>,
}

impl AppFormFieldSpec {
    pub fn new(label: impl Into<SharedString>, note: Option<impl Into<SharedString>>) -> Self {
        Self {
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

pub fn app_surface_window(background: u32, content: impl IntoElement) -> impl IntoElement {
    div()
        .size_full()
        .overflow_hidden()
        .bg(rgb(background))
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content)
}

pub fn app_surface_sidebar(content: impl IntoElement) -> impl IntoElement {
    div()
        .h_full()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.card_background))
        .child(content)
}

pub fn app_surface_panel(content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.chrome_background))
        .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
        .child(content)
}

pub fn app_surface_card(content: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .bg(rgb(APP_UI_THEME.foundation.surfaces.card_background))
        .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
        .child(
            div()
                .w_full()
                .p(px(APP_UI_THEME.shells.home_card_padding_px))
                .child(content),
        )
}

pub fn app_surface_card_section(
    title: impl Into<SharedString>,
    body: impl IntoElement,
) -> impl IntoElement {
    app_surface_card(app_form_section(title, body))
}

pub fn app_divider() -> impl IntoElement {
    div()
        .w_full()
        .h(px(APP_UI_THEME.foundation.borders.divider_thickness_px))
        .bg(rgb(APP_UI_THEME.foundation.surfaces.divider))
}

pub fn app_heading_view(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.startup_title_text_px))
        .font_weight(gpui::FontWeight::NORMAL)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content.into())
}

pub fn app_heading_section(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content.into())
}

pub fn app_text_body(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content.into())
}

pub fn app_text_body_subtle(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
        .line_height(relative(1.2))
        .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
        .child(content.into())
}

pub fn app_text_label(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content.into())
}

pub fn app_text_value(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px * 2.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(content.into())
}

pub fn app_text_badge(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.accent))
        .child(content.into())
}

pub fn utility_title_row(title: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w_full()
        .h(px(APP_UI_THEME.shells.utility_title_row_height_px))
        .flex()
        .justify_center()
        .items_center()
        .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
        .child(title.into())
}

pub fn label_value_list(rows: impl IntoIterator<Item = LabelValueRow>) -> impl IntoElement {
    let rows = rows
        .into_iter()
        .map(|row| {
            let line = format!("{}: {}", row.label, row.value);
            div()
                .w_full()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(line)
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(APP_UI_THEME.shells.metadata_row_gap_px))
        .children(rows)
}

pub fn app_form_section(
    title: impl Into<SharedString>,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(APP_UI_THEME.foundation.spacing.small_px))
        .child(app_heading_section(title))
        .child(content)
}

pub fn app_form_field(spec: AppFormFieldSpec, field: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(APP_UI_THEME.foundation.spacing.tight_px))
        .child(app_text_label(spec.label))
        .child(field)
        .when_some(spec.note, |this, note| {
            this.child(app_text_body_subtle(note))
        })
}

pub fn app_form_input_text(
    spec: AppFormFieldSpec,
    input: &Entity<InputState>,
    disabled: bool,
) -> impl IntoElement {
    app_form_field(spec, app_input_text(input, disabled).w_full())
}

fn app_checkbox(
    id: &'static str,
    checked: bool,
    cx: &App,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let colors = APP_UI_THEME.components.app_checkbox_field;
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
    let row_text_px = APP_UI_THEME.foundation.typography.settings_row_text_px;
    let note_text_px = APP_UI_THEME.foundation.typography.utility_title_text_px;
    let note_indent_px = APP_UI_THEME.components.app_checkbox_field.size_px
        + APP_UI_THEME.shells.settings_checkbox_label_gap_px;
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
                        .foreground(rgb(APP_UI_THEME.foundation.text.primary).into())
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
                        .gap(px(APP_UI_THEME.shells.settings_checkbox_label_gap_px))
                        .child(app_checkbox(checkbox_id, checked, cx, {
                            let on_change = Rc::clone(&on_change);
                            move |checked, window, cx| on_change(checked, window, cx)
                        }))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(row_text_px))
                                .line_height(relative(1.1))
                                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
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
                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                    .child(note),
            )
        })
}

pub fn app_segment_button_icon(
    spec: AppSegmentButtonIconSpec,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let colors = APP_UI_THEME.components.app_segment_button_icon.colors;
    let sizing = APP_UI_THEME.components.app_segment_button_icon.sizing;
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

pub fn app_input_text(input: &Entity<InputState>, disabled: bool) -> Input {
    let tokens = APP_UI_THEME.components.app_input_text;
    let background = if disabled {
        tokens.disabled_background
    } else {
        tokens.background
    };
    let foreground = if disabled {
        APP_UI_THEME.foundation.text.secondary
    } else {
        APP_UI_THEME.foundation.text.primary
    };

    Input::new(input)
        .with_size(Size::Medium)
        .disabled(disabled)
        .focus_bordered(false)
        .bg(rgb(background))
        .text_color(rgb(foreground))
        .border_color(rgb(tokens.border))
        .border_1()
        .rounded(px(tokens.corner_radius_px))
}

pub fn app_button_secondary(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_button_label(
        app_button_base(id, AppButtonVariant::Secondary, on_click, cx),
        label.into(),
        APP_UI_THEME
            .components
            .app_button
            .sizing
            .horizontal_padding_px,
        AppButtonVariant::Secondary,
    )
}

pub fn app_button_primary(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_button_label(
        app_button_base(id, AppButtonVariant::Primary, on_click, cx),
        label.into(),
        APP_UI_THEME
            .components
            .app_button
            .sizing
            .horizontal_padding_px,
        AppButtonVariant::Primary,
    )
}

pub fn app_button_primary_disabled(
    id: &'static str,
    label: impl Into<SharedString>,
    cx: &App,
) -> impl IntoElement {
    app_button_label(
        app_button_base_disabled(id, AppButtonVariant::Primary, cx),
        label.into(),
        APP_UI_THEME
            .components
            .app_button
            .sizing
            .horizontal_padding_px,
        AppButtonVariant::Primary,
    )
}

pub fn app_button_compact(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    app_button_label(
        app_button_base(id, AppButtonVariant::Secondary, on_click, cx),
        label.into(),
        APP_UI_THEME
            .components
            .app_button
            .sizing
            .compact_horizontal_padding_px,
        AppButtonVariant::Secondary,
    )
}

fn app_button_label(
    button: Button,
    label: SharedString,
    horizontal_padding_px: f32,
    variant: AppButtonVariant,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = app_button_colors(variant);
    button.child(
        div()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .px(px(horizontal_padding_px))
            .whitespace_nowrap()
            .text_size(px(sizing.label_size_px))
            .text_color(rgb(colors.foreground))
            .child(label),
    )
}

pub fn app_button_icon(
    id: &'static str,
    icon: IconName,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = app_button_colors(AppButtonVariant::Secondary);

    app_button_base(id, AppButtonVariant::Secondary, on_click, cx)
        .with_size(Size::Size(px(sizing.square_width_px)))
        .icon(
            Icon::new(icon)
                .with_size(Size::Size(px(sizing.icon_size_px)))
                .text_color(rgb(colors.foreground)),
        )
}

pub fn app_status_indicator(color: u32) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_status_indicator;

    div()
        .size(px(sizing.size_px))
        .bg(rgb(color))
        .rounded(px(sizing.size_px / 2.0))
}

fn app_button_base(
    id: &'static str,
    variant: AppButtonVariant,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Button {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = app_button_colors(variant);
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

fn app_button_base_disabled(id: &'static str, variant: AppButtonVariant, cx: &App) -> Button {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = app_button_disabled_colors(variant);

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(colors.background).into())
                .foreground(rgb(colors.foreground).into())
                .border(transparent_black())
                .hover(rgb(colors.hover_background).into())
                .active(rgb(colors.active_background).into()),
        )
        .rounded(ButtonRounded::Size(px(sizing.corner_radius_px)))
        .h(px(sizing.height_px))
}

fn app_button_colors(variant: AppButtonVariant) -> crate::AppButtonColors {
    match variant {
        AppButtonVariant::Secondary => APP_UI_THEME.components.app_button.secondary_colors,
        AppButtonVariant::Primary => APP_UI_THEME.components.app_button.primary_colors,
    }
}

fn app_button_disabled_colors(variant: AppButtonVariant) -> crate::AppButtonColors {
    match variant {
        AppButtonVariant::Secondary | AppButtonVariant::Primary => {
            APP_UI_THEME.components.app_button.primary_disabled_colors
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_component::IconName;

    use super::{AppCheckboxFieldSpec, AppFormFieldSpec, AppSegmentButtonIconSpec};

    #[test]
    fn icon_segment_spec_preserves_id_and_label() {
        let spec = AppSegmentButtonIconSpec::new("settings", "Settings", IconName::Settings2);

        assert_eq!(spec.id, "settings");
        assert_eq!(spec.label.as_ref(), "Settings");
    }

    #[test]
    fn checkbox_field_spec_preserves_optional_note() {
        let spec = AppCheckboxFieldSpec::new("launch", "Launch at login", Some("Optional note"));

        assert_eq!(spec.id, "launch");
        assert_eq!(spec.label.as_ref(), "Launch at login");
        assert_eq!(
            spec.note.as_ref().map(|note| note.as_ref()),
            Some("Optional note")
        );
    }

    #[test]
    fn form_field_spec_preserves_optional_note() {
        let spec = AppFormFieldSpec::new("Farm name", Some("Saved locally"));

        assert_eq!(spec.label.as_ref(), "Farm name");
        assert_eq!(
            spec.note.as_ref().map(|note| note.as_ref()),
            Some("Saved locally")
        );
    }
}
