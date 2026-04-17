use gpui::{AppContext, Application, WindowOptions, px, size};
use radroots_studio_app_core::{APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeSnapshot};
use radroots_studio_app_i18n::select_locale_from_host;
use radroots_studio_app_ui::APP_UI_THEME;

use crate::menus::install_native_app_menu;
use crate::runtime::DesktopAppRuntime;
use crate::window::{HomeView, home_titlebar_options};

pub fn launch() {
    let snapshot = AppRuntimeSnapshot::capture(build_identity());
    let runtime = DesktopAppRuntime::bootstrap();
    let app = Application::new();

    app.run(move |cx| {
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
            cx.open_window(
                WindowOptions {
                    app_id: Some(snapshot.host.app_identifier.clone()),
                    window_min_size: Some(size(
                        px(APP_UI_THEME.windows.home_min_width_px),
                        px(APP_UI_THEME.windows.home_min_height_px),
                    )),
                    titlebar: Some(home_titlebar_options()),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| HomeView::new(snapshot.clone(), runtime.clone())),
            )
            .expect("main radroots app window should open");

            cx.update(|cx| cx.activate(true))
                .expect("radroots app activation should succeed");
        })
        .detach();
    });
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
