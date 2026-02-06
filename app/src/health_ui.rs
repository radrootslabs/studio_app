#![forbid(unsafe_code)]

use leptos::prelude::{LocalStorage, RwSignal, Set};
use leptos::task::spawn_local;

use crate::{
    app_datastore_read_state,
    app_health_check_all,
    app_log_buffer_flush_deferred,
    app_state_notifications_permission_value,
    app_state_timestamp_ms,
    t,
    RadrootsAppConfig,
    RadrootsAppHealthCheckResult,
    RadrootsAppHealthCheckStatus,
    RadrootsAppHealthReport,
    RadrootsAppNotifications,
    RadrootsAppTangleClientStub,
};
use radroots_studio_app_core::idb::IDB_CONFIG_LOGS;

const APP_HEALTH_CHECK_DELAY_MS: u32 = 300;

pub fn app_health_check_delay_ms() -> u32 {
    APP_HEALTH_CHECK_DELAY_MS
}

pub fn health_status_class(status: RadrootsAppHealthCheckStatus) -> &'static str {
    match status {
        RadrootsAppHealthCheckStatus::Ok => "status-ok",
        RadrootsAppHealthCheckStatus::Error => "status-error",
        RadrootsAppHealthCheckStatus::Skipped => "status-warn",
    }
}

pub fn health_status_label(status: RadrootsAppHealthCheckStatus) -> String {
    match status {
        RadrootsAppHealthCheckStatus::Ok => t!("app.home.health.status.ok"),
        RadrootsAppHealthCheckStatus::Error => t!("app.home.health.status.error"),
        RadrootsAppHealthCheckStatus::Skipped => t!("app.home.health.status.skipped"),
    }
}

pub fn health_message_label(message: &str) -> String {
    match message {
        "missing" => t!("app.home.health.message.missing"),
        "mismatch" => t!("app.home.health.message.mismatch"),
        "uninitialized" => t!("app.home.health.message.uninitialized"),
        "unavailable" => t!("app.home.health.message.unavailable"),
        _ => message.to_string(),
    }
}

pub fn health_result_label(result: &RadrootsAppHealthCheckResult) -> String {
    let status = health_status_label(result.status);
    match result.message.as_deref() {
        Some(message) => format!("{}: {}", status, health_message_label(message)),
        None => status,
    }
}

pub fn health_report_summary(report: &RadrootsAppHealthReport) -> RadrootsAppHealthCheckStatus {
    let statuses = [
        report.key_maps.status,
        report.bootstrap_state.status,
        report.state_active_key.status,
        report.notifications.status,
        report.tangle.status,
        report.datastore_roundtrip.status,
        report.keystore.status,
    ];
    if statuses
        .iter()
        .any(|status| matches!(status, RadrootsAppHealthCheckStatus::Error))
    {
        return RadrootsAppHealthCheckStatus::Error;
    }
    if statuses
        .iter()
        .any(|status| matches!(status, RadrootsAppHealthCheckStatus::Skipped))
    {
        return RadrootsAppHealthCheckStatus::Skipped;
    }
    RadrootsAppHealthCheckStatus::Ok
}

pub fn active_key_label(value: Option<String>) -> String {
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

fn logs_datastore() -> radroots_studio_app_core::datastore::RadrootsClientWebDatastore {
    radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(Some(IDB_CONFIG_LOGS))
}

pub fn spawn_health_checks(
    config: RadrootsAppConfig,
    setup_required: bool,
    health_report: RwSignal<RadrootsAppHealthReport, LocalStorage>,
    health_running: RwSignal<bool, LocalStorage>,
    active_key: RwSignal<Option<String>, LocalStorage>,
    notifications_status: RwSignal<Option<String>, LocalStorage>,
    last_run: RwSignal<Option<i64>, LocalStorage>,
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
        last_run.set(Some(app_state_timestamp_ms()));
        health_running.set(false);
        let key_maps = config.datastore.key_maps.clone();
        spawn_local(async move {
            let log_datastore = logs_datastore();
            let _ = app_log_buffer_flush_deferred(&log_datastore, &key_maps, true).await;
        });
    });
}
