use gpui::{Application, rgb};
use gpui_component::{Theme, ThemeMode};
use radroots_studio_app_core::{
    APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeConfig, AppRuntimeConfigError,
    AppRuntimeSnapshot, bootstrap_logging, install_panic_hook, launch_startup_event,
};
use radroots_studio_app_i18n::select_locale_from_host;
use radroots_studio_app_ui::APP_UI_THEME;
use thiserror::Error;
use tracing::{error, info};

use crate::menus::install_native_app_menu;
use crate::runtime::{DesktopAppRuntime, DesktopAppRuntimeSummary};
use crate::window::{home_window_options, open_home_window};

#[derive(Debug, Error)]
pub enum AppLaunchError {
    #[error(transparent)]
    RuntimeConfig(#[from] AppRuntimeConfigError),
    #[error(transparent)]
    Logging(#[from] radroots_studio_app_core::AppLoggingError),
}

pub fn launch() -> Result<(), AppLaunchError> {
    let build = build_identity();
    let runtime_config = AppRuntimeConfig::from_env()?;
    let snapshot = AppRuntimeSnapshot::capture_for_mode(build, runtime_config.runtime_mode);
    bootstrap_logging(&snapshot, runtime_config.local_log_root.as_path())?;
    install_panic_hook();

    let runtime =
        DesktopAppRuntime::bootstrap(runtime_config.nostr_relay_urls.clone(), snapshot.clone());
    if let Err(error) = runtime.sync_on_app_launch() {
        error!(
            target: "sync",
            event = "sync.launch_attempt_failed",
            error = %error,
            "failed to execute launch sync attempt"
        );
    }
    let runtime_summary = runtime.summary();
    emit_runtime_events(&snapshot, &runtime_summary);

    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        Theme::change(ThemeMode::Light, None, cx);
        Theme::global_mut(cx).ring = rgb(APP_UI_THEME.components.app_input_text.border).into();
        select_locale_from_host(&snapshot.host.host_locale);
        install_native_app_menu(runtime.clone(), cx);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let snapshot = snapshot.clone();
        let runtime = runtime.clone();
        let mut primary_window_options = home_window_options(cx);
        primary_window_options.app_id = Some(snapshot.host.app_identifier.clone());
        cx.spawn(async move |cx| {
            let open_result = cx.open_window(primary_window_options, |window, cx| {
                window.activate_window();
                open_home_window(window, cx, runtime.clone())
            });

            if let Err(error) = open_result {
                error!(
                    target: "window",
                    event = "window.primary_open_failed",
                    error = %error,
                    "failed to open primary window"
                );
                let _ = cx.update(|cx| cx.quit());
                return;
            }

            info!(
                target: "window",
                event = "window.primary_opened",
                app_id = %snapshot.host.app_identifier,
                "primary window opened"
            );

            if let Err(error) = cx.update(|cx| cx.activate(true)) {
                error!(
                    target: "window",
                    event = "window.app_activation_failed",
                    error = %error,
                    "failed to activate app"
                );
            }
        })
        .detach();
    });

    Ok(())
}

fn build_identity() -> AppBuildIdentity {
    AppBuildIdentity {
        package_name: env!("CARGO_PKG_NAME").to_owned(),
        package_version: env!("CARGO_PKG_VERSION").to_owned(),
        build_profile: option_env!("PROFILE").unwrap_or("debug").to_owned(),
        target_triple: option_env!("TARGET").unwrap_or("unknown-target").to_owned(),
        projection_source: APP_PROJECTION_SOURCE.to_owned(),
        git_commit: option_env!("RADROOTS_GIT_COMMIT").map(str::to_owned),
    }
}

fn emit_launch_event(snapshot: &AppRuntimeSnapshot) {
    let launch_event = launch_startup_event(snapshot);
    info!(
        target: "bootstrap",
        event = launch_event.name,
        home_screen = %launch_event.metadata.home_screen,
        core_package = %launch_event.metadata.core_package,
        host_surface = %launch_event.metadata.host_surface,
        runtime_mode = %launch_event.metadata.runtime_mode,
        "{}",
        launch_event.message
    );
}

fn emit_runtime_events(snapshot: &AppRuntimeSnapshot, runtime: &DesktopAppRuntimeSummary) {
    emit_launch_event(snapshot);

    if let Some(startup_issue) = runtime.startup_issue.as_deref() {
        emit_degraded_runtime_event(startup_issue);
    }
}

fn emit_degraded_runtime_event(startup_issue: &str) {
    error!(
        target: "runtime",
        event = "runtime.degraded",
        startup_issue = %startup_issue,
        "desktop runtime degraded"
    );
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use radroots_studio_app_core::{
        APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeCapture, AppRuntimeMode,
        AppRuntimeSnapshot,
    };
    use radroots_studio_app_state::{AppShellProjection, FarmWorkspaceReadinessProjection, HomeRoute};
    use radroots_studio_app_view::{
        AppStartupGate, LoggedOutStartupProjection, SettingsAccountProjection,
        TodayAgendaProjection,
    };
    use tracing::{
        Event, Level, Subscriber,
        field::{Field, Visit},
    };
    use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

    use crate::runtime::DesktopAppRuntimeSummary;

    use super::emit_runtime_events;
    use crate::window::{HomeStage, PrimaryWindowTarget, home_stage, primary_window_target};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct CapturedEvent {
        level: Level,
        target: String,
        event: Option<String>,
        message: Option<String>,
        startup_issue: Option<String>,
    }

    #[derive(Default)]
    struct EventFieldVisitor {
        event: Option<String>,
        message: Option<String>,
        startup_issue: Option<String>,
    }

    struct CaptureLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    impl EventFieldVisitor {
        fn record_value(&mut self, field: &Field, value: String) {
            match field.name() {
                "event" => self.event = Some(value),
                "message" => self.message = Some(value),
                "startup_issue" => self.startup_issue = Some(value),
                _ => {}
            }
        }
    }

    impl Visit for EventFieldVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.record_value(field, value.to_owned());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.record_value(field, format!("{value:?}").trim_matches('"').to_owned());
        }
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = EventFieldVisitor::default();
            event.record(&mut visitor);
            self.events
                .lock()
                .expect("capture lock")
                .push(CapturedEvent {
                    level: *event.metadata().level(),
                    target: event.metadata().target().to_owned(),
                    event: visitor.event,
                    message: visitor.message,
                    startup_issue: visitor.startup_issue,
                });
        }
    }

    fn test_snapshot() -> AppRuntimeSnapshot {
        AppRuntimeSnapshot::from_capture(
            AppBuildIdentity {
                package_name: "radroots_studio_app".to_owned(),
                package_version: "0.1.0".to_owned(),
                build_profile: "debug".to_owned(),
                target_triple: "aarch64-apple-darwin".to_owned(),
                projection_source: APP_PROJECTION_SOURCE.to_owned(),
                git_commit: None,
            },
            AppRuntimeMode::LocalhostDev,
            AppRuntimeCapture {
                host_locale: "en_US.UTF-8".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "app-localhost-dev-20260418T000000Z-deadbeefcafefeed".to_owned(),
            },
        )
    }

    fn summary_with_gate(
        startup_gate: AppStartupGate,
        home_route: HomeRoute,
        startup_issue: Option<&str>,
    ) -> DesktopAppRuntimeSummary {
        DesktopAppRuntimeSummary {
            shell_projection: AppShellProjection::default(),
            settings_account_projection: SettingsAccountProjection::default(),
            startup_gate,
            home_route,
            personal_projection: Default::default(),
            farm_setup_projection: Default::default(),
            farm_rules_projection: Default::default(),
            farm_readiness_projection: FarmWorkspaceReadinessProjection::default(),
            today_projection: TodayAgendaProjection::default(),
            products_projection: Default::default(),
            orders_projection: Default::default(),
            pack_day_projection: Default::default(),
            reminder_log: Default::default(),
            runtime_metadata: crate::runtime::DesktopAppRuntimeMetadataSummary::default(),
            logged_out_startup: LoggedOutStartupProjection::default(),
            sync_status: crate::runtime::DesktopAppSyncStatusSummary::default(),
            startup_issue: startup_issue.map(str::to_owned),
        }
    }

    #[test]
    fn degraded_runtime_emits_launch_and_degraded_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(CaptureLayer {
            events: Arc::clone(&events),
        });
        let summary = summary_with_gate(
            AppStartupGate::SetupRequired,
            HomeRoute::SetupRequired,
            Some("desktop runtime roots require HOME for macos"),
        );

        tracing::subscriber::with_default(subscriber, || {
            emit_runtime_events(&test_snapshot(), &summary);
        });

        let events = events.lock().expect("events lock");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.as_deref(), Some("runtime.launch"));
        assert_eq!(events[0].target, "bootstrap");
        assert_eq!(events[1].event.as_deref(), Some("runtime.degraded"));
        assert_eq!(events[1].level, Level::ERROR);
        assert_eq!(events[1].target, "runtime");
        assert_eq!(
            events[1].startup_issue.as_deref(),
            Some("desktop runtime roots require HOME for macos")
        );
        assert_eq!(
            events[1].message.as_deref(),
            Some("desktop runtime degraded")
        );
    }

    #[test]
    fn blocked_and_setup_runtime_target_the_home_window() {
        let blocked = summary_with_gate(AppStartupGate::Blocked, HomeRoute::Blocked, None);
        let setup = summary_with_gate(
            AppStartupGate::SetupRequired,
            HomeRoute::SetupRequired,
            None,
        );

        assert_eq!(primary_window_target(&blocked), PrimaryWindowTarget::Home);
        assert_eq!(primary_window_target(&setup), PrimaryWindowTarget::Home);
    }

    #[test]
    fn ready_runtime_targets_the_home_window() {
        let personal = summary_with_gate(AppStartupGate::Personal, HomeRoute::Personal, None);
        let farmer =
            summary_with_gate(AppStartupGate::Farmer, HomeRoute::FarmSetupOnboarding, None);

        assert_eq!(primary_window_target(&personal), PrimaryWindowTarget::Home);
        assert_eq!(primary_window_target(&farmer), PrimaryWindowTarget::Home);
    }

    #[test]
    fn degraded_runtime_targets_the_home_window() {
        let degraded = summary_with_gate(
            AppStartupGate::Personal,
            HomeRoute::Personal,
            Some("runtime unavailable"),
        );

        assert_eq!(primary_window_target(&degraded), PrimaryWindowTarget::Home);
    }

    #[test]
    fn home_stage_tracks_setup_personal_and_farmer_states() {
        let setup = summary_with_gate(
            AppStartupGate::SetupRequired,
            HomeRoute::SetupRequired,
            None,
        );
        let personal = summary_with_gate(AppStartupGate::Personal, HomeRoute::Personal, None);
        let farmer =
            summary_with_gate(AppStartupGate::Farmer, HomeRoute::FarmSetupOnboarding, None);
        let blocked = summary_with_gate(
            AppStartupGate::Farmer,
            HomeRoute::FarmSetupOnboarding,
            Some("runtime unavailable"),
        );

        assert_eq!(home_stage(&setup), HomeStage::Setup);
        assert_eq!(home_stage(&personal), HomeStage::BuyerWorkspace);
        assert_eq!(home_stage(&farmer), HomeStage::FarmerWorkspace);
        assert_eq!(home_stage(&blocked), HomeStage::Setup);
    }
}
