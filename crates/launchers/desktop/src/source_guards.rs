use std::collections::BTreeSet;

const ALLOWED_MENU_LITERALS: &[&str] = &["cmd-q"];

const ALLOWED_WINDOW_LITERALS: &[&str] = &[
    "account-add",
    "account-admin-console",
    "account-log-out",
    "account-more",
    "home-today-scroll",
    "settings window should open",
    "settings-allow-relay-connections",
    "settings-launch-at-login",
    "settings-manage-media-servers",
    "settings-nav-about",
    "settings-nav-accounts",
    "settings-nav-settings",
    "settings-panel-scroll",
    "settings-use-media-servers",
    "settings-use-nip05",
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
