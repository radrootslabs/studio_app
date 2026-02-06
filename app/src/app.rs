use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::hooks::{use_location, use_navigate};
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
    RadrootsAppUiIcon,
    RadrootsAppUiIconKey,
    RadrootsAppUiNavHeader,
    RadrootsAppUiNavHeaderBgMode,
    RadrootsAppUiNavHeaderCollapseMode,
    RadrootsAppUiNavTabs,
    RadrootsAppUiScrollContainer,
    RadrootsAppUiScrollContext,
    RadrootsAppUiSpinner,
};
use uuid::Uuid;

use crate::t;
use crate::{
    app_init_assets,
    app_init_backends,
    app_init_has_completed,
    app_init_setup_status,
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
    app_datastore_write_profile_seed,
    app_datastore_write_setup_draft,
    app_keystore_nostr_ensure_key,
    app_setup_flow_role_from_choices,
    app_setup_flow_validate,
    app_setup_lock_acquire,
    app_setup_lock_enabled,
    app_setup_lock_release,
    app_setup_lock_ttl_ms,
    app_state_timestamp_ms,
    app_setup_eula_date,
    app_setup_finalize_with_key,
    app_setup_gate_from_status,
    app_setup_step_default,
    RadrootsAppBackends,
    RadrootsAppInitError,
    RadrootsAppInitStage,
    RadrootsAppNotifications,
    RadrootsAppLogsPage,
    RadrootsAppKeystoreError,
    RadrootsAppProfileSeed,
    RadrootsAppRole,
    RadrootsAppSettingsPage,
    RadrootsAppSetupDraft,
    RadrootsAppSetupFlowDraft,
    RadrootsAppSetupKeyChoice,
    RadrootsAppSetupFarmerChoice,
    RadrootsAppSetupBusinessChoice,
    RadrootsAppSetupLock,
    RadrootsAppSetupLockStatus,
    RadrootsAppSetupStatus,
    RadrootsAppUiDemoPage,
    RadrootsAppSetupStep,
    RadrootsAppSettingsStatusPage,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum HomeView {
    Activity,
    Profile,
}

impl HomeView {
    fn label(self) -> &'static str {
        match self {
            HomeView::Activity => "Activity",
            HomeView::Profile => "Profile",
        }
    }
}

#[component]
pub(crate) fn AppPageChrome(
    title: String,
    #[prop(optional)] header_right: Option<ChildrenFn>,
    #[prop(optional)] show_tabs: Option<bool>,
    #[prop(optional)] bg_mode: Option<RadrootsAppUiNavHeaderBgMode>,
    #[prop(optional)] collapse_mode: Option<RadrootsAppUiNavHeaderCollapseMode>,
    children: Children,
) -> impl IntoView {
    let scroll_context = RadrootsAppUiScrollContext::new();
    provide_context(scroll_context.clone());
    let show_tabs = show_tabs.unwrap_or(true);
    let bg_mode = bg_mode.unwrap_or(RadrootsAppUiNavHeaderBgMode::AutoBlur);
    let collapse_mode = collapse_mode.unwrap_or(RadrootsAppUiNavHeaderCollapseMode::Scroll);
    let location = use_location();
    let is_home = move || location.pathname.get() == "/";
    let is_test = move || location.pathname.get().starts_with("/test");
    let is_settings = move || location.pathname.get().starts_with("/settings");
    view! {
        <div class="app-page-shell">
            <RadrootsAppUiScrollContainer
                id=None
                classes=Some("app-page app-page-scroll app-page-chrome".to_string())
                collapse_range=None
                context=Some(scroll_context.clone())
            >
                <RadrootsAppUiNavHeader
                    label=title
                    on_label_click=None
                    bg_mode=Some(bg_mode)
                    collapse_mode=Some(collapse_mode)
                    right=header_right
                    id=None
                    class=None
                />
                <div class="app-page-body">
                    {children()}
                </div>
            </RadrootsAppUiScrollContainer>
            {move || {
                if show_tabs {
                    view! {
                        <RadrootsAppUiNavTabs>
                            <A
                                href="/"
                                attr:class="nav-tabs__item"
                                attr:data-active=move || if is_home() { "true" } else { "false" }
                                attr:aria-label=t!("app.nav.home")
                            >
                                <RadrootsAppUiIcon key=RadrootsAppUiIconKey::Home size=22 />
                            </A>
                            <A
                                href="/test"
                                attr:class="nav-tabs__item"
                                attr:data-active=move || if is_test() { "true" } else { "false" }
                                attr:aria-label=t!("app.nav.ui")
                            >
                                <RadrootsAppUiIcon key=RadrootsAppUiIconKey::Beaker size=22 />
                            </A>
                            <A
                                href="/settings"
                                attr:class="nav-tabs__item"
                                attr:data-active=move || if is_settings() { "true" } else { "false" }
                                attr:aria-label=t!("app.nav.settings")
                            >
                                <RadrootsAppUiIcon key=RadrootsAppUiIconKey::Settings size=22 />
                            </A>
                        </RadrootsAppUiNavTabs>
                    }
                    .into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
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

fn setup_touch_callback(action: &'static str) -> Callback<MouseEvent> {
    Callback::new(move |_| {
        let _ = app_log_debug_emit("log.app.setup.choice", action, None);
    })
}

fn log_init_stage(stage: RadrootsAppInitStage) {
    let _ = app_log_debug_emit("log.app.init.stage", stage.as_str(), None);
}

fn logs_datastore() -> radroots_studio_app_core::datastore::RadrootsClientWebDatastore {
    radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(Some(IDB_CONFIG_LOGS))
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
    let fallback_setup_status = RwSignal::new_local(RadrootsAppSetupStatus::Unknown);
    let setup_status = context
        .as_ref()
        .map(|value| value.setup_status)
        .unwrap_or(fallback_setup_status);
    let navigate = use_navigate();
    let navigate_guard = navigate.clone();
    let navigate_home = navigate.clone();
    let setup_step = RwSignal::new_local(app_setup_step_default());
    let setup_key_choice = RwSignal::new_local(None::<RadrootsAppSetupKeyChoice>);
    let setup_farmer_choice = RwSignal::new_local(None::<RadrootsAppSetupFarmerChoice>);
    let setup_business_choice = RwSignal::new_local(None::<RadrootsAppSetupBusinessChoice>);
    let setup_eula_scrolled = RwSignal::new_local(false);
    let setup_eula_scroll_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let nostr_key_add = RwSignal::new_local(String::new());
    let profile_name = RwSignal::new_local(String::new());
    let profile_nip05 = RwSignal::new_local(true);
    let setup_draft_loaded = RwSignal::new_local(false);
    let setup_lock_owner = RwSignal::new_local(Uuid::new_v4().to_string());
    let setup_lock_status = RwSignal::new_local(None::<RadrootsAppSetupLockStatus>);
    let setup_lock_attempted = RwSignal::new_local(false);
    let setup_flow = move || RadrootsAppSetupFlowDraft {
        step: setup_step.get(),
        key_choice: setup_key_choice.get(),
        farmer_choice: setup_farmer_choice.get(),
        business_choice: setup_business_choice.get(),
        profile_name: profile_name.get(),
        profile_nip05: profile_nip05.get(),
    };
    let setup_validation = move || app_setup_flow_validate(&setup_flow());
    let setup_lock_ready = move || {
        !app_setup_lock_enabled()
            || matches!(
                setup_lock_status.get(),
                Some(RadrootsAppSetupLockStatus::Acquired(_))
            )
    };
    let setup_locked = move || {
        matches!(
            setup_lock_status.get(),
            Some(RadrootsAppSetupLockStatus::Locked(_))
        )
    };
    let setup_lock_pending = move || app_setup_lock_enabled() && setup_lock_status.get().is_none();
    let retry_setup_lock: Callback<MouseEvent> = {
        let setup_lock_status = setup_lock_status.clone();
        let setup_lock_attempted = setup_lock_attempted.clone();
        let setup_lock_owner = setup_lock_owner.clone();
        Callback::new(move |_| {
            setup_lock_status.set(None);
            setup_lock_attempted.set(false);
            setup_lock_owner.set(Uuid::new_v4().to_string());
        })
    };
    let on_generate_key = setup_touch_callback("generate_key");
    let on_add_key = setup_touch_callback("add_key");
    Effect::new(move || {
        match setup_status.get() {
            RadrootsAppSetupStatus::Configured => {
                navigate_guard("/", Default::default());
            }
            RadrootsAppSetupStatus::Corrupt => {
                navigate_guard("/recovery", Default::default());
            }
            _ => {}
        }
    });
    Effect::new({
        let backends = backends.clone();
        let setup_lock_status = setup_lock_status.clone();
        let setup_lock_attempted = setup_lock_attempted.clone();
        let setup_lock_owner = setup_lock_owner.clone();
        move |_| {
            if setup_lock_attempted.get() {
                return;
            }
            if !app_setup_lock_enabled() {
                setup_lock_attempted.set(true);
                return;
            }
            let Some((datastore, key_maps)) = backends
                .with(|value| value.as_ref().map(|backends| (backends.datastore.clone(), backends.config.datastore.key_maps.clone())))
            else {
                return;
            };
            setup_lock_attempted.set(true);
            let owner = setup_lock_owner.get();
            let setup_lock_status = setup_lock_status.clone();
            spawn_local(async move {
                let now_ms = u64::try_from(app_state_timestamp_ms()).unwrap_or(0);
                let ttl_ms = app_setup_lock_ttl_ms();
                match app_setup_lock_acquire(
                    datastore.as_ref(),
                    &key_maps,
                    &owner,
                    now_ms,
                    ttl_ms,
                )
                .await
                {
                    Ok(status) => setup_lock_status.set(Some(status)),
                    Err(err) => {
                        let _ = app_log_error_emit(&err);
                        let fallback = RadrootsAppSetupLock {
                            owner,
                            expires_at_ms: now_ms.saturating_add(ttl_ms),
                        };
                        setup_lock_status.set(Some(RadrootsAppSetupLockStatus::Acquired(fallback)));
                    }
                }
            });
        }
    });
    Effect::new({
        let backends = backends.clone();
        let setup_draft_loaded = setup_draft_loaded.clone();
        let setup_lock_status = setup_lock_status.clone();
        let setup_key_choice = setup_key_choice.clone();
        let setup_farmer_choice = setup_farmer_choice.clone();
        let setup_business_choice = setup_business_choice.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let profile_nip05 = profile_nip05.clone();
        move |_| {
            if app_setup_lock_enabled()
                && !matches!(
                    setup_lock_status.get(),
                    Some(RadrootsAppSetupLockStatus::Acquired(_))
                )
            {
                return;
            }
            if setup_draft_loaded.get() {
                return;
            }
            let Some((datastore, key_maps)) = backends
                .with(|value| value.as_ref().map(|backends| (backends.datastore.clone(), backends.config.datastore.key_maps.clone())))
            else {
                return;
            };
            setup_draft_loaded.set(true);
            setup_key_choice.set(None);
            setup_farmer_choice.set(None);
            setup_business_choice.set(None);
            nostr_key_add.set(String::new());
            profile_name.set(String::new());
            profile_nip05.set(true);
            spawn_local(async move {
                let _ = app_datastore_clear_setup_draft(datastore.as_ref(), &key_maps).await;
            });
        }
    });
    Effect::new({
        let backends = backends.clone();
        let setup_draft_loaded = setup_draft_loaded.clone();
        let setup_lock_status = setup_lock_status.clone();
        let setup_key_choice = setup_key_choice.clone();
        let setup_farmer_choice = setup_farmer_choice.clone();
        let setup_business_choice = setup_business_choice.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let profile_nip05 = profile_nip05.clone();
        move |_| {
            if app_setup_lock_enabled()
                && !matches!(
                    setup_lock_status.get(),
                    Some(RadrootsAppSetupLockStatus::Acquired(_))
                )
            {
                return;
            }
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
            let role = app_setup_flow_role_from_choices(
                setup_farmer_choice.get(),
                setup_business_choice.get(),
            );
            let draft = RadrootsAppSetupDraft {
                nostr_public_key,
                profile_name,
                role,
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
        let setup_farmer_choice = setup_farmer_choice.clone();
        let setup_business_choice = setup_business_choice.clone();
        let setup_lock_status = setup_lock_status.clone();
        let nostr_key_add = nostr_key_add.clone();
        let profile_name = profile_name.clone();
        let setup_status = setup_status.clone();
        Callback::new(move |_| {
            if app_setup_lock_enabled()
                && !matches!(
                    setup_lock_status.get(),
                    Some(RadrootsAppSetupLockStatus::Acquired(_))
                )
            {
                return;
            }
            let draft = RadrootsAppSetupFlowDraft {
                step: setup_step.get(),
                key_choice: setup_key_choice.get(),
                farmer_choice: setup_farmer_choice.get(),
                business_choice: setup_business_choice.get(),
                profile_name: profile_name.get(),
                profile_nip05: profile_nip05.get(),
            };
            let validation = app_setup_flow_validate(&draft);
            let current_step = draft.step;
            if matches!(current_step, RadrootsAppSetupStep::Eula) {
                let key_choice = draft.key_choice;
                let setup_role = app_setup_flow_role_from_choices(
                    setup_farmer_choice.get(),
                    setup_business_choice.get(),
                )
                .unwrap_or_else(RadrootsAppRole::default);
                let nostr_key_add = nostr_key_add.get();
                let profile_name = draft.profile_name;
                let profile_nip05 = draft.profile_nip05;
                let eula_date = app_setup_eula_date();
                let setup_status = setup_status.clone();
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
                        setup_role,
                    )
                    .await
                    {
                        let _ = app_log_error_emit(&err);
                        return;
                    }
                    let _ = app_datastore_clear_setup_draft(datastore.as_ref(), &key_maps).await;
                    if app_setup_lock_enabled() {
                        let _ = app_setup_lock_release(datastore.as_ref(), &key_maps).await;
                    }
                    setup_status.set(RadrootsAppSetupStatus::Configured);
                });
                return;
            }
            if !validation.can_continue {
                return;
            }
            if matches!(current_step, RadrootsAppSetupStep::Profile) {
                if draft.profile_name.trim().is_empty() {
                    let setup_step = setup_step.clone();
                    let confirm_message = t!("app.setup.profile.confirm_no_name");
                    let next_step = validation.next_step;
                    spawn_local(async move {
                        let notifications = RadrootsAppNotifications::new(None);
                        let confirm = notifications.confirm_message(&confirm_message).await;
                        if confirm {
                            setup_step.set(next_step);
                        }
                    });
                    return;
                }
            }
            setup_step.set(validation.next_step);
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
        let setup_farmer_choice = setup_farmer_choice.clone();
        let setup_business_choice = setup_business_choice.clone();
        let setup_lock_status = setup_lock_status.clone();
        let profile_name = profile_name.clone();
        let profile_nip05 = profile_nip05.clone();
        Callback::new(move |_| {
            if app_setup_lock_enabled()
                && !matches!(
                    setup_lock_status.get(),
                    Some(RadrootsAppSetupLockStatus::Acquired(_))
                )
            {
                return;
            }
            let draft = RadrootsAppSetupFlowDraft {
                step: setup_step.get(),
                key_choice: setup_key_choice.get(),
                farmer_choice: setup_farmer_choice.get(),
                business_choice: setup_business_choice.get(),
                profile_name: profile_name.get(),
                profile_nip05: profile_nip05.get(),
            };
            let validation = app_setup_flow_validate(&draft);
            if !validation.can_back {
                return;
            }
            let prev_step = validation.prev_step;
            setup_step.set(prev_step);
            if matches!(prev_step, RadrootsAppSetupStep::Intro) {
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
    Effect::new({
        let setup_step = setup_step.clone();
        let setup_eula_scrolled = setup_eula_scrolled.clone();
        let setup_eula_scroll_ref = setup_eula_scroll_ref.clone();
        move |_| {
            if !matches!(setup_step.get(), RadrootsAppSetupStep::Eula) {
                return;
            }
            let Some(target) = setup_eula_scroll_ref.get() else {
                return;
            };
            if target.scroll_height() <= target.client_height() {
                setup_eula_scrolled.set(true);
            }
        }
    });
    view! {
        <main
            id="app-setup"
            class="app-page app-page-fixed relative w-full flex flex-col"
        >
            {move || {
                if setup_lock_pending() {
                    return view! {
                        <section
                            id="app-setup-lock-pending"
                            class="app-view app-view-enter flex flex-col h-[100dvh] w-full px-6 pt-10 pb-16"
                        >
                            <div
                                id="app-setup-lock-pending-body"
                                class="flex flex-1 w-full flex-col justify-center items-center gap-4"
                            >
                                <RadrootsAppUiSpinner class="text-[24px]".to_string() />
                                <p class="font-sans font-[600] text-ly0-gl text-2xl text-center">
                                    {t!("app.setup.lock.pending.title")}
                                </p>
                                <p class="font-mono font-[400] text-ly0-gl text-base text-center">
                                    {t!("app.setup.lock.pending.body")}
                                </p>
                            </div>
                        </section>
                    }
                    .into_any();
                }
                if setup_locked() {
                    return view! {
                        <section
                            id="app-setup-lock"
                            class="app-view app-view-enter flex flex-col h-[100dvh] w-full px-6 pt-10 pb-16"
                        >
                            <div
                                id="app-setup-lock-body"
                                class="flex flex-1 w-full flex-col justify-center items-center gap-4"
                            >
                                <p class="font-sans font-[600] text-ly0-gl text-2xl text-center">
                                    {t!("app.setup.locked.title")}
                                </p>
                                <p class="font-mono font-[400] text-ly0-gl text-base text-center">
                                    {t!("app.setup.locked.body")}
                                </p>
                            </div>
                            <div
                                id="app-setup-lock-actions"
                                class="flex flex-col w-full pt-4 justify-center items-center"
                            >
                                {{
                                    let retry_action = RadrootsAppUiButtonLayoutAction {
                                        label: t!("app.setup.lock.retry"),
                                        disabled: false,
                                        loading: false,
                                        on_click: retry_setup_lock.clone(),
                                        class: None,
                                        class_label: None,
                                        style: None,
                                    };
                                    view! { <RadrootsAppUiButtonLayoutPair continue_action=retry_action class="gap-2".to_string() /> }
                                }}
                            </div>
                        </section>
                    }
                    .into_any();
                }
                match setup_step.get() {
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
                                        class="absolute bottom-0 left-0 flex flex-col h-[20rem] w-full gap-2 justify-start items-center"
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
                RadrootsAppSetupStep::BusinessSetup => view! {
                    <section
                        id="app-setup-business"
                        class="app-view app-view-enter flex flex-col w-full px-6 pt-10 pb-16"
                        on:click=move |_| {
                            setup_business_choice.set(None);
                        }
                    >
                        <div
                            id="app-setup-business-body"
                            class="flex flex-1 w-full flex-col justify-center items-center"
                        >
                            <div
                                id="app-setup-business-card"
                                class="flex flex-col h-[16rem] w-full gap-10 justify-start items-center"
                            >
                                <div
                                    id="app-setup-business-title"
                                    class="flex flex-row w-full justify-center items-center"
                                >
                                    <p class="font-sans font-[600] text-ly0-gl text-3xl">
                                        {t!("app.setup.business.title")}
                                    </p>
                                </div>
                                <div
                                    id="app-setup-business-actions"
                                    class="flex flex-col w-full gap-5 justify-center items-center"
                                >
                                    <button
                                        id="app-setup-business-yes"
                                        type="button"
                                        class=move || {
                                            if setup_business_choice.get()
                                                == Some(RadrootsAppSetupBusinessChoice::Yes)
                                            {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
                                            } else {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                            }
                                        }
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            setup_business_choice.set(Some(RadrootsAppSetupBusinessChoice::Yes));
                                        }
                                    >
                                        <span class="font-sans font-[600] text-ly0-gl text-xl">
                                            {t!("app.common.yes")}
                                        </span>
                                    </button>
                                    <button
                                        id="app-setup-business-no"
                                        type="button"
                                        class=move || {
                                            if setup_business_choice.get()
                                                == Some(RadrootsAppSetupBusinessChoice::No)
                                            {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch ly1-selected-press el-re"
                                            } else {
                                                "flex flex-col h-bold_button w-lo_ios0 ios1:w-lo_ios1 justify-center items-center rounded-touch bg-ly1 el-re"
                                            }
                                        }
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            setup_business_choice.set(Some(RadrootsAppSetupBusinessChoice::No));
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
                                    {t!("app.setup.eula.title")}
                                </p>
                            </header>
                            <div
                                id="app-setup-eula-scroll"
                                class="app-page-scroll scroll-hide flex flex-col flex-1 min-h-0 w-full gap-6 px-1 pb-20 se-compact:pb-12 overscroll-contain font-mono"
                                node_ref=setup_eula_scroll_ref
                                on:scroll=move |ev| {
                                    if setup_eula_scrolled.get() {
                                        return;
                                    }
                                    let target = event_target::<HtmlElement>(&ev);
                                    let scroll_top = target.scroll_top();
                                    let scroll_height = target.scroll_height();
                                    let client_height = target.client_height();
                                    if scroll_top + client_height + 1 >= scroll_height {
                                        setup_eula_scrolled.set(true);
                                    }
                                }
                            >
                                <section
                                    id="app-setup-eula-introduction"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.introduction.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.introduction.body")}
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-prohibited-content"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.prohibited_content.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.prohibited_content.body")}
                                    </p>
                                    <ul class="flex flex-col gap-1 pl-5 list-disc text-sm text-ly0-gl leading-relaxed">
                                        <li>{t!("app.setup.eula.prohibited_content.item.illegal")}</li>
                                        <li>{t!("app.setup.eula.prohibited_content.item.pornographic")}</li>
                                        <li>{t!("app.setup.eula.prohibited_content.item.hate_speech")}</li>
                                        <li>{t!("app.setup.eula.prohibited_content.item.minors")}</li>
                                        <li>{t!("app.setup.eula.prohibited_content.item.harass")}</li>
                                        <li>{t!("app.setup.eula.prohibited_content.item.impersonate")}</li>
                                    </ul>
                                </section>
                                <section
                                    id="app-setup-eula-prohibited-conduct"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.prohibited_conduct.title")}
                                    </h3>
                                    <ul class="flex flex-col gap-1 pl-5 list-disc text-sm text-ly0-gl leading-relaxed">
                                        <li>{t!("app.setup.eula.prohibited_conduct.item.harass")}</li>
                                        <li>{t!("app.setup.eula.prohibited_conduct.item.impersonate")}</li>
                                        <li>{t!("app.setup.eula.prohibited_conduct.item.intimidate")}</li>
                                        <li>{t!("app.setup.eula.prohibited_conduct.item.violence")}</li>
                                    </ul>
                                </section>
                                <section
                                    id="app-setup-eula-consequences"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.consequences.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.consequences.body")}
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-disclaimer"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.disclaimer.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.disclaimer.body")}
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-changes"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.changes.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.changes.body")}
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-contact"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.contact.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.contact.body")}
                                    </p>
                                </section>
                                <section
                                    id="app-setup-eula-acceptance"
                                    class="flex flex-col gap-2"
                                >
                                    <h3 class="font-sans font-[600] text-ly0-gl text-base">
                                        {t!("app.setup.eula.acceptance.title")}
                                    </h3>
                                    <p class="font-mono font-[400] text-ly0-gl text-sm leading-relaxed">
                                        {t!("app.setup.eula.acceptance.body")}
                                    </p>
                                </section>
                            </div>
                        </div>
                        <div
                            id="app-setup-eula-actions"
                            class="flex flex-col w-full pt-4 pb-2 justify-center items-center"
                        >
                            {move || {
                                let continue_action = RadrootsAppUiButtonLayoutAction {
                                    label: t!("app.common.agree"),
                                    disabled: !setup_eula_scrolled.get(),
                                    loading: false,
                                    on_click: advance_step_click.clone(),
                                    class: Some("button-layout-accent button-layout-compact".to_string()),
                                    class_label: Some("text-base".to_string()),
                                    style: None,
                                };
                                let back_action = RadrootsAppUiButtonLayoutBackAction {
                                    visible: true,
                                    label: Some(t!("app.common.disagree")),
                                    disabled: false,
                                    on_click: rewind_step.clone(),
                                    compact: true,
                                };
                                view! {
                                    <RadrootsAppUiButtonLayoutPair
                                        continue_action=continue_action
                                        back=back_action
                                        class="gap-2".to_string()
                                    />
                                }.into_any()
                            }}
                        </div>
                    </section>
                }.into_any(),
                }
            }}
            <footer
                id="app-setup-actions"
                class="z-10 absolute bottom-4 left-0 flex flex-col w-full justify-center items-center se-compact:bottom-0"
            >
                {move || {
                    if !setup_lock_ready() {
                        return view! { <></> }.into_any();
                    }
                    let step = setup_step.get();
                    if matches!(step, RadrootsAppSetupStep::Eula) {
                        return view! { <></> }.into_any();
                    }
                    let validation = setup_validation();
                    let continue_disabled = !validation.can_continue;
                    let continue_label = t!("app.common.continue");
                    let back_label = t!("app.common.back");
                    let continue_action = RadrootsAppUiButtonLayoutAction {
                        label: continue_label,
                        disabled: continue_disabled,
                        loading: false,
                        on_click: advance_step_click.clone(),
                        class: None,
                        class_label: None,
                        style: None,
                    };
                    let back_action = RadrootsAppUiButtonLayoutBackAction {
                        visible: validation.can_back,
                        label: Some(back_label),
                        disabled: false,
                        on_click: rewind_step.clone(),
                        compact: false,
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
fn RecoveryPage() -> impl IntoView {
    let context = app_context();
    let fallback_backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let fallback_setup_status = RwSignal::new_local(RadrootsAppSetupStatus::Unknown);
    let backends = context
        .as_ref()
        .map(|value| value.backends)
        .unwrap_or(fallback_backends);
    let setup_status = context
        .as_ref()
        .map(|value| value.setup_status)
        .unwrap_or(fallback_setup_status);
    let reset_running = RwSignal::new_local(false);
    let reset_status = RwSignal::new_local(None::<String>);
    let navigate = use_navigate();
    let reset_disabled = move || backends.with(|value| value.is_none()) || reset_running.get();
    let reset_label = move || reset_status.get().as_deref().map(reset_status_label);
    let on_reset: Callback<MouseEvent> = {
        let backends = backends.clone();
        let reset_running = reset_running.clone();
        let reset_status = reset_status.clone();
        let setup_status = setup_status.clone();
        let navigate = navigate.clone();
        Callback::new(move |_| {
            if reset_running.get() {
                return;
            }
            reset_status.set(None);
            let config = backends
                .with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
            let reset_running = reset_running.clone();
            let reset_status = reset_status.clone();
            let setup_status = setup_status.clone();
            let navigate = navigate.clone();
            spawn_local(async move {
                let Some(config) = config else {
                    reset_status.set(Some("reset_missing_backends".to_string()));
                    return;
                };
                let notifications = RadrootsAppNotifications::new(None);
                let confirm_message = t!("app.recovery.reset.confirm");
                let confirm = notifications.confirm_message(&confirm_message).await;
                if !confirm {
                    return;
                }
                reset_running.set(true);
                reset_status.set(Some("resetting".to_string()));
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
                            reset_running.set(false);
                            return;
                        }
                        reset_status.set(Some("reset_done".to_string()));
                        setup_status.set(RadrootsAppSetupStatus::Required);
                        navigate("/setup", Default::default());
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
                reset_running.set(false);
            });
        })
    };
    view! {
        <main id="app-recovery" class="app-page app-page-fixed relative w-full flex flex-col">
            <section
                id="app-recovery-view"
                class="app-view app-view-enter flex flex-col h-[100dvh] w-full px-6 pt-10 pb-16"
            >
                <div
                    id="app-recovery-body"
                    class="flex flex-1 w-full flex-col justify-center items-center gap-4"
                >
                    <p class="font-sans font-[600] text-ly0-gl text-2xl text-center">
                        {t!("app.recovery.title")}
                    </p>
                    <p class="font-mono font-[400] text-ly0-gl text-base text-center">
                        {t!("app.recovery.body")}
                    </p>
                    {move || {
                        reset_label()
                            .map(|label| {
                                view! {
                                    <p class="font-mono font-[400] text-ly0-gl text-sm text-center">
                                        {label}
                                    </p>
                                }
                                .into_any()
                            })
                            .unwrap_or_else(|| view! { <></> }.into_any())
                    }}
                </div>
                <div
                    id="app-recovery-actions"
                    class="flex flex-col w-full pt-6 justify-center items-center"
                >
                    {move || {
                        let reset_action = RadrootsAppUiButtonLayoutAction {
                            label: t!("app.recovery.reset.button"),
                            disabled: reset_disabled(),
                            loading: reset_running.get(),
                            on_click: on_reset.clone(),
                            class: None,
                            class_label: None,
                            style: None,
                        };
                        view! { <RadrootsAppUiButtonLayoutPair continue_action=reset_action class="gap-2".to_string() /> }
                    }}
                </div>
            </section>
        </main>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let current_view = RwSignal::new_local(HomeView::Activity);
    let is_activity = move || matches!(current_view.get(), HomeView::Activity);
    let is_profile = move || matches!(current_view.get(), HomeView::Profile);
    view! {
        <AppPageChrome title=t!("app.nav.home")>
            <section
                id="app-home"
                class="flex flex-col items-center justify-start gap-6 pt-6"
            >
                <div
                    id="app-home-toggle"
                    class="home-toggle"
                    class:home-toggle--left=is_activity
                    class:home-toggle--right=is_profile
                    role="tablist"
                    aria-label="Home view"
                >
                    <div class="home-toggle__indicator" aria-hidden="true"></div>
                    <button
                        id="app-home-toggle-activity"
                        class="home-toggle__button"
                        class:is-active=is_activity
                        type="button"
                        role="tab"
                        aria-selected=move || if is_activity() { "true" } else { "false" }
                        on:click=move |_| current_view.set(HomeView::Activity)
                    >
                        {"Activity"}
                    </button>
                    <button
                        id="app-home-toggle-profile"
                        class="home-toggle__button"
                        class:is-active=is_profile
                        type="button"
                        role="tab"
                        aria-selected=move || if is_profile() { "true" } else { "false" }
                        on:click=move |_| current_view.set(HomeView::Profile)
                    >
                        {"Profile"}
                    </button>
                </div>
                <p id="app-home-view-title" class="home-toggle__title">
                    {move || current_view.get().label()}
                </p>
            </section>
        </AppPageChrome>
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
    let setup_status = RwSignal::new_local(RadrootsAppSetupStatus::Unknown);
    let navigate = use_navigate();
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    provide_context(setup_status);
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
                    let setup_status = setup_status.clone();
                    spawn_local(async move {
                        let keystore = radroots_studio_app_core::keystore::RadrootsClientWebKeystoreNostr::new(
                            Some(keystore_config),
                        );
                        match app_init_setup_status(datastore.as_ref(), &keystore, &key_maps).await {
                            Ok(status) => {
                                setup_status.set(status);
                                match status {
                                    RadrootsAppSetupStatus::Required | RadrootsAppSetupStatus::Locked => {
                                        navigate("/setup", Default::default());
                                    }
                                    RadrootsAppSetupStatus::Corrupt => {
                                        navigate("/recovery", Default::default());
                                    }
                                    _ => {}
                                }
                            }
                            Err(err) => {
                                let _ = app_log_error_emit(&err);
                                setup_status.set(RadrootsAppSetupStatus::Corrupt);
                                navigate("/recovery", Default::default());
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
    let setup_gate = move || app_setup_gate_from_status(setup_status.get());
    view! {
        <Show
            when=move || {
                init_state.get().stage == RadrootsAppInitStage::Ready
                    && !matches!(setup_status.get(), RadrootsAppSetupStatus::Unknown)
            }
            fallback=|| view! { <SplashPage /> }
        >
            {move || {
                let gate = setup_gate();
                if gate.show_recovery {
                    return view! { <RecoveryPage /> }.into_any();
                }
                if gate.show_setup {
                    return view! { <SetupPage /> }.into_any();
                }
                if gate.show_app {
                    return view! {
                        <div id="app-shell">
                            <Routes
                                fallback=|| view! {
                                    <main id="app-not-found" class="app-page app-page-fixed">
                                        <p id="app-not-found-label">{t!("app.not_found")}</p>
                                    </main>
                                }
                            >
                                <Route path=path!("") view=HomePage />
                                <Route path=path!("settings/logs") view=RadrootsAppLogsPage />
                                <Route path=path!("test") view=RadrootsAppUiDemoPage />
                                <Route
                                    path=path!("settings/status")
                                    view=RadrootsAppSettingsStatusPage
                                />
                                <Route path=path!("settings") view=RadrootsAppSettingsPage />
                            </Routes>
                        </div>
                    }
                    .into_any();
                }
                view! { <SplashPage /> }.into_any()
            }}
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use crate::app_health_check_delay_ms;

    #[test]
    fn health_check_delay_is_positive() {
        assert!(app_health_check_delay_ms() > 0);
    }
}
