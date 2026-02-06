#![forbid(unsafe_code)]

use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{
    app::AppPageChrome,
    active_key_label,
    app_context,
    app_health_check_delay_ms,
    health_result_label,
    health_status_class,
    spawn_health_checks,
    t,
    RadrootsAppBackends,
    RadrootsAppConfigStatus,
    RadrootsAppHealthCheckResult,
    RadrootsAppHealthReport,
    RadrootsAppSetupStatus,
};
use radroots_studio_app_ui_components::{
    RadrootsAppUiList,
    RadrootsAppUiListItem,
    RadrootsAppUiListItemKind,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelText,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
    RadrootsAppUiListTitle,
    RadrootsAppUiListTitleValue,
    RadrootsAppUiListTouch,
    RadrootsAppUiListView,
};

fn status_dot(status_class: &str) -> RadrootsAppUiListLabelValue {
    RadrootsAppUiListLabelValue {
        classes_wrap: Some("pr-1".to_string()),
        hide_truncate: true,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value: "●".to_string(),
            classes: Some(format!("status-dot {}", status_class)),
        }),
    }
}

fn status_text(value: String) -> RadrootsAppUiListLabelValue {
    RadrootsAppUiListLabelValue {
        classes_wrap: None,
        hide_truncate: false,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value,
            classes: None,
        }),
    }
}

fn config_status_label(status: RadrootsAppConfigStatus) -> String {
    match status {
        RadrootsAppConfigStatus::Unknown => t!("app.common.unknown"),
        RadrootsAppConfigStatus::Required => String::from("required"),
        RadrootsAppConfigStatus::Configured => String::from("configured"),
        RadrootsAppConfigStatus::Corrupt => String::from("corrupt"),
    }
}

fn config_status_class(status: RadrootsAppConfigStatus) -> &'static str {
    match status {
        RadrootsAppConfigStatus::Configured => "status-good",
        RadrootsAppConfigStatus::Required => "status-warn",
        RadrootsAppConfigStatus::Corrupt => "status-error",
        RadrootsAppConfigStatus::Unknown => "status-neutral",
    }
}

fn setup_status_label(status: RadrootsAppSetupStatus) -> String {
    match status {
        RadrootsAppSetupStatus::Unknown => t!("app.common.unknown"),
        RadrootsAppSetupStatus::Required => String::from("required"),
        RadrootsAppSetupStatus::Configured => String::from("configured"),
        RadrootsAppSetupStatus::Corrupt => String::from("corrupt"),
        RadrootsAppSetupStatus::Locked => String::from("locked"),
    }
}

fn setup_status_class(status: RadrootsAppSetupStatus) -> &'static str {
    match status {
        RadrootsAppSetupStatus::Configured => "status-good",
        RadrootsAppSetupStatus::Required | RadrootsAppSetupStatus::Locked => "status-warn",
        RadrootsAppSetupStatus::Corrupt => "status-error",
        RadrootsAppSetupStatus::Unknown => "status-neutral",
    }
}

fn status_row(label: String, result: RadrootsAppHealthCheckResult) -> RadrootsAppUiListItem {
    let status_label = health_result_label(&result);
    let status_class = health_status_class(result.status);
    RadrootsAppUiListItem {
        kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
            label: RadrootsAppUiListLabel {
                left: vec![status_text(label)],
                right: vec![status_text(status_label), status_dot(status_class)],
            },
            display: None,
            end: None,
            on_click: None,
        }),
        loading: false,
        hide_active: true,
        hide_field: false,
        full_rounded: false,
        offset: None,
    }
}

fn format_timestamp(ms: i64) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsValue;

        let date = js_sys::Date::new(&JsValue::from_f64(ms as f64));
        return date
            .to_locale_string("en-US", &JsValue::UNDEFINED)
            .as_string()
            .unwrap_or_else(|| ms.to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ms.to_string()
    }
}

#[component]
pub fn RadrootsAppSettingsStatusPage() -> impl IntoView {
    let context = app_context();
    let fallback_backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let fallback_setup_status = RwSignal::new_local(RadrootsAppSetupStatus::Unknown);
    let fallback_config_status = RwSignal::new_local(RadrootsAppConfigStatus::Unknown);
    let backends = context
        .as_ref()
        .map(|value| value.backends)
        .unwrap_or(fallback_backends);
    let setup_status = context
        .as_ref()
        .map(|value| value.setup_status)
        .unwrap_or(fallback_setup_status);
    let config_status = context
        .as_ref()
        .map(|value| value.config_status)
        .unwrap_or(fallback_config_status);
    let health_report = RwSignal::new_local(RadrootsAppHealthReport::empty());
    let health_running = RwSignal::new_local(false);
    let health_autorun = RwSignal::new_local(false);
    let active_key = RwSignal::new_local(None::<String>);
    let notifications_status = RwSignal::new_local(None::<String>);
    let last_run = RwSignal::new_local(None::<i64>);
    Effect::new(move || {
        if health_autorun.get() {
            return;
        }
        let setup_status = setup_status.get();
        if matches!(setup_status, RadrootsAppSetupStatus::Unknown) {
            return;
        }
        let setup_required_value = !matches!(setup_status, RadrootsAppSetupStatus::Configured);
        let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
        let Some(config) = config else {
            return;
        };
        health_autorun.set(true);
        let delay_ms = app_health_check_delay_ms();
        spawn_local(async move {
            TimeoutFuture::new(delay_ms).await;
            spawn_health_checks(
                config,
                setup_required_value,
                health_report,
                health_running,
                active_key,
                notifications_status,
                last_run,
            );
        });
    });
    let health_disabled = move || {
        backends.with(|value| value.is_none())
            || health_running.get()
            || matches!(setup_status.get(), RadrootsAppSetupStatus::Unknown)
    };
    let last_updated_label = move || {
        let value = last_run.get().map(format_timestamp);
        value.unwrap_or_else(|| t!("app.common.unknown"))
    };
    view! {
        <AppPageChrome title=t!("app.settings.status.title")>
            <header id="app-settings-status-header" class="flex flex-col gap-2">
                <div class="flex flex-row items-center gap-4">
                    <button
                        on:click=move |_| {
                            let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                            let Some(config) = config else {
                                return;
                            };
                            let setup_required_value =
                                !matches!(setup_status.get(), RadrootsAppSetupStatus::Configured);
                            spawn_health_checks(
                                config,
                                setup_required_value,
                                health_report,
                                health_running,
                                active_key,
                                notifications_status,
                                last_run,
                            );
                        }
                        disabled=health_disabled
                    >
                        {move || {
                            if health_running.get() {
                                t!("app.home.health.button.checking")
                            } else {
                                t!("app.home.health.button.run")
                            }
                        }}
                    </button>
                    <div id="app-settings-status-updated" class="text-xs text-[var(--text-secondary)]">
                        {move || format!("{}: {}", t!("app.settings.status.updated"), last_updated_label())}
                    </div>
                </div>
            </header>
            <section id="app-settings-status-content" class="flex flex-col gap-4 mt-3">
                {move || {
                    let report = health_report.get();
                    let active = active_key_label(active_key.get());
                    let setup_value = setup_status.get();
                    let config_value = config_status.get();
                    let config_list = RadrootsAppUiList {
                        id: Some("settings-config-status-list".to_string()),
                        view: Some("settings-config-status".to_string()),
                        classes: None,
                        title: Some(RadrootsAppUiListTitle {
                            value: RadrootsAppUiListTitleValue::Text(String::from("Configuration")),
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
                                        left: vec![status_text(String::from("setup status"))],
                                        right: vec![
                                            status_text(setup_status_label(setup_value)),
                                            status_dot(setup_status_class(setup_value)),
                                        ],
                                    },
                                    display: None,
                                    end: None,
                                    on_click: None,
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
                                        left: vec![status_text(String::from("config status"))],
                                        right: vec![
                                            status_text(config_status_label(config_value)),
                                            status_dot(config_status_class(config_value)),
                                        ],
                                    },
                                    display: None,
                                    end: None,
                                    on_click: None,
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
                    let list = RadrootsAppUiList {
                        id: Some("settings-status-list".to_string()),
                        view: Some("settings-status".to_string()),
                        classes: None,
                        title: Some(RadrootsAppUiListTitle {
                            value: RadrootsAppUiListTitleValue::Text(t!("app.home.health.title")),
                            classes: None,
                            mod_value: None,
                            link: None,
                            on_click: None,
                        }),
                        default_state: None,
                        list: Some(vec![
                            Some(status_row(t!("app.home.health.item.key_maps"), report.key_maps)),
                            Some(status_row(
                                t!("app.home.health.item.bootstrap_state"),
                                report.bootstrap_state,
                            )),
                            Some(status_row(
                                t!("app.home.health.item.state_active_key"),
                                report.state_active_key,
                            )),
                            Some(status_row(
                                t!("app.home.health.item.notifications"),
                                report.notifications,
                            )),
                            Some(status_row(t!("app.home.health.item.tangle"), report.tangle)),
                            Some(status_row(
                                t!("app.home.health.item.datastore_roundtrip"),
                                report.datastore_roundtrip,
                            )),
                            Some(status_row(t!("app.home.health.item.keystore"), report.keystore)),
                            Some(RadrootsAppUiListItem {
                                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                                    label: RadrootsAppUiListLabel {
                                        left: vec![status_text(t!("app.home.health.item.active_key"))],
                                        right: vec![status_text(active), status_dot("status-neutral")],
                                    },
                                    display: None,
                                    end: None,
                                    on_click: None,
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
                        <div class="flex flex-col gap-4">
                            <RadrootsAppUiListView basis=config_list />
                            <RadrootsAppUiListView basis=list />
                        </div>
                    }
                    .into_any()
                }}
            </section>
        </AppPageChrome>
    }
}
