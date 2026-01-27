use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::ev::MouseEvent;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::path;

use radroots_studio_app_core::datastore::RadrootsClientDatastore;
use radroots_studio_app_core::idb::IDB_CONFIG_LOGS;
use radroots_studio_app_ui_components::{
    RadrootsAppUiButtonLayoutAction,
    RadrootsAppUiButtonLayoutBackAction,
    RadrootsAppUiButtonLayoutPair,
    RadrootsAppUiList,
    RadrootsAppUiListIcon,
    RadrootsAppUiListItem,
    RadrootsAppUiListItemKind,
    RadrootsAppUiListLabel,
    RadrootsAppUiListLabelText,
    RadrootsAppUiListLabelValue,
    RadrootsAppUiListLabelValueKind,
    RadrootsAppUiListTouch,
    RadrootsAppUiListTouchEnd,
    RadrootsAppUiListView,
};

use crate::{
    app_init_assets,
    app_init_backends,
    app_init_has_completed,
    app_init_needs_setup,
    app_init_state_default,
    app_init_mark_completed,
    app_init_reset,
    app_init_progress_add,
    app_init_stage_set,
    app_init_total_add,
    app_init_total_unknown,
    app_context,
    app_log_buffer_flush_deferred,
    app_log_debug_emit,
    app_log_error_emit,
    app_log_error_store,
    app_config_default,
    app_datastore_read_state,
    app_state_notifications_permission_value,
    app_state_set_notifications_permission_value,
    app_setup_step_default,
    app_health_check_all,
    RadrootsAppBackends,
    RadrootsAppConfig,
    RadrootsAppHealthCheckResult,
    RadrootsAppHealthCheckStatus,
    RadrootsAppHealthReport,
    RadrootsAppInitError,
    RadrootsAppInitStage,
    RadrootsAppNotifications,
    RadrootsAppLogsPage,
    RadrootsAppSettingsPage,
    RadrootsAppUiDemoPage,
    RadrootsAppSetupStep,
    RadrootsAppTangleClientStub,
};

fn health_status_color(status: RadrootsAppHealthCheckStatus) -> &'static str {
    match status {
        RadrootsAppHealthCheckStatus::Ok => "green",
        RadrootsAppHealthCheckStatus::Error => "red",
        RadrootsAppHealthCheckStatus::Skipped => "gray",
    }
}

fn health_result_label(result: &RadrootsAppHealthCheckResult) -> String {
    match result.message.as_deref() {
        Some(message) => format!("{}: {}", result.status.as_str(), message),
        None => result.status.as_str().to_string(),
    }
}

fn setup_label(value: &str) -> RadrootsAppUiListLabelValue {
    RadrootsAppUiListLabelValue {
        classes_wrap: None,
        hide_truncate: false,
        value: RadrootsAppUiListLabelValueKind::Text(RadrootsAppUiListLabelText {
            value: value.to_string(),
            classes: Some("capitalize".to_string()),
        }),
    }
}

fn setup_touch_callback(action: &'static str) -> Callback<MouseEvent> {
    Callback::new(move |_| {
        let _ = app_log_debug_emit("log.app.setup.choice", action, None);
    })
}

fn active_key_label(value: Option<String>) -> String {
    let Some(value) = value else {
        return "missing".to_string();
    };
    if value.len() <= 12 {
        return value;
    }
    let head = &value[..8];
    let tail = &value[value.len() - 4..];
    format!("{head}...{tail}")
}

fn log_init_stage(stage: RadrootsAppInitStage) {
    let _ = app_log_debug_emit("log.app.init.stage", stage.as_str(), None);
}

fn logs_datastore() -> radroots_studio_app_core::datastore::RadrootsClientWebDatastore {
    radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(Some(IDB_CONFIG_LOGS))
}

fn spawn_health_checks(
    config: RadrootsAppConfig,
    setup_required: bool,
    health_report: RwSignal<RadrootsAppHealthReport, LocalStorage>,
    health_running: RwSignal<bool, LocalStorage>,
    active_key: RwSignal<Option<String>, LocalStorage>,
    notifications_status: RwSignal<Option<String>, LocalStorage>,
) {
    health_running.set(true);
    spawn_local(async move {
        let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(
            Some(config.datastore.idb_config),
        );
        let keystore = radroots_studio_app_core::keystore::RadrootsClientWebKeystoreNostr::new(
            Some(config.keystore.nostr_store),
        );
        let notifications = RadrootsAppNotifications::new(None);
        let tangle = RadrootsAppTangleClientStub::new();
        let report = app_health_check_all(
            &datastore,
            &keystore,
            &notifications,
            &tangle,
            &config.datastore.key_maps,
            setup_required,
        )
        .await;
        let mut active_key_value = None;
        let mut notifications_value = None;
        if !setup_required {
            let app_data = app_datastore_read_state(&datastore, &config.datastore.key_maps)
                .await
                .ok();
            active_key_value = app_data.as_ref().and_then(|data| {
                if data.active_key.is_empty() {
                    None
                } else {
                    Some(data.active_key.clone())
                }
            });
            notifications_value = app_data
                .as_ref()
                .and_then(app_state_notifications_permission_value)
                .map(|permission| permission.as_str().to_string());
        }
        health_report.set(report);
        active_key.set(active_key_value);
        notifications_status.set(notifications_value);
        health_running.set(false);
        let key_maps = config.datastore.key_maps.clone();
        spawn_local(async move {
            let log_datastore = logs_datastore();
            let _ = app_log_buffer_flush_deferred(&log_datastore, &key_maps, true).await;
        });
    });
}

const APP_HEALTH_CHECK_DELAY_MS: u32 = 300;

fn app_health_check_delay_ms() -> u32 {
    APP_HEALTH_CHECK_DELAY_MS
}

#[component]
fn SplashPage() -> impl IntoView {
    view! {
        <main
            id="app-splash"
            style="min-height:100dvh;background:white;display:flex;align-items:center;justify-content:center;"
        >
        </main>
    }
}

#[component]
fn LogoCircle() -> impl IntoView {
    view! {
        <div class="relative flex flex-col h-[196px] w-full justify-center items-center">
            <div class="relative flex flex-row h-36 w-36 justify-center items-center bg-ly2 rounded-full">
                <p class="font-sans font-[900] text-6xl text-ly0-gl -tracking-[0.4rem] -translate-x-[6px]">
                    "\u{00BB}`,"
                </p>
                <p class="font-sans font-[900] text-6xl text-ly0-gl translate-x-[8px]">
                    "-"
                </p>
            </div>
        </div>
    }
}

#[component]
fn SetupPage() -> impl IntoView {
    let context = app_context();
    let fallback_setup_required = RwSignal::new_local(None::<bool>);
    let setup_required = context
        .as_ref()
        .map(|value| value.setup_required)
        .unwrap_or(fallback_setup_required);
    let navigate = use_navigate();
    let navigate_guard = navigate.clone();
    let navigate_home = navigate.clone();
    let setup_step = RwSignal::new_local(app_setup_step_default());
    let key_choice_list = RadrootsAppUiList {
        id: Some("setup-key-choice".to_string()),
        view: Some("setup".to_string()),
        classes: None,
        title: None,
        default_state: None,
        list: Some(vec![
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                    label: RadrootsAppUiListLabel {
                        left: vec![setup_label("generate new key")],
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
                    on_click: Some(setup_touch_callback("generate_key")),
                }),
                loading: false,
                hide_active: false,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
            Some(RadrootsAppUiListItem {
                kind: RadrootsAppUiListItemKind::Touch(RadrootsAppUiListTouch {
                    label: RadrootsAppUiListLabel {
                        left: vec![setup_label("add existing key")],
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
                    on_click: Some(setup_touch_callback("add_key")),
                }),
                loading: false,
                hide_active: false,
                hide_field: false,
                full_rounded: false,
                offset: None,
            }),
        ]),
        hide_offset: false,
        styles: None,
    };
    Effect::new(move || {
        if setup_required.get() == Some(false) {
            navigate_guard("/", Default::default());
        }
    });
    let advance_step: Callback<MouseEvent> = {
        let setup_step = setup_step.clone();
        Callback::new(move |_| {
            setup_step.update(|step| {
                *step = step.next();
            });
        })
    };
    let rewind_step: Callback<MouseEvent> = {
        let setup_step = setup_step.clone();
        Callback::new(move |_| {
            setup_step.update(|step| {
                *step = step.prev();
            });
        })
    };
    view! {
        <main
            id="app-setup"
            data-app-scroll
            class="relative min-h-[100dvh] h-[100dvh] w-full flex flex-col"
        >
            {move || match setup_step.get() {
                RadrootsAppSetupStep::Intro => {
                    let navigate_home = navigate_home.clone();
                    view! {
                        <section
                            id="app-setup-intro"
                        class="app-view app-view-enter relative flex flex-col h-[100dvh] w-full justify-start items-center"
                        >
                            <div class="flex flex-col h-full w-full justify-start items-center">
                                <div class="relative flex flex-col h-full w-full justify-center items-center">
                                    <div class="flex flex-row w-full justify-start items-center -translate-y-16">
                                        <button
                                            type="button"
                                            class="flex flex-row w-full justify-center items-center"
                                            on:click=move |_| navigate_home("/", Default::default())
                                        >
                                            <LogoCircle />
                                        </button>
                                    </div>
                                    <div class="absolute bottom-0 left-0 flex flex-col h-[20rem] w-full px-10 gap-2 justify-start items-center">
                                        <div class="flex flex-row w-full justify-start items-center">
                                            <p class="font-sans font-[400] text-sm uppercase text-ly0-gl-label">
                                                "Configure"
                                            </p>
                                        </div>
                                        <div class="flex flex-col w-full gap-2 justify-start items-center">
                                            <div class="flex flex-row w-full justify-start items-center">
                                                <p class="font-mono font-[400] text-[1.1rem] text-ly0-gl">
                                                    "Welcome to Radroots!"
                                                </p>
                                            </div>
                                            <div class="flex flex-row w-full justify-start items-center">
                                                <p class="font-mono font-[400] text-[1.1rem] text-ly0-gl">
                                                    "Your device will be configured by the setup wizard."
                                                </p>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </section>
                    }
                    .into_any()
                },
                RadrootsAppSetupStep::KeyChoice => view! {
                    <section
                        id="app-setup-key-choice"
                        class="app-view app-view-enter flex flex-col w-full gap-4 px-6 pt-10 pb-16"
                    >
                        <header class="flex flex-col gap-2">
                            <p class="font-sans text-sm uppercase tracking-[0.14em] text-ly0-gl-label">
                                "Setup"
                            </p>
                            <h2 class="font-sans font-[600] text-2xl text-ly0-gl">
                                "Choose your key"
                            </h2>
                            <p class="font-sans text-line_d_e text-ly0-gl-label">
                                "Select how you want to add your Nostr key."
                            </p>
                        </header>
                        <RadrootsAppUiListView basis=key_choice_list.clone() />
                    </section>
                }.into_any(),
            }}
            <div class="z-10 absolute bottom-10 left-0 flex flex-col w-full justify-center items-center">
                {move || {
                    let step = setup_step.get();
                    let continue_action = RadrootsAppUiButtonLayoutAction {
                        label: "Continue".to_string(),
                        disabled: step.is_terminal(),
                        loading: false,
                        on_click: advance_step.clone(),
                    };
                    let back_action = RadrootsAppUiButtonLayoutBackAction {
                        visible: !matches!(step, RadrootsAppSetupStep::Intro),
                        label: Some("Back".to_string()),
                        disabled: false,
                        on_click: rewind_step.clone(),
                    };
                    view! {
                        <RadrootsAppUiButtonLayoutPair
                            continue_action=continue_action
                            back=Some(back_action)
                        />
                    }
                }}
            </div>
        </main>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let context = app_context();
    let fallback_backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let fallback_init_error = RwSignal::new_local(None::<RadrootsAppInitError>);
    let fallback_init_state = RwSignal::new_local(app_init_state_default());
    let fallback_setup_required = RwSignal::new_local(None::<bool>);
    let backends = context
        .as_ref()
        .map(|value| value.backends)
        .unwrap_or(fallback_backends);
    let init_state = context
        .as_ref()
        .map(|value| value.init_state)
        .unwrap_or(fallback_init_state);
    let setup_required = context
        .as_ref()
        .map(|value| value.setup_required)
        .unwrap_or(fallback_setup_required);
    let _init_error = context
        .as_ref()
        .map(|value| value.init_error)
        .unwrap_or(fallback_init_error);
    let reset_status = RwSignal::new_local(None::<String>);
    let health_report = RwSignal::new_local(RadrootsAppHealthReport::empty());
    let health_running = RwSignal::new_local(false);
    let health_autorun = RwSignal::new_local(false);
    let active_key = RwSignal::new_local(None::<String>);
    let notifications_status = RwSignal::new_local(None::<String>);
    let notifications_requesting = RwSignal::new_local(false);
    Effect::new(move || {
        if init_state.get().stage != RadrootsAppInitStage::Ready {
            return;
        }
        if health_autorun.get() {
            return;
        }
        let setup_required = setup_required.get();
        let Some(setup_required_value) = setup_required else {
            return;
        };
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
            );
        });
    });
    let status_color = move || match init_state.get().stage {
        RadrootsAppInitStage::Ready => "green",
        RadrootsAppInitStage::Error => "red",
        RadrootsAppInitStage::Storage => "orange",
        RadrootsAppInitStage::DownloadSql => "orange",
        RadrootsAppInitStage::DownloadGeo => "orange",
        RadrootsAppInitStage::Database => "orange",
        RadrootsAppInitStage::Geocoder => "orange",
        RadrootsAppInitStage::Idle => "gray",
    };
    let reset_disabled = move || backends.with(|value| value.is_none());
    let reset_label = move || {
        reset_status
            .get()
            .unwrap_or_else(|| "reset_idle".to_string())
    };
    let health_disabled = move || {
        backends.with(|value| value.is_none())
            || health_running.get()
            || setup_required.get().is_none()
    };
    let notifications_disabled = move || {
        backends.with(|value| value.is_none()) || notifications_requesting.get()
    };
    let notifications_label = move || {
        notifications_status
            .get()
            .unwrap_or_else(|| "unknown".to_string())
    };
    let notifications_button_label = move || {
        if notifications_requesting.get() {
            "requesting"
        } else {
            "request"
        }
    };
    view! {
        <main id="app-home" data-app-scroll>
            <div>"app"</div>
            <div style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                <span
                    style=move || format!(
                        "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                        status_color()
                    )
                ></span>
                <span>{move || init_state.get().stage.as_str()}</span>
            </div>
            <div style="margin-top: 12px; display: flex; align-items: center; gap: 8px;">
                <button
                    on:click=move |_| {
                        let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                        reset_status.set(Some("resetting".to_string()));
                        health_report.set(RadrootsAppHealthReport::empty());
                        active_key.set(None);
                        notifications_status.set(None);
                        setup_required.set(Some(true));
                        spawn_local(async move {
                            let Some(config) = config else {
                                reset_status.set(Some("reset_missing_backends".to_string()));
                                return;
                            };
                            let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(
                                Some(config.datastore.idb_config),
                            );
                            let keystore = radroots_studio_app_core::keystore::RadrootsClientWebKeystoreNostr::new(
                                Some(config.keystore.nostr_store),
                            );
                            match app_init_reset(
                                Some(&datastore),
                                Some(&config.datastore.key_maps),
                                Some(&keystore),
                            )
                            .await
                            {
                                Ok(()) => {
                                    let log_datastore = logs_datastore();
                                    if let Err(err) = log_datastore.reset().await {
                                        let reset_err = RadrootsAppInitError::Datastore(err);
                                        let _ = app_log_error_emit(&reset_err);
                                        reset_status.set(Some(reset_err.to_string()));
                                        return;
                                    }
                                    reset_status.set(Some("reset_done".to_string()));
                                    spawn_health_checks(
                                        config,
                                        true,
                                        health_report,
                                        health_running,
                                        active_key,
                                        notifications_status,
                                    );
                                }
                                Err(err) => {
                                    let log_datastore = logs_datastore();
                                    let _ = app_log_error_store(
                                        &log_datastore,
                                        &config.datastore.key_maps,
                                        &err,
                                    )
                                    .await;
                                    reset_status.set(Some(err.to_string()));
                                }
                            }
                        });
                    }
                    disabled=reset_disabled
                >
                    "reset"
                </button>
                <span>{reset_label}</span>
            </div>
            <div style="margin-top: 16px;">
                <div style="font-weight: 600;">"notifications"</div>
                <div style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                    <button
                        on:click=move |_| {
                            let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                            notifications_requesting.set(true);
                            spawn_local(async move {
                                let Some(config) = config else {
                                    notifications_requesting.set(false);
                                    return;
                                };
                                let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(
                                    Some(config.datastore.idb_config),
                                );
                                let notifications = RadrootsAppNotifications::new(None);
                                match notifications.request_permission().await {
                                    Ok(permission) => {
                                        let value = permission.as_str().to_string();
                                        let _ = app_state_set_notifications_permission_value(
                                            &datastore,
                                            &config.datastore.key_maps,
                                            permission,
                                        )
                                        .await;
                                        notifications_status.set(Some(value));
                                        spawn_health_checks(
                                            config,
                                            false,
                                            health_report,
                                            health_running,
                                            active_key,
                                            notifications_status,
                                        );
                                    }
                                    Err(err) => {
                                        let log_datastore = logs_datastore();
                                        let _ = app_log_error_store(
                                            &log_datastore,
                                            &config.datastore.key_maps,
                                            &err,
                                        )
                                        .await;
                                        notifications_status.set(Some(err.to_string()));
                                    }
                                }
                                notifications_requesting.set(false);
                            });
                        }
                        disabled=notifications_disabled
                    >
                        {notifications_button_label}
                    </button>
                    <span>{notifications_label}</span>
                </div>
            </div>
            <div style="margin-top: 16px;">
                <div style="font-weight: 600;">"health checks"</div>
                <div style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                    <button
                        on:click=move |_| {
                            let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                            let Some(config) = config else {
                                return;
                            };
                            let setup_required_value = setup_required
                                .get()
                                .unwrap_or(false);
                            spawn_health_checks(
                                config,
                                setup_required_value,
                                health_report,
                                health_running,
                                active_key,
                                notifications_status,
                            );
                        }
                        disabled=health_disabled
                    >
                        {move || if health_running.get() { "checking" } else { "run checks" }}
                    </button>
                </div>
                <div style="margin-top: 8px; display: grid; gap: 6px;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().key_maps.status)
                            )
                        ></span>
                        <span>"key_maps"</span>
                        <span>{move || health_result_label(&health_report.get().key_maps)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().bootstrap_state.status)
                            )
                        ></span>
                        <span>"bootstrap_state"</span>
                        <span>{move || health_result_label(&health_report.get().bootstrap_state)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().state_active_key.status)
                            )
                        ></span>
                        <span>"state_active_key"</span>
                        <span>{move || health_result_label(&health_report.get().state_active_key)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().notifications.status)
                            )
                        ></span>
                        <span>"notifications"</span>
                        <span>{move || health_result_label(&health_report.get().notifications)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().tangle.status)
                            )
                        ></span>
                        <span>"tangle"</span>
                        <span>{move || health_result_label(&health_report.get().tangle)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().datastore_roundtrip.status)
                            )
                        ></span>
                        <span>"datastore_roundtrip"</span>
                        <span>{move || health_result_label(&health_report.get().datastore_roundtrip)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().keystore.status)
                            )
                        ></span>
                        <span>"keystore"</span>
                        <span>{move || health_result_label(&health_report.get().keystore)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span>"active_key"</span>
                        <span>{move || active_key_label(active_key.get())}</span>
                    </div>
                </div>
            </div>
        </main>
    }
}

#[component]
pub fn RadrootsApp() -> impl IntoView {
    view! {
        <Router>
            <AppShell />
        </Router>
    }
}

#[component]
fn AppShell() -> impl IntoView {
    let backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let init_error = RwSignal::new_local(None::<RadrootsAppInitError>);
    let init_state = RwSignal::new_local(app_init_state_default());
    let setup_required = RwSignal::new_local(None::<bool>);
    let navigate = use_navigate();
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    provide_context(setup_required);
    Effect::new(move || {
        let navigate = navigate.clone();
        spawn_local(async move {
            let stage = RadrootsAppInitStage::Storage;
            init_state.update(|state| app_init_stage_set(state, stage));
            log_init_stage(stage);
            let config = app_config_default();
            if !app_init_has_completed() {
                init_state.update(|state| {
                    state.loaded_bytes = 0;
                    state.total_bytes = Some(0);
                });
                let assets_result = app_init_assets(
                    &config,
                    |stage| {
                        init_state.update(|state| app_init_stage_set(state, stage));
                        log_init_stage(stage);
                    },
                    |loaded, total| {
                        init_state.update(|state| {
                            app_init_progress_add(state, loaded);
                            match total {
                                Some(value) => app_init_total_add(state, value),
                                None => app_init_total_unknown(state),
                            }
                        });
                    },
                )
                .await;
                if let Err(err) = assets_result {
                    let init_err = RadrootsAppInitError::Assets(err);
                    let _ = app_log_error_emit(&init_err);
                    init_error.set(Some(init_err));
                    let stage = RadrootsAppInitStage::Error;
                    init_state.update(|state| app_init_stage_set(state, stage));
                    log_init_stage(stage);
                    return;
                }
                let stage = RadrootsAppInitStage::Storage;
                init_state.update(|state| app_init_stage_set(state, stage));
                log_init_stage(stage);
            }
            match app_init_backends(config).await {
                Ok(value) => {
                    let key_maps = value.config.datastore.key_maps.clone();
                    let datastore = value.datastore.clone();
                    let keystore_config = value.nostr_keystore.get_config();
                    backends.set(Some(value));
                    app_init_mark_completed();
                    let stage = RadrootsAppInitStage::Ready;
                    init_state.update(|state| app_init_stage_set(state, stage));
                    log_init_stage(stage);
                    let navigate = navigate.clone();
                    let setup_required = setup_required.clone();
                    spawn_local(async move {
                        let keystore = radroots_studio_app_core::keystore::RadrootsClientWebKeystoreNostr::new(
                            Some(keystore_config),
                        );
                        match app_init_needs_setup(datastore.as_ref(), &keystore, &key_maps).await {
                            Ok(needs_setup) => {
                                setup_required.set(Some(needs_setup));
                                if needs_setup {
                                    navigate("/setup", Default::default());
                                }
                            }
                            Err(err) => {
                                let _ = app_log_error_emit(&err);
                                setup_required.set(Some(true));
                                navigate("/setup", Default::default());
                            }
                        }
                    });
                    let flush_ctx = backends.with_untracked(|value| {
                        value.as_ref().map(|backends| backends.config.datastore.key_maps.clone())
                    });
                    if let Some(key_maps) = flush_ctx {
                        spawn_local(async move {
                            let _ = app_log_buffer_flush_deferred(
                                &logs_datastore(),
                                &key_maps,
                                true,
                            )
                            .await;
                        });
                    }
                }
                Err(err) => {
                    let _ = app_log_error_emit(&err);
                    init_error.set(Some(err));
                    let stage = RadrootsAppInitStage::Error;
                    init_state.update(|state| app_init_stage_set(state, stage));
                    log_init_stage(stage);
                }
            }
        })
    });
    view! {
        <Show
            when=move || {
                init_state.get().stage == RadrootsAppInitStage::Ready
                    && setup_required.get().is_some()
            }
            fallback=|| view! { <SplashPage /> }
        >
        <Show
            when=move || setup_required.get() == Some(false)
            fallback=|| view! { <SetupPage /> }
        >
            <div id="app-shell">
                <nav id="app-nav" aria-label="Primary" style="display:flex;gap:12px;margin-bottom:12px;">
                    <A href="/" exact=true>"home"</A>
                    <A href="/logs">"logs"</A>
                    <A href="/ui">"ui"</A>
                    <A href="/settings">"settings"</A>
                    <A href="/setup">"setup"</A>
                </nav>
                <Routes fallback=|| view! { <div>"not_found"</div> }>
                    <Route path=path!("") view=HomePage />
                    <Route path=path!("logs") view=RadrootsAppLogsPage />
                    <Route path=path!("ui") view=RadrootsAppUiDemoPage />
                    <Route path=path!("settings") view=RadrootsAppSettingsPage />
                    <Route path=path!("setup") view=SetupPage />
                </Routes>
            </div>
        </Show>
    </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::app_health_check_delay_ms;

    #[test]
    fn health_check_delay_is_positive() {
        assert!(app_health_check_delay_ms() > 0);
    }
}
