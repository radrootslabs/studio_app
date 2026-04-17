use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, actions};
use radroots_studio_app_i18n::{AppTextKey, app_text};

use crate::{
    runtime::DesktopAppRuntime,
    window::{SettingsPanelViewKey, open_settings_window},
};

actions!(radroots_studio_app, [OpenAboutWindow, QuitApp]);

const fn about_menu_settings_view() -> SettingsPanelViewKey {
    SettingsPanelViewKey::About
}

pub fn install_native_app_menu(runtime: DesktopAppRuntime, cx: &mut App) {
    cx.on_action(move |_: &OpenAboutWindow, cx| {
        open_settings_window(cx, runtime.clone(), about_menu_settings_view());
    });
    cx.on_action(|_: &QuitApp, cx| cx.quit());
    cx.bind_keys([KeyBinding::new("cmd-q", QuitApp, None)]);

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

#[cfg(test)]
mod tests {
    use super::about_menu_settings_view;
    use crate::window::SettingsPanelViewKey;

    #[test]
    fn about_menu_targets_the_about_settings_panel() {
        assert_eq!(about_menu_settings_view(), SettingsPanelViewKey::About);
    }
}
