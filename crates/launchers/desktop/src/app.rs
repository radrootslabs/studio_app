use gpui::{AppContext, Application, WindowOptions, px, size};
use radroots_studio_app_core::APP_ID;
use radroots_studio_app_i18n::select_locale_for_process;
use radroots_studio_app_ui::{APP_UI_THEME, PlaceholderView};

fn titlebar_options() -> gpui::TitlebarOptions {
    gpui::TitlebarOptions {
        title: None,
        appears_transparent: true,
        ..Default::default()
    }
}

pub fn launch() {
    let app = Application::new();

    app.run(|cx| {
        select_locale_for_process();

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    app_id: Some(APP_ID.to_owned()),
                    window_min_size: Some(size(
                        px(APP_UI_THEME.windows.home_min_width_px),
                        px(APP_UI_THEME.windows.home_min_height_px),
                    )),
                    titlebar: Some(titlebar_options()),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| PlaceholderView),
            )
            .expect("main radroots app window should open");

            cx.update(|cx| cx.activate(true))
                .expect("radroots app activation should succeed");
        })
        .detach();
    });
}
