use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{A, Route, Router, Routes};
use leptos_router::path;

use crate::{
    app_init_assets,
    app_init_backends,
    app_init_has_completed,
    app_init_state_default,
    app_init_mark_completed,
    app_init_reset,
    app_init_progress_add,
    app_init_stage_set,
    app_init_total_add,
    app_init_total_unknown,
    app_log_buffer_flush_critical,
    app_log_debug_emit,
    app_log_error_emit,
    app_log_error_store,
    app_config_default,
    app_datastore_read_state,
    app_state_set_notifications_permission,
    app_health_check_all_logged,
    RadrootsAppBackends,
    RadrootsAppConfig,
    RadrootsAppHealthCheckResult,
    RadrootsAppHealthCheckStatus,
    RadrootsAppHealthReport,
    RadrootsAppInitError,
    RadrootsAppInitStage,
    RadrootsAppNotifications,
    RadrootsAppTangleClientStub,
    LogsPage,
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

fn spawn_health_checks(
    config: RadrootsAppConfig,
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
        let report = app_health_check_all_logged(
            &datastore,
            &keystore,
            &notifications,
            &tangle,
            &config.datastore.key_maps,
        )
        .await;
        let app_data = app_datastore_read_state(&datastore, &config.datastore.key_maps)
            .await
            .ok();
        let active_key_value = app_data.as_ref().and_then(|data| {
            if data.active_key.is_empty() {
                None
            } else {
                Some(data.active_key.clone())
            }
        });
        let notifications_value = app_data.and_then(|data| data.notifications_permission);
        health_report.set(report);
        active_key.set(active_key_value);
        notifications_status.set(notifications_value);
        health_running.set(false);
    });
}

const APP_HEALTH_CHECK_DELAY_MS: u32 = 300;

fn app_health_check_delay_ms() -> u32 {
    APP_HEALTH_CHECK_DELAY_MS
}

#[component]
fn HomePage() -> impl IntoView {
    let backends = RwSignal::new_local(None::<RadrootsAppBackends>);
    let init_error = RwSignal::new_local(None::<RadrootsAppInitError>);
    let init_state = RwSignal::new_local(app_init_state_default());
    let reset_status = RwSignal::new_local(None::<String>);
    let health_report = RwSignal::new_local(RadrootsAppHealthReport::empty());
    let health_running = RwSignal::new_local(false);
    let health_autorun = RwSignal::new_local(false);
    let active_key = RwSignal::new_local(None::<String>);
    let notifications_status = RwSignal::new_local(None::<String>);
    let notifications_requesting = RwSignal::new_local(false);
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    Effect::new(move || {
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
                    let _ = app_log_buffer_flush_critical(
                        &value.datastore,
                        &value.config.datastore.key_maps,
                    )
                    .await;
                    backends.set(Some(value));
                    app_init_mark_completed();
                    let stage = RadrootsAppInitStage::Ready;
                    init_state.update(|state| app_init_stage_set(state, stage));
                    log_init_stage(stage);
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
    Effect::new(move || {
        if init_state.get().stage != RadrootsAppInitStage::Ready {
            return;
        }
        if health_autorun.get() {
            return;
        }
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
    let health_disabled =
        move || backends.with(|value| value.is_none()) || health_running.get();
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
        <main>
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
                                    reset_status.set(Some("reset_done".to_string()));
                                    spawn_health_checks(
                                        config,
                                        health_report,
                                        health_running,
                                        active_key,
                                        notifications_status,
                                    );
                                }
                                Err(err) => {
                                    let _ = app_log_error_store(&datastore, &config.datastore.key_maps, &err).await;
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
                                        let _ = app_state_set_notifications_permission(
                                            &datastore,
                                            &config.datastore.key_maps,
                                            &value,
                                        )
                                        .await;
                                        notifications_status.set(Some(value));
                                        spawn_health_checks(
                                            config,
                                            health_report,
                                            health_running,
                                            active_key,
                                            notifications_status,
                                        );
                                    }
                                    Err(err) => {
                                        let _ = app_log_error_store(
                                            &datastore,
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
                            spawn_health_checks(
                                config,
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
                                health_status_color(health_report.get().bootstrap_config.status)
                            )
                        ></span>
                        <span>"bootstrap_config"</span>
                        <span>{move || health_result_label(&health_report.get().bootstrap_config)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().bootstrap_app_data.status)
                            )
                        ></span>
                        <span>"bootstrap_app_data"</span>
                        <span>{move || health_result_label(&health_report.get().bootstrap_app_data)}</span>
                    </div>
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span
                            style=move || format!(
                                "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                                health_status_color(health_report.get().app_data_active_key.status)
                            )
                        ></span>
                        <span>"app_data_active_key"</span>
                        <span>{move || health_result_label(&health_report.get().app_data_active_key)}</span>
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
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <nav style="display:flex;gap:12px;margin-bottom:12px;">
                <A href="/" exact=true>"home"</A>
                <A href="/logs">"logs"</A>
            </nav>
            <Routes fallback=|| view! { <div>"not_found"</div> }>
                <Route path=path!("") view=HomePage />
                <Route path=path!("logs") view=LogsPage />
            </Routes>
        </Router>
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
