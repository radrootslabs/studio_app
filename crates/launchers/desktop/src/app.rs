use gpui::{AppContext, Application, WindowOptions, px, size};
use radroots_studio_app_core::{APP_ID, HOME_WINDOW_METRICS};
use radroots_studio_app_ui::{HOME_WINDOW_MIN_HEIGHT_PX, HOME_WINDOW_MIN_WIDTH_PX, PlaceholderView};

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
                        px(HOME_WINDOW_METRICS.min_width_px.max(HOME_WINDOW_MIN_WIDTH_PX)),
                        px(HOME_WINDOW_METRICS.min_height_px.max(HOME_WINDOW_MIN_HEIGHT_PX)),
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
