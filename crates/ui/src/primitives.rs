use gpui::{
    AnyElement, App, ClickEvent, Context, Corner, Div, ElementId, Entity, InteractiveElement,
    IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px, relative, rgb, transparent_black,
};
use gpui_component::{
    Icon, IconName, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariants, DropdownButton},
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenu},
    tab::{Tab, TabBar},
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

pub struct AppIconButtonSpec {
    pub id: &'static str,
    pub label: SharedString,
    pub icon: IconName,
}

impl AppIconButtonSpec {
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

pub struct AppUnderlineTabSpec {
    pub label: SharedString,
}

impl AppUnderlineTabSpec {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

pub struct AppPillTabSpec {
    pub label: SharedString,
}

impl AppPillTabSpec {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
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

pub fn app_focused_task_view(
    title: impl Into<SharedString>,
    body: impl IntoElement,
    actions: impl IntoElement,
) -> AnyElement {
    app_focused_view(
        APP_UI_THEME.shells.focused_task_max_width_px,
        title,
        body,
        actions,
    )
}

pub fn app_focused_detail_view(
    title: impl Into<SharedString>,
    body: impl IntoElement,
    actions: impl IntoElement,
) -> AnyElement {
    app_focused_view(
        APP_UI_THEME.shells.focused_detail_max_width_px,
        title,
        body,
        actions,
    )
}

fn app_focused_view(
    max_width_px: f32,
    title: impl Into<SharedString>,
    body: impl IntoElement,
    actions: impl IntoElement,
) -> AnyElement {
    div()
        .w_full()
        .max_w(px(max_width_px))
        .mx_auto()
        .child(app_surface_card(
            app_stack_v(APP_UI_THEME.shells.home_stack_gap_px)
                .w_full()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .justify_between()
                        .gap(px(APP_UI_THEME.shells.home_stack_gap_px))
                        .child(app_text_value(title))
                        .child(actions),
                )
                .child(body),
        ))
        .into_any_element()
}

pub fn app_stack_v(gap_px: f32) -> Div {
    div().flex().flex_col().gap(px(gap_px))
}

pub fn app_stack_h(gap_px: f32) -> Div {
    div().flex().items_center().gap(px(gap_px))
}

pub fn app_cluster(gap_px: f32) -> Div {
    div().flex().flex_wrap().items_center().gap(px(gap_px))
}

pub fn app_split_shell(sidebar: impl IntoElement, main_content: impl IntoElement) -> AnyElement {
    let sidebar = sidebar.into_any_element();
    let main_content = main_content.into_any_element();

    app_surface_window(
        APP_UI_THEME.foundation.surfaces.window_background,
        div()
            .size_full()
            .overflow_hidden()
            .flex()
            .child(sidebar)
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
                    .bg(rgb(APP_UI_THEME.foundation.surfaces.window_background))
                    .overflow_hidden()
                    .child(
                        div()
                            .size_full()
                            .p(px(APP_UI_THEME.shells.home_window_padding_px))
                            .child(main_content),
                    ),
            ),
    )
    .into_any_element()
}

pub fn app_scroll_panel(
    id: &'static str,
    content_padding_px: f32,
    content_max_width_px: Option<f32>,
    content: impl IntoElement,
) -> AnyElement {
    let content = content.into_any_element();
    let content: AnyElement = match content_max_width_px {
        Some(content_max_width_px) => div()
            .w_full()
            .max_w(px(content_max_width_px))
            .mx_auto()
            .child(content)
            .into_any_element(),
        None => div().w_full().child(content).into_any_element(),
    };

    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .w_full()
                .when(content_padding_px > 0.0, |this| {
                    this.p(px(content_padding_px))
                })
                .child(content),
        )
        .into_any_element()
}

pub fn app_divider() -> impl IntoElement {
    div()
        .w_full()
        .h(px(APP_UI_THEME.foundation.borders.divider_thickness_px))
        .bg(rgb(APP_UI_THEME.foundation.surfaces.divider))
}

pub fn app_underline_tabs(
    id: &'static str,
    tabs: impl IntoIterator<Item = AppUnderlineTabSpec>,
    selected_index: usize,
    on_click: impl Fn(&usize, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tab_text_px = APP_UI_THEME.foundation.typography.body_text_px + 1.0;
    let active_foreground = APP_UI_THEME.components.app_button.primary_colors.background;
    let inactive_foreground = APP_UI_THEME.foundation.text.secondary;
    let tab_gap_px = 16.0;
    let tabs = tabs.into_iter().collect::<Vec<_>>();
    let tab_widths = tabs
        .iter()
        .map(|tab| app_underline_tab_width_px(&tab.label, tab_text_px))
        .collect::<Vec<_>>();
    let selected_width_px = tab_widths.get(selected_index).copied().unwrap_or(0.0);
    let selected_offset_px = tab_widths.iter().take(selected_index).copied().sum::<f32>()
        + tab_gap_px * selected_index as f32;

    div()
        .relative()
        .w_full()
        .child(
            TabBar::new(id)
                .underline()
                .with_size(Size::Medium)
                .w_full()
                .children(
                    tabs.into_iter()
                        .enumerate()
                        .map(|(index, tab)| {
                            let is_selected = index == selected_index;
                            let foreground = if is_selected {
                                active_foreground
                            } else {
                                inactive_foreground
                            };

                            Tab::new().child(
                                div()
                                    .w(px(tab_widths.get(index).copied().unwrap_or(36.0)))
                                    .text_size(px(tab_text_px))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(rgb(foreground))
                                    .child(tab.label),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
                .on_click(on_click),
        )
        .child(
            div()
                .absolute()
                .left(px(selected_offset_px))
                .bottom_0()
                .w(px(selected_width_px))
                .h(px(2.0))
                .rounded(px(1.0))
                .bg(rgb(active_foreground)),
        )
}

pub fn app_pill_tabs(
    id: &'static str,
    tabs: impl IntoIterator<Item = AppPillTabSpec>,
    selected_index: usize,
    on_click: impl Fn(&usize, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let on_click: Rc<dyn Fn(&usize, &mut Window, &mut App)> = Rc::new(on_click);
    let tabs = tabs.into_iter().collect::<Vec<_>>();
    let height_px = APP_UI_THEME.components.app_button.sizing.height_px;
    let radius_px = height_px / 2.0;
    let horizontal_padding_px = APP_UI_THEME
        .components
        .app_button
        .sizing
        .compact_horizontal_padding_px;
    let label_size_px = APP_UI_THEME.components.app_button.sizing.label_size_px;
    let gap_px = APP_UI_THEME.foundation.spacing.micro_px;
    let primary = APP_UI_THEME.components.app_button.primary_colors;
    let inactive_foreground = APP_UI_THEME.foundation.text.secondary;
    let inactive_hover_background = APP_UI_THEME.foundation.surfaces.card_background;

    div()
        .id(id)
        .flex()
        .items_center()
        .w_full()
        .gap(px(gap_px))
        .children(tabs.into_iter().enumerate().map(|(index, tab)| {
            let is_selected = index == selected_index;
            let foreground = if is_selected {
                primary.foreground
            } else {
                inactive_foreground
            };
            let variant = if is_selected {
                ButtonCustomVariant::new(cx)
                    .color(rgb(primary.background).into())
                    .foreground(rgb(primary.foreground).into())
                    .border(transparent_black())
                    .hover(rgb(primary.hover_background).into())
                    .active(rgb(primary.active_background).into())
            } else {
                ButtonCustomVariant::new(cx)
                    .color(transparent_black().into())
                    .foreground(rgb(inactive_foreground).into())
                    .border(transparent_black())
                    .hover(rgb(inactive_hover_background).into())
                    .active(rgb(inactive_hover_background).into())
            };
            let on_click = Rc::clone(&on_click);

            Button::new((id, index))
                .custom(variant)
                .h(px(height_px))
                .rounded(ButtonRounded::Size(px(radius_px)))
                .on_click(move |_, window, cx| on_click(&index, window, cx))
                .child(
                    div()
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .px(px(horizontal_padding_px))
                        .text_size(px(label_size_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(foreground))
                        .child(tab.label),
                )
        }))
}

fn app_underline_tab_width_px(label: &SharedString, tab_text_px: f32) -> f32 {
    (label.as_ref().chars().count() as f32 * tab_text_px * 0.56).max(36.0)
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

pub fn app_detail_row(label: impl Into<SharedString>, value: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(APP_UI_THEME.shells.settings_account_detail_value_gap_px))
        .child(
            div()
                .text_size(px(APP_UI_THEME
                    .foundation
                    .typography
                    .settings_account_detail_text_px))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(label.into()),
        )
        .child(value)
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

    button.tab_stop(false)
}

pub fn app_checkbox_button(
    id: &'static str,
    checked: bool,
    cx: &App,
    on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    app_checkbox(id, checked, cx, on_change)
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
        .gap(px(APP_UI_THEME.foundation.spacing.micro_px))
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
        .focus_bordered(true)
        .bg(rgb(background))
        .text_color(rgb(foreground))
        .border_color(rgb(tokens.border))
        .border_1()
        .rounded(px(tokens.corner_radius_px))
}

pub fn app_button_secondary(
    id: impl Into<ElementId>,
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

pub fn app_button_secondary_full_width(
    id: impl Into<ElementId>,
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
    .w_full()
}

pub fn app_button_secondary_disabled(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> impl IntoElement {
    app_button_label(
        app_button_base_disabled(id, AppButtonVariant::Secondary, cx),
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
    id: impl Into<ElementId>,
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

pub fn app_button_primary_compact(
    id: impl Into<ElementId>,
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
            .compact_horizontal_padding_px,
        AppButtonVariant::Primary,
    )
}

pub fn app_button_primary_compact_disabled(
    id: impl Into<ElementId>,
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
            .compact_horizontal_padding_px,
        AppButtonVariant::Primary,
    )
}

pub fn app_button_primary_full_width(
    id: impl Into<ElementId>,
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
    .w_full()
}

pub fn app_button_primary_disabled(
    id: impl Into<ElementId>,
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
    id: impl Into<ElementId>,
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

pub fn app_button_square_dropdown_secondary(
    id: &'static str,
    menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    _cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = APP_UI_THEME.components.app_button.secondary_colors;

    div()
        .w(px(sizing.square_width_px))
        .h(px(sizing.height_px))
        .rounded(px(sizing.corner_radius_px))
        .bg(rgb(colors.background))
        .overflow_hidden()
        .child(
            DropdownButton::new(id)
                .button(
                    Button::new((id, 0usize))
                        .tab_stop(false)
                        .w(px(0.0))
                        .overflow_hidden()
                        .ghost()
                        .with_size(Size::Size(px(sizing.square_width_px))),
                )
                .dropdown_menu(menu)
                .ghost()
                .rounded(ButtonRounded::Size(px(sizing.corner_radius_px)))
                .with_size(Size::Size(px(sizing.square_width_px))),
        )
}

pub fn app_button_ellipsis_menu(
    id: &'static str,
    menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    _cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_button.sizing;

    Button::new(id)
        .ghost()
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .foundation
            .radii
            .medium_px)))
        .with_size(Size::Size(px(sizing.square_width_px)))
        .tab_stop(false)
        .child(
            Icon::new(IconName::Ellipsis)
                .with_size(Size::Size(px(sizing.icon_size_px)))
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary)),
        )
        .dropdown_menu_with_anchor(Corner::BottomRight, menu)
}

pub fn app_button_sidebar_account_menu(
    id: &'static str,
    label: impl Into<SharedString>,
    menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    cx: &App,
) -> impl IntoElement {
    let label = label.into();
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let row = APP_UI_THEME.components.app_account_selector_row;
    let horizontal_padding_px = APP_UI_THEME
        .shells
        .settings_account_sidebar_footer_button_gap_px;
    let icon_label_gap_px = APP_UI_THEME.foundation.spacing.micro_px;
    let icon_size = Size::Size(px(sizing.icon_size_px));
    let width_px = APP_UI_THEME.shells.home_sidebar_width_px
        - (APP_UI_THEME.shells.home_window_padding_px * 2.0);

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(row.inactive_background).into())
                .foreground(rgb(APP_UI_THEME.foundation.text.secondary).into())
                .border(transparent_black())
                .hover(rgb(row.active_background).into())
                .active(rgb(row.active_background).into()),
        )
        .w(px(width_px))
        .h(px(sizing.height_px))
        .p(px(0.0))
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .shells
            .settings_account_sidebar_button_corner_radius_px)))
        .tab_stop(false)
        .child(
            div()
                .w(px(width_px))
                .h_full()
                .px(px(horizontal_padding_px))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(APP_UI_THEME
                    .shells
                    .settings_account_sidebar_button_gap_px))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .min_w_0()
                        .gap(px(icon_label_gap_px))
                        .child(
                            div().flex_none().child(
                                Icon::new(IconName::CircleUser)
                                    .with_size(icon_size)
                                    .text_color(rgb(APP_UI_THEME.foundation.text.secondary)),
                            ),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .settings_row_text_px))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(label),
                        ),
                )
                .child(
                    div().flex_none().child(
                        Icon::new(IconName::ChevronsUpDown)
                            .with_size(icon_size)
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary)),
                    ),
                ),
        )
        .dropdown_menu_with_anchor(Corner::TopLeft, menu)
}

fn app_button_label(
    button: Button,
    label: SharedString,
    horizontal_padding_px: f32,
    variant: AppButtonVariant,
) -> Button {
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
    spec: AppIconButtonSpec,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let sizing = APP_UI_THEME.components.app_button.sizing;
    let colors = app_button_colors(AppButtonVariant::Secondary);

    app_button_base(spec.id, AppButtonVariant::Secondary, on_click, cx)
        .with_size(Size::Size(px(sizing.square_width_px)))
        .tooltip(spec.label)
        .icon(
            Icon::new(spec.icon)
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

pub fn app_button_text(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(transparent_black().into())
                .foreground(rgb(APP_UI_THEME.foundation.text.secondary).into())
                .border(transparent_black())
                .hover(transparent_black().into())
                .active(transparent_black().into()),
        )
        .rounded(ButtonRounded::Size(px(0.0)))
        .on_click(on_click)
        .child(
            div()
                .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                .child(label.into()),
        )
}

pub fn app_button_choice(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    is_active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let variant = if is_active {
        AppButtonVariant::Primary
    } else {
        AppButtonVariant::Secondary
    };

    app_button_label(
        app_button_base(id, variant, on_click, cx),
        label.into(),
        APP_UI_THEME
            .components
            .app_button
            .sizing
            .compact_horizontal_padding_px,
        variant,
    )
    .into_any_element()
}

pub fn app_button_list_row(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    subtitle: Option<SharedString>,
    is_selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let selected_background = rgb(APP_UI_THEME.foundation.surfaces.window_background);

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(if is_selected {
                    selected_background.into()
                } else {
                    transparent_black().into()
                })
                .foreground(rgb(APP_UI_THEME.foundation.text.primary).into())
                .border(transparent_black())
                .hover(selected_background.into())
                .active(selected_background.into()),
        )
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .foundation
            .radii
            .medium_px)))
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
                        .text_size(px(APP_UI_THEME.foundation.typography.body_text_px))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                        .child(title.into()),
                )
                .when_some(subtitle, |this, subtitle| {
                    this.child(
                        div()
                            .text_size(px(APP_UI_THEME.foundation.typography.utility_title_text_px))
                            .line_height(relative(1.2))
                            .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                            .child(subtitle),
                    )
                }),
        )
}

pub fn app_button_account_selector_row(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
    is_selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let tokens = APP_UI_THEME.components.app_account_selector_row;
    let background = if is_selected {
        tokens.active_background
    } else {
        tokens.inactive_background
    };

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(background).into())
                .foreground(rgb(APP_UI_THEME.foundation.text.primary).into())
                .border(transparent_black())
                .hover(rgb(background).into())
                .active(rgb(background).into()),
        )
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .shells
            .settings_account_sidebar_button_corner_radius_px)))
        .h(px(APP_UI_THEME
            .shells
            .settings_account_sidebar_button_height_px))
        .w_full()
        .min_w_0()
        .p(px(0.0))
        .on_click(on_click)
        .child(
            div()
                .w_full()
                .h_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(APP_UI_THEME
                    .shells
                    .settings_account_sidebar_button_gap_px))
                .px(px(APP_UI_THEME.foundation.spacing.small_px))
                .child(
                    div()
                        .size(px(APP_UI_THEME
                            .shells
                            .settings_account_sidebar_avatar_size_px))
                        .rounded_full()
                        .bg(rgb(APP_UI_THEME.foundation.surfaces.divider))
                        .flex_shrink_0(),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(APP_UI_THEME
                            .shells
                            .settings_account_identity_text_gap_px))
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .max_w_full()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .settings_account_identity_text_px
                                    + 1.0))
                                .font_weight(gpui::FontWeight::LIGHT)
                                .text_color(rgb(APP_UI_THEME.foundation.text.primary))
                                .child(title.into()),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .max_w_full()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(px(APP_UI_THEME
                                    .foundation
                                    .typography
                                    .utility_title_text_px))
                                .font_weight(gpui::FontWeight::NORMAL)
                                .text_color(rgb(APP_UI_THEME.foundation.text.secondary))
                                .child(subtitle.into()),
                        ),
                ),
        )
}

pub fn app_button_card(
    id: impl Into<ElementId>,
    is_selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
    content: impl IntoElement,
) -> impl IntoElement {
    let selected_background = rgb(APP_UI_THEME.foundation.surfaces.window_background);

    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(APP_UI_THEME.foundation.surfaces.card_background).into())
                .foreground(rgb(APP_UI_THEME.foundation.text.primary).into())
                .border(transparent_black())
                .hover(selected_background.into())
                .active(selected_background.into()),
        )
        .rounded(ButtonRounded::Size(px(APP_UI_THEME
            .foundation
            .radii
            .medium_px)))
        .w_full()
        .on_click(on_click)
        .child(
            div()
                .w_full()
                .min_w_0()
                .bg(rgb(if is_selected {
                    APP_UI_THEME.foundation.surfaces.window_background
                } else {
                    APP_UI_THEME.foundation.surfaces.card_background
                }))
                .rounded(px(APP_UI_THEME.foundation.radii.medium_px))
                .child(content),
        )
}

fn app_button_base(
    id: impl Into<ElementId>,
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

fn app_button_base_disabled(
    id: impl Into<ElementId>,
    variant: AppButtonVariant,
    cx: &App,
) -> Button {
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

    use super::{
        AppCheckboxFieldSpec, AppFormFieldSpec, AppIconButtonSpec, AppSegmentButtonIconSpec,
    };

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

    #[test]
    fn icon_button_spec_preserves_id_label_and_icon() {
        let spec = AppIconButtonSpec::new("more", "More actions", IconName::ChevronDown);

        assert_eq!(spec.id, "more");
        assert_eq!(spec.label.as_ref(), "More actions");
        assert!(matches!(spec.icon, IconName::ChevronDown));
    }
}
