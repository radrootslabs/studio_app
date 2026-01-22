#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

use radroots_studio_app_ui_components::{
    RadrootsAppUiList,
    RadrootsAppUiListIcon,
    RadrootsAppUiListItem,
    RadrootsAppUiListItemKind,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelText,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
    RadrootsAppUiListSelect,
    RadrootsAppUiListSelectField,
    RadrootsAppUiListSelectOption,
    RadrootsAppUiListTitle,
    RadrootsAppUiListTitleValue,
    RadrootsAppUiListTouch,
    RadrootsAppUiListTouchEnd,
    RadrootsAppUiListView,
};

fn log_settings_action(action: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::log_1(&action.into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("{action}");
    }
}

fn settings_touch_callback(action: &'static str) -> Callback<MouseEvent> {
    Callback::new(move |_| log_settings_action(action))
}

fn settings_label(value: &str, classes: Option<&str>) -> RadrootsAppUiListLabelValue {
    RadrootsAppUiListLabelValue {
        classes_wrap: None,
        hide_truncate: false,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value: value.to_string(),
            classes: classes.map(str::to_string),
        }),
    }
}

#[component]
pub fn RadrootsAppSettingsPage() -> impl IntoView {
    let color_mode_callback = Callback::new(move |_value: String| {
        log_settings_action("settings_color_mode");
    });
    let appearance_list = RadrootsAppUiList {
        id: Some("settings-appearance".to_string()),
        view: Some("settings".to_string()),
        classes: None,
        title: Some(RadrootsAppUiListTitle {
            value: RadrootsAppUiListTitleValue::Text("Appearance".to_string()),
            classes: None,
            mod_value: None,
            link: None,
            on_click: None,
        }),
        default_state: None,
        list: Some(vec![Some(RadrootsAppUiListItem {
            kind: RadrootsAppUiListItemKind::Select(RadrootsAppUiListSelect {
                field: RadrootsAppUiListSelectField {
                    value: "light".to_string(),
                    options: vec![
                        RadrootsAppUiListSelectOption {
                            label: "Light".to_string(),
                            value: "light".to_string(),
                            classes: None,
                        },
                        RadrootsAppUiListSelectOption {
                            label: "Dark".to_string(),
                            value: "dark".to_string(),
                            classes: None,
                        },
                    ],
                    disabled: false,
                    classes: None,
                    id: Some("settings-color-mode".to_string()),
                    on_change: Some(color_mode_callback),
                },
                label: RadrootsAppUiListLabel {
                    left: vec![settings_label("color mode", Some("capitalize"))],
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
            hide_active: true,
            hide_field: false,
            full_rounded: false,
            offset: None,
        })]),
        hide_offset: false,
        styles: None,
    };
    let actions_list = RadrootsAppUiList {
        id: Some("settings-actions".to_string()),
        view: Some("settings".to_string()),
        classes: None,
        title: None,
        default_state: None,
        list: Some(vec![
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                    label: RadrootsAppUiListLabel {
                        left: vec![settings_label("export database", Some("capitalize"))],
                        right: Vec::new(),
                    },
                    display: None,
                    end: Some(RadrootsAppUiListTouchEnd {
                        icon: RadrootsAppUiListIcon {
                            key: "caret-right".to_string(),
                            class: None,
                        },
                        on_click: None,
                    }),
                    on_click: Some(settings_touch_callback("settings_export_database")),
                }),
                loading: false,
                hide_active: true,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                    label: RadrootsAppUiListLabel {
                        left: vec![settings_label("logout", Some("capitalize"))],
                        right: Vec::new(),
                    },
                    display: None,
                    end: Some(RadrootsAppUiListTouchEnd {
                        icon: RadrootsAppUiListIcon {
                            key: "caret-right".to_string(),
                            class: None,
                        },
                        on_click: None,
                    }),
                    on_click: Some(settings_touch_callback("settings_logout")),
                }),
                loading: false,
                hide_active: true,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
        ]),
        hide_offset: false,
        styles: None,
    };
    view! {
        <main style="padding: 16px;">
            <div style="font: var(--type-title2); margin-bottom: 12px;">"settings"</div>
            <div style="display:flex;flex-direction:column;gap:16px;">
                <RadrootsAppUiListView basis=appearance_list />
                <RadrootsAppUiListView basis=actions_list />
            </div>
        </main>
    }
}
