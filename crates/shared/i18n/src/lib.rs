#![forbid(unsafe_code)]

use mf2_i18n_core::{LanguageTag, negotiate_lookup};

mod keys;

pub use keys::AppTextKey;

include!(concat!(env!("OUT_DIR"), "/app_i18n/generated_module.rs"));
include!(concat!(env!("OUT_DIR"), "/app_i18n/generated_catalog.rs"));

pub fn app_text(key: AppTextKey) -> String {
    generated::tr(key.id())
        .unwrap_or_else(|error| panic!("missing localized app text for key {}: {error}", key.id()))
}

pub fn default_locale() -> &'static str {
    DEFAULT_LOCALE_ID
}

pub fn supported_locales() -> &'static [&'static str] {
    SUPPORTED_LOCALE_IDS
}

pub fn resolve_locale_from_host(host_locale: &str) -> String {
    let normalized = normalize_host_locale(host_locale);
    let requested_locale = match LanguageTag::parse(&normalized) {
        Ok(locale) => locale,
        Err(_) => return DEFAULT_LOCALE_ID.to_owned(),
    };
    let supported = SUPPORTED_LOCALE_IDS
        .iter()
        .map(|locale| LanguageTag::parse(locale).expect("supported locale should parse"))
        .collect::<Vec<_>>();
    let default_locale =
        LanguageTag::parse(DEFAULT_LOCALE_ID).expect("default locale should parse");

    negotiate_lookup(&[requested_locale], &supported, &default_locale)
        .selected
        .normalized()
        .to_owned()
}

pub fn select_locale_from_host(host_locale: &str) -> String {
    let locale = resolve_locale_from_host(host_locale);
    generated::set_locale(&locale).unwrap_or_else(|_| DEFAULT_LOCALE_ID.to_owned())
}

fn normalize_host_locale(host_locale: &str) -> String {
    let trimmed = host_locale.trim();
    if trimmed.is_empty() {
        return DEFAULT_LOCALE_ID.to_owned();
    }

    let without_fallbacks = trimmed.split(':').next().unwrap_or(DEFAULT_LOCALE_ID);
    let without_encoding = without_fallbacks
        .split('.')
        .next()
        .unwrap_or(DEFAULT_LOCALE_ID)
        .split('@')
        .next()
        .unwrap_or(DEFAULT_LOCALE_ID)
        .trim();
    if without_encoding.is_empty() {
        return DEFAULT_LOCALE_ID.to_owned();
    }

    without_encoding.replace('_', "-")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        AppTextKey, app_text, default_locale, resolve_locale_from_host, supported_locales,
    };

    #[test]
    fn generated_catalog_matches_typed_key_registry() {
        let catalog_keys = super::DEFAULT_CATALOG_KEY_IDS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let typed_keys = AppTextKey::ALL
            .iter()
            .map(|key| key.id())
            .collect::<BTreeSet<_>>();

        assert_eq!(typed_keys, catalog_keys);
    }

    #[test]
    fn english_catalog_covers_all_defined_text_keys() {
        assert_eq!(super::generated::default_locale(), default_locale());
        assert_eq!(
            super::generated::supported_locales(),
            supported_locales()
                .iter()
                .map(|locale| (*locale).to_owned())
                .collect::<Vec<_>>()
        );

        for key in AppTextKey::ALL {
            assert!(!app_text(*key).trim().is_empty());
        }
    }

    #[test]
    fn english_identity_copy_matches_the_macos_menu_contract() {
        assert_eq!(app_text(AppTextKey::AppName), "Radroots");
        assert_eq!(app_text(AppTextKey::MenuAbout), "About Radroots");
        assert_eq!(app_text(AppTextKey::MenuQuit), "Quit Radroots");
    }

    #[test]
    fn english_auth_copy_matches_the_local_account_workflow_contract() {
        assert_eq!(
            app_text(AppTextKey::HomeTodayEmptySetupBody),
            "Add a local account to start using Radroots on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountNoSelectionBody),
            "Add a local account to start using Radroots on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountActivationLabel),
            "Farmer Activation"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountOpenWorkspaceAction),
            "Open Workspace..."
        );
    }

    #[test]
    fn english_shell_reset_copy_matches_setup_and_utility_contract() {
        assert_eq!(
            app_text(AppTextKey::HomeSetupCreateAccountAction),
            "Create account"
        );
        assert_eq!(app_text(AppTextKey::SettingsTitle), "Radroots Settings");
        assert_eq!(
            app_text(AppTextKey::SettingsAccountNoSelectionTitle),
            "No account selected"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountNoSelectionBody),
            "Add a local account to start using Radroots on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountStatusLoggedOut),
            "Logged Out"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountActivationInactive),
            "Not activated"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountAddAction),
            "Add Account..."
        );
        assert_eq!(app_text(AppTextKey::SettingsAccountLogOutAction), "Log Out");
        assert_eq!(
            app_text(AppTextKey::SettingsAccountOpenWorkspaceAction),
            "Open Workspace..."
        );
    }

    #[test]
    fn host_locale_negotiation_reduces_to_supported_base_locale() {
        assert_eq!(resolve_locale_from_host("en_US.UTF-8"), "en");
        assert_eq!(resolve_locale_from_host("en-GB"), "en");
        assert_eq!(resolve_locale_from_host("en:fr"), "en");
        assert_eq!(resolve_locale_from_host(""), "en");
        assert_eq!(resolve_locale_from_host("C.UTF-8"), "en");
    }
}
