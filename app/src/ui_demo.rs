use leptos::prelude::*;

use radroots_studio_app_ui_components::{
    RadrootsAppUiList,
    RadrootsAppUiListDisplay,
    RadrootsAppUiListDisplayValue,
    RadrootsAppUiListIcon,
    RadrootsAppUiListInput,
    RadrootsAppUiListInputAction,
    RadrootsAppUiListInputField,
    RadrootsAppUiListInputLineLabel,
    RadrootsAppUiListItem,
    RadrootsAppUiListItemKind,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelText,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
    RadrootsAppUiListSelect,
    RadrootsAppUiListSelectField,
    RadrootsAppUiListSelectOption,
    RadrootsAppUiListStyles,
    RadrootsAppUiListTitle,
    RadrootsAppUiListTitleValue,
    RadrootsAppUiListTouch,
    RadrootsAppUiListTouchEnd,
    RadrootsAppUiListView,
    RadrootsAppUiSheetClose,
    RadrootsAppUiSheetContent,
    RadrootsAppUiSheetDescription,
    RadrootsAppUiSheetOverlay,
    RadrootsAppUiSheetPortal,
    RadrootsAppUiSheetRoot,
    RadrootsAppUiSheetTitle,
    RadrootsAppUiSheetTrigger,
};

#[component]
pub fn RadrootsAppUiDemoPage() -> impl IntoView {
    let sheet_open = RwSignal::new(false);
    let sheet_open_read = sheet_open.read_only();
    let sheet_open_set = Callback::new(move |value| sheet_open.set(value));
    let input_value = RwSignal::new(String::new());
    let select_value = RwSignal::new("daily".to_string());
    let on_input = Callback::new(move |value| input_value.set(value));
    let on_select = Callback::new(move |value| select_value.set(value));
    let text_label = |value: &str| RadrootsAppUiListLabelValue {
        classes_wrap: None,
        hide_truncate: false,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value: value.to_string(),
            classes: None,
        }),
    };
    let list = RadrootsAppUiList {
        id: Some("ui-demo-list".to_string()),
        view: Some("ui-demo".to_string()),
        classes: None,
        title: Some(RadrootsAppUiListTitle {
            value: RadrootsAppUiListTitleValue::Text("List Preview".to_string()),
            classes: None,
            mod_value: None,
            link: None,
            on_click: None,
        }),
        default_state: None,
        list: Some(vec![
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                    label: RadrootsAppUiListLabel {
                        left: vec![text_label("Notifications")],
                        right: Vec::new(),
                    },
                    display: Some(RadrootsAppUiListDisplay {
                        value: RadrootsAppUiListDisplayValue::Label(RadrootsAppUiListLabelText {
                            value: "Enabled".to_string(),
                            classes: None,
                        }),
                        loading: false,
                        on_click: None,
                    }),
                    end: Some(RadrootsAppUiListTouchEnd {
                        icon: RadrootsAppUiListIcon {
                            key: "chevron-right".to_string(),
                            class: None,
                        },
                        on_click: None,
                    }),
                    on_click: None,
                }),
                loading: false,
                hide_active: false,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Input(RadrootsAppUiListInput {
                    field: RadrootsAppUiListInputField {
                        value: input_value.get_untracked(),
                        placeholder: Some("Add a note".to_string()),
                        disabled: false,
                        classes: None,
                        id: Some("ui-demo-note".to_string()),
                        on_input: Some(on_input),
                    },
                    line_label: Some(RadrootsAppUiListInputLineLabel {
                        value: "Note".to_string(),
                        classes: None,
                    }),
                    action: Some(RadrootsAppUiListInputAction {
                        visible: true,
                        loading: false,
                        icon: Some(RadrootsAppUiListIcon {
                            key: "plus".to_string(),
                            class: None,
                        }),
                        on_click: None,
                    }),
                }),
                loading: false,
                hide_active: true,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Select(RadrootsAppUiListSelect {
                    field: RadrootsAppUiListSelectField {
                        value: select_value.get_untracked(),
                        options: vec![
                            RadrootsAppUiListSelectOption {
                                label: "Daily".to_string(),
                                value: "daily".to_string(),
                                classes: None,
                            },
                            RadrootsAppUiListSelectOption {
                                label: "Weekly".to_string(),
                                value: "weekly".to_string(),
                                classes: None,
                            },
                            RadrootsAppUiListSelectOption {
                                label: "Never".to_string(),
                                value: "never".to_string(),
                                classes: None,
                            },
                        ],
                        disabled: false,
                        classes: None,
                        id: Some("ui-demo-sync".to_string()),
                        on_change: Some(on_select),
                    },
                    label: RadrootsAppUiListLabel {
                        left: vec![text_label("Sync Frequency")],
                        right: Vec::new(),
                    },
                    display: None,
                    end: Some(RadrootsAppUiListTouchEnd {
                        icon: RadrootsAppUiListIcon {
                            key: "chevrons-up-down".to_string(),
                            class: None,
                        },
                        on_click: None,
                    }),
                    loading: false,
                    on_click: None,
                }),
                loading: false,
                hide_active: false,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
        ]),
        hide_offset: false,
        styles: Some(RadrootsAppUiListStyles {
            hide_border_top: None,
            hide_border_bottom: None,
            hide_rounded: None,
            set_title_background: Some(true),
            set_default_background: None,
        }),
    };
    view! {
        <main id="app-ui-demo" class="app-page app-page-scroll" style="padding: 16px;">
            <header id="app-ui-demo-header" style="font: var(--type-title2); margin-bottom: 12px;">
                <h1 id="app-ui-demo-title">"UI Demo"</h1>
            </header>
            <section id="app-ui-demo-content">
                <RadrootsAppUiListView basis=list />

                <RadrootsAppUiSheetRoot
                    open=Some(sheet_open_read)
                    default_open=false
                    modal=None
                    on_open_change=Some(sheet_open_set)
                >
                    <RadrootsAppUiSheetTrigger
                        disabled=false
                        class=Some("ui-card".to_string())
                        id=None
                        style=Some("padding:12px 16px; width: 100%; text-align: left;".to_string())
                    >
                        "Open Sheet"
                    </RadrootsAppUiSheetTrigger>
                    <RadrootsAppUiSheetPortal>
                        <RadrootsAppUiSheetOverlay
                            close_on_click=None
                            class=None
                            id=None
                            style=None
                        />
                        <RadrootsAppUiSheetContent
                            disable_outside_pointer_events=false
                            show_handle=true
                            class=None
                            id=None
                            style=None
                        >
                            <RadrootsAppUiSheetTitle
                                class=None
                                id=None
                                style=None
                            >
                                "Sheet Preview"
                            </RadrootsAppUiSheetTitle>
                            <RadrootsAppUiSheetDescription
                                class=None
                                id=None
                                style=Some("margin-top: 6px;".to_string())
                            >
                                "This is a placeholder sheet for iOS styling."
                            </RadrootsAppUiSheetDescription>
                            <RadrootsAppUiSheetClose
                                class=Some("ui-card".to_string())
                                id=None
                                style=Some("margin-top: 16px; padding: 10px 14px;".to_string())
                            >
                                "Close"
                            </RadrootsAppUiSheetClose>
                        </RadrootsAppUiSheetContent>
                    </RadrootsAppUiSheetPortal>
                </RadrootsAppUiSheetRoot>
            </section>
        </main>
    }
}
