#![forbid(unsafe_code)]

use leptos::prelude::*;

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

#[cfg(test)]
mod tests {
    use super::{
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
}
