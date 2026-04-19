use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const ALLOWED_MENU_LITERALS: &[&str] = &["cmd-q", "settings window should open"];

const ALLOWED_WINDOW_LITERALS: &[&str] = &[
    "",
    "  ",
    "${dollars}.{cents:02} / {}",
    ", ",
    "0",
    "1111111111111111111111111111111111111111111111111111111111111111",
    "14",
    "14.5",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333333333333333333333333333",
    "6",
    "6.",
    "6.5",
    "6.50",
    "6.500",
    "Salad mix",
    "USD",
    "Untitled draft",
    "{}.{:02}",
    "abc",
    "account-add",
    "account-open-workspace",
    "account-log-out",
    "account-more",
    "bunker uri",
    "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
    "failed to add relay `{relay_url}`: {error}",
    "failed to load farm settings projection",
    "failed to mark order completed",
    "failed to mark order packed",
    "failed to open existing product editor",
    "failed to open new product editor",
    "failed to open order detail",
    "failed to route into pack day view",
    "failed to route into orders view",
    "failed to save farm settings projection",
    "failed to save product editor draft",
    "failed to update orders filter",
    "failed to route into products view",
    "failed to update product stock",
    "failed to update products filter",
    "failed to update products search query",
    "failed to update products sort",
    "home-connect-signer",
    "home-connect-signer-submit",
    "home-continue",
    "home-farm-setup-continue",
    "home-farm-setup-delivery",
    "home-farm-setup-finish",
    "home-farm-setup-pickup",
    "home-farm-setup-shipping",
    "home-farm-setup-start",
    "home-generate-key",
    "home-nav-orders",
    "home-nav-pack-day",
    "home-nav-products",
    "home-nav-today",
    "home-orders-scroll",
    "home-pack-day-scroll",
    "home-signer-back",
    "home-signer-source-input",
    "home-today-open-orders",
    "home-today-open-products-drafts",
    "home-today-open-products-low-stock",
    "home-products-scroll",
    "home-today-scroll",
    "https://auth.example/challenge",
    "identity",
    "none",
    "npub1",
    "orders",
    "pack_day",
    "pack_day.route_failed",
    "orders-detail-mark-completed",
    "orders-detail-mark-packed",
    "orders-filter-all",
    "orders-filter-completed",
    "orders-filter-needs-action",
    "orders-filter-packed",
    "orders-filter-refunded",
    "orders-filter-scheduled",
    "orders-row-action-mark-completed",
    "orders-row-action-mark-packed",
    "orders-row-action-review",
    "orders-row-open",
    "orders.detail_open_failed",
    "orders.filter_update_failed",
    "orders.mark_completed_failed",
    "orders.mark_packed_failed",
    "orders.route_failed",
    "preview",
    "products",
    "products-filter-all",
    "products-filter-archived",
    "products-filter-drafts",
    "products-filter-live",
    "products-filter-need-attention",
    "products-filter-paused",
    "products-row-stock-action",
    "products-row-open",
    "products-sort-availability",
    "products-sort-name",
    "products-sort-price",
    "products-sort-stock",
    "products-sort-updated",
    "products-add-product",
    "products-editor-close",
    "products-editor-save",
    "products-editor-status-archived",
    "products-editor-status-draft",
    "products-editor-status-live",
    "products-editor-status-paused",
    "products.editor_open_failed",
    "products.editor_save_failed",
    "products.new_editor_open_failed",
    "products-stock-editor-cancel",
    "products-stock-editor-close",
    "products-stock-editor-save",
    "products.filter_update_failed",
    "products.route_failed",
    "products.search_query_update_failed",
    "products.stock_update_failed",
    "products.sort_update_failed",
    "remote signer connection failed: relay refused the request",
    "remote signer did not respond yet",
    "runtime unavailable",
    "settings",
    "settings-add-blackout-period",
    "settings-add-fulfillment-window",
    "settings-allow-relay-connections",
    "settings-farm-add-pickup",
    "settings-farm-default-pickup",
    "settings-farm-remove-pickup",
    "settings-farm-save",
    "settings-fulfillment-window-pickup-location",
    "settings-launch-at-login",
    "settings-manage-media-servers",
    "settings-nav-about",
    "settings-nav-accounts",
    "settings-nav-farm",
    "settings-nav-settings",
    "settings-panel-scroll",
    "settings-remove-blackout-period",
    "settings-remove-fulfillment-window",
    "settings-use-media-servers",
    "settings-use-nip05",
    "settings.farm.load_failed",
    "settings.farm.save_failed",
    "sign_event:kind:1, switch_relays",
    "startup-title-radroots",
    "startup-title-starting",
    "wss://relay.radroots.example",
    "{quantity} {unit_label}",
    "{} {}",
];

const REQUIRED_WINDOW_COPY_KEYS: &[&str] = &[
    "AppTextKey::HomeSetupBackAction",
    "AppTextKey::HomeSetupConnectSignerAction",
    "AppTextKey::HomeSetupContinueAction",
    "AppTextKey::HomeSetupGenerateKeyAction",
    "AppTextKey::HomeSetupSignerConnectAction",
    "AppTextKey::HomeSetupSignerSourcePlaceholder",
    "AppTextKey::HomeSetupSignerReviewTitle",
    "AppTextKey::HomeSetupSignerSourceLabel",
    "AppTextKey::HomeSetupSignerSignerLabel",
    "AppTextKey::HomeSetupSignerRelaysLabel",
    "AppTextKey::HomeSetupSignerPermissionsLabel",
    "AppTextKey::HomeSetupSignerConnectingTitle",
    "AppTextKey::HomeSetupSignerPendingTitle",
    "AppTextKey::HomeSetupSignerAuthChallengeTitle",
    "AppTextKey::HomeSetupSignerApprovedTitle",
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
    "AppTextKey::HomeFarmSetupContinueAction",
    "AppTextKey::HomeTodayOpenInProductsAction",
    "AppTextKey::HomeNavToday",
    "AppTextKey::HomeNavProducts",
    "AppTextKey::HomeNavOrders",
    "AppTextKey::HomeTodayOpenInOrdersAction",
    "AppTextKey::OrdersTitle",
    "AppTextKey::OrdersFiltersTitle",
    "AppTextKey::OrdersSummaryTotal",
    "AppTextKey::OrdersFilterAll",
    "AppTextKey::OrdersStatusNeedsAction",
    "AppTextKey::OrdersStatusScheduled",
    "AppTextKey::OrdersStatusPacked",
    "AppTextKey::OrdersStatusCompleted",
    "AppTextKey::OrdersStatusRefunded",
    "AppTextKey::OrdersTableTitle",
    "AppTextKey::OrdersColumnOrder",
    "AppTextKey::OrdersColumnStatus",
    "AppTextKey::OrdersColumnWindow",
    "AppTextKey::OrdersColumnPickup",
    "AppTextKey::OrdersColumnAction",
    "AppTextKey::OrdersActionReview",
    "AppTextKey::OrdersActionMarkPacked",
    "AppTextKey::OrdersActionMarkCompleted",
    "AppTextKey::OrdersEmptyTitle",
    "AppTextKey::OrdersEmptyBody",
    "AppTextKey::OrdersEmptyNeedsActionTitle",
    "AppTextKey::OrdersEmptyNeedsActionBody",
    "AppTextKey::OrdersDetailTitle",
    "AppTextKey::OrdersDetailEmptyBody",
    "AppTextKey::OrdersDetailItemsTitle",
    "AppTextKey::OrdersDetailCustomerLabel",
    "AppTextKey::OrdersDetailStatusLabel",
    "AppTextKey::OrdersDetailWindowLabel",
    "AppTextKey::OrdersDetailPickupLabel",
    "AppTextKey::PackDayTitle",
    "AppTextKey::PackDayWindowSummaryTitle",
    "AppTextKey::PackDayTotalsTitle",
    "AppTextKey::PackDayPackListTitle",
    "AppTextKey::PackDayPickupRosterTitle",
    "AppTextKey::PackDayEmptyTitle",
    "AppTextKey::PackDayEmptyBody",
    "AppTextKey::ProductsTitle",
    "AppTextKey::ProductsFiltersTitle",
    "AppTextKey::ProductsSearchPlaceholder",
    "AppTextKey::ProductsSummaryTotal",
    "AppTextKey::ProductsSummaryLive",
    "AppTextKey::ProductsSummaryNeedAttention",
    "AppTextKey::ProductsSummaryDrafts",
    "AppTextKey::ProductsFilterAll",
    "AppTextKey::ProductsFilterLive",
    "AppTextKey::ProductsFilterDrafts",
    "AppTextKey::ProductsFilterNeedAttention",
    "AppTextKey::ProductsFilterPaused",
    "AppTextKey::ProductsFilterArchived",
    "AppTextKey::ProductsSortTitle",
    "AppTextKey::ProductsSortUpdated",
    "AppTextKey::ProductsSortName",
    "AppTextKey::ProductsSortAvailability",
    "AppTextKey::ProductsSortStock",
    "AppTextKey::ProductsSortPrice",
    "AppTextKey::ProductsTableTitle",
    "AppTextKey::ProductsColumnProduct",
    "AppTextKey::ProductsColumnStatus",
    "AppTextKey::ProductsColumnAvailability",
    "AppTextKey::ProductsColumnStock",
    "AppTextKey::ProductsColumnPrice",
    "AppTextKey::ProductsColumnUpdated",
    "AppTextKey::ProductsColumnAction",
    "AppTextKey::ProductsAddAction",
    "AppTextKey::ProductsUpdateStockAction",
    "AppTextKey::ProductsEditorTitle",
    "AppTextKey::ProductsEditorBody",
    "AppTextKey::ProductsEditorFieldTitle",
    "AppTextKey::ProductsEditorFieldSubtitle",
    "AppTextKey::ProductsEditorFieldUnit",
    "AppTextKey::ProductsEditorFieldPrice",
    "AppTextKey::ProductsEditorFieldStock",
    "AppTextKey::ProductsEditorFieldStatus",
    "AppTextKey::ProductsEditorCloseAction",
    "AppTextKey::ProductsEditorSaveAction",
    "AppTextKey::ProductsEditorSaveFailed",
    "AppTextKey::ProductsEditorInvalidPrice",
    "AppTextKey::ProductsEditorInvalidStock",
    "AppTextKey::ProductsEditorPublishReadinessTitle",
    "AppTextKey::ProductsEditorReady",
    "AppTextKey::ProductsEditorBlockerAddProductName",
    "AppTextKey::ProductsEditorBlockerChooseUnit",
    "AppTextKey::ProductsEditorBlockerSetPrice",
    "AppTextKey::ProductsEditorBlockerAttachAvailability",
    "AppTextKey::ProductsUntitledDraft",
    "AppTextKey::ProductsStockEditorTitle",
    "AppTextKey::ProductsStockEditorFieldLabel",
    "AppTextKey::ProductsStockEditorSaveAction",
    "AppTextKey::ProductsStockEditorCancelAction",
    "AppTextKey::ProductsStockEditorInvalidQuantity",
    "AppTextKey::ProductsStockEditorSaveFailed",
    "AppTextKey::ProductsStatusDraft",
    "AppTextKey::ProductsStatusLive",
    "AppTextKey::ProductsStatusPaused",
    "AppTextKey::ProductsStatusArchived",
    "AppTextKey::ProductsEmptyTitle",
    "AppTextKey::ProductsEmptyBody",
    "AppTextKey::ProductsEmptyNeedAttentionTitle",
    "AppTextKey::ProductsEmptyNeedAttentionBody",
    "AppTextKey::SettingsAccountNoSelectionTitle",
    "AppTextKey::SettingsAccountNoSelectionBody",
    "AppTextKey::SettingsAccountStatusLoggedOut",
    "AppTextKey::SettingsAccountActivationInactive",
    "AppTextKey::SettingsAccountAddAction",
    "AppTextKey::SettingsAccountLogOutAction",
    "AppTextKey::SettingsAccountOpenWorkspaceAction",
    "AppTextKey::SettingsNavFarm",
    "AppTextKey::SettingsFarmPanelBody",
    "AppTextKey::SettingsFarmUnavailableBody",
    "AppTextKey::SettingsFarmSaveAction",
    "AppTextKey::SettingsFarmSaveSaved",
    "AppTextKey::SettingsFarmSavePending",
    "AppTextKey::SettingsFarmSaveBlocked",
    "AppTextKey::SettingsFarmSaveFailed",
    "AppTextKey::SettingsFarmFieldTimezone",
    "AppTextKey::SettingsFarmFieldCurrency",
    "AppTextKey::SettingsPickupLocationsSectionLabel",
    "AppTextKey::SettingsPickupLocationsEmptyBody",
    "AppTextKey::SettingsPickupLocationsAddAction",
    "AppTextKey::SettingsPickupLocationsMakeDefaultAction",
    "AppTextKey::SettingsPickupLocationsDefaultBadge",
    "AppTextKey::SettingsPickupLocationsRemoveAction",
    "AppTextKey::SettingsPickupLocationsFieldLabel",
    "AppTextKey::SettingsPickupLocationsFieldAddress",
    "AppTextKey::SettingsPickupLocationsFieldDirections",
    "AppTextKey::SettingsPickupLocationsFieldDefault",
    "AppTextKey::SettingsSettingsPanelBody",
    "AppTextKey::SettingsOperatingRulesSectionLabel",
    "AppTextKey::SettingsOperatingRulesFieldPromiseLeadTime",
    "AppTextKey::SettingsOperatingRulesFieldSubstitutionPolicy",
    "AppTextKey::SettingsOperatingRulesFieldMissedPickupPolicy",
    "AppTextKey::SettingsOperatingRulesInvalidPromiseLeadTime",
    "AppTextKey::SettingsFulfillmentWindowsSectionLabel",
    "AppTextKey::SettingsFulfillmentWindowsEmptyBody",
    "AppTextKey::SettingsFulfillmentWindowsPickupLocationsBody",
    "AppTextKey::SettingsFulfillmentWindowsAddAction",
    "AppTextKey::SettingsFulfillmentWindowsRemoveAction",
    "AppTextKey::SettingsFulfillmentWindowsItemLabel",
    "AppTextKey::SettingsFulfillmentWindowsFieldLabel",
    "AppTextKey::SettingsFulfillmentWindowsFieldPickupLocation",
    "AppTextKey::SettingsFulfillmentWindowsFieldStartsAt",
    "AppTextKey::SettingsFulfillmentWindowsFieldEndsAt",
    "AppTextKey::SettingsFulfillmentWindowsFieldOrderCutoff",
    "AppTextKey::SettingsFulfillmentWindowsValidationCompleteBeforeSave",
    "AppTextKey::SettingsFulfillmentWindowsValidationChoosePickupLocation",
    "AppTextKey::SettingsBlackoutPeriodsSectionLabel",
    "AppTextKey::SettingsBlackoutPeriodsEmptyBody",
    "AppTextKey::SettingsBlackoutPeriodsAddAction",
    "AppTextKey::SettingsBlackoutPeriodsRemoveAction",
    "AppTextKey::SettingsBlackoutPeriodsItemLabel",
    "AppTextKey::SettingsBlackoutPeriodsFieldLabel",
    "AppTextKey::SettingsBlackoutPeriodsFieldStartsAt",
    "AppTextKey::SettingsBlackoutPeriodsFieldEndsAt",
    "AppTextKey::SettingsBlackoutPeriodsValidationCompleteBeforeSave",
    "AppTextKey::SettingsReadinessSectionLabel",
    "AppTextKey::SettingsReadinessFieldMissingProfileBasics",
    "AppTextKey::SettingsReadinessFieldMissingPickupLocation",
    "AppTextKey::SettingsReadinessFieldMissingFulfillmentWindow",
    "AppTextKey::SettingsReadinessFieldMissingOperatingRules",
    "AppTextKey::SettingsReadinessFieldInvalidTimingConflicts",
    "AppTextKey::SettingsReadinessFieldFulfillmentWindowEndsBeforeStart",
    "AppTextKey::SettingsReadinessFieldFulfillmentWindowCutoffAfterStart",
    "AppTextKey::SettingsReadinessFieldBlackoutPeriodEndsBeforeStart",
    "AppTextKey::SettingsReadinessFieldBlackoutOverlapsFulfillmentWindow",
    "AppTextKey::SettingsReadinessReady",
];

const FORBIDDEN_LAUNCHER_UI_BYPASS_PATTERNS: &[(&str, &str)] = &[
    (
        "Button::new(",
        "launcher code must use radroots_studio_app_ui button primitives",
    ),
    (
        "Checkbox::new(",
        "launcher code must use radroots_studio_app_ui checkbox primitives",
    ),
    (
        "Input::new(",
        "launcher code must use radroots_studio_app_ui input primitives",
    ),
    (
        "TextInput::new(",
        "launcher code must use radroots_studio_app_ui input primitives",
    ),
    (
        "pub fn app_",
        "shared app_* helpers belong in radroots_studio_app_ui, not in launcher code",
    ),
    (
        "fn app_",
        "shared app_* helpers belong in radroots_studio_app_ui, not in launcher code",
    ),
];

const REMOVED_WINDOW_HELPER_FAMILIES: &[&str] = &[
    "fn settings_account_detail_row(",
    "fn settings_checkbox_row(",
    "fn settings_text_field(",
    "fn settings_dynamic_action_button(",
    "fn settings_inventory_panel(",
    "fn settings_inventory_field_row(",
    "fn settings_validation_rows(",
    "fn home_farm_setup_blocker(",
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

#[test]
fn desktop_launcher_source_keeps_shared_ui_boundary_enforced() {
    for (path, source) in launcher_source_files() {
        for (pattern, reason) in FORBIDDEN_LAUNCHER_UI_BYPASS_PATTERNS {
            assert!(
                !source.contains(pattern),
                "{} contains forbidden UI bypass pattern `{pattern}`: {reason}",
                path.display()
            );
        }
    }
}

#[test]
fn desktop_window_source_does_not_reintroduce_removed_ui_helper_families() {
    let source = include_str!("window.rs");

    for helper_name in REMOVED_WINDOW_HELPER_FAMILIES {
        assert!(
            !source.contains(helper_name),
            "window.rs reintroduced removed launcher-local helper family `{helper_name}`"
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

fn launcher_source_files() -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    collect_rust_source_files(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src").as_path(),
        &mut paths,
    );
    paths.sort();
    paths
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("source_guards.rs"))
        .map(|path| {
            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("failed to read launcher source {}: {error}", path.display())
            });
            (path, source)
        })
        .collect()
}

fn collect_rust_source_files(root: &Path, paths: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|error| {
        panic!(
            "failed to read launcher source directory {}: {error}",
            root.display()
        )
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "failed to inspect launcher source directory {}: {error}",
                root.display()
            )
        });
        let path = entry.path();

        if path.is_dir() {
            collect_rust_source_files(path.as_path(), paths);
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            paths.push(path);
        }
    }
}
