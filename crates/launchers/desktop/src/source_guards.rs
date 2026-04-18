use std::collections::BTreeSet;

const ALLOWED_MENU_LITERALS: &[&str] = &["cmd-q", "settings window should open"];

const ALLOWED_WINDOW_LITERALS: &[&str] = &[
    "account-add",
    "account-open-workspace",
    "account-log-out",
    "account-more",
    "failed to add relay `{relay_url}`: {error}",
    "home-create-account",
    "home-farm-setup-delivery",
    "home-farm-setup-finish",
    "home-farm-setup-pickup",
    "home-farm-setup-shipping",
    "home-farm-setup-start",
    "home-today-scroll",
    "settings-allow-relay-connections",
    "settings-launch-at-login",
    "settings-manage-media-servers",
    "settings-nav-about",
    "settings-nav-accounts",
    "settings-nav-settings",
    "settings-panel-scroll",
    "settings-use-media-servers",
    "settings-use-nip05",
    "startup-title-radroots",
    "startup-title-starting",
];

const REQUIRED_WINDOW_COPY_KEYS: &[&str] = &[
    "AppTextKey::HomeSetupCreateAccountAction",
    "AppTextKey::HomeFarmSetupOnboardingTitle",
    "AppTextKey::HomeFarmSetupOnboardingBody",
    "AppTextKey::HomeFarmSetupOnboardingAction",
    "AppTextKey::HomeFarmSetupSectionFarm",
    "AppTextKey::HomeFarmSetupSectionLocation",
    "AppTextKey::HomeFarmSetupSectionOrderMethods",
    "AppTextKey::HomeFarmSetupFieldFarmName",
    "AppTextKey::HomeFarmSetupFieldLocationOrServiceArea",
    "AppTextKey::HomeFarmSetupOrderMethodPickup",
    "AppTextKey::HomeFarmSetupOrderMethodDelivery",
    "AppTextKey::HomeFarmSetupOrderMethodShipping",
    "AppTextKey::HomeFarmSetupBlockerAddFarmName",
    "AppTextKey::HomeFarmSetupBlockerAddLocationOrServiceArea",
    "AppTextKey::HomeFarmSetupBlockerChooseOrderMethod",
    "AppTextKey::HomeFarmSetupSaveAutosavesLocally",
    "AppTextKey::HomeFarmSetupSaveSavedLocally",
    "AppTextKey::HomeFarmSetupSaveFailedLocally",
    "AppTextKey::HomeFarmSetupFinishAction",
    "AppTextKey::SettingsAccountNoSelectionTitle",
    "AppTextKey::SettingsAccountNoSelectionBody",
    "AppTextKey::SettingsAccountStatusLoggedOut",
    "AppTextKey::SettingsAccountActivationInactive",
    "AppTextKey::SettingsAccountAddAction",
    "AppTextKey::SettingsAccountLogOutAction",
    "AppTextKey::SettingsAccountOpenWorkspaceAction",
];

#[test]
fn desktop_menu_source_uses_localized_copy_paths() {
    assert_eq!(
        extract_string_literals(include_str!("menus.rs")),
        ALLOWED_MENU_LITERALS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn desktop_window_source_uses_localized_copy_paths() {
    assert_eq!(
        extract_string_literals(include_str!("window.rs")),
        ALLOWED_WINDOW_LITERALS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn desktop_window_source_keeps_shell_reset_copy_keyed() {
    let source = include_str!("window.rs");

    for copy_key in REQUIRED_WINDOW_COPY_KEYS {
        assert!(
            source.contains(copy_key),
            "desktop window is missing localized copy key {copy_key}"
        );
    }
}

fn extract_string_literals(source: &str) -> BTreeSet<&str> {
    let mut literals = BTreeSet::new();
    let bytes = source.as_bytes();
    let mut start = None;
    let mut escaped = false;

    for (index, byte) in bytes.iter().copied().enumerate() {
        match (start, byte, escaped) {
            (None, b'"', _) => start = Some(index + 1),
            (Some(_), b'\\', false) => escaped = true,
            (Some(begin), b'"', false) => {
                literals.insert(&source[begin..index]);
                start = None;
            }
            (Some(_), _, true) => escaped = false,
            _ => {}
        }
    }

    literals
}
