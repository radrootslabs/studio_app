use gpui::{AppContext, Application, WindowOptions, px, size};

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
                    app_id: Some("org.radroots.app".to_owned()),
                    window_min_size: Some(size(px(640.0), px(480.0))),
                    titlebar: Some(titlebar_options()),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| crate::window::PlaceholderView),
            )
            .expect("main radroots app window should open");

            cx.update(|cx| cx.activate(true))
                .expect("radroots app activation should succeed");
        })
        .detach();
    });

}
