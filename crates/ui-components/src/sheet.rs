use leptos::prelude::*;

use super::{
    RadrootsAppUiDialogClose,
    RadrootsAppUiDialogContent,
    RadrootsAppUiDialogDescription,
    RadrootsAppUiDialogOverlay,
    RadrootsAppUiDialogPortal,
    RadrootsAppUiDialogRoot,
    RadrootsAppUiDialogTitle,
    RadrootsAppUiDialogTrigger,
};

pub fn radroots_studio_app_ui_sheet_data_ui_value() -> &'static str {
    "sheet"
}

pub fn radroots_studio_app_ui_sheet_overlay_data_ui_value() -> &'static str {
    "sheet-overlay"
}

pub fn radroots_studio_app_ui_sheet_handle_data_ui_value() -> &'static str {
    "sheet-handle"
}

#[component]
pub fn RadrootsAppUiSheetRoot(
    open: Option<ReadSignal<bool>>,
    #[prop(optional)] default_open: bool,
    modal: Option<bool>,
    on_open_change: Option<Callback<bool>>,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogRoot
            open=open
            default_open=default_open
            modal=modal
            on_open_change=on_open_change
        >
            {children()}
        </RadrootsAppUiDialogRoot>
    }
}

#[component]
pub fn RadrootsAppUiSheetTrigger(
    #[prop(optional)] disabled: bool,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogTrigger
            disabled=disabled
            class=class
            id=id
            style=style
        >
            {children()}
        </RadrootsAppUiDialogTrigger>
    }
}

#[component]
pub fn RadrootsAppUiSheetPortal(children: ChildrenFn) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogPortal>
            {children()}
        </RadrootsAppUiDialogPortal>
    }
}

#[component]
pub fn RadrootsAppUiSheetOverlay(
    close_on_click: Option<bool>,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogOverlay
            close_on_click=close_on_click
            data_ui=Some(radroots_studio_app_ui_sheet_overlay_data_ui_value().to_string())
            class=class
            id=id
            style=style
        ></RadrootsAppUiDialogOverlay>
    }
}

#[component]
pub fn RadrootsAppUiSheetContent(
    #[prop(optional)] disable_outside_pointer_events: bool,
    #[prop(optional)] show_handle: bool,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let handle = show_handle;
    let children = StoredValue::new(children);
    let content_children = move || {
        let inner = (children.get_value())();
        if handle {
            view! {
                <div data-ui=radroots_studio_app_ui_sheet_handle_data_ui_value()></div>
                {inner}
            }
            .into_any()
        } else {
            inner
        }
    };
    view! {
        <RadrootsAppUiDialogContent
            disable_outside_pointer_events=disable_outside_pointer_events
            data_ui=Some(radroots_studio_app_ui_sheet_data_ui_value().to_string())
            class=class
            id=id
            style=style
        >
            {content_children}
        </RadrootsAppUiDialogContent>
    }
}

#[component]
pub fn RadrootsAppUiSheetTitle(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogTitle
            class=class
            id=id
            style=style
        >
            {children()}
        </RadrootsAppUiDialogTitle>
    }
}

#[component]
pub fn RadrootsAppUiSheetDescription(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogDescription
            class=class
            id=id
            style=style
        >
            {children()}
        </RadrootsAppUiDialogDescription>
    }
}

#[component]
pub fn RadrootsAppUiSheetClose(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <RadrootsAppUiDialogClose
            class=class
            id=id
            style=style
        >
            {children()}
        </RadrootsAppUiDialogClose>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_sheet_data_ui_value,
        radroots_studio_app_ui_sheet_handle_data_ui_value,
        radroots_studio_app_ui_sheet_overlay_data_ui_value,
    };

    #[test]
    fn sheet_data_ui_values() {
        assert_eq!(radroots_studio_app_ui_sheet_data_ui_value(), "sheet");
        assert_eq!(radroots_studio_app_ui_sheet_overlay_data_ui_value(), "sheet-overlay");
        assert_eq!(radroots_studio_app_ui_sheet_handle_data_ui_value(), "sheet-handle");
    }
}
