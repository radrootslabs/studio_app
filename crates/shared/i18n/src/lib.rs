#![forbid(unsafe_code)]

use mf2_i18n::{LanguageTag, negotiate_lookup};

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
    fn english_orders_copy_matches_the_queue_contract() {
        assert_eq!(app_text(AppTextKey::HomeNavOrders), "Orders");
        assert_eq!(
            app_text(AppTextKey::HomeTodayOpenInOrdersAction),
            "View all"
        );
        assert_eq!(
            app_text(AppTextKey::HomeTodayOpenInPackDayAction),
            "Open pack day"
        );
        assert_eq!(app_text(AppTextKey::OrdersTitle), "Orders");
        assert_eq!(
            app_text(AppTextKey::OrdersStatusNeedsAction),
            "Needs action"
        );
        assert_eq!(app_text(AppTextKey::OrdersActionMarkPacked), "Mark packed");
        assert_eq!(
            app_text(AppTextKey::OrdersActionMarkCompleted),
            "Mark completed"
        );
        assert_eq!(app_text(AppTextKey::OrdersDetailTitle), "Order detail");
    }

    #[test]
    fn english_marketplace_detail_copy_matches_the_buyer_detail_contract() {
        assert_eq!(app_text(AppTextKey::PersonalDetailBackAction), "Back");
        assert_eq!(
            app_text(AppTextKey::PersonalDetailQuantityLabel),
            "Quantity"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalDetailAddToCartAction),
            "Add to cart"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalDetailReplaceCartAction),
            "Replace cart"
        );
    }

    #[test]
    fn english_marketplace_checkout_copy_matches_the_local_order_contract() {
        assert_eq!(
            app_text(AppTextKey::PersonalCartContinueCheckoutAction),
            "Continue to checkout"
        );
        assert_eq!(app_text(AppTextKey::PersonalCheckoutTitle), "Checkout");
        assert_eq!(
            app_text(AppTextKey::PersonalCheckoutPlaceOrderAction),
            "Place order"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalCheckoutLocalOnlyBody),
            "This places a local order on this device. It does not charge a card."
        );
    }

    #[test]
    fn english_marketplace_orders_copy_matches_the_buyer_history_contract() {
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersSurfaceBody),
            "Review orders placed on this device."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersEmptyTitle),
            "No orders yet"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersListTitle),
            "Order history"
        );
        assert_eq!(app_text(AppTextKey::PersonalOrdersStatusPlaced), "Placed");
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersStatusScheduled),
            "Scheduled"
        );
        assert_eq!(app_text(AppTextKey::PersonalOrdersStatusReady), "Ready");
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersStatusCompleted),
            "Completed"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersStatusRefunded),
            "Refunded"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersDetailTitle),
            "Order detail"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersDetailEmptyBody),
            "Select an order to review the details."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersDetailFulfillmentLabel),
            "Fulfillment"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersDetailNoteLabel),
            "Order note"
        );
    }

    #[test]
    fn english_pack_day_copy_matches_the_contextual_execution_contract() {
        assert_eq!(app_text(AppTextKey::PackDayTitle), "Pack day");
        assert_eq!(
            app_text(AppTextKey::PackDayWindowSummaryTitle),
            "Window summary"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayTotalsTitle),
            "Totals by product"
        );
        assert_eq!(app_text(AppTextKey::PackDayPackListTitle), "Pack list");
        assert_eq!(
            app_text(AppTextKey::PackDayPickupRosterTitle),
            "Pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayEmptyTitle),
            "Nothing to pack yet"
        );
    }

    #[test]
    fn english_farm_rules_host_copy_matches_the_frozen_utility_window_inventory() {
        assert_eq!(app_text(AppTextKey::SettingsNavFarm), "Farm");
        assert_eq!(
            app_text(AppTextKey::SettingsFarmPanelBody),
            "Farm profile and pickup details stay local on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFarmUnavailableBody),
            "Finish setting up a farm before editing farm settings on this device."
        );
        assert_eq!(app_text(AppTextKey::SettingsFarmSaveAction), "Save changes");
        assert_eq!(
            app_text(AppTextKey::SettingsFarmSaveSaved),
            "Saved locally on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFarmSavePending),
            "Save changes to keep this on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFarmSaveBlocked),
            "Complete the highlighted fields before saving."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFarmSaveFailed),
            "Could not save farm settings on this device."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsSectionLabel),
            "Pickup locations"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsEmptyBody),
            "Add a pickup location so customers know where to collect orders."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsAddAction),
            "Add pickup location"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsMakeDefaultAction),
            "Make default"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsDefaultBadge),
            "Default"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsPickupLocationsRemoveAction),
            "Remove"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsOperatingRulesSectionLabel),
            "Operating rules"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsOperatingRulesInvalidPromiseLeadTime),
            "Enter whole hours, for example 24."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFulfillmentWindowsSectionLabel),
            "Fulfillment windows"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFulfillmentWindowsEmptyBody),
            "Add a fulfillment window so customers know when orders are ready."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFulfillmentWindowsPickupLocationsBody),
            "Add a pickup location before saving a fulfillment window."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFulfillmentWindowsAddAction),
            "Add fulfillment window"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsFulfillmentWindowsItemLabel),
            "Fulfillment window"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsBlackoutPeriodsSectionLabel),
            "Blackout periods"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsBlackoutPeriodsEmptyBody),
            "Add a blackout period for days when this farm is unavailable."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsBlackoutPeriodsAddAction),
            "Add blackout period"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsBlackoutPeriodsItemLabel),
            "Blackout period"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsReadinessSectionLabel),
            "Readiness"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsReadinessFieldInvalidTimingConflicts),
            "Invalid timing conflicts"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsReadinessFieldFulfillmentWindowEndsBeforeStart),
            "A fulfillment window ends before it starts."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsReadinessFieldBlackoutOverlapsFulfillmentWindow),
            "A blackout period overlaps a fulfillment window."
        );
        assert_eq!(app_text(AppTextKey::SettingsReadinessReady), "Ready");
    }

    #[test]
    fn startup_identity_choice_keys_remain_defined_in_the_typed_registry_source() {
        let source = include_str!("keys.rs");

        for entry in [
            "HomeSetupContinueAction => \"home.setup.continue_action\"",
            "HomeSetupGenerateKeyAction => \"home.setup.generate_key_action\"",
            "HomeSetupConnectSignerAction => \"home.setup.connect_signer_action\"",
            "HomeSetupSignerSourcePlaceholder => \"home.setup.signer_source.placeholder\"",
            "HomeSetupSignerConnectAction => \"home.setup.signer_connect_action\"",
            "HomeSetupBackAction => \"home.setup.back_action\"",
            "HomeSetupSignerReviewTitle => \"home.setup.signer.review_title\"",
            "HomeSetupSignerSourceLabel => \"home.setup.signer.source_label\"",
            "HomeSetupSignerSignerLabel => \"home.setup.signer.signer_label\"",
            "HomeSetupSignerRelaysLabel => \"home.setup.signer.relays_label\"",
            "HomeSetupSignerPermissionsLabel => \"home.setup.signer.permissions_label\"",
            "HomeSetupSignerConnectingTitle => \"home.setup.signer.connecting_title\"",
            "HomeSetupSignerPendingTitle => \"home.setup.signer.pending_title\"",
            "HomeSetupSignerAuthChallengeTitle => \"home.setup.signer.auth_challenge_title\"",
            "HomeSetupSignerApprovedTitle => \"home.setup.signer.approved_title\"",
        ] {
            assert!(
                source.contains(entry),
                "typed startup identity-choice registry is missing {entry}"
            );
        }
    }

    #[test]
    fn english_startup_identity_choice_copy_matches_the_next_launcher_contract() {
        assert_eq!(app_text(AppTextKey::HomeSetupContinueAction), "Continue");
        assert_eq!(
            app_text(AppTextKey::HomeSetupGenerateKeyAction),
            "Generate key"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupConnectSignerAction),
            "Connect signer"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerSourcePlaceholder),
            "Paste bunker URI or discovery URL"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerConnectAction),
            "Connect signer"
        );
        assert_eq!(app_text(AppTextKey::HomeSetupBackAction), "Back");
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerReviewTitle),
            "Review signer details"
        );
        assert_eq!(app_text(AppTextKey::HomeSetupSignerSourceLabel), "Source");
        assert_eq!(app_text(AppTextKey::HomeSetupSignerSignerLabel), "Signer");
        assert_eq!(app_text(AppTextKey::HomeSetupSignerRelaysLabel), "Relays");
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerPermissionsLabel),
            "Permissions"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerConnectingTitle),
            "Connecting to signer"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerPendingTitle),
            "Waiting for signer approval"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerAuthChallengeTitle),
            "Continue in your signer"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerApprovedTitle),
            "Signer approved"
        );
    }

    #[test]
    fn english_products_workflow_copy_matches_the_editor_contract() {
        assert_eq!(app_text(AppTextKey::ProductsAddAction), "Add product");
        assert_eq!(app_text(AppTextKey::ProductsEditorTitle), "Product details");
        assert_eq!(
            app_text(AppTextKey::ProductsEditorBody),
            "Saved locally on this device."
        );
        assert_eq!(app_text(AppTextKey::ProductsEditorFieldTitle), "Name");
        assert_eq!(app_text(AppTextKey::ProductsEditorFieldSubtitle), "Details");
        assert_eq!(app_text(AppTextKey::ProductsEditorFieldUnit), "Unit");
        assert_eq!(
            app_text(AppTextKey::ProductsEditorFieldPrice),
            "Price (USD)"
        );
        assert_eq!(app_text(AppTextKey::ProductsEditorFieldStock), "Stock");
        assert_eq!(app_text(AppTextKey::ProductsEditorFieldStatus), "Status");
        assert_eq!(app_text(AppTextKey::ProductsEditorCloseAction), "Close");
        assert_eq!(
            app_text(AppTextKey::ProductsEditorSaveAction),
            "Save changes"
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorPublishReadinessTitle),
            "Publish readiness"
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorReady),
            "This product is ready to publish."
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorBlockerAddProductName),
            "Add a product name."
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorBlockerChooseUnit),
            "Choose a unit."
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorBlockerSetPrice),
            "Set a price."
        );
        assert_eq!(
            app_text(AppTextKey::ProductsEditorBlockerAttachAvailability),
            "Attach an availability window."
        );
        assert_eq!(
            app_text(AppTextKey::ProductsUntitledDraft),
            "Untitled draft"
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
