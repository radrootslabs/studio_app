#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

use crate::{
    radroots_studio_app_ui_list_icon_key,
    RadrootsAppUiIcon,
    RadrootsAppUiListDisplay,
    RadrootsAppUiListDisplayValue,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
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

#[component]
pub fn RadrootsAppUiListGroup(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <div
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_group_data_ui_value()
        >
            {children()}
        </div>
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
        <div
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_section_data_ui_value()
        >
            {children()}
        </div>
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
        <div
            id=id
            class=class
            style=style
            data-ui=radroots_studio_app_ui_list_row_data_ui_value()
        >
            {children()}
        </div>
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
    let wrap_class = radroots_studio_app_ui_list_class_merge(&[
        Some("flex flex-row h-full items-center"),
        if hide_truncate { None } else { Some("truncate") },
        classes_wrap.as_deref(),
    ]);
    let active_class = if hide_active { None } else { Some("opacity-active") };
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
            let icon_class = radroots_studio_app_ui_list_class_merge(&[
                Some("ui-text-secondary"),
                if hide_active { None } else { Some("opacity-active") },
                icon.class.as_deref(),
            ]);
            if let Some(icon_key) = icon_key {
                view! { <RadrootsAppUiIcon key=icon_key class=icon_class size=18 /> }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }
        }
        RadrootsAppUiListDisplayValue::Label(label) => {
            let text_class = radroots_studio_app_ui_list_class_merge(&[
                Some("text-line_d_e ui-text-secondary line-clamp-1"),
                if hide_active { None } else { Some("opacity-active") },
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

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_list_class_merge,
        radroots_studio_app_ui_list_group_data_ui_value,
        radroots_studio_app_ui_list_row_data_ui_value,
        radroots_studio_app_ui_list_row_leading_data_ui_value,
        radroots_studio_app_ui_list_row_trailing_data_ui_value,
        radroots_studio_app_ui_list_section_data_ui_value,
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
}
