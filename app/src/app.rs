use gloo_timers::future::TimeoutFuture;
use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::path;
use web_sys::HtmlElement;

use radroots_studio_app_core::datastore::RadrootsClientDatastore;
use radroots_studio_app_core::idb::IDB_CONFIG_LOGS;
use radroots_studio_app_core::keystore::{
    RadrootsClientKeystoreError,
    RadrootsClientKeystoreNostr,
    RadrootsClientWebKeystoreNostr,
};
use radroots_studio_app_ui_components::{
    RadrootsAppUiButtonLayoutAction,
    RadrootsAppUiButtonLayoutBackAction,
    RadrootsAppUiButtonLayoutPair,
};

use crate::t;
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
    app_i18n_init,
    app_log_buffer_flush_deferred,
    app_log_debug_emit,
    app_log_error_emit,
    app_log_error_store,
    app_config_default,
    app_datastore_clear_setup_draft,
    app_datastore_read_state,
    app_datastore_read_setup_draft,
    app_datastore_write_profile_seed,
    app_datastore_write_setup_draft,
    app_keystore_nostr_ensure_key,
    app_state_notifications_permission_value,
    app_state_set_notifications_permission_value,
    app_setup_eula_date,
    app_setup_finalize_with_key,
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
    RadrootsAppKeystoreError,
    RadrootsAppProfileSeed,
    RadrootsAppRole,
    RadrootsAppSettingsPage,
    RadrootsAppSetupDraft,
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

fn init_stage_label(stage: RadrootsAppInitStage) -> String {
    match stage {
        RadrootsAppInitStage::Idle => t!("app.init.stage.idle"),
        RadrootsAppInitStage::Storage => t!("app.init.stage.storage"),
        RadrootsAppInitStage::DownloadSql => t!("app.init.stage.download_sql"),
        RadrootsAppInitStage::DownloadGeo => t!("app.init.stage.download_geo"),
        RadrootsAppInitStage::Database => t!("app.init.stage.database"),
        RadrootsAppInitStage::Geocoder => t!("app.init.stage.geocoder"),
        RadrootsAppInitStage::Ready => t!("app.init.stage.ready"),
        RadrootsAppInitStage::Error => t!("app.init.stage.error"),
    }
}

fn health_status_label(status: RadrootsAppHealthCheckStatus) -> String {
    match status {
        RadrootsAppHealthCheckStatus::Ok => t!("app.home.health.status.ok"),
        RadrootsAppHealthCheckStatus::Error => t!("app.home.health.status.error"),
        RadrootsAppHealthCheckStatus::Skipped => t!("app.home.health.status.skipped"),
    }
}

fn health_message_label(message: &str) -> String {
    match message {
        "missing" => t!("app.home.health.message.missing"),
        "mismatch" => t!("app.home.health.message.mismatch"),
        "uninitialized" => t!("app.home.health.message.uninitialized"),
        "unavailable" => t!("app.home.health.message.unavailable"),
        _ => message.to_string(),
    }
}

fn health_result_label(result: &RadrootsAppHealthCheckResult) -> String {
    let status = health_status_label(result.status);
    match result.message.as_deref() {
        Some(message) => format!("{}: {}", status, health_message_label(message)),
        None => status,
    }
}

fn error_label(key: &str) -> Option<String> {
    let label = match key {
        "error.app.init.idb" => t!("error.app.init.idb"),
        "error.app.init.datastore" => t!("error.app.init.datastore"),
        "error.app.init.keystore" => t!("error.app.init.keystore"),
        "error.app.init.config" => t!("error.app.init.config"),
        "error.app.init.assets" => t!("error.app.init.assets"),
        "error.app.state.missing" => t!("error.app.state.missing"),
        "error.app.state.corrupt" => t!("error.app.state.corrupt"),
        "error.app.state.checksum_invalid" => t!("error.app.state.checksum_invalid"),
        "error.app.state.schema_unsupported" => t!("error.app.state.schema_unsupported"),
        "error.app.state.already_exists" => t!("error.app.state.already_exists"),
        "error.client.notifications.unavailable" => t!("error.client.notifications.unavailable"),
        "error.client.notifications.read_failure" => t!("error.client.notifications.read_failure"),
        _ => return None,
    };
    Some(label)
}

fn reset_status_label(value: &str) -> String {
    match value {
        "reset_idle" => t!("app.home.reset.status.idle"),
        "resetting" => t!("app.home.reset.status.resetting"),
        "reset_missing_backends" => t!("app.home.reset.status.missing_backends"),
        "reset_done" => t!("app.home.reset.status.done"),
        _ => error_label(value).unwrap_or_else(|| value.to_string()),
    }
}

fn notifications_status_label(value: &str) -> String {
    match value {
        "granted" => t!("app.home.notifications.status.granted"),
        "denied" => t!("app.home.notifications.status.denied"),
        "default" => t!("app.home.notifications.status.default"),
        "unavailable" => t!("app.home.notifications.status.unavailable"),
        _ => error_label(value).unwrap_or_else(|| value.to_string()),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RadrootsAppSetupFarmerChoice {
    Yes,
    No,
}

fn active_key_label(value: Option<String>) -> String {
    let Some(value) = value else {
        return t!("app.common.missing");
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
    let fallback_backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let backends = context
        .as_ref()
        .map(|value| value.backends)
        .unwrap_or(fallback_backends);
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
    let setup_farmer_choice = RwSignal::new_local(None::<RadrootsAppSetupFarmerChoice>);
    let setup_eula_scrolled = RwSignal::new_local(false);
    let nostr_key_add = RwSignal::new_local(String::new());
    let profile_name = RwSignal::new_local(String::new());
    let profile_nip05 = RwSignal::new_local(true);
    let setup_draft_loaded = RwSignal::new_local(false);
    let on_generate_key = setup_touch_callback("generate_key");
    let on_add_key = setup_touch_callback("add_key");
    Effect::new(move || {
        if setup_required.get() == Some(false) {
            navigate_guard("/", Default::default());
        }
    });
    Effect::new({
        let backends = backends.clone();
        let setup_draft_loaded = setup_draft_loaded.clone();
        let setup_key_choice = setup_key_choice.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let profile_nip05 = profile_nip05.clone();
        move |_| {
            if setup_draft_loaded.get() {
                return;
            }
            let Some((datastore, key_maps)) = backends
                .with(|value| value.as_ref().map(|backends| (backends.datastore.clone(), backends.config.datastore.key_maps.clone())))
            else {
                return;
            };
            spawn_local(async move {
                if let Ok(Some(draft)) = app_datastore_read_setup_draft(datastore.as_ref(), &key_maps).await {
                    if let Some(public_key) = draft.nostr_public_key {
                        nostr_key_add.set(public_key);
                        setup_key_choice.set(Some(RadrootsAppSetupKeyChoice::AddExisting));
                    }
                    if let Some(name) = draft.profile_name {
                        profile_name.set(name);
                    }
                    if let Some(nip05_request) = draft.nip05_request {
                        profile_nip05.set(nip05_request);
                    }
                }
                setup_draft_loaded.set(true);
            });
        }
    });
    Effect::new({
        let backends = backends.clone();
        let setup_draft_loaded = setup_draft_loaded.clone();
        let setup_key_choice = setup_key_choice.clone();
        let setup_farmer_choice = setup_farmer_choice.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let profile_nip05 = profile_nip05.clone();
        move |_| {
            if !setup_draft_loaded.get() {
                return;
            }
            let Some((datastore, key_maps)) = backends
                .with(|value| value.as_ref().map(|backends| (backends.datastore.clone(), backends.config.datastore.key_maps.clone())))
            else {
                return;
            };
            let nostr_public_key = match setup_key_choice.get() {
                Some(RadrootsAppSetupKeyChoice::AddExisting) => {
                    let value = nostr_key_add.get();
                    let value = value.trim();
                    if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                }
                _ => None,
            };
            let profile_value = profile_name.get();
            let profile_name = if profile_value.trim().is_empty() {
                None
            } else {
                Some(profile_value)
            };
            let draft = RadrootsAppSetupDraft {
                nostr_public_key,
                profile_name,
                role: setup_farmer_choice.get().map(|_| RadrootsAppRole::default()),
                nip05_request: Some(profile_nip05.get()),
            };
            spawn_local(async move {
                let _ = app_datastore_write_setup_draft(datastore.as_ref(), &key_maps, &draft).await;
            });
        }
    });
    let advance_step: Callback<()> = {
        let backends = backends.clone();
        let setup_step = setup_step.clone();
        let setup_key_choice = setup_key_choice.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let setup_required = setup_required.clone();
        Callback::new(move |_| {
            let current_step = setup_step.get();
            if matches!(current_step, RadrootsAppSetupStep::Eula) {
                let key_choice = setup_key_choice.get();
                let nostr_key_add = nostr_key_add.get();
                let profile_name = profile_name.get();
                let profile_nip05 = profile_nip05.get();
                let eula_date = app_setup_eula_date();
                let setup_required = setup_required.clone();
                let backends = backends.clone();
                spawn_local(async move {
                    let Some((datastore, key_maps, keystore_config)) = backends.with_untracked(|value| {
                        value.as_ref().map(|backends| {
                            (
                                backends.datastore.clone(),
                                backends.config.datastore.key_maps.clone(),
                                backends.nostr_keystore.get_config(),
                            )
                        })
                    }) else {
                        return;
                    };
                    let keystore = RadrootsClientWebKeystoreNostr::new(Some(keystore_config));
                    let active_key = match key_choice {
                        Some(RadrootsAppSetupKeyChoice::AddExisting) => {
                            let secret_key = nostr_key_add.trim();
                            if secret_key.is_empty() {
                                let err = RadrootsAppInitError::Keystore(
                                    RadrootsClientKeystoreError::NostrInvalidSecretKey,
                                );
                                let _ = app_log_error_emit(&err);
                                return;
                            }
                            match keystore.add(secret_key).await {
                                Ok(value) => value,
                                Err(err) => {
                                    let init_err = RadrootsAppInitError::Keystore(err);
                                    let _ = app_log_error_emit(&init_err);
                                    return;
                                }
                            }
                        }
                        _ => match app_keystore_nostr_ensure_key(&keystore).await {
                            Ok(value) => value,
                            Err(err) => {
                                let init_err = match err {
                                    RadrootsAppKeystoreError::Keystore(inner) => {
                                        RadrootsAppInitError::Keystore(inner)
                                    }
                                    RadrootsAppKeystoreError::KeyMismatch => RadrootsAppInitError::Keystore(
                                        RadrootsClientKeystoreError::NostrInvalidSecretKey,
                                    ),
                                };
                                let _ = app_log_error_emit(&init_err);
                                return;
                            }
                        },
                    };
                    let nip05_key = if profile_nip05 {
                        let profile_name = profile_name.trim();
                        if profile_name.is_empty() {
                            None
                        } else {
                            Some(profile_name.to_string())
                        }
                    } else {
                        None
                    };
                    if !profile_name.trim().is_empty() {
                        let profile_seed = RadrootsAppProfileSeed {
                            public_key: active_key.clone(),
                            name: profile_name.trim().to_string(),
                            display_name: Some(profile_name.trim().to_string()),
                            nip05_request: profile_nip05,
                        };
                        if let Err(err) = app_datastore_write_profile_seed(
                            datastore.as_ref(),
                            &key_maps,
                            &profile_seed,
                        )
                        .await
                        {
                            let _ = app_log_error_emit(&err);
                            return;
                        }
                    }
                    if let Err(err) = app_setup_finalize_with_key(
                        datastore.as_ref(),
                        &key_maps,
                        active_key,
                        eula_date,
                        nip05_key,
                    )
                    .await
                    {
                        let _ = app_log_error_emit(&err);
                        return;
                    }
                    let _ = app_datastore_clear_setup_draft(datastore.as_ref(), &key_maps).await;
                    setup_required.set(Some(false));
                });
                return;
            }
            if matches!(current_step, RadrootsAppSetupStep::Profile) {
                let profile_name = profile_name.get();
                if profile_name.trim().is_empty() {
                    let setup_step = setup_step.clone();
                    let confirm_message = t!("app.setup.profile.confirm_no_name");
                    spawn_local(async move {
                        let notifications = RadrootsAppNotifications::new(None);
                        let confirm = notifications.confirm_message(&confirm_message).await;
                        if confirm {
                            setup_step.set(RadrootsAppSetupStep::FarmerSetup);
                        }
                    });
                    return;
                }
                setup_step.set(RadrootsAppSetupStep::FarmerSetup);
                return;
            }
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
                    RadrootsAppSetupStep::Profile => RadrootsAppSetupStep::FarmerSetup,
                    RadrootsAppSetupStep::FarmerSetup => RadrootsAppSetupStep::Eula,
                    RadrootsAppSetupStep::Eula => RadrootsAppSetupStep::Eula,
                };
            });
        })
    };
    let advance_step_click: Callback<MouseEvent> = {
        let advance_step = advance_step.clone();
        Callback::new(move |_| {
            advance_step.run(());
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
                RadrootsAppSetupStep::FarmerSetup => RadrootsAppSetupStep::Profile,
                RadrootsAppSetupStep::Eula => RadrootsAppSetupStep::FarmerSetup,
            };
            setup_step.set(next_step);
            if matches!(next_step, RadrootsAppSetupStep::Intro) {
                setup_key_choice.set(None);
            }
        })
    };
    let on_generate_key = on_generate_key.clone();
    let on_add_key = on_add_key.clone();
    Effect::new({
        let setup_step = setup_step.clone();
        let setup_eula_scrolled = setup_eula_scrolled.clone();
        move |_| {
            if !matches!(setup_step.get(), RadrootsAppSetupStep::Eula) {
                setup_eula_scrolled.set(false);
            }
        }
    });
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
                                            {t!("app.setup.intro.kicker")}
                                        </p>
                                        <div
                                            id="app-setup-intro-copy"
                                            class="flex flex-col w-full gap-2 justify-start items-center"
                                        >
                                            <p
                                                id="app-setup-intro-line-welcome"
                                                class="w-full text-left font-mono font-[400] text-[1.1rem] text-ly0-gl"
                                            >
                                                {t!("app.setup.intro.welcome")}
                                            </p>
                                            <p
                                                id="app-setup-intro-line-body"
                                                class="w-full text-left font-mono font-[400] text-[1.1rem] text-ly0-gl"
                                            >
                                                {t!("app.setup.intro.body")}
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
                                    {t!("app.setup.key_choice.title")}
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
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
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
                                        {t!("app.setup.key_choice.create")}
                                    </span>
                                </button>
                                <button
                                    id="app-setup-key-choice-add"
                                    type="button"
                                    class=move || {
                                        if setup_key_choice.get()
                                            == Some(RadrootsAppSetupKeyChoice::AddExisting)
                                        {
                                            "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
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
                                        {t!("app.setup.key_choice.use_existing")}
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
                                    {t!("app.setup.key_add.title")}
                                </p>
                                <input
                                    id="app-setup-key-add-existing-input"
                                    class="input-base w-lo_ios0 ios1:w-lo_ios1 text-[1.25rem] text-center placeholder:opacity-60"
                                    type="text"
                                    placeholder=t!("app.setup.key_add.placeholder")
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
                                    {t!("app.setup.profile.title")}
                                </p>
                                <div
                                    id="app-setup-profile-fields"
                                    class="flex flex-col w-full gap-4 justify-center items-center"
                                >
                                    <input
                                        id="app-setup-profile-name"
                                        class="input-base w-lo_ios0 ios1:w-lo_ios1 text-[1.25rem] text-center placeholder:opacity-60"
                                        type="text"
                                        placeholder=t!("app.setup.profile.placeholder")
                                        prop:value=move || profile_name.get()
                                        on:keydown=move |ev: KeyboardEvent| {
                                            if ev.key() == "Enter" {
                                                ev.prevent_default();
                                                advance_step.run(());
                                            }
                                        }
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
                                                {t!("app.setup.profile.nip05.prefix")}
                                                {" "}
                                                <span class="font-mono font-[500] tracking-tight px-[3px]">
                                                    "@radroots"
                                                </span>
                                                {" "}
                                                {t!("app.setup.profile.nip05.suffix")}
                                            </span>
                                        </label>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </section>
                }.into_any(),
                RadrootsAppSetupStep::FarmerSetup => view! {
                    <section
                        id="app-setup-farmer"
                        class="app-view app-view-enter flex flex-col w-full px-6 pt-10 pb-16"
                        on:click=move |_| {
                            setup_farmer_choice.set(None);
                        }
                    >
                        <div
                            id="app-setup-farmer-body"
                            class="flex flex-1 w-full flex-col justify-center items-center"
                        >
                            <div
                                id="app-setup-farmer-card"
                                class="flex flex-col h-[16rem] w-full gap-10 justify-start items-center"
                            >
                                <div
                                    id="app-setup-farmer-title"
                                    class="flex flex-row w-full justify-center items-center"
                                >
                                    <p class="font-sans font-[600] text-ly0-gl text-3xl">
                                        {t!("app.setup.farmer.title")}
                                    </p>
                                </div>
                                <div
                                    id="app-setup-farmer-actions"
                                    class="flex flex-col w-full gap-5 justify-center items-center"
                                >
                                    <button
                                        id="app-setup-farmer-yes"
                                        type="button"
                                        class=move || {
                                            if setup_farmer_choice.get()
                                                == Some(RadrootsAppSetupFarmerChoice::Yes)
                                            {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
                                            } else {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                            }
                                        }
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            setup_farmer_choice.set(Some(RadrootsAppSetupFarmerChoice::Yes));
                                        }
                                    >
                                        <span class="font-sans font-[600] text-ly0-gl text-xl">
                                            {t!("app.common.yes")}
                                        </span>
                                    </button>
                                    <button
                                        id="app-setup-farmer-no"
                                        type="button"
                                        class=move || {
                                            if setup_farmer_choice.get()
                                                == Some(RadrootsAppSetupFarmerChoice::No)
                                            {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
                                            } else {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                            }
                                        }
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            setup_farmer_choice.set(Some(RadrootsAppSetupFarmerChoice::No));
                                        }
                                    >
                                        <span class="font-sans font-[600] text-ly0-gl text-xl">
                                            {t!("app.common.no")}
                                        </span>
                                    </button>
                                </div>
                            </div>
                        </div>
                    </section>
                }.into_any(),
                RadrootsAppSetupStep::Eula => view! {
                    <section
                        id="app-setup-eula"
                        class="app-view app-view-enter flex flex-col h-full w-full px-6 pt-8 pb-6"
                    >
                        <div
                            id="app-setup-eula-body"
                            class="flex flex-col flex-1 min-h-0 w-full gap-5"
                        >
                            <header
                                id="app-setup-eula-header"
                                class="flex flex-row w-full justify-center items-center"
                            >
                                <p class="font-sans font-[600] text-ly0-gl text-2xl text-center">
                                    "End User License Agreement"
                                </p>
                            </header>
                            <div
                                id="app-setup-eula-scroll"
                                class="app-page-scroll scroll-hide flex flex-col flex-1 min-h-0 w-full gap-6 px-1 pb-6 overscroll-contain"
                                on:scroll=move |ev| {
                                    if setup_eula_scrolled.get() {
                                        return;
                                    }
                                    let target = event_target::<HtmlElement>(&ev);
                                    let scroll_top = target.scroll_top();
                                    let scroll_height = target.scroll_height();
                                    let client_height = target.client_height();
                                    if scroll_top + client_height >= scroll_height {
                                        setup_eula_scrolled.set(true);
                                    }
                                }
                            >
                                <section
                                    id="app-setup-eula-introduction"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Introduction"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "This End User License Agreement (\"EULA\") is a legal agreement between you and Radroots Inc. for the use of our mobile application Radroots. By installing, accessing, or using our application, you agree to be bound by the terms and conditions of this EULA."
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-prohibited-content"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Prohibited Content and Conduct"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "You agree not to use our application to create, upload, post, send, or store any content that:"
                                    </p>
                                    <ul class="flex flex-col gap-1 pl-5 list-disc text-sm text-ly0-gl leading-relaxed">
                                        <li>"Is illegal, infringing, or fraudulent"</li>
                                        <li>"Is pornographic, obscene, or offensive"</li>
                                        <li>"Is discriminatory or promotes hate speech"</li>
                                        <li>"Is harmful to minors"</li>
                                        <li>"Is intended to harass or bully others"</li>
                                        <li>"Is intended to impersonate others"</li>
                                    </ul>
                                </section>
                                <section
                                    id="app-setup-eula-prohibited-conduct"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "You also agree not to engage in any conduct that:"
                                    </h3>
                                    <ul class="flex flex-col gap-1 pl-5 list-disc text-sm text-ly0-gl leading-relaxed">
                                        <li>"Harasses or bullies others"</li>
                                        <li>"Impersonates others"</li>
                                        <li>"Is intended to intimidate or threaten others"</li>
                                        <li>"Is intended to promote or incite violence"</li>
                                    </ul>
                                </section>
                                <section
                                    id="app-setup-eula-consequences"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Consequences of Violation"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "Any violation of this EULA, including the prohibited content and conduct outlined above, may result in the termination of your access to our application."
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-disclaimer"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Disclaimer of Warranties and Limitation of Liability"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "Our application is provided \"as is\" and \"as available\" without warranty of any kind, either express or implied, including but not limited to the implied warranties of merchantability and fitness for a particular purpose. We do not guarantee that our application will be uninterrupted or error-free. In no event shall Radroots Inc. be liable for any damages whatsoever, including but not limited to direct, indirect, special, incidental, or consequential damages, arising out of or in connection with the use or inability to use our application."
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-changes"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Changes to EULA"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "We reserve the right to update or modify this EULA at any time and without prior notice. Your continued use of our application following any changes to this EULA will be deemed to be your acceptance of such changes."
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-contact"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Contact Information"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "If you have any questions about this EULA, please contact us at info@radroots.org."
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-acceptance"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        "Acceptance of Terms"
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        "By using our application, you signify your acceptance of this EULA. If you do not agree to this EULA, you may not use our application."
                                    </p>
                                </section>
                            </div>
                        </div>
                        <div
                            id="app-setup-eula-actions"
                            class="flex flex-row w-full pt-4 justify-center items-center"
                        >
                            <button
                                type="button"
                                class=move || {
                                    if setup_eula_scrolled.get() {
                                        "group flex flex-row basis-1/2 gap-3 justify-center items-center"
                                    } else {
                                        "group flex flex-row basis-1/2 gap-3 justify-center items-center opacity-80"
                                    }
                                }
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    rewind_step.run(ev);
                                }
                            >
                                <span class="font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl/80 el-re">
                                    "-"
                                </span>
                                <span class="font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl/80 el-re">
                                    "Disagree"
                                </span>
                                <span class="font-mono font-[400] text-sm text-ly0-gl group-active:text-ly0-gl/80 el-re">
                                    "-"
                                </span>
                            </button>
                            <button
                                type="button"
                                aria-disabled=move || !setup_eula_scrolled.get()
                                class=move || {
                                    if setup_eula_scrolled.get() {
                                        "relative group flex flex-row basis-1/2 gap-3 justify-center items-center el-re"
                                    } else {
                                        "relative group flex flex-row basis-1/2 gap-3 justify-center items-center opacity-40 pointer-events-none"
                                    }
                                }
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    if setup_eula_scrolled.get() {
                                        advance_step.run(());
                                    }
                                }
                            >
                                <span class="font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re">
                                    "-"
                                </span>
                                <span class="font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re">
                                    "Agree"
                                </span>
                                <span class="font-mono font-[400] text-sm text-ly0-gl-hl group-active:text-ly0-gl-hl/80 el-re">
                                    "-"
                                </span>
                            </button>
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
                    if matches!(step, RadrootsAppSetupStep::Eula) {
                        return view! { <></> }.into_any();
                    }
                    let continue_disabled = (matches!(step, RadrootsAppSetupStep::KeyChoice)
                        && setup_key_choice.get().is_none())
                        || (matches!(step, RadrootsAppSetupStep::FarmerSetup)
                            && setup_farmer_choice.get().is_none());
                    let continue_label = t!("app.common.continue");
                    let back_label = t!("app.common.back");
                    let continue_action = RadrootsAppUiButtonLayoutAction {
                        label: continue_label,
                        disabled: continue_disabled,
                        loading: false,
                        on_click: advance_step_click.clone(),
                    };
                    let back_action = RadrootsAppUiButtonLayoutBackAction {
                        visible: !matches!(step, RadrootsAppSetupStep::Intro),
                        label: Some(back_label),
                        disabled: false,
                        on_click: rewind_step.clone(),
                    };
                    view! {
                        <RadrootsAppUiButtonLayoutPair
                            continue_action=continue_action
                            back=back_action
                        />
                    }.into_any()
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
            .as_deref()
            .map(reset_status_label)
            .unwrap_or_else(|| t!("app.home.reset.status.idle"))
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
            .as_deref()
            .map(notifications_status_label)
            .unwrap_or_else(|| t!("app.common.unknown"))
    };
    let notifications_button_label = move || {
        if notifications_requesting.get() {
            t!("app.home.notifications.button.requesting")
        } else {
            t!("app.home.notifications.button.request")
        }
    };
    view! {
        <main id="app-home" class="app-page app-page-scroll">
            <header id="app-home-header">
                <h1 id="app-home-title">{t!("app.home.title")}</h1>
            </header>
            <section id="app-home-status" aria-label=t!("app.home.status.aria")>
                <div id="app-home-status-row" style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                    <span
                        style=move || format!(
                            "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                            status_color()
                        )
                    ></span>
                    <span>{move || init_stage_label(init_state.get().stage)}</span>
                </div>
            </section>
            <section id="app-home-reset" aria-label=t!("app.home.reset.aria")>
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
                        {t!("app.home.reset.button")}
                    </button>
                    <span>{reset_label}</span>
                </div>
            </section>
            <section id="app-home-notifications" aria-label=t!("app.home.notifications.aria") style="margin-top: 16px;">
                <header id="app-home-notifications-header">
                    <h2 id="app-home-notifications-title" style="font-weight: 600;">{t!("app.home.notifications.title")}</h2>
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
            <section id="app-home-health" aria-label=t!("app.home.health.aria") style="margin-top: 16px;">
                <header id="app-home-health-header">
                    <h2 id="app-home-health-title" style="font-weight: 600;">{t!("app.home.health.title")}</h2>
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
                        {move || {
                            if health_running.get() {
                                t!("app.home.health.button.checking")
                            } else {
                                t!("app.home.health.button.run")
                            }
                        }}
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
                        <span>{t!("app.home.health.item.key_maps")}</span>
                        <span>{move || health_result_label(&health_report.get().key_maps)}</span>
                    </li>
                    <li id="app-home-health-bootstrap-state" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().bootstrap_state.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.bootstrap_state")}</span>
                        <span>{move || health_result_label(&health_report.get().bootstrap_state)}</span>
                    </li>
                    <li id="app-home-health-active-key-state" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().state_active_key.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.state_active_key")}</span>
                        <span>{move || health_result_label(&health_report.get().state_active_key)}</span>
                    </li>
                    <li id="app-home-health-notifications" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().notifications.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.notifications")}</span>
                        <span>{move || health_result_label(&health_report.get().notifications)}</span>
                    </li>
                    <li id="app-home-health-tangle" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().tangle.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.tangle")}</span>
                        <span>{move || health_result_label(&health_report.get().tangle)}</span>
                    </li>
                    <li id="app-home-health-datastore" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().datastore_roundtrip.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.datastore_roundtrip")}</span>
                        <span>{move || health_result_label(&health_report.get().datastore_roundtrip)}</span>
                    </li>
                    <li id="app-home-health-keystore" style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().keystore.status)
                            )
                        ></span>
                        <span>{t!("app.home.health.item.keystore")}</span>
                        <span>{move || health_result_label(&health_report.get().keystore)}</span>
                    </li>
                    <li id="app-home-health-active-key" style="display: flex; align-items: center; gap: 8px;">
                        <span>{t!("app.home.health.item.active_key")}</span>
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
    provide_context(app_i18n_init());
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
                <nav id="app-nav" aria-label=t!("app.nav.primary_aria") style="display:flex;gap:12px;margin-bottom:12px;">
                    <A href="/" exact=true>{t!("app.nav.home")}</A>
                    <A href="/logs">{t!("app.nav.logs")}</A>
                    <A href="/ui">{t!("app.nav.ui")}</A>
                    <A href="/settings">{t!("app.nav.settings")}</A>
                    <A href="/setup">{t!("app.nav.setup")}</A>
                </nav>
                <Routes
                    fallback=|| view! {
                        <main id="app-not-found" class="app-page app-page-fixed">
                            <p id="app-not-found-label">{t!("app.not_found")}</p>
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
