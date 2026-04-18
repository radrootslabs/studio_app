use gpui::{Application, WindowOptions, px, size};
use radroots_studio_app_core::{
    APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeConfig, AppRuntimeConfigError,
    AppRuntimeSnapshot, bootstrap_logging, install_panic_hook, launch_startup_event,
};
use radroots_studio_app_i18n::select_locale_from_host;
use radroots_studio_app_ui::APP_UI_THEME;
use thiserror::Error;
use tracing::{error, info};

use crate::menus::install_native_app_menu;
use crate::runtime::DesktopAppRuntime;
use crate::window::{home_titlebar_options, open_home_window};

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
    let snapshot = AppRuntimeSnapshot::from_config(build, &runtime_config);
    bootstrap_logging(&snapshot, runtime_config.local_log_root.as_path())?;
    install_panic_hook();
    emit_launch_event(&snapshot);

    let runtime = DesktopAppRuntime::bootstrap();
    if let Some(startup_issue) = runtime.summary().startup_issue {
        error!(
            target: "runtime",
            event = "runtime.degraded",
            startup_issue = %startup_issue,
            "desktop runtime degraded"
        );
    }

    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
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
        cx.spawn(async move |cx| {
            if let Err(error) = cx.open_window(
                WindowOptions {
                    app_id: Some(snapshot.host.app_identifier.clone()),
                    window_min_size: Some(size(
                        px(APP_UI_THEME.windows.home_min_width_px),
                        px(APP_UI_THEME.windows.home_min_height_px),
                    )),
                    titlebar: Some(home_titlebar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    window.activate_window();
                    open_home_window(window, cx, runtime.clone())
                },
            ) {
                error!(
                    target: "window",
                    event = "window.home_open_failed",
                    error = %error,
                    "failed to open home window"
                );
                let _ = cx.update(|cx| cx.quit());
                return;
            }

            info!(
                target: "window",
                event = "window.home_opened",
                app_id = %snapshot.host.app_identifier,
                "home window opened"
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
