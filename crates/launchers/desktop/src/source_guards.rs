use std::collections::BTreeSet;

const ALLOWED_MENU_LITERALS: &[&str] = &["cmd-q", "settings window should open"];

const ALLOWED_WINDOW_LITERALS: &[&str] = &[
    "${dollars}.{cents:02} / {}",
    ", ",
    "0",
    "account-add",
    "account-open-workspace",
    "account-log-out",
    "account-more",
    "failed to add relay `{relay_url}`: {error}",
    "failed to route into products view",
    "failed to update product stock",
    "failed to update products filter",
    "failed to update products search query",
    "failed to update products sort",
    "home-create-account",
    "home-farm-setup-continue",
    "home-farm-setup-delivery",
    "home-farm-setup-finish",
    "home-farm-setup-pickup",
    "home-farm-setup-shipping",
    "home-farm-setup-start",
    "home-nav-products",
    "home-nav-today",
    "home-today-open-products-drafts",
    "home-today-open-products-low-stock",
    "home-products-scroll",
    "home-today-scroll",
    "products",
    "products-filter-all",
    "products-filter-archived",
    "products-filter-drafts",
    "products-filter-live",
    "products-filter-need-attention",
    "products-filter-paused",
    "products-row-stock-action",
    "products-sort-availability",
    "products-sort-name",
    "products-sort-price",
    "products-sort-stock",
    "products-sort-updated",
    "products-stock-editor-cancel",
    "products-stock-editor-close",
    "products-stock-editor-save",
    "products.filter_update_failed",
    "products.route_failed",
    "products.search_query_update_failed",
    "products.stock_update_failed",
    "products.sort_update_failed",
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
    "{quantity} {unit_label}",
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
    "AppTextKey::HomeFarmSetupContinueAction",
    "AppTextKey::HomeTodayOpenInProductsAction",
    "AppTextKey::HomeNavToday",
    "AppTextKey::HomeNavProducts",
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
    "AppTextKey::ProductsUpdateStockAction",
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
