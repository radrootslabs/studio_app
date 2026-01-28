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

fn setup_touch_callback(action: &'static str) -> Callback<MouseEvent> {
    Callback::new(move |_| {
        let _ = app_log_debug_emit("log.app.setup.choice", action, None);
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RadrootsAppSetupKeyChoice {
    Generate,
    AddExisting,
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
            class="app-page app-page-fixed"
            style="min-height:100dvh;background:white;display:flex;align-items:center;justify-content:center;"
        >
        </main>
    }
}

#[component]
fn LogoCircle() -> impl IntoView {
    view! {
        <div
            id="app-logo-circle"
            class="relative flex flex-col h-[196px] w-full justify-center items-center"
        >
            <div
                id="app-logo-mark"
                class="relative flex flex-row h-36 w-36 justify-center items-center bg-ly2 rounded-full"
            >
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
    let setup_key_choice = RwSignal::new_local(None::<RadrootsAppSetupKeyChoice>);
    let nostr_key_add = RwSignal::new_local(String::new());
    let profile_name = RwSignal::new_local(String::new());
    let profile_nip05 = RwSignal::new_local(true);
    let on_generate_key = setup_touch_callback("generate_key");
    let on_add_key = setup_touch_callback("add_key");
    Effect::new(move || {
        if setup_required.get() == Some(false) {
            navigate_guard("/", Default::default());
        }
    });
    let advance_step: Callback<MouseEvent> = {
        let setup_step = setup_step.clone();
        let setup_key_choice = setup_key_choice.clone();
        Callback::new(move |_| {
            setup_step.update(|step| {
                *step = match *step {
                    RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::KeyChoice,
                    RadrootsAppSetupStep::KeyChoice => {
                        match setup_key_choice.get() {
                            Some(RadrootsAppSetupKeyChoice::Generate) => {
                                RadrootsAppSetupStep::Profile
                            }
                            Some(RadrootsAppSetupKeyChoice::AddExisting) => {
                                RadrootsAppSetupStep::KeyAddExisting
                            }
                            None => RadrootsAppSetupStep::KeyChoice,
                        }
                    }
                    RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::Profile,
                    RadrootsAppSetupStep::Profile => RadrootsAppSetupStep::Profile,
                };
            });
        })
    };
    let rewind_step: Callback<MouseEvent> = {
        let setup_step = setup_step.clone();
        let setup_key_choice = setup_key_choice.clone();
        Callback::new(move |_| {
            let current_step = setup_step.get();
            let next_step = match current_step {
                RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::Intro,
                RadrootsAppSetupStep::KeyChoice => RadrootsAppSetupStep::Intro,
                RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::KeyChoice,
                RadrootsAppSetupStep::Profile => match setup_key_choice.get() {
                    Some(RadrootsAppSetupKeyChoice::AddExisting) => {
                        RadrootsAppSetupStep::KeyAddExisting
                    }
                    _ => RadrootsAppSetupStep::KeyChoice,
                },
            };
            setup_step.set(next_step);
            if matches!(next_step, RadrootsAppSetupStep::Intro) {
                setup_key_choice.set(None);
            }
        })
    };
    let on_generate_key = on_generate_key.clone();
    let on_add_key = on_add_key.clone();
    view! {
        <main
            id="app-setup"
            class="app-page app-page-fixed relative w-full flex flex-col"
        >
            {move || match setup_step.get() {
                RadrootsAppSetupStep::Intro => {
                    let navigate_home = navigate_home.clone();
                    view! {
                        <section
                            id="app-setup-intro"
                            class="app-view app-view-enter relative flex flex-col h-[100dvh] w-full justify-start items-center"
                        >
                            <div
                                id="app-setup-intro-body"
                                class="flex flex-col h-full w-full justify-start items-center"
                            >
                                <div
                                    id="app-setup-intro-stage"
                                    class="relative flex flex-col h-full w-full justify-center items-center"
                                >
                                    <header
                                        id="app-setup-intro-header"
                                        class="flex flex-row w-full justify-start items-center -translate-y-16"
                                    >
                                        <button
                                            type="button"
                                            id="app-setup-intro-logo-button"
                                            class="flex flex-row w-full justify-center items-center"
                                            on:click=move |_| navigate_home("/", Default::default())
                                        >
                                            <LogoCircle />
                                        </button>
                                    </header>
                                    <footer
                                        id="app-setup-intro-footer"
                                        class="absolute bottom-0 left-0 flex flex-col h-[20rem] w-full px-10 gap-2 justify-start items-center"
                                    >
                                        <p
                                            id="app-setup-intro-kicker"
                                            class="w-full text-left font-sans font-[400] text-sm uppercase text-ly0-gl-label"
                                        >
                                            "Configure"
                                        </p>
                                        <div
                                            id="app-setup-intro-copy"
                                            class="flex flex-col w-full gap-2 justify-start items-center"
                                        >
                                            <p
                                                id="app-setup-intro-line-welcome"
                                                class="w-full text-left font-mono font-[400] text-[1.1rem] text-ly0-gl"
                                            >
                                                "Welcome to Radroots!"
                                            </p>
                                            <p
                                                id="app-setup-intro-line-body"
                                                class="w-full text-left font-mono font-[400] text-[1.1rem] text-ly0-gl"
                                            >
                                                "Your device will be configured by the setup wizard."
                                            </p>
                                        </div>
                                    </footer>
                                </div>
                            </div>
                        </section>
                    }
                    .into_any()
                },
                RadrootsAppSetupStep::KeyChoice => view! {
                    <section
                        id="app-setup-key-choice"
                        class="app-view app-view-enter flex flex-col w-full px-6 pt-10 pb-16"
                        on:click=move |_| {
                            setup_key_choice.set(None);
                        }
                    >
                        <div
                            id="app-setup-key-choice-body"
                            class="flex flex-1 w-full flex-col justify-center items-center gap-8"
                        >
                            <div
                                id="app-setup-key-choice-title"
                                class="flex flex-row w-full justify-center items-center"
                            >
                                <p class="font-sans font-[600] text-ly0-gl text-3xl">
                                    "Configure Device"
                                </p>
                            </div>
                            <div
                                id="app-setup-key-choice-actions"
                                class="flex flex-col w-full gap-6 justify-center items-center"
                            >
                                <button
                                    id="app-setup-key-choice-generate"
                                    type="button"
                                    class=move || {
                                        if setup_key_choice.get()
                                            == Some(RadrootsAppSetupKeyChoice::Generate)
                                        {
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-apply-active ly1-raise-apply ly1-ring-apply el-re"
                                        } else {
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                        }
                                    }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        setup_key_choice.set(Some(RadrootsAppSetupKeyChoice::Generate));
                                        on_generate_key.run(ev);
                                    }
                                >
                                    <span class="font-sans font-[600] text-ly0-gl text-xl">
                                        "Create new keypair"
                                    </span>
                                </button>
                                <button
                                    id="app-setup-key-choice-add"
                                    type="button"
                                    class=move || {
                                        if setup_key_choice.get()
                                            == Some(RadrootsAppSetupKeyChoice::AddExisting)
                                        {
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-apply-active ly1-raise-apply ly1-ring-apply el-re"
                                        } else {
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                        }
                                    }
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        setup_key_choice.set(Some(RadrootsAppSetupKeyChoice::AddExisting));
                                        on_add_key.run(ev);
                                    }
                                >
                                    <span class="font-sans font-[600] text-ly0-gl text-xl">
                                        "Use existing keypair"
                                    </span>
                                </button>
                            </div>
                        </div>
                    </section>
                }.into_any(),
                RadrootsAppSetupStep::KeyAddExisting => view! {
                    <section
                        id="app-setup-key-add-existing"
                        class="app-view app-view-enter flex flex-col w-full px-6 pt-10 pb-16"
                    >
                        <div
                            id="app-setup-key-add-existing-body"
                            class="flex flex-1 w-full flex-col justify-center items-center"
                        >
                            <div
                                id="app-setup-key-add-existing-card"
                                class="flex flex-col w-full gap-6 justify-center items-center"
                            >
                                <p
                                    id="app-setup-key-add-existing-title"
                                    class="font-sans font-[600] text-ly0-gl text-3xl capitalize"
                                >
                                    "Add existing key"
                                </p>
                                <input
                                    id="app-setup-key-add-existing-input"
                                    class="input-base w-lo_ios0 ios1:w-lo_ios1 text-[1.25rem] text-center placeholder:opacity-60"
                                    type="text"
                                    placeholder="Enter nostr nsec/hex"
                                    prop:value=move || nostr_key_add.get()
                                    on:input=move |ev| {
                                        nostr_key_add.set(event_target_value(&ev));
                                    }
                                />
                            </div>
                        </div>
                    </section>
                }.into_any(),
                RadrootsAppSetupStep::Profile => view! {
                    <section
                        id="app-setup-profile"
                        class="app-view app-view-enter flex flex-col w-full px-6 pt-10 pb-16"
                    >
                        <div
                            id="app-setup-profile-body"
                            class="flex flex-1 w-full flex-col justify-center items-center"
                        >
                            <div
                                id="app-setup-profile-card"
                                class="flex flex-col h-[16rem] w-full px-4 gap-6 justify-start items-center"
                            >
                                <p
                                    id="app-setup-profile-title"
                                    class="font-sans font-[600] text-ly0-gl text-3xl"
                                >
                                    "Add Profile"
                                </p>
                                <div
                                    id="app-setup-profile-fields"
                                    class="flex flex-col w-full gap-4 justify-center items-center"
                                >
                                    <input
                                        id="app-setup-profile-name"
                                        class="input-base w-lo_ios0 ios1:w-lo_ios1 text-[1.25rem] text-center placeholder:opacity-60"
                                        type="text"
                                        placeholder="Enter profile name"
                                        prop:value=move || profile_name.get()
                                        on:input=move |ev| {
                                            profile_name.set(event_target_value(&ev));
                                        }
                                    />
                                    <div
                                        id="app-setup-profile-nip05"
                                        class="flex flex-row w-full gap-2 justify-center items-center"
                                    >
                                        <input
                                            id="app-setup-profile-nip05-toggle"
                                            type="checkbox"
                                            prop:checked=move || profile_nip05.get()
                                            on:change=move |ev| {
                                                profile_nip05.set(event_target_checked(&ev));
                                            }
                                        />
                                        <label
                                            for="app-setup-profile-nip05-toggle"
                                            class="flex flex-row justify-center items-center"
                                        >
                                            <span class="font-sans font-[400] text-ly0-gl text-[14px] tracking-wide">
                                                "Create "
                                                <span class="font-mono font-[500] tracking-tight px-[3px]">
                                                    "@radroots"
                                                </span>
                                                " NIP-05 address"
                                            </span>
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </section>
                }.into_any(),
            }}
            <footer
                id="app-setup-actions"
                class="z-10 absolute bottom-4 left-0 flex flex-col w-full justify-center items-center se-compact:bottom-0"
            >
                {move || {
                    let step = setup_step.get();
                    let continue_disabled = matches!(step, RadrootsAppSetupStep::KeyChoice)
                        && setup_key_choice.get().is_none();
                    let continue_action = RadrootsAppUiButtonLayoutAction {
                        label: "Continue".to_string(),
                        disabled: continue_disabled,
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
                            back=back_action
                        />
                    }
                }}
            </footer>
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
        <main id="app-home" class="app-page app-page-scroll">
            <header id="app-home-header">
                <h1 id="app-home-title">"app"</h1>
            </header>
            <section id="app-home-status" aria-label="Status">
                <div id="app-home-status-row" style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                    <span
                        style=move || format!(
                            "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                            status_color()
                        )
                    ></span>
                    <span>{move || init_state.get().stage.as_str()}</span>
                </div>
            </section>
            <section id="app-home-reset" aria-label="Reset">
                <div id="app-home-reset-row" style="margin-top: 12px; display: flex; align-items: center; gap: 8px;">
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
            </section>
            <section id="app-home-notifications" aria-label="Notifications" style="margin-top: 16px;">
                <header id="app-home-notifications-header">
                    <h2 id="app-home-notifications-title" style="font-weight: 600;">"notifications"</h2>
                </header>
                <div id="app-home-notifications-actions" style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
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
            </section>
            <section id="app-home-health" aria-label="Health checks" style="margin-top: 16px;">
                <header id="app-home-health-header">
                    <h2 id="app-home-health-title" style="font-weight: 600;">"health checks"</h2>
                </header>
                <div id="app-home-health-actions" style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
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
                <ul id="app-home-health-list" style="margin-top: 8px; display: grid; gap: 6px;">
                    <li id="app-home-health-key-maps" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().key_maps.status)
                            )
                        ></span>
                        <span>"key_maps"</span>
                        <span>{move || health_result_label(&health_report.get().key_maps)}</span>
                    </li>
                    <li id="app-home-health-bootstrap-state" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().bootstrap_state.status)
                            )
                        ></span>
                        <span>"bootstrap_state"</span>
                        <span>{move || health_result_label(&health_report.get().bootstrap_state)}</span>
                    </li>
                    <li id="app-home-health-active-key-state" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().state_active_key.status)
                            )
                        ></span>
                        <span>"state_active_key"</span>
                        <span>{move || health_result_label(&health_report.get().state_active_key)}</span>
                    </li>
                    <li id="app-home-health-notifications" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().notifications.status)
                            )
                        ></span>
                        <span>"notifications"</span>
                        <span>{move || health_result_label(&health_report.get().notifications)}</span>
                    </li>
                    <li id="app-home-health-tangle" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().tangle.status)
                            )
                        ></span>
                        <span>"tangle"</span>
                        <span>{move || health_result_label(&health_report.get().tangle)}</span>
                    </li>
                    <li id="app-home-health-datastore" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().datastore_roundtrip.status)
                            )
                        ></span>
                        <span>"datastore_roundtrip"</span>
                        <span>{move || health_result_label(&health_report.get().datastore_roundtrip)}</span>
                    </li>
                    <li id="app-home-health-keystore" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().keystore.status)
                            )
                        ></span>
                        <span>"keystore"</span>
                        <span>{move || health_result_label(&health_report.get().keystore)}</span>
                    </li>
                    <li id="app-home-health-active-key" style="display: flex; align-items: center; gap: 8px;">
                        <span>"active_key"</span>
                        <span>{move || active_key_label(active_key.get())}</span>
                    </li>
                </ul>
            </section>
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
                <Routes
                    fallback=|| view! {
                        <main id="app-not-found" class="app-page app-page-fixed">
                            <p id="app-not-found-label">"not_found"</p>
                        </main>
                    }
                >
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
