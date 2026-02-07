#![forbid(unsafe_code)]

use std::sync::Arc;

use leptos::ev::MouseEvent;
use leptos::prelude::*;

use crate::{
    radroots_studio_app_ui_list_icon_key,
    radroots_studio_app_ui_list_styles_resolve,
    RadrootsAppUiIcon,
    RadrootsAppUiIconKey,
    RadrootsAppUiList,
    RadrootsAppUiListDisplay,
    RadrootsAppUiListDisplayValue,
    RadrootsAppUiListDefault,
    RadrootsAppUiListDefaultLabel,
    RadrootsAppUiListInput,
    RadrootsAppUiListInputAction,
    RadrootsAppUiListItem,
    RadrootsAppUiListItemKind,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
    RadrootsAppUiListOffset,
    RadrootsAppUiListOffsetMod,
    RadrootsAppUiListSelect,
    RadrootsAppUiListStylesResolved,
    RadrootsAppUiListTitle,
    RadrootsAppUiListTitleValue,
    RadrootsAppUiListTouch,
    RadrootsAppUiListTouchEnd,
    RadrootsAppUiListToggle,
    RadrootsAppUiSpinner,
};

pub fn radroots_studio_app_ui_list_group_data_ui_value() -> &'static str {
    "list-group"
}

pub fn radroots_studio_app_ui_list_section_data_ui_value() -> &'static str {
    "list-section"
}

pub fn radroots_studio_app_ui_list_row_data_ui_value() -> &'static str {
    "list-row"
}

pub fn radroots_studio_app_ui_list_row_leading_data_ui_value() -> &'static str {
    "list-row-leading"
}

pub fn radroots_studio_app_ui_list_row_trailing_data_ui_value() -> &'static str {
    "list-row-trailing"
}

fn radroots_studio_app_ui_list_base_id(id: Option<&str>, view: Option<&str>) -> String {
    let suffix = id.or(view).unwrap_or("default");
    format!("app-list-{suffix}")
}

fn radroots_studio_app_ui_list_title_id(base_id: &str) -> String {
    format!("{base_id}-title")
}

fn radroots_studio_app_ui_list_items_id(base_id: &str) -> String {
    format!("{base_id}-items")
}

fn radroots_studio_app_ui_list_item_id(base_id: &str, index: usize) -> String {
    format!("{base_id}-item-{index}")
}

fn radroots_studio_app_ui_list_line_id(base_id: &str, index: usize) -> String {
    format!("{base_id}-line-{index}")
}

#[component]
pub fn RadrootsAppUiListGroup(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <section
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_group_data_ui_value()
        >
            {children()}
        </section>
    }
}

#[component]
pub fn RadrootsAppUiListSection(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <section
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_section_data_ui_value()
        >
            {children()}
        </section>
    }
}

#[component]
pub fn RadrootsAppUiListRow(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <li
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_row_data_ui_value()
        >
            {children()}
        </li>
    }
}

#[component]
pub fn RadrootsAppUiListRowLeading(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_row_leading_data_ui_value()
        >
            {children()}
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListRowTrailing(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_row_trailing_data_ui_value()
        >
            {children()}
        </div>
    }
}

fn radroots_studio_app_ui_list_class_merge(parts: &[Option<&str>]) -> String {
    let mut result = String::new();
    for part in parts {
        if let Some(value) = part {
            if value.is_empty() {
                continue;
            }
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(value);
        }
    }
    result
}

fn radroots_studio_app_ui_list_active_class(hide_active: bool) -> Option<&'static str> {
    if hide_active { None } else { Some("opacity-active") }
}

pub fn radroots_studio_app_ui_list_border_classes(
    hide_border_top: bool,
    hide_border_bottom: bool,
) -> String {
    let top = if hide_border_top {
        "group-first:border-t-0"
    } else {
        "group-first:border-t-line"
    };
    let bottom = if hide_border_bottom {
        "group-last:border-b-0"
    } else {
        "group-last:border-b-line"
    };
    format!("{top} {bottom}")
}

#[component]
pub fn RadrootsAppUiListLine(
    #[prop(optional)] id: String,
    as_button: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] hide_border_top: bool,
    #[prop(optional)] hide_border_bottom: bool,
    on_click: Option<Callback<MouseEvent>>,
    end: Option<ChildrenFn>,
    #[prop(optional)] overlay: Option<ChildrenFn>,
    children: ChildrenFn,
) -> impl IntoView {
    let border_class = radroots_studio_app_ui_list_border_classes(hide_border_top, hide_border_bottom);
    let line_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-full w-full justify-center items-center border-t-line el-re"),
        Some(border_class.as_str()),
    ]);
    let line_state = if loading { "loading" } else { "ready" };
    let end_view = end.map(|slot| slot());
    let overlay_view = overlay.map(|slot| slot());
    let id = if id.is_empty() { None } else { Some(id) };
    let line_inner = view! {
        <div class=line_class data-ui="list-line">
            {if loading {
                view! {
                    <div class="flex flex-row h-full w-full justify-center items-center">
                        <RadrootsAppUiSpinner />
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="relative group flex flex-row h-line w-full pr-[2px] justify-between items-center el-re">
                        <div class="flex flex-row h-full w-trellis_display justify-between items-center">
                            {children()}
                        </div>
                        {end_view}
                        {overlay_view}
                    </div>
                }
                .into_any()
            }}
        </div>
    };
    let has_click = on_click.is_some();
    let click_handler = move |ev: MouseEvent| {
        if let Some(callback) = &on_click {
            callback.run(ev);
        }
    };
    if as_button {
        view! {
            <button
                type="button"
                id=id
                class="flex flex-row flex-grow overflow-hidden"
                aria-busy=loading
                data-state=line_state
                on:click=click_handler
            >
                {line_inner}
            </button>
        }
        .into_any()
    } else {
        let role = if has_click { Some("button") } else { None };
        let tabindex = if has_click { Some(0) } else { None };
        view! {
            <div
                id=id
                class="flex flex-row flex-grow overflow-hidden"
                aria-busy=loading
                data-state=line_state
                role=role
                tabindex=tabindex
                on:click=click_handler
            >
                {line_inner}
            </div>
        }
        .into_any()
    }
}

fn radroots_studio_app_ui_list_title_padding_class(mod_value: Option<&RadrootsAppUiListOffsetMod>) -> Option<&'static str> {
    match mod_value {
        Some(RadrootsAppUiListOffsetMod::Small) => Some("pl-[16px]"),
        Some(RadrootsAppUiListOffsetMod::Glyph)
        | Some(RadrootsAppUiListOffsetMod::Icon { .. })
        | Some(RadrootsAppUiListOffsetMod::IconCircle { .. }) => Some("pl-[36px]"),
        None => None,
    }
}

fn radroots_studio_app_ui_list_default_labels(
    labels: Option<&[RadrootsAppUiListDefaultLabel]>,
) -> Vec<RadrootsAppUiListDefaultLabel> {
    labels.map_or_else(
        || {
            vec![RadrootsAppUiListDefaultLabel {
                label: "No items to display.".to_string(),
                classes: None,
                on_click: None,
            }]
        },
        |labels| labels.to_vec(),
    )
}

fn radroots_studio_app_ui_list_offset_mod(
    mod_value: Option<&RadrootsAppUiListOffsetMod>,
) -> RadrootsAppUiListOffsetMod {
    mod_value.cloned().unwrap_or(RadrootsAppUiListOffsetMod::Small)
}

fn radroots_studio_app_ui_list_input_action_icon_key(
    action: &RadrootsAppUiListInputAction,
) -> RadrootsAppUiIconKey {
    action
        .icon
        .as_ref()
        .and_then(radroots_studio_app_ui_list_icon_key)
        .unwrap_or(RadrootsAppUiIconKey::Plus)
}

fn radroots_studio_app_ui_list_display_loading(display: Option<&RadrootsAppUiListDisplay>) -> bool {
    display.map(|value| value.loading).unwrap_or(false)
}

fn radroots_studio_app_ui_list_title_visible(
    title: Option<&RadrootsAppUiListTitle>,
    default_state: Option<&RadrootsAppUiListDefault>,
) -> bool {
    match title {
        None => false,
        Some(_) => default_state.map(|value| value.show_title).unwrap_or(true),
    }
}

fn radroots_studio_app_ui_list_row_class(
    item: &RadrootsAppUiListItem,
    styles: &RadrootsAppUiListStylesResolved,
) -> String {
    let active_class = radroots_studio_app_ui_list_active_class(item.hide_active);
    radroots_studio_app_ui_list_class_merge(&[
        Some("group flex flex-row h-full w-full justify-end items-center el-re"),
        Some("list-row-surface"),
        if item.hide_field { Some("hidden") } else { None },
        if item.full_rounded { Some("rounded-touch") } else { None },
        if styles.hide_rounded {
            None
        } else {
            Some("first:rounded-t-2xl last:rounded-b-2xl")
        },
        active_class,
    ])
}

fn radroots_studio_app_ui_list_label_value_view(
    value: RadrootsAppUiListLabelValue,
    is_right: bool,
    hide_active: bool,
) -> AnyView {
    let RadrootsAppUiListLabelValue {
        classes_wrap,
        hide_truncate,
        value,
    } = value;
    let active_class = radroots_studio_app_ui_list_active_class(hide_active);
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-full items-center"),
        if hide_truncate { None } else { Some("truncate") },
        classes_wrap.as_deref(),
    ]);
    let view = match value {
        RadrootsAppUiListLabelValueKind::Text(value) => {
            let text_class = radroots_studio_app_ui_list_class_merge(&[
                Some("text-line_d"),
                if is_right { Some("ui-text-secondary") } else { None },
                active_class,
                if hide_truncate { None } else { Some("truncate") },
                value.classes.as_deref(),
            ]);
            view! { <p class=text_class>{value.value}</p> }.into_any()
        }
        RadrootsAppUiListLabelValueKind::Icon(icon) => {
            let icon_key = radroots_studio_app_ui_list_icon_key(&icon);
            let icon_class = radroots_studio_app_ui_list_class_merge(&[
                if is_right { Some("ui-text-secondary") } else { None },
                active_class,
                icon.class.as_deref(),
            ]);
            if let Some(icon_key) = icon_key {
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=16 /> }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }
        }
    };
    view! { <div class=wrap_class>{view}</div> }.into_any()
}

#[component]
pub fn RadrootsAppUiListRowLabel(
    basis: RadrootsAppUiListLabel,
    #[prop(optional)] hide_active: bool,
) -> impl IntoView {
    let left_values = basis.left;
    let right_values = basis.right;
    let left_view = left_values
        .into_iter()
        .map(|value| radroots_studio_app_ui_list_label_value_view(value, false, hide_active))
        .collect_view();
    let right_view = right_values
        .into_iter()
        .rev()
        .map(|value| radroots_studio_app_ui_list_label_value_view(value, true, hide_active))
        .collect_view();
    view! {
        <div class="flex flex-row h-full w-full items-center justify-between">
            <div class="flex flex-row h-full items-center truncate">
                {left_view}
            </div>
            <div class="flex flex-row h-full items-center justify-end pr-4">
                {right_view}
            </div>
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListRowDisplayValue(
    basis: RadrootsAppUiListDisplay,
    #[prop(optional)] hide_active: bool,
) -> impl IntoView {
    let on_click = basis.on_click;
    let display = match basis.value {
        RadrootsAppUiListDisplayValue::Icon(icon) => {
            let icon_key = radroots_studio_app_ui_list_icon_key(&icon);
            let active_class = radroots_studio_app_ui_list_active_class(hide_active);
            let icon_class = radroots_studio_app_ui_list_class_merge(&[
                Some("ui-text-secondary"),
                active_class,
                icon.class.as_deref(),
            ]);
            if let Some(icon_key) = icon_key {
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=18 /> }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }
        }
        RadrootsAppUiListDisplayValue::Label(label) => {
            let active_class = radroots_studio_app_ui_list_active_class(hide_active);
            let text_class = radroots_studio_app_ui_list_class_merge(&[
                Some("font-sans text-line_d_e line-clamp-1 text-ly0-gl-label el-re"),
                active_class,
                label.classes.as_deref(),
            ]);
            view! { <p class=text_class>{label.value}</p> }.into_any()
        }
    };
    view! {
        <button
            type="button"
            class="z-10 flex flex-grow justify-end"
            on:click=move |ev: MouseEvent| {
                ev.stop_propagation();
                if let Some(callback) = &on_click {
                    callback.run(ev);
                }
            }
        >
            {display}
        </button>
    }
}

#[component]
pub fn RadrootsAppUiListOffsetView(
    basis: Option<RadrootsAppUiListOffset>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let basis = basis.unwrap_or(RadrootsAppUiListOffset {
        mod_value: None,
        classes: None,
        hide_space: false,
        hide_offset: false,
        on_click: None,
    });
    if basis.hide_offset {
        return view! { <div></div> }.into_any();
    }
    let mod_value = radroots_studio_app_ui_list_offset_mod(basis.mod_value.as_ref());
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-full"),
        class.as_deref(),
        basis.classes.as_deref(),
    ]);
    let on_click = basis.on_click;
    match mod_value {
        RadrootsAppUiListOffsetMod::Small => view! {
            <div class=wrap_class>
                <div class="flex flex-row h-full w-[22px]">
                    <div class="flex-fluid"></div>
                </div>
            </div>
        }
        .into_any(),
        RadrootsAppUiListOffsetMod::Glyph => view! {
            <div class=wrap_class>
                <div class="flex flex-row pr-[2px]">
                    <div class="flex flex-row h-full w-trellisOffset">
                        <div class="flex-fluid"></div>
                    </div>
                </div>
            </div>
        }
        .into_any(),
        RadrootsAppUiListOffsetMod::Icon { icon, loading } => {
            let icon_key = radroots_studio_app_ui_list_icon_key(&icon);
            let icon_class = radroots_studio_app_ui_list_class_merge(&[
                Some("ui-text-secondary"),
                icon.class.as_deref(),
            ]);
            let button_class = radroots_studio_app_ui_list_class_merge(&[
                Some("fade-in pl-2 translate-x-[3px] translate-y-[1px]"),
            ]);
            let icon_view = if loading {
                view! { <RadrootsAppUiSpinner class="text-[12px]".to_string() /> }.into_any()
            } else if let Some(icon_key) = icon_key {
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=16 /> }.into_any()
            } else {
                view! { <div></div> }.into_any()
            };
            view! {
                <div class=wrap_class>
                    <div class="flex flex-row h-full min-w-[20px] w-trellisOffset justify-center items-center pr-3">
                        <button
                            type="button"
                            class=button_class
                            on:click=move |ev: MouseEvent| {
                                if loading {
                                    return;
                                }
                                if let Some(callback) = &on_click {
                                    callback.run(ev);
                                }
                            }
                        >
                            {icon_view}
                        </button>
                    </div>
                </div>
            }
            .into_any()
        }
        RadrootsAppUiListOffsetMod::IconCircle { icon, loading } => {
            let icon_key = radroots_studio_app_ui_list_icon_key(&icon);
            let icon_class = radroots_studio_app_ui_list_class_merge(&[
                Some("ui-text-secondary"),
                icon.class.as_deref(),
            ]);
            let button_class = radroots_studio_app_ui_list_class_merge(&[
                Some("fade-in pl-2 translate-x-[3px] translate-y-[1px] rounded-full"),
            ]);
            let icon_view = if loading {
                view! { <RadrootsAppUiSpinner class="text-[12px]".to_string() /> }.into_any()
            } else if let Some(icon_key) = icon_key {
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=16 /> }.into_any()
            } else {
                view! { <div></div> }.into_any()
            };
            view! {
                <div class=wrap_class>
                    <div class="flex flex-row h-full min-w-[20px] w-trellisOffset justify-center items-center pr-3">
                        <button
                            type="button"
                            class=button_class
                            on:click=move |ev: MouseEvent| {
                                if loading {
                                    return;
                                }
                                if let Some(callback) = &on_click {
                                    callback.run(ev);
                                }
                            }
                        >
                            {icon_view}
                        </button>
                    </div>
                </div>
            }
            .into_any()
        }
    }
}

#[component]
pub fn RadrootsAppUiListTouchEndView(
    basis: RadrootsAppUiListTouchEnd,
    #[prop(optional)] hide_active: bool,
) -> impl IntoView {
    let icon_key = radroots_studio_app_ui_list_icon_key(&basis.icon);
    let active_class = radroots_studio_app_ui_list_active_class(hide_active);
    let icon_class = radroots_studio_app_ui_list_class_merge(&[
        Some("ui-text-secondary opacity-70 translate-y-[1px]"),
        active_class,
        basis.icon.class.as_deref(),
    ]);
    let on_click = basis.on_click;
    let icon_view = icon_key.map(|icon_key| {
        view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=14 /> }.into_any()
    });
    view! {
        <div class="absolute top-0 right-0 h-full w-max flex flex-row justify-center items-center">
            <button
                type="button"
                class="flex pr-3"
                on:click=move |ev: MouseEvent| {
                    if let Some(callback) = &on_click {
                        callback.run(ev);
                    }
                }
            >
                {icon_view}
            </button>
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListTouchRow(
    basis: RadrootsAppUiListTouch,
    #[prop(optional)] line_id: String,
    #[prop(optional)] hide_active: bool,
    #[prop(optional)] hide_border_top: bool,
    #[prop(optional)] hide_border_bottom: bool,
    #[prop(optional)] loading: bool,
) -> impl IntoView {
    let label = basis.label;
    let display = basis.display;
    let end = basis.end;
    let on_click = basis.on_click;
    let end_slot = end.map(|end| {
        let hide_active = hide_active;
        Arc::new(move || {
            let end_value = end.clone();
            view! { <RadrootsAppUiListTouchEndView basis=end_value hide_active=hide_active /> }.into_any()
        }) as ChildrenFn
    });
    view! {
        <RadrootsAppUiListLine
            id=line_id
            as_button=true
            loading=loading
            hide_border_top=hide_border_top
            hide_border_bottom=hide_border_bottom
            on_click=on_click
            end=end_slot
        >
            <RadrootsAppUiListRowLabel basis=label.clone() hide_active=hide_active />
            {display.as_ref().map(|display| {
                let display = display.clone();
                view! { <RadrootsAppUiListRowDisplayValue basis=display hide_active=hide_active /> }.into_any()
            })}
        </RadrootsAppUiListLine>
    }
}

#[component]
pub fn RadrootsAppUiListToggleRow(
    basis: RadrootsAppUiListToggle,
    #[prop(optional)] line_id: String,
    #[prop(optional)] hide_active: bool,
    #[prop(optional)] hide_border_top: bool,
    #[prop(optional)] hide_border_bottom: bool,
    #[prop(optional)] loading: bool,
) -> impl IntoView {
    let label = basis.label;
    let checked = basis.checked;
    let disabled = basis.disabled;
    let on_toggle = basis.on_toggle;
    let switch_class = if checked {
        "ios-switch ios-switch--checked"
    } else {
        "ios-switch"
    };
    let end_slot = Arc::new(move || {
        view! {
            <span class="flex flex-row h-full items-center pr-3">
                <span class=switch_class aria-hidden="true">
                    <span class="ios-switch__thumb"></span>
                </span>
            </span>
        }
        .into_any()
    }) as ChildrenFn;
    let on_click = if disabled {
        None
    } else {
        let on_toggle = on_toggle.clone();
        Some(Callback::new(move |_ev: MouseEvent| {
            if let Some(callback) = &on_toggle {
                callback.run(!checked);
            }
        }))
    };
    view! {
        <RadrootsAppUiListLine
            id=line_id
            as_button=true
            loading=loading
            hide_border_top=hide_border_top
            hide_border_bottom=hide_border_bottom
            on_click=on_click
            end=Some(end_slot)
        >
            <RadrootsAppUiListRowLabel basis=label.clone() hide_active=hide_active />
        </RadrootsAppUiListLine>
    }
}

#[component]
pub fn RadrootsAppUiListInputRow(
    basis: RadrootsAppUiListInput,
    #[prop(optional)] line_id: String,
    #[prop(optional)] hide_border_top: bool,
    #[prop(optional)] hide_border_bottom: bool,
) -> impl IntoView {
    let RadrootsAppUiListInput {
        field,
        line_label,
        action,
    } = basis;
    let line_id = if line_id.is_empty() { None } else { Some(line_id) };
    let border_class = radroots_studio_app_ui_list_border_classes(hide_border_top, hide_border_bottom);
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-line w-full justify-start items-center border-t-line overflow-hidden"),
        Some(border_class.as_str()),
    ]);
    let line_label_view = line_label.map(|line_label| {
        let label_class = radroots_studio_app_ui_list_class_merge(&[
            Some("text-form_base ui-text-secondary"),
            line_label.classes.as_deref(),
        ]);
        view! {
            <div class="flex flex-row h-full justify-start items-center overflow-x-hidden">
                <p class=label_class>{line_label.value}</p>
            </div>
        }
        .into_any()
    });
    let input_class = radroots_studio_app_ui_list_class_merge(&[
        Some("el-input"),
        field.classes.as_deref(),
    ]);
    let input_id = field.id;
    let input_value = field.value;
    let input_placeholder = field.placeholder;
    let input_disabled = field.disabled;
    let on_input = field.on_input;
    let action_view = action.and_then(|action| {
        if !action.visible {
            return None;
        }
        let action_loading = action.loading;
        let action_icon_key = radroots_studio_app_ui_list_input_action_icon_key(&action);
        let action_icon_class = radroots_studio_app_ui_list_class_merge(&[
            Some("ui-text-secondary"),
            action.icon.as_ref().and_then(|icon| icon.class.as_deref()),
        ]);
        let on_click = action.on_click;
        Some(
            view! {
                <div class="absolute top-0 right-0 flex flex-row h-full w-12 pr-4 justify-end items-center fade-in">
                    {if action_loading {
                        view! { <RadrootsAppUiSpinner class="text-[12px]".to_string() /> }
                            .into_any()
                    } else {
                        view! {
                            <button
                                type="button"
                                class="group fade-in-long"
                                on:click=move |ev: MouseEvent| {
                                    if let Some(callback) = &on_click {
                                        callback.run(ev);
                                    }
                                }
                            >
                                <RadrootsAppUiIcon key=action_icon_key class=action_icon_class size=18 />
                            </button>
                        }
                        .into_any()
                    }}
                </div>
            }
            .into_any(),
        )
    });
    view! {
        <div
            id=line_id
            class="flex flex-row flex-grow h-full w-full"
            data-ui="list-input"
        >
            <div class=wrap_class>
                {line_label_view}
                <div class="relative flex flex-row flex-grow h-full pr-12 justify-start items-center">
                    <input
                        id=input_id
                        class=input_class
                        disabled=input_disabled
                        placeholder=input_placeholder
                        prop:value=input_value
                        on:input=move |ev| {
                            if let Some(callback) = &on_input {
                                callback.run(event_target_value(&ev));
                            }
                        }
                    />
                    {action_view}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListSelectRow(
    basis: RadrootsAppUiListSelect,
    #[prop(optional)] line_id: String,
    #[prop(optional)] hide_active: bool,
    #[prop(optional)] hide_border_top: bool,
    #[prop(optional)] hide_border_bottom: bool,
) -> impl IntoView {
    let RadrootsAppUiListSelect {
        field,
        label,
        display,
        end,
        loading,
        on_click,
    } = basis;
    let end_slot = end.map(|end| {
        let hide_active = hide_active;
        Arc::new(move || {
            let end_value = end.clone();
            view! { <RadrootsAppUiListTouchEndView basis=end_value hide_active=hide_active /> }.into_any()
        }) as ChildrenFn
    });
    let display_loading = radroots_studio_app_ui_list_display_loading(display.as_ref());
    let select_class = radroots_studio_app_ui_list_class_merge(&[
        Some("el-select"),
        Some("list-select-hit"),
        field.classes.as_deref(),
    ]);
    let select_id = field.id;
    let select_value = field.value.clone();
    let select_disabled = field.disabled || loading;
    let on_change = field.on_change;
    let options = Arc::new(field.options);
    let selected_value = RwSignal::new(select_value.clone());
    let selected_label = RwSignal::new(
        options
            .iter()
            .find(|option| option.value == select_value)
            .map(|option| option.label.clone())
            .unwrap_or_default(),
    );
    let selected_class = radroots_studio_app_ui_list_class_merge(&[
        Some("font-sans text-line_d_e line-clamp-1 text-ly0-gl-label el-re"),
        radroots_studio_app_ui_list_active_class(hide_active),
    ]);
    let select_overlay = {
        let select_class = select_class.clone();
        let select_id = select_id.clone();
        let on_change = on_change.clone();
        let on_click = on_click.clone();
        let options = Arc::clone(&options);
        let selected_label = selected_label;
        Arc::new(move || {
            let options_for_change = Arc::clone(&options);
            let options_for_view = Arc::clone(&options);
            view! {
                <select
                    id=select_id.clone()
                    class=select_class.clone()
                    disabled=select_disabled
                    prop:value=move || selected_value.get()
                    on:click=move |ev| {
                        if let Some(callback) = &on_click {
                            callback.run(ev);
                        }
                    }
                    on:change=move |ev| {
                        let next_value = event_target_value(&ev);
                        selected_value.set(next_value.clone());
                        let next_label = options_for_change
                            .iter()
                            .find(|option| option.value == next_value)
                            .map(|option| option.label.clone())
                            .unwrap_or_default();
                        selected_label.set(next_label);
                        if let Some(callback) = &on_change {
                            callback.run(next_value);
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            use leptos::wasm_bindgen::JsCast;
                            use leptos::web_sys;

                            if let Some(target) = ev.target() {
                                if let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>() {
                                    let _ = select.blur();
                                }
                            }
                        }
                    }
                >
                    {options_for_view
                        .iter()
                        .cloned()
                        .map(|option| {
                            let class = radroots_studio_app_ui_list_class_merge(&[
                                option.classes.as_deref(),
                            ]);
                            view! { <option value=option.value class=class>{option.label}</option> }
                        })
                        .collect_view()}
                </select>
            }
            .into_any()
        }) as ChildrenFn
    };
    view! {
        <RadrootsAppUiListLine
            id=line_id
            as_button=false
            loading=loading
            hide_border_top=hide_border_top
            hide_border_bottom=hide_border_bottom
            on_click=None
            end=end_slot
            overlay=select_overlay
        >
            <RadrootsAppUiListRowLabel basis=label.clone() hide_active=hide_active />
            <div class="relative flex flex-row pr-3 justify-center items-end" data-ui="list-select">
                {if display_loading {
                    view! { <RadrootsAppUiSpinner class="text-[12px]".to_string() /> }.into_any()
                } else if let Some(display) = display.as_ref() {
                    let display = display.clone();
                    view! { <RadrootsAppUiListRowDisplayValue basis=display hide_active=hide_active /> }.into_any()
                } else {
                    view! {
                        <p class=selected_class.clone()>
                            {move || selected_label.get()}
                        </p>
                    }
                    .into_any()
                }}
            </div>
        </RadrootsAppUiListLine>
    }
}

#[component]
pub fn RadrootsAppUiListTitleView(
    basis: RadrootsAppUiListTitle,
    id: Option<String>,
) -> impl IntoView {
    let title_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-[24px] w-full pl-[2px] gap-1 items-center"),
        basis.classes.as_deref(),
    ]);
    let padding_class = radroots_studio_app_ui_list_title_padding_class(basis.mod_value.as_ref());
    let button_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-full w-max items-center gap-1"),
        padding_class,
    ]);
    let on_click = basis.on_click;
    let has_click = on_click.is_some();
    let title_value = match basis.value {
        RadrootsAppUiListTitleValue::Spacer => {
            view! { <div class="flex-fluid"></div> }.into_any()
        }
        RadrootsAppUiListTitleValue::Text(value) => {
            view! { <p class="text-trellis_ti uppercase ui-text-tertiary">{value}</p> }.into_any()
        }
    };
    let link_view = basis.link.map(|link| {
        let label_view = link.label.map(|label| match label.value {
            RadrootsAppUiListLabelValueKind::Text(text) => {
                let class = radroots_studio_app_ui_list_class_merge(&[
                    Some("text-trellis_ti uppercase fade-in"),
                    text.classes.as_deref(),
                ]);
                view! { <p class=class>{text.value}</p> }.into_any()
            }
            RadrootsAppUiListLabelValueKind::Icon(icon) => {
                let icon_key = radroots_studio_app_ui_list_icon_key(&icon);
                let icon_class = radroots_studio_app_ui_list_class_merge(&[
                    Some("fade-in"),
                    icon.class.as_deref(),
                ]);
                if let Some(icon_key) = icon_key {
                    view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=16 /> }
                        .into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }
        });
        let icon_view = link.icon.and_then(|icon| {
            radroots_studio_app_ui_list_icon_key(&icon).map(|icon_key| {
                let icon_class = radroots_studio_app_ui_list_class_merge(&[
                    Some("fade-in"),
                    icon.class.as_deref(),
                ]);
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=16 /> }.into_any()
            })
        });
        let link_class = radroots_studio_app_ui_list_class_merge(&[
            Some("group flex flex-row h-full w-max items-center"),
            link.classes.as_deref(),
        ]);
        let on_click = link.on_click;
        view! {
            <button
                type="button"
                class=link_class
                on:click=move |_| {
                    if let Some(callback) = &on_click {
                        callback.run(());
                    }
                }
            >
                {label_view}
                {icon_view}
            </button>
        }
        .into_any()
    });
    let title_button = if has_click {
        view! {
            <button
                type="button"
                id=id.clone()
                class=button_class
                on:click=move |_| {
                    if let Some(callback) = &on_click {
                        callback.run(());
                    }
                }
            >
                {title_value}
            </button>
        }
        .into_any()
    } else {
        view! {
            <div id=id.clone() class=button_class>
                {title_value}
            </div>
        }
        .into_any()
    };
    view! {
        <div class=title_class>
            {title_button}
            {link_view}
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListDefaultLabels(
    labels: Option<Vec<RadrootsAppUiListDefaultLabel>>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let labels = radroots_studio_app_ui_list_default_labels(labels.as_deref());
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row"),
        class.as_deref(),
    ]);
    let items = labels
        .into_iter()
        .map(|label| {
            let inner_class = radroots_studio_app_ui_list_class_merge(&[
                Some("text-trellis_ti"),
                label.classes.as_deref(),
            ]);
            let on_click = label.on_click;
            if on_click.is_some() {
                view! {
                    <button
                        type="button"
                        class=inner_class
                        on:click=move |_| {
                            if let Some(callback) = &on_click {
                                callback.run(());
                            }
                        }
                    >
                        {label.label}
                    </button>
                }
                .into_any()
            } else {
                view! { <span class=inner_class>{label.label}</span> }.into_any()
            }
        })
        .collect_view();
    view! {
        <div class=wrap_class>
            <p class="text-trellis_ti ui-text-tertiary">{items}</p>
        </div>
    }
}

#[component]
pub fn RadrootsAppUiListView(basis: RadrootsAppUiList) -> impl IntoView {
    let RadrootsAppUiList {
        id,
        view,
        classes,
        title,
        default_state,
        list,
        hide_offset,
        styles,
    } = basis;
    let base_id = radroots_studio_app_ui_list_base_id(id.as_deref(), view.as_deref());
    let title_id = radroots_studio_app_ui_list_title_id(base_id.as_str());
    let items_id = radroots_studio_app_ui_list_items_id(base_id.as_str());
    let resolved_styles = radroots_studio_app_ui_list_styles_resolve(styles.as_ref());
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-col"),
        classes.as_deref(),
    ]);
    let group_class = radroots_studio_app_ui_list_class_merge(&[
        Some("relative flex flex-col h-auto w-full gap-[3px]"),
    ]);
    let list_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-col w-full justify-center items-center"),
        if resolved_styles.set_title_background {
            Some("list-group-surface")
        } else {
            None
        },
    ]);
    let view_value = view.unwrap_or_default();
    let title_view = if radroots_studio_app_ui_list_title_visible(title.as_ref(), default_state.as_ref())
    {
        let title = title.map(|title| {
            view! { <RadrootsAppUiListTitleView basis=title id=Some(title_id.clone()) /> }
                .into_any()
        });
        Some(
            view! {
                <header class="flex flex-col w-full" data-ui="list-header">
                    {title}
                </header>
            }
            .into_any(),
        )
    } else {
        None
    };
    let content_view = if let Some(default_state) = default_state {
        let default_class = radroots_studio_app_ui_list_class_merge(&[
            Some("flex flex-col h-auto w-full justify-center items-center"),
            if resolved_styles.set_default_background {
                Some("ui-surface")
            } else {
                None
            },
            default_state.classes.as_deref(),
        ]);
        Some(
            view! {
                <div class=default_class>
                    <RadrootsAppUiListDefaultLabels labels=default_state.labels />
                </div>
            }
            .into_any(),
        )
    } else if let Some(list) = list {
        let items = list
            .into_iter()
            .enumerate()
            .filter_map(|(index, item)| item.map(|item| (index, item)))
            .map(|(index, item)| {
                let row_class = radroots_studio_app_ui_list_row_class(&item, &resolved_styles);
                let row_id = radroots_studio_app_ui_list_item_id(base_id.as_str(), index);
                let line_id = radroots_studio_app_ui_list_line_id(base_id.as_str(), index);
                let row_state = if item.loading { "loading" } else { "ready" };
                let offset_view = if hide_offset {
                    None
                } else {
                    Some(
                        view! { <RadrootsAppUiListOffsetView basis=item.offset.clone() /> }
                            .into_any(),
                    )
                };
                let row_view = match item.kind {
                    RadrootsAppUiListItemKind::Touch(touch) => view! {
                        <RadrootsAppUiListTouchRow
                            basis=touch
                            loading=item.loading
                            hide_active=item.hide_active
                            hide_border_top=resolved_styles.hide_border_top
                            hide_border_bottom=resolved_styles.hide_border_bottom
                            line_id=line_id.clone()
                        />
                    }
                    .into_any(),
                    RadrootsAppUiListItemKind::Toggle(toggle) => view! {
                        <RadrootsAppUiListToggleRow
                            basis=toggle
                            loading=item.loading
                            hide_active=item.hide_active
                            hide_border_top=resolved_styles.hide_border_top
                            hide_border_bottom=resolved_styles.hide_border_bottom
                            line_id=line_id.clone()
                        />
                    }
                    .into_any(),
                    RadrootsAppUiListItemKind::Input(input) => view! {
                        <RadrootsAppUiListInputRow
                            basis=input
                            hide_border_top=resolved_styles.hide_border_top
                            hide_border_bottom=resolved_styles.hide_border_bottom
                            line_id=line_id.clone()
                        />
                    }
                    .into_any(),
                    RadrootsAppUiListItemKind::Select(select) => view! {
                        <RadrootsAppUiListSelectRow
                            basis=select
                            hide_active=item.hide_active
                            hide_border_top=resolved_styles.hide_border_top
                            hide_border_bottom=resolved_styles.hide_border_bottom
                            line_id=line_id.clone()
                        />
                    }
                    .into_any(),
                };
                view! {
                    <li
                        id=row_id
                        class=row_class
                        data-ui="list-row"
                        data-state=row_state
                    >
                        <div class="flex flex-row h-full w-full gap-1 items-center overflow-y-hidden">
                            {offset_view}
                            {row_view}
                        </div>
                    </li>
                }
                .into_any()
            })
            .collect_view();
        Some(
            view! { <ul id=items_id class=list_class>{items}</ul> }.into_any(),
        )
    } else {
        None
    };
    let has_title = title_view.is_some();
    view! {
        <section
            id=base_id
            class=wrap_class
            data-view=view_value
            data-ui="list-group"
            aria-labelledby=if has_title { Some(title_id.clone()) } else { None }
        >
            <div class=group_class>
                {title_view}
                {content_view}
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_list_active_class,
        radroots_studio_app_ui_list_class_merge,
        radroots_studio_app_ui_list_border_classes,
        radroots_studio_app_ui_list_group_data_ui_value,
        radroots_studio_app_ui_list_row_data_ui_value,
        radroots_studio_app_ui_list_row_leading_data_ui_value,
        radroots_studio_app_ui_list_row_trailing_data_ui_value,
        radroots_studio_app_ui_list_section_data_ui_value,
        radroots_studio_app_ui_list_default_labels,
        radroots_studio_app_ui_list_offset_mod,
        radroots_studio_app_ui_list_input_action_icon_key,
        radroots_studio_app_ui_list_display_loading,
        radroots_studio_app_ui_list_row_class,
        radroots_studio_app_ui_list_title_visible,
        radroots_studio_app_ui_list_title_padding_class,
    };
    use crate::{
        RadrootsAppUiIconKey,
        RadrootsAppUiListInputAction,
        RadrootsAppUiListInputField,
        RadrootsAppUiListInput,
        RadrootsAppUiListItem,
        RadrootsAppUiListItemKind,
        RadrootsAppUiListOffsetMod,
        RadrootsAppUiListStylesResolved,
        RadrootsAppUiListTitle,
        RadrootsAppUiListTitleValue,
    };

    #[test]
    fn list_data_ui_values() {
        assert_eq!(radroots_studio_app_ui_list_group_data_ui_value(), "list-group");
        assert_eq!(radroots_studio_app_ui_list_section_data_ui_value(), "list-section");
        assert_eq!(radroots_studio_app_ui_list_row_data_ui_value(), "list-row");
        assert_eq!(
            radroots_studio_app_ui_list_row_leading_data_ui_value(),
            "list-row-leading"
        );
        assert_eq!(
            radroots_studio_app_ui_list_row_trailing_data_ui_value(),
            "list-row-trailing"
        );
    }

    #[test]
    fn list_class_merge_skips_empty_values() {
        let merged = radroots_studio_app_ui_list_class_merge(&[
            Some("alpha"),
            Some(""),
            None,
            Some("beta"),
        ]);
        assert_eq!(merged, "alpha beta");
    }

    #[test]
    fn list_active_class_respects_flag() {
        assert_eq!(radroots_studio_app_ui_list_active_class(true), None);
        assert_eq!(radroots_studio_app_ui_list_active_class(false), Some("opacity-active"));
    }

    #[test]
    fn list_border_classes_match_flags() {
        let classes = radroots_studio_app_ui_list_border_classes(true, false);
        assert_eq!(classes, "group-first:border-t-0 group-last:border-b-line");
        let classes = radroots_studio_app_ui_list_border_classes(false, true);
        assert_eq!(classes, "group-first:border-t-line group-last:border-b-0");
    }

    #[test]
    fn list_title_padding_matches_mod() {
        assert_eq!(
            radroots_studio_app_ui_list_title_padding_class(Some(&RadrootsAppUiListOffsetMod::Small)),
            Some("pl-[16px]")
        );
        assert_eq!(
            radroots_studio_app_ui_list_title_padding_class(Some(&RadrootsAppUiListOffsetMod::Glyph)),
            Some("pl-[36px]")
        );
        assert_eq!(radroots_studio_app_ui_list_title_padding_class(None), None);
    }

    #[test]
    fn list_default_labels_fallbacks() {
        let labels = radroots_studio_app_ui_list_default_labels(None);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].label, "No items to display.");
    }

    #[test]
    fn list_offset_defaults_to_small() {
        let resolved = radroots_studio_app_ui_list_offset_mod(None);
        assert!(matches!(resolved, RadrootsAppUiListOffsetMod::Small));
    }

    #[test]
    fn list_input_action_defaults_to_plus() {
        let action = RadrootsAppUiListInputAction {
            visible: true,
            loading: false,
            icon: None,
            on_click: None,
        };
        assert_eq!(
            radroots_studio_app_ui_list_input_action_icon_key(&action),
            RadrootsAppUiIconKey::Plus
        );
    }

    #[test]
    fn list_display_loading_defaults_false() {
        assert!(!radroots_studio_app_ui_list_display_loading(None));
    }

    #[test]
    fn list_title_visible_requires_title() {
        assert!(!radroots_studio_app_ui_list_title_visible(None, None));
        let title = RadrootsAppUiListTitle {
            value: RadrootsAppUiListTitleValue::Text("Title".to_string()),
            classes: None,
            mod_value: None,
            link: None,
            on_click: None,
        };
        assert!(radroots_studio_app_ui_list_title_visible(Some(&title), None));
        let default_state = crate::RadrootsAppUiListDefault {
            labels: None,
            show_title: false,
            classes: None,
        };
        assert!(!radroots_studio_app_ui_list_title_visible(
            Some(&title),
            Some(&default_state)
        ));
    }

    #[test]
    fn list_row_class_flags_hidden_and_rounding() {
        let item = RadrootsAppUiListItem {
            kind: RadrootsAppUiListItemKind::Input(RadrootsAppUiListInput {
                field: RadrootsAppUiListInputField {
                    value: String::new(),
                    placeholder: None,
                    disabled: false,
                    classes: None,
                    id: None,
                    on_input: None,
                },
                line_label: None,
                action: None,
            }),
            loading: false,
            hide_active: true,
            hide_field: true,
            full_rounded: true,
            offset: None,
        };
        let styles = RadrootsAppUiListStylesResolved {
            hide_border_top: false,
            hide_border_bottom: false,
            hide_rounded: false,
            set_title_background: false,
            set_default_background: false,
        };
        let class = radroots_studio_app_ui_list_row_class(&item, &styles);
        assert!(class.contains("hidden"));
        assert!(class.contains("rounded-touch"));
        assert!(class.contains("first:rounded-t-2xl"));
    }
}
