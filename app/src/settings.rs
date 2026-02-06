#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::{
    app::AppPageChrome,
    app_context,
    app_datastore_clear_config,
    app_log_error_emit,
    app_theme_apply_mode,
    app_theme_mode_from_value,
    app_theme_read_mode,
    app_theme_store_mode,
    t,
    RadrootsAppBackends,
    RadrootsAppConfigStatus,
    RadrootsAppThemeMode,
};
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

fn settings_capitalize(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(chars.as_str());
    out
}

fn settings_label(value: String, classes: Option<&str>) -> RadrootsAppUiListLabelValue {
    RadrootsAppUiListLabelValue {
        classes_wrap: None,
        hide_truncate: false,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value,
            classes: classes.map(str::to_string),
        }),
    }
}

#[component]
pub fn RadrootsAppSettingsPage() -> impl IntoView {
    let context = app_context();
    let fallback_backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let fallback_config_status = RwSignal::new_local(RadrootsAppConfigStatus::Unknown);
    let backends = context
        .as_ref()
        .map(|value| value.backends)
        .unwrap_or(fallback_backends);
    let config_status = context
        .as_ref()
        .map(|value| value.config_status)
        .unwrap_or(fallback_config_status);
    let navigate = use_navigate();
    let initial_mode = app_theme_read_mode().unwrap_or(RadrootsAppThemeMode::System);
    let color_mode_value = initial_mode.as_str().to_string();
    let color_mode_callback = Callback::new(move |value: String| {
        log_settings_action("settings_color_mode");
        let Some(mode) = app_theme_mode_from_value(&value) else {
            return;
        };
        let _ = app_theme_store_mode(mode);
        let _ = app_theme_apply_mode(mode);
    });
    let appearance_list = RadrootsAppUiList {
        id: Some("settings-appearance".to_string()),
        view: Some("settings".to_string()),
        classes: None,
        title: Some(RadrootsAppUiListTitle {
            value: RadrootsAppUiListTitleValue::Text(t!("app.settings.appearance.title")),
            classes: None,
            mod_value: None,
            link: None,
            on_click: None,
        }),
        default_state: None,
        list: Some(vec![Some(RadrootsAppUiListItem {
            kind: RadrootsAppUiListItemKind::Select(RadrootsAppUiListSelect {
                field: RadrootsAppUiListSelectField {
                    value: color_mode_value,
                    options: vec![
                        RadrootsAppUiListSelectOption {
                            label: settings_capitalize(
                                &t!("app.settings.appearance.color_mode.option.system"),
                            ),
                            value: "system".to_string(),
                            classes: None,
                        },
                        RadrootsAppUiListSelectOption {
                            label: settings_capitalize(
                                &t!("app.settings.appearance.color_mode.option.light"),
                            ),
                            value: "light".to_string(),
                            classes: None,
                        },
                        RadrootsAppUiListSelectOption {
                            label: settings_capitalize(
                                &t!("app.settings.appearance.color_mode.option.dark"),
                            ),
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
                    left: vec![settings_label(
                        t!("app.settings.appearance.color_mode.label"),
                        Some("capitalize"),
                    )],
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
    let logs_navigate = navigate.clone();
    let reconfigure_action = {
        let navigate = navigate.clone();
        let backends = backends.clone();
        let config_status = config_status.clone();
        Callback::new(move |_| {
            let Some((datastore, key_maps)) = backends.with(|value| {
                value.as_ref().map(|backends| {
                    (
                        backends.datastore.clone(),
                        backends.config.datastore.key_maps.clone(),
                    )
                })
            }) else {
                return;
            };
            let navigate = navigate.clone();
            let config_status = config_status.clone();
            spawn_local(async move {
                match app_datastore_clear_config(datastore.as_ref(), &key_maps).await {
                    Ok(()) => {
                        config_status.set(RadrootsAppConfigStatus::Required);
                        navigate("/setup/config", Default::default());
                    }
                    Err(err) => {
                        let _ = app_log_error_emit(&err);
                    }
                }
            });
        })
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
                        left: vec![settings_label(
                            t!("app.settings.actions.export_db"),
                            Some("capitalize"),
                        )],
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
                        left: vec![settings_label(
                            "update configuration".to_string(),
                            Some("capitalize"),
                        )],
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
                    on_click: Some(reconfigure_action),
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
                        left: vec![settings_label(t!("app.nav.logs"), Some("capitalize"))],
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
                    on_click: Some(Callback::new(move |_| {
                        logs_navigate("/settings/logs", Default::default());
                    })),
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
                        left: vec![settings_label(
                            t!("app.settings.actions.logout"),
                            Some("capitalize"),
                        )],
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
    let system_status_action = {
        let navigate = navigate.clone();
        Callback::new(move |_| {
            navigate("/settings/status", Default::default());
        })
    };
    let system_list = RadrootsAppUiList {
        id: Some("settings-system".to_string()),
        view: Some("settings".to_string()),
        classes: None,
        title: Some(RadrootsAppUiListTitle {
            value: RadrootsAppUiListTitleValue::Text(t!("app.settings.system.title")),
            classes: None,
            mod_value: None,
            link: None,
            on_click: None,
        }),
        default_state: None,
        list: Some(vec![Some(RadrootsAppUiListItem {
            kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                label: RadrootsAppUiListLabel {
                    left: vec![settings_label(
                        t!("app.settings.system.status"),
                        Some("capitalize"),
                    )],
                    right: Vec::new(),
                },
                display: None,
                end: Some(RadrootsAppUiListTouchEnd {
                    icon: RadrootsAppUiListIcon {
                        key: "chevron-right".to_string(),
                        class: None,
                    },
                    on_click: None,
                }),
                on_click: Some(system_status_action),
            }),
            loading: false,
            hide_active: false,
            hide_field: false,
            full_rounded: false,
            offset: None,
        })]),
        hide_offset: false,
        styles: None,
    };
    view! {
        <AppPageChrome title=t!("app.settings.title")>
            <section id="app-settings-content" class="flex flex-col gap-4">
                <RadrootsAppUiListView basis=appearance_list />
                <RadrootsAppUiListView basis=actions_list />
                <RadrootsAppUiListView basis=system_list />
            </section>
        </AppPageChrome>
    }
}
