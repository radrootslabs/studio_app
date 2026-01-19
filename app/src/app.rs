use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{
    app_init_backends,
    app_init_state_default,
    app_init_mark_completed,
    app_init_reset,
    app_config_default,
    app_datastore_read_app_data,
    app_health_check_all,
    AppBackends,
    AppConfig,
    AppHealthCheckResult,
    AppHealthCheckStatus,
    AppHealthReport,
    AppInitError,
    AppInitStage,
};

fn health_status_color(status: AppHealthCheckStatus) -> &'static str {
    match status {
        AppHealthCheckStatus::Ok => "green",
        AppHealthCheckStatus::Error => "red",
        AppHealthCheckStatus::Skipped => "gray",
    }
}

fn health_result_label(result: &AppHealthCheckResult) -> String {
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

fn spawn_health_checks(
    config: AppConfig,
    health_report: RwSignal<AppHealthReport, LocalStorage>,
    health_running: RwSignal<bool, LocalStorage>,
    active_key: RwSignal<Option<String>, LocalStorage>,
) {
    health_running.set(true);
    spawn_local(async move {
        let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(
            Some(config.datastore.idb_config),
        );
        let keystore = radroots_studio_app_core::keystore::RadrootsClientWebKeystoreNostr::new(
            Some(config.keystore.nostr_store),
        );
        let report = app_health_check_all(&datastore, &keystore, &config.datastore.key_maps).await;
        let active_key_value = match app_datastore_read_app_data(&datastore, &config.datastore.key_maps).await {
            Ok(data) if data.active_key.is_empty() => None,
            Ok(data) => Some(data.active_key),
            Err(_) => None,
        };
        health_report.set(report);
        active_key.set(active_key_value);
        health_running.set(false);
    });
}

#[component]
pub fn App() -> impl IntoView {
    let backends = RwSignal::new_local(None::<AppBackends>);
    let init_error = RwSignal::new_local(None::<AppInitError>);
    let init_state = RwSignal::new_local(app_init_state_default());
    let reset_status = RwSignal::new_local(None::<String>);
    let health_report = RwSignal::new_local(AppHealthReport::empty());
    let health_running = RwSignal::new_local(false);
    let health_autorun = RwSignal::new_local(false);
    let active_key = RwSignal::new_local(None::<String>);
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    Effect::new(move || {
        spawn_local(async move {
            init_state.update(|state| state.stage = AppInitStage::Storage);
            match app_init_backends(app_config_default()).await {
                Ok(value) => {
                    backends.set(Some(value));
                    app_init_mark_completed();
                    init_state.update(|state| state.stage = AppInitStage::Ready);
                }
                Err(err) => {
                    init_error.set(Some(err));
                    init_state.update(|state| state.stage = AppInitStage::Error);
                }
            }
        })
    });
    Effect::new(move || {
        if init_state.get().stage != AppInitStage::Ready {
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
        spawn_health_checks(config, health_report, health_running, active_key);
    });
    let status_color = move || match init_state.get().stage {
        AppInitStage::Ready => "green",
        AppInitStage::Error => "red",
        AppInitStage::Storage => "orange",
        AppInitStage::DownloadSql => "orange",
        AppInitStage::DownloadGeo => "orange",
        AppInitStage::Database => "orange",
        AppInitStage::Geocoder => "orange",
        AppInitStage::Idle => "gray",
    };
    let reset_disabled = move || backends.with(|value| value.is_none());
    let reset_label = move || {
        reset_status
            .get()
            .unwrap_or_else(|| "reset_idle".to_string())
    };
    let health_disabled = move || {
        backends.with(|value| value.is_none()) || health_running.get()
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
                        health_report.set(AppHealthReport::empty());
                        active_key.set(None);
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
                                    spawn_health_checks(config, health_report, health_running, active_key);
                                }
                                Err(err) => reset_status.set(Some(err.to_string())),
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
                <div style="font-weight: 600;">"health checks"</div>
                <div style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                    <button
                        on:click=move |_| {
                            let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                            let Some(config) = config else {
                                return;
                            };
                            spawn_health_checks(config, health_report, health_running, active_key);
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
