use gpui::{
    App, AppContext, Bounds, KeyBinding, Menu, MenuItem, SystemMenuType, WindowBounds,
    WindowOptions, actions, px, size,
};
use radroots_studio_app_i18n::{AppTextKey, app_text};
use radroots_studio_app_ui::APP_UI_THEME;

use crate::window::{SettingsPanelViewKey, SettingsWindowView, settings_titlebar_options};

actions!(radroots_studio_app, [OpenAboutWindow, QuitApp]);

pub fn install_native_app_menu(cx: &mut App) {
    cx.on_action(|_: &OpenAboutWindow, cx| open_settings_window(cx));
    cx.on_action(|_: &QuitApp, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenAboutWindow, None),
        KeyBinding::new("cmd-q", QuitApp, None),
    ]);

    let app_name = app_text(AppTextKey::AppName);
    cx.set_menus(vec![Menu {
        name: app_name.into(),
        items: vec![
            MenuItem::action(app_text(AppTextKey::MenuAbout), OpenAboutWindow),
            MenuItem::separator(),
            MenuItem::os_submenu(app_text(AppTextKey::MenuServices), SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action(app_text(AppTextKey::MenuQuit), QuitApp),
        ],
    }]);
}

fn open_settings_window(cx: &mut App) {
    let bounds = Bounds::centered(
        None,
        size(
            px(APP_UI_THEME.windows.settings_width_px),
            px(APP_UI_THEME.windows.settings_height_px),
        ),
        cx,
    );

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(
                px(APP_UI_THEME.windows.settings_width_px),
                px(APP_UI_THEME.windows.settings_height_px),
            )),
            titlebar: Some(settings_titlebar_options()),
            ..Default::default()
        },
        |_, cx| cx.new(|_| SettingsWindowView::new(SettingsPanelViewKey::default())),
    )
    .expect("settings window should open");
}
