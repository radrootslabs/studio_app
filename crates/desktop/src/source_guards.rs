use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const ALLOWED_MENU_LITERALS: &[&str] = &["cmd-q", "settings window should open"];

const ALLOWED_WINDOW_LITERALS: &[&str] = &[
    "",
    "  ",
    "${dollars}.{cents:02}",
    "${dollars}.{cents:02} / {}",
    ", ",
    "+",
    "-",
    "0",
    "1111111111111111111111111111111111111111111111111111111111111111",
    "127.0.0.1",
    "14",
    "14.5",
    "2",
    "2 bags",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333333333333333333333333333",
    "../../../platforms/macos/App/Resources/AppIconSource.png",
    "2026-04-23T15:00:00Z",
    "6",
    "6.",
    "6.5",
    "6.50",
    "6.500",
    "  Farm Profile  ",
    "Farm Profile",
    "Salad mix",
    "USD",
    "[::1]",
    "/tmp/radroots/data/apps/app",
    "/tmp/radroots/data/apps/app/sdk",
    "/tmp/radroots/logs/apps/app",
    "{}.{:02}",
    "abc",
    "app.sqlite3",
    "account-add",
    "account-open-workspace",
    "account-log-out",
    "account-more",
    "account-profile-change-photo",
    "account-profile-remove-photo",
    "account-settings-add-relay",
    "account-settings-blossom-product-photos",
    "account-settings-blossom-profile-farm-media",
    "account-settings-relay-localhost-8080",
    "account-settings-relay-localhost-8081",
    "account-settings-relay-menu-localhost-8080",
    "account-settings-relay-menu-localhost-8081",
    "account-settings-reset-blossom",
    "account-settings-reset-relays",
    "account-settings-save",
    "account-settings-save-draft",
    "account-farm-card-scroll",
    "account-farm-details-tabs",
    "account-farm-save",
    "account-farm-save-draft",
    "account-farm-add-pickup-window",
    "account-farm-profile-preview",
    "account-farm-view-profile",
    "account-scroll",
    "account-tabs",
    "account_1",
    "buyer",
    "buyer-detail-add-to-cart",
    "buyer-detail-back",
    "buyer-detail-confirm-replace",
    "buyer-detail-keep-current",
    "buyer-detail-quantity-decrease",
    "buyer-detail-quantity-increase",
    "buyer-cart-open-order-review",
    "buyer-cart-remove-line",
    "buyer-order-review-back",
    "buyer-order-review-place-order",
    "buyer-listing-open",
    "buyer-order-accept-change",
    "buyer-order-confirm-replace",
    "buyer-order-cancel",
    "buyer-order-detail-back",
    "buyer-order-keep-current",
    "buyer-order-keep-order",
    "buyer-order-repeat-demand",
    "buyer-orders-retry-coordination",
    "personal_orders",
    "buyer.add_to_cart_failed",
    "buyer.cart_remove_failed",
    "buyer.order_review_place_failed",
    "buyer.order_review_save_failed",
    "buyer.detail_open_failed",
    "buyer.order_open_failed",
    "buyer.order_cancel_failed",
    "buyer.order_coordination_retry_failed",
    "buyer.order_revision_accept_failed",
    "buyer.order_revision_decline_failed",
    "buyer.repeat_demand_failed",
    "buyer.section_select_failed",
    "buyer_notice",
    "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
    "buyer.fulfillment_filter_update_failed",
    "buyer.search_query_update_failed",
    "CARGO_PKG_VERSION",
    "clock",
    "configuration",
    "configure_relay_targets",
    "customer_labels.txt",
    "desktop runtime paths should resolve",
    "desktop runtime roots require HOME for macos",
    "directory",
    "disk unavailable",
    "eggs",
    "event_store.sqlite",
    "failed to add buyer product to cart",
    "failed to open buyer order detail",
    "failed to place buyer order",
    "failed to retry buyer order coordination",
    "failed to remove buyer cart line",
    "failed to reorder buyer order",
    "failed to save buyer order review draft",
    "failed to select buyer section",
    "failed to open buyer product detail",
    "failed to update buyer fulfillment filter",
    "failed to update buyer search query",
    "failed to add relay `{relay_url}`: {error}",
    "failed to load farm settings projection",
    "failed to accept buyer order change",
    "failed to cancel buyer order",
    "failed to keep buyer order",
    "failed to open existing product editor",
    "failed to open new product editor",
    "failed to acknowledge reminder",
    "failed to export pack day",
    "failed to complete pack day batch print",
    "failed to complete pack day host handoff",
    "failed to complete pack day print",
    "failed to prepare pack day batch print",
    "failed to prepare pack day host handoff",
    "failed to prepare pack day print",
    "failed to open order detail",
    "failed to route into pack day view",
    "failed to route into orders view",
    "failed to save farm settings projection",
    "failed to save product editor draft",
    "failed to switch into farm mode",
    "failed to switch into marketplace mode",
    "failed to update orders filter",
    "failed to route into products view",
    "failed to update product stock",
    "failed to update products filter",
    "failed to update products search query",
    "failed to update products sort",
    "home-browse-marketplace",
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
    "home-sidebar-account-menu",
    "home-nav-orders",
    "home-nav-pack-day",
    "home-nav-products",
    "home-nav-today",
    "home-orders-scroll",
    "home-pack-day-scroll",
    "buyer-browse-scroll",
    "buyer-cart-scroll",
    "buyer-nav-browse",
    "buyer-nav-cart",
    "buyer-nav-orders",
    "buyer-nav-search",
    "buyer-order-open",
    "buyer-orders-scroll",
    "personal-search-delivery",
    "personal-search-pickup",
    "personal-search-shipping",
    "presented reminder",
    "buyer-search-scroll",
    "home-today-open-pack-day",
    "home-today-order-open",
    "home-signer-back",
    "home-signer-source-input",
    "home-today-open-orders",
    "home-today-open-products-drafts",
    "home-today-open-products-low-stock",
    "home-products-scroll",
    "home-today-scroll",
    "http://",
    "http://localhost:8082",
    "https://",
    "today-reminder-chip",
    "https://auth.example/challenge",
    "identity",
    "localhost",
    "npub1",
    "npub1qqqqq...qqqqqq",
    "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
    "npub1sxczr...5lkheq",
    "npub1sxczrq2dp4jtehcm8mtemj975u5ytf2d7mc6dpuuq3rzkjzr76ls5lkheq",
    "guest",
    "finder unavailable",
    "orders",
    "orders-reminders",
    "orders-detail-back",
    "pack-day-export",
    "pack-day-open-customer-labels",
    "pack-day-open-pack-sheet",
    "pack-day-open-pickup-roster",
    "pack-day-print-all",
    "pack-day-print-customer-labels",
    "pack-day-print-pack-sheet",
    "pack-day-print-pickup-roster",
    "pack_day",
    "pack_day.batch_print_failed",
    "pack_day.batch_print_prepare_failed",
    "pack_day.host_handoff_failed",
    "pack_day.host_handoff_prepare_failed",
    "pack_day.print_failed",
    "pack_day.print_prepare_failed",
    "pack-day-reminders",
    "pack-day-reveal-bundle",
    "pack_day.export_failed",
    "pack_day.route_failed",
    "orders-filter-all",
    "orders-filter-completed",
    "orders-filter-needs-action",
    "orders-filter-packed",
    "orders-filter-scheduled",
    "orders-row-action-review",
    "orders-row-open",
    "orders.detail_open_failed",
    "orders.filter_update_failed",
    "orders.route_failed",
    "outbox.sqlite",
    "preview",
    "pack_sheet.txt",
    "pack_sheet.txt, pickup_roster.txt, customer_labels.txt",
    "pickup_roster.txt",
    "products",
    "reminder-banner-action",
    "reminder-banner-dismiss",
    "reminders",
    "reminders.ack_failed",
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
    "products-editor-availability",
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
    "discovery url does not contain a remote signer uri",
    "enter a bunker or discovery url to continue",
    "enter a bunker or discovery url from the signer; raw nostrconnect client uris are signer-side only",
    "exports/pack_day/window-1/20260423T150000Z",
    "invalid discovery url:",
    "invalid discovery url: relative URL without a base",
    "invalid remote signer uri:",
    "invalid remote signer uri: invalid public key",
    "invalid_relay_url",
    "a remote signer connection is already pending approval",
    "raw nostrconnect client uris are signer-side only",
    "remote signer",
    "remote signer connection failed: relay refused the request",
    "remote signer did not respond yet",
    "retry_startup",
    "retry_status_refresh",
    "review_runtime_configuration",
    "runtime unavailable",
    "radroots_home_view_{label}_{suffix}",
    "sign_event:kind:1",
    "shell",
    "shell-account-entry",
    "shell-mode-farm",
    "shell-mode-marketplace",
    "shell.switch_farm_failed",
    "shell.switch_marketplace_failed",
    "sdk_canonical_stores",
    "settings",
    "settings-add-blackout-period",
    "settings-add-fulfillment-window",
    "settings-farm-add-pickup",
    "settings-farm-default-pickup",
    "settings-farm-remove-pickup",
    "settings-farm-save",
    "settings-fulfillment-window-pickup-location",
    "settings-nav-about",
    "settings-nav-accounts",
    "settings-nav-farm",
    "settings-nav-settings",
    "settings-panel-scroll",
    "settings-about-acknowledgements",
    "settings-about-conflict-action",
    "settings-about-privacy-policy",
    "settings-about-refresh-sync",
    "settings-about-refresh-sync-disabled",
    "settings-about-report-issue",
    "settings-about-terms",
    "settings-account-row",
    "settings-remove-blackout-period",
    "settings-remove-fulfillment-window",
    "settings.farm.load_failed",
    "settings.farm.save_failed",
    "settings.about.sync_refresh_failed",
    "settings.about.conflict_resolution_failed",
    "settings.account.select_failed",
    "failed to refresh sync from the about panel",
    "failed to resolve sync conflict from the about panel",
    "failed to select account from settings panel",
    "switch_relays",
    "startup-title-radroots",
    "startup-title-starting",
    "wait_for_sdk_lifecycle",
    "ws://localhost:8080",
    "ws://localhost:8081",
    "wss://relay.example",
    "wss://relay.radroots.example",
    "{currency_code} {dollars}.{cents:02}",
    "{prefix}...{suffix}",
    "{}, {}",
    "{}: {}",
    "{} {} {}.",
    "{quantity} {unit_label}",
    "{} {}",
];

const REQUIRED_WINDOW_COPY_KEYS: &[&str] = &[
    "AppTextKey::AppName",
    "AppTextKey::HomeHeaderMarketplaceMode",
    "AppTextKey::HomeHeaderFarmMode",
    "AppTextKey::HomeHeaderAccountSetupAction",
    "AppTextKey::HomeHeaderGuestLabel",
    "AppTextKey::AccountTitle",
    "AppTextKey::AccountTabProfile",
    "AppTextKey::AccountTabFarmDetails",
    "AppTextKey::AccountTabPreferences",
    "AppTextKey::AccountTabSettings",
    "AppTextKey::AccountNotImplemented",
    "AppTextKey::AccountFormSaveAction",
    "AppTextKey::AccountFormSaveDraftAction",
    "AppTextKey::AccountFarmDetailsTabProfile",
    "AppTextKey::AccountFarmDetailsTabLocation",
    "AppTextKey::AccountFarmDetailsTabOperations",
    "AppTextKey::AccountFarmDetailsTabFulfilment",
    "AppTextKey::AccountProfilePersonalDetailsTitle",
    "AppTextKey::AccountProfilePictureLabel",
    "AppTextKey::AccountProfileChangePhotoAction",
    "AppTextKey::AccountProfileRemovePhotoAction",
    "AppTextKey::AccountProfileFullNameLabel",
    "AppTextKey::AccountProfileEmailLabel",
    "AppTextKey::AccountProfilePhoneLabel",
    "AppTextKey::AccountProfileRoleLabel",
    "AppTextKey::AccountProfileTimeZoneLabel",
    "AppTextKey::AccountProfileLanguageLabel",
    "AppTextKey::AccountProfileFullNameValue",
    "AppTextKey::AccountProfileEmailValue",
    "AppTextKey::AccountProfilePhoneValue",
    "AppTextKey::AccountProfileRoleValue",
    "AppTextKey::AccountProfileRoleFarmManagerValue",
    "AppTextKey::AccountProfileRoleTeamMemberValue",
    "AppTextKey::AccountProfileTimeZoneValue",
    "AppTextKey::AccountProfileTimeZoneMountainValue",
    "AppTextKey::AccountProfileTimeZoneEasternValue",
    "AppTextKey::AccountProfileLanguageValue",
    "AppTextKey::AccountProfileLanguageFrenchValue",
    "AppTextKey::AccountProfileLanguageSpanishValue",
    "AppTextKey::AccountFarmDetailsTitle",
    "AppTextKey::AccountFarmDetailsFarmProfileTitle",
    "AppTextKey::AccountFarmDetailsFarmProfileIntro",
    "AppTextKey::AccountFarmDetailsFarmNameLabel",
    "AppTextKey::AccountFarmDetailsPublicFarmNameLabel",
    "AppTextKey::AccountFarmDetailsShortDescriptionLabel",
    "AppTextKey::AccountFarmDetailsFarmTypeLabel",
    "AppTextKey::AccountFarmDetailsContactEmailLabel",
    "AppTextKey::AccountFarmDetailsPublicPhoneLabel",
    "AppTextKey::AccountFarmDetailsWebsiteLabel",
    "AppTextKey::AccountFarmDetailsEstablishedYearLabel",
    "AppTextKey::AccountFarmDetailsAboutFarmLabel",
    "AppTextKey::AccountFarmDetailsFarmNameValue",
    "AppTextKey::AccountFarmDetailsPublicFarmNameValue",
    "AppTextKey::AccountFarmDetailsShortDescriptionValue",
    "AppTextKey::AccountFarmDetailsContactEmailValue",
    "AppTextKey::AccountFarmDetailsPublicPhoneValue",
    "AppTextKey::AccountFarmDetailsWebsiteValue",
    "AppTextKey::AccountFarmDetailsEstablishedYearValue",
    "AppTextKey::AccountFarmDetailsAboutFarmValue",
    "AppTextKey::AccountFarmDetailsFarmLocationValue",
    "AppTextKey::AccountFarmDetailsSummaryTitle",
    "AppTextKey::AccountFarmDetailsFarmTypeSummaryLabel",
    "AppTextKey::AccountFarmDetailsEstablishedSummaryLabel",
    "AppTextKey::AccountFarmDetailsViewFarmProfileAction",
    "AppTextKey::AccountFarmDetailsFarmTypeVegetableFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeFruitOrchard",
    "AppTextKey::AccountFarmDetailsFarmTypeBerryFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeHerbFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeFlowerFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeMushroomFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeGrainFieldCropFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeDairyFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeEggPoultryFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeLivestockFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeHoneyApiary",
    "AppTextKey::AccountFarmDetailsFarmTypeNurseryPlantFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeMixedFarm",
    "AppTextKey::AccountFarmDetailsFarmTypeOther",
    "AppTextKey::AccountFarmDetailsLocationTitle",
    "AppTextKey::AccountFarmDetailsLocationIntro",
    "AppTextKey::AccountFarmDetailsMapNotImplemented",
    "AppTextKey::AccountFarmDetailsStreetAddressLabel",
    "AppTextKey::AccountFarmDetailsStreetAddressValue",
    "AppTextKey::AccountFarmDetailsCityLabel",
    "AppTextKey::AccountFarmDetailsCityValue",
    "AppTextKey::AccountFarmDetailsProvinceLabel",
    "AppTextKey::AccountFarmDetailsProvinceBritishColumbia",
    "AppTextKey::AccountFarmDetailsProvinceAlberta",
    "AppTextKey::AccountFarmDetailsPostalCodeLabel",
    "AppTextKey::AccountFarmDetailsPostalCodeValue",
    "AppTextKey::AccountFarmDetailsCountryLabel",
    "AppTextKey::AccountFarmDetailsCountryCanada",
    "AppTextKey::AccountFarmDetailsCountryUnitedStates",
    "AppTextKey::AccountFarmDetailsServiceAreaLabel",
    "AppTextKey::AccountFarmDetailsServiceAreaValue",
    "AppTextKey::AccountFarmDetailsServiceAreaHelper",
    "AppTextKey::AccountFarmDetailsExactAddressPublicLabel",
    "AppTextKey::AccountFarmDetailsExactAddressPublicHelper",
    "AppTextKey::AccountFarmDetailsLocationPreviewTitle",
    "AppTextKey::AccountFarmDetailsLocationPreviewHelper",
    "AppTextKey::AccountFarmDetailsOperatingTitle",
    "AppTextKey::AccountFarmDetailsOperatingIntro",
    "AppTextKey::AccountFarmDetailsGrowingPracticesLabel",
    "AppTextKey::AccountFarmDetailsGrowingPracticeRegenerative",
    "AppTextKey::AccountFarmDetailsGrowingPracticeOrganic",
    "AppTextKey::AccountFarmDetailsProductionMethodsLabel",
    "AppTextKey::AccountFarmDetailsProductionMethodOrganicPractices",
    "AppTextKey::AccountFarmDetailsProductionMethodNoSpray",
    "AppTextKey::AccountFarmDetailsSeasonDatesLabel",
    "AppTextKey::AccountFarmDetailsSeasonStartValue",
    "AppTextKey::AccountFarmDetailsSeasonEndValue",
    "AppTextKey::AccountFarmDetailsOrderDaysLabel",
    "AppTextKey::AccountFarmDetailsOrderDaysSummaryValue",
    "AppTextKey::AccountFarmDetailsDayMon",
    "AppTextKey::AccountFarmDetailsDayTue",
    "AppTextKey::AccountFarmDetailsDayWed",
    "AppTextKey::AccountFarmDetailsDayThu",
    "AppTextKey::AccountFarmDetailsDayFri",
    "AppTextKey::AccountFarmDetailsDaySat",
    "AppTextKey::AccountFarmDetailsDaySun",
    "AppTextKey::AccountFarmDetailsAboutProductsLabel",
    "AppTextKey::AccountFarmDetailsAboutProductsValue",
    "AppTextKey::AccountFarmDetailsCertificationsTitle",
    "AppTextKey::AccountFarmDetailsCertificationsHelper",
    "AppTextKey::AccountFarmDetailsCertificationCertifiedOrganic",
    "AppTextKey::AccountFarmDetailsCertificationNaturallyGrown",
    "AppTextKey::AccountFarmDetailsCertificationSmallFamilyFarm",
    "AppTextKey::AccountFarmDetailsCertificationDeliveryAvailable",
    "AppTextKey::AccountFarmDetailsCustomerNoteTitle",
    "AppTextKey::AccountFarmDetailsCustomerNoteHelper",
    "AppTextKey::AccountFarmDetailsCustomerNoteValue",
    "AppTextKey::AccountFarmDetailsProfilePreviewTitle",
    "AppTextKey::AccountFarmDetailsGrowingPracticesSummaryLabel",
    "AppTextKey::AccountFarmDetailsSeasonSummaryLabel",
    "AppTextKey::AccountFarmDetailsOrderDaysSummaryLabel",
    "AppTextKey::AccountFarmDetailsPickupFulfillmentTitle",
    "AppTextKey::AccountFarmDetailsPickupFulfillmentIntro",
    "AppTextKey::AccountFarmDetailsFulfillmentModeLabel",
    "AppTextKey::AccountFarmDetailsFulfillmentPickupOnly",
    "AppTextKey::AccountFarmDetailsFulfillmentDelivery",
    "AppTextKey::AccountFarmDetailsFulfillmentBoth",
    "AppTextKey::AccountFarmDetailsPrimaryPickupLocationLabel",
    "AppTextKey::AccountFarmDetailsPrimaryPickupLocationTitleValue",
    "AppTextKey::AccountFarmDetailsPrimaryPickupLocationAddressValue",
    "AppTextKey::AccountFarmDetailsPickupInstructionsLabel",
    "AppTextKey::AccountFarmDetailsPickupInstructionsValue",
    "AppTextKey::AccountFarmDetailsPickupInstructionsHelper",
    "AppTextKey::AccountFarmDetailsPickupWindowsLabel",
    "AppTextKey::AccountFarmDetailsPickupWindowDayHeader",
    "AppTextKey::AccountFarmDetailsPickupWindowStartHeader",
    "AppTextKey::AccountFarmDetailsPickupWindowEndHeader",
    "AppTextKey::AccountFarmDetailsPickupWindowWednesday",
    "AppTextKey::AccountFarmDetailsPickupWindowSaturday",
    "AppTextKey::AccountFarmDetailsPickupWindowWednesdayStart",
    "AppTextKey::AccountFarmDetailsPickupWindowWednesdayEnd",
    "AppTextKey::AccountFarmDetailsPickupWindowSaturdayStart",
    "AppTextKey::AccountFarmDetailsPickupWindowSaturdayEnd",
    "AppTextKey::AccountFarmDetailsAddPickupWindowAction",
    "AppTextKey::AccountFarmDetailsOrderCutoffLabel",
    "AppTextKey::AccountFarmDetailsOrderCutoffHelper",
    "AppTextKey::AccountFarmDetailsOrderCutoffNoonValue",
    "AppTextKey::AccountFarmDetailsDeliveryRadiusTitle",
    "AppTextKey::AccountFarmDetailsDeliveryRadiusHelper",
    "AppTextKey::AccountFarmDetailsDeliveryRadiusValue",
    "AppTextKey::AccountFarmDetailsDeliveryRadiusUnit",
    "AppTextKey::AccountFarmDetailsDeliveryRadiusNote",
    "AppTextKey::AccountFarmDetailsCustomerExperienceTitle",
    "AppTextKey::AccountFarmDetailsCustomerExperienceIntro",
    "AppTextKey::AccountFarmDetailsCustomerExperiencePickupTitle",
    "AppTextKey::AccountFarmDetailsCustomerExperienceDeliveryTitle",
    "AppTextKey::AccountFarmDetailsCustomerExperienceDeliveryBody",
    "AppTextKey::AccountSettingsTitle",
    "AppTextKey::AccountSettingsNostrRelaysTitle",
    "AppTextKey::AccountSettingsNostrRelaysHelper",
    "AppTextKey::AccountSettingsRelayAccessReadWrite",
    "AppTextKey::AccountSettingsRelayAccessReadOnly",
    "AppTextKey::AccountSettingsRelayMenuAbout",
    "AppTextKey::AccountSettingsRelayMenuView",
    "AppTextKey::AccountSettingsRemoveRelayAction",
    "AppTextKey::AccountSettingsRelayMenuCheckConnection",
    "AppTextKey::AccountSettingsRelayMenuCopy",
    "AppTextKey::AccountSettingsRelayMenuCopyShortcut",
    "AppTextKey::AccountSettingsAddRelayLabel",
    "AppTextKey::AccountSettingsAddRelayPlaceholder",
    "AppTextKey::AccountSettingsAddRelayAction",
    "AppTextKey::AccountSettingsResetRelaysAction",
    "AppTextKey::AccountSettingsDefaultRelaysNote",
    "AppTextKey::AccountSettingsBlossomServerTitle",
    "AppTextKey::AccountSettingsBlossomServerHelper",
    "AppTextKey::AccountSettingsBlossomServerUrlLabel",
    "AppTextKey::AccountSettingsBlossomProductPhotosLabel",
    "AppTextKey::AccountSettingsBlossomProfileFarmMediaLabel",
    "AppTextKey::AccountSettingsResetBlossomServerAction",
    "AppTextKey::AccountSettingsBlossomConnectionHealthy",
    "AppTextKey::AccountSettingsBlossomConnectionLocal",
    "AppTextKey::AccountSettingsBlossomConnectionInvalid",
    "AppTextKey::AccountSettingsBlossomUploadsAvailable",
    "AppTextKey::AccountSettingsBlossomUploadsPending",
    "AppTextKey::AccountSettingsBlossomUploadsUnavailable",
    "AppTextKey::HomeSetupBackAction",
    "AppTextKey::HomeSetupBrowseMarketplaceAction",
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
    "AppTextKey::HomeSetupIssueUnavailableBody",
    "AppTextKey::HomeSetupErrorStartupFailed",
    "AppTextKey::HomeSetupSignerSourceValueBunkerUri",
    "AppTextKey::HomeSetupSignerSourceValueDiscoveryUrl",
    "AppTextKey::HomeSetupSignerPermissionSignEventKind1",
    "AppTextKey::HomeSetupSignerPermissionSwitchRelays",
    "AppTextKey::HomeSetupSignerPermissionAdditional",
    "AppTextKey::HomeSetupSignerErrorEnterSource",
    "AppTextKey::HomeSetupSignerErrorUseSignerUri",
    "AppTextKey::HomeSetupSignerErrorMissingDiscoveryUri",
    "AppTextKey::HomeSetupSignerErrorInvalidDiscoveryUrl",
    "AppTextKey::HomeSetupSignerErrorInvalidRemoteSignerUri",
    "AppTextKey::HomeSetupSignerErrorPendingApprovalExists",
    "AppTextKey::HomeSetupSignerErrorConnectionFailed",
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
    "AppTextKey::HomeNavBrowse",
    "AppTextKey::HomeNavSearch",
    "AppTextKey::HomeNavCart",
    "AppTextKey::HomeNavToday",
    "AppTextKey::HomeNavProducts",
    "AppTextKey::HomeNavOrders",
    "AppTextKey::PersonalSearchFiltersTitle",
    "AppTextKey::PersonalSearchPlaceholder",
    "AppTextKey::PersonalBrowseEmptyTitle",
    "AppTextKey::PersonalBrowseEmptyBody",
    "AppTextKey::PersonalSearchEmptyTitle",
    "AppTextKey::PersonalSearchEmptyBody",
    "AppTextKey::PersonalBrowsePlaceholderBody",
    "AppTextKey::PersonalSearchPlaceholderBody",
    "AppTextKey::PersonalMarketplaceRefreshFailedNotice",
    "AppTextKey::PersonalDetailOpenFailedNotice",
    "AppTextKey::PersonalOrderPlaceFailedNotice",
    "AppTextKey::PersonalOrderCoordinationFailedNotice",
    "AppTextKey::PersonalCartPlaceholderBody",
    "AppTextKey::PersonalOrdersSurfaceBody",
    "AppTextKey::PersonalOrdersEmptyTitle",
    "AppTextKey::PersonalOrdersEmptyBody",
    "AppTextKey::PersonalOrdersListTitle",
    "AppTextKey::PersonalOrdersDetailTitle",
    "AppTextKey::PersonalOrdersDetailFarmLabel",
    "AppTextKey::PersonalOrdersDetailFulfillmentLabel",
    "AppTextKey::PersonalOrdersDetailTotalLabel",
    "AppTextKey::PersonalOrdersDetailNoteLabel",
    "AppTextKey::PersonalOrdersDetailItemsTitle",
    "AppTextKey::PersonalOrdersActionCancel",
    "AppTextKey::PersonalOrdersActionAcceptChange",
    "AppTextKey::PersonalOrdersActionKeepOrder",
    "AppTextKey::PersonalOrdersRepeatDemandTitle",
    "AppTextKey::PersonalOrdersRepeatDemandActionEligible",
    "AppTextKey::PersonalOrdersRepeatDemandActionPartial",
    "AppTextKey::PersonalOrdersRepeatDemandNotePartialSingle",
    "AppTextKey::PersonalOrdersRepeatDemandNotePartialMultiple",
    "AppTextKey::PersonalOrdersRepeatDemandNoteUnavailable",
    "AppTextKey::PersonalCartSurfaceBody",
    "AppTextKey::PersonalOrderSummaryTitle",
    "AppTextKey::PersonalFulfillmentTitle",
    "AppTextKey::PersonalCartRemoveLineAction",
    "AppTextKey::PersonalCartReviewOrderAction",
    "AppTextKey::PersonalCartLineQuantityLabel",
    "AppTextKey::PersonalCartLineUnitPriceLabel",
    "AppTextKey::PersonalCartLineTotalLabel",
    "AppTextKey::PersonalSummaryFarmLabel",
    "AppTextKey::PersonalSummaryItemsLabel",
    "AppTextKey::PersonalSummarySubtotalLabel",
    "AppTextKey::PersonalDetailBackAction",
    "AppTextKey::PersonalDetailQuantityLabel",
    "AppTextKey::PersonalDetailAddToCartAction",
    "AppTextKey::PersonalDetailReplaceCartTitle",
    "AppTextKey::PersonalDetailReplaceCartBody",
    "AppTextKey::PersonalDetailReplaceCartAction",
    "AppTextKey::PersonalDetailKeepCurrentCartAction",
    "AppTextKey::PersonalOrderReviewTitle",
    "AppTextKey::PersonalOrderReviewBackAction",
    "AppTextKey::PersonalOrderReviewContactTitle",
    "AppTextKey::PersonalOrderReviewFieldName",
    "AppTextKey::PersonalOrderReviewFieldEmail",
    "AppTextKey::PersonalOrderReviewFieldPhone",
    "AppTextKey::PersonalOrderReviewFieldOrderNote",
    "AppTextKey::PersonalOrderReviewLocalOnlyBody",
    "AppTextKey::PersonalOrderReviewPlaceOrderAction",
    "AppTextKey::HomeTodayOpenInOrdersAction",
    "AppTextKey::HomeTodayOpenInPackDayAction",
    "AppTextKey::OrdersTitle",
    "AppTextKey::OrdersFiltersTitle",
    "AppTextKey::OrdersSummaryTotal",
    "AppTextKey::OrdersFilterAll",
    "AppTextKey::OrdersStatusNeedsAction",
    "AppTextKey::OrdersStatusScheduled",
    "AppTextKey::OrdersStatusInHandoff",
    "AppTextKey::OrdersStatusCompleted",
    "AppTextKey::OrdersTableTitle",
    "AppTextKey::OrdersColumnOrder",
    "AppTextKey::OrdersColumnStatus",
    "AppTextKey::OrdersColumnWindow",
    "AppTextKey::OrdersColumnPickup",
    "AppTextKey::OrdersColumnAction",
    "AppTextKey::OrdersActionReview",
    "AppTextKey::OrdersEmptyTitle",
    "AppTextKey::OrdersEmptyBody",
    "AppTextKey::OrdersEmptyNeedsActionTitle",
    "AppTextKey::OrdersEmptyNeedsActionBody",
    "AppTextKey::OrdersDetailTitle",
    "AppTextKey::OrdersDetailItemsTitle",
    "AppTextKey::OrdersDetailCustomerLabel",
    "AppTextKey::OrdersDetailWindowLabel",
    "AppTextKey::OrdersDetailPickupLabel",
    "AppTextKey::OrdersDetailTotalLabel",
    "AppTextKey::TradeValidationReceiptSectionLabel",
    "AppTextKey::TradeValidationReceiptRecordedAtLabel",
    "AppTextKey::TradeValidationReceiptResultValid",
    "AppTextKey::TradeValidationReceiptResultNeedsReview",
    "AppTextKey::TradeValidationReceiptTypeListingValidation",
    "AppTextKey::TradeValidationReceiptTypeTradeTransition",
    "AppTextKey::TradeValidationReceiptTypeInventoryState",
    "AppTextKey::TradeValidationReceiptTypeStateCheckpoint",
    "AppTextKey::TradeWorkflowAxisAgreement",
    "AppTextKey::TradeWorkflowAxisRevision",
    "AppTextKey::TradeWorkflowAxisInventory",
    "AppTextKey::TradeWorkflowAxisSource",
    "AppTextKey::TradeWorkflowAgreementRequested",
    "AppTextKey::TradeWorkflowAgreementRevisionProposed",
    "AppTextKey::TradeWorkflowAgreementAgreedPendingRhi",
    "AppTextKey::TradeWorkflowAgreementCommitted",
    "AppTextKey::TradeWorkflowAgreementDeclined",
    "AppTextKey::TradeWorkflowAgreementCancelled",
    "AppTextKey::TradeWorkflowAgreementInvalid",
    "AppTextKey::TradeWorkflowRevisionNone",
    "AppTextKey::TradeWorkflowRevisionChangeProposed",
    "AppTextKey::TradeWorkflowRevisionUpdated",
    "AppTextKey::TradeWorkflowRevisionKeptAsPlaced",
    "AppTextKey::TradeWorkflowInventoryAvailable",
    "AppTextKey::TradeWorkflowInventoryReserved",
    "AppTextKey::TradeWorkflowInventorySoldOut",
    "AppTextKey::TradeWorkflowInventoryNeedsReview",
    "AppTextKey::TradeWorkflowProvenanceApp",
    "AppTextKey::TradeWorkflowProvenanceCli",
    "AppTextKey::TradeWorkflowProvenanceRelay",
    "AppTextKey::TradeWorkflowProvenanceLocalEvents",
    "AppTextKey::TradeWorkflowProvenanceUnknown",
    "AppTextKey::OrdersRemindersTitle",
    "AppTextKey::OrdersReminderLogTitle",
    "AppTextKey::OrdersReminderLogEmptyBody",
    "AppTextKey::PackDayTitle",
    "AppTextKey::PackDayRemindersTitle",
    "AppTextKey::PackDayWindowSummaryTitle",
    "AppTextKey::PackDayTotalsTitle",
    "AppTextKey::PackDayPackListTitle",
    "AppTextKey::PackDayPickupRosterTitle",
    "AppTextKey::PackDayEmptyTitle",
    "AppTextKey::PackDayEmptyBody",
    "AppTextKey::PackDayExportTitle",
    "AppTextKey::PackDayExportReadyTitle",
    "AppTextKey::PackDayExportReadyBody",
    "AppTextKey::PackDayExportUnavailableTitle",
    "AppTextKey::PackDayExportUnavailableBody",
    "AppTextKey::PackDayExportRunningTitle",
    "AppTextKey::PackDayExportRunningBody",
    "AppTextKey::PackDayExportSucceededTitle",
    "AppTextKey::PackDayExportSucceededBody",
    "AppTextKey::PackDayExportFailedTitle",
    "AppTextKey::PackDayExportFailedBody",
    "AppTextKey::PackDayExportAction",
    "AppTextKey::PackDayExportActionRunning",
    "AppTextKey::PackDayExportFolderLabel",
    "AppTextKey::PackDayExportFilesLabel",
    "AppTextKey::PackDayExportErrorLabel",
    "AppTextKey::PackDayBatchPrintAction",
    "AppTextKey::PackDayBatchPrintActionRunning",
    "AppTextKey::PackDayBatchPrintQueuedTitle",
    "AppTextKey::PackDayBatchPrintSucceededTitle",
    "AppTextKey::PackDayBatchPrintFailedTitle",
    "AppTextKey::PackDayBatchPrintFailedPreflightTitle",
    "AppTextKey::PackDayBatchPrintFailedQueueLaunchTitle",
    "AppTextKey::PackDayBatchPrintFailedQueueExitTitle",
    "AppTextKey::PackDayBatchPrintCustomerLabelsAvery5160OverflowFailedTitle",
    "AppTextKey::PackDayPrintCustomerLabelsAvery5160OverflowFailedTitle",
    "AppTextKey::PackDayHostHandoffRevealAction",
    "AppTextKey::PackDayHostHandoffRevealActionRunning",
    "AppTextKey::PackDayHostHandoffOpenPackSheetAction",
    "AppTextKey::PackDayHostHandoffOpenPackSheetActionRunning",
    "AppTextKey::PackDayHostHandoffOpenPickupRosterAction",
    "AppTextKey::PackDayHostHandoffOpenPickupRosterActionRunning",
    "AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction",
    "AppTextKey::PackDayHostHandoffOpenCustomerLabelsActionRunning",
    "AppTextKey::PackDayHostHandoffRevealRunningTitle",
    "AppTextKey::PackDayHostHandoffRevealSucceededTitle",
    "AppTextKey::PackDayHostHandoffRevealFailedTitle",
    "AppTextKey::PackDayHostHandoffOpenPackSheetRunningTitle",
    "AppTextKey::PackDayHostHandoffOpenPackSheetSucceededTitle",
    "AppTextKey::PackDayHostHandoffOpenPackSheetFailedTitle",
    "AppTextKey::PackDayHostHandoffOpenPickupRosterRunningTitle",
    "AppTextKey::PackDayHostHandoffOpenPickupRosterSucceededTitle",
    "AppTextKey::PackDayHostHandoffOpenPickupRosterFailedTitle",
    "AppTextKey::PackDayHostHandoffOpenCustomerLabelsRunningTitle",
    "AppTextKey::PackDayHostHandoffOpenCustomerLabelsSucceededTitle",
    "AppTextKey::PackDayHostHandoffOpenCustomerLabelsFailedTitle",
    "AppTextKey::HomeTodayRemindersTitle",
    "AppTextKey::ReminderDeadlineLabel",
    "AppTextKey::ReminderUrgencyUpcoming",
    "AppTextKey::ReminderUrgencyDueSoon",
    "AppTextKey::ReminderUrgencyOverdue",
    "AppTextKey::ReminderUrgencyBlocking",
    "AppTextKey::ReminderPresentationTitle",
    "AppTextKey::ReminderPresentationDismissAction",
    "AppTextKey::ReminderDeliveryStateScheduled",
    "AppTextKey::ReminderDeliveryStatePresented",
    "AppTextKey::ReminderDeliveryStateAcknowledged",
    "AppTextKey::ReminderDeliveryStateResolved",
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
    "AppTextKey::ProductsEditorFieldCategory",
    "AppTextKey::ProductsEditorFieldUnit",
    "AppTextKey::ProductsEditorFieldPrice",
    "AppTextKey::ProductsEditorFieldStock",
    "AppTextKey::ProductsEditorFieldAvailability",
    "AppTextKey::ProductsEditorFieldStatus",
    "AppTextKey::ProductsEditorAvailabilityEmpty",
    "AppTextKey::ProductsEditorCloseAction",
    "AppTextKey::ProductsEditorSaveAction",
    "AppTextKey::ProductsEditorSaveFailed",
    "AppTextKey::ProductsEditorPublishQueueFailed",
    "AppTextKey::ProductsEditorInvalidPrice",
    "AppTextKey::ProductsEditorInvalidStock",
    "AppTextKey::ProductsEditorPublishReadinessTitle",
    "AppTextKey::ProductsEditorReady",
    "AppTextKey::ProductsEditorBlockerAddProductName",
    "AppTextKey::ProductsEditorBlockerChooseCategory",
    "AppTextKey::ProductsEditorBlockerChooseUnit",
    "AppTextKey::ProductsEditorBlockerSetPrice",
    "AppTextKey::ProductsEditorBlockerSetStock",
    "AppTextKey::ProductsEditorBlockerAttachAvailability",
    "AppTextKey::ProductsUntitledDraft",
    "AppTextKey::ProductsStockEditorTitle",
    "AppTextKey::ProductsStockEditorFieldLabel",
    "AppTextKey::ProductsStockEditorSaveAction",
    "AppTextKey::ProductsStockEditorCancelAction",
    "AppTextKey::ProductsStockEditorInvalidQuantity",
    "AppTextKey::ProductsStockEditorSaveFailed",
    "AppTextKey::ProductsStockEditorPublishQueueFailed",
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
    "AppTextKey::SettingsAccountImportFileAction",
    "AppTextKey::SettingsAccountImportDatabaseAction",
    "AppTextKey::SettingsAccountConnectRemoteBunkerAction",
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
    "AppTextKey::SettingsAboutCompanyName",
    "AppTextKey::SettingsAboutVersionLabel",
    "AppTextKey::SettingsAboutVariantLabel",
    "AppTextKey::SettingsAboutAcknowledgementsAction",
    "AppTextKey::SettingsAboutPrivacyPolicyAction",
    "AppTextKey::SettingsAboutTermsAction",
    "AppTextKey::SettingsAboutReportIssueAction",
    "AppTextKey::SettingsAboutCopyrightNotice",
    "AppTextKey::SettingsAboutTrademarkNotice",
    "AppTextKey::SettingsAboutStatusSectionLabel",
    "AppTextKey::SettingsAboutConflictReviewSectionLabel",
    "AppTextKey::SettingsAboutRuntimeSectionLabel",
    "AppTextKey::SettingsAboutConflictReviewUnavailable",
    "AppTextKey::SettingsAboutConflictReviewClear",
    "AppTextKey::SettingsAboutConflictReviewNeedsAttention",
    "AppTextKey::SettingsAboutConflictReviewBlocking",
    "AppTextKey::MetadataSelectedAccount",
    "AppTextKey::MetadataSyncPendingWriteCount",
    "AppTextKey::MetadataSyncBlockingConflictCount",
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

const FORBIDDEN_HARDCODED_WORKFLOW_UI_LITERALS: &[&str] = &[
    "Agreement",
    "Change",
    "Fulfillment",
    "Stock",
    "Requested",
    "Agreed",
    "Payment",
    "Source",
    "Ordered",
    "Confirmed",
    "Declined",
    "Cancelled",
    "Completed",
    "Needs review",
    "No change",
    "Change proposed",
    "Updated",
    "Kept as placed",
    "Preparing",
    "Ready for pickup",
    "Out for delivery",
    "Delivered",
    "Available",
    "Reserved",
    "Sold out",
    "Not recorded",
    "Pending",
    "Recorded",
    "Settled",
    "App",
    "CLI",
    "Relay",
    "Local events",
    "Unknown",
];

const FORBIDDEN_STALE_SELLER_LIFECYCLE_PATTERNS: &[&str] = &[
    concat!("orders-detail-", "mark-packed"),
    concat!("orders-detail-", "mark-completed"),
    concat!("orders-row-action-", "mark-packed"),
    concat!("orders-row-action-", "mark-completed"),
    concat!("orders.", "mark_delivered_failed"),
    concat!("OrderPrimaryAction::", "MarkPacked"),
    concat!("OrderPrimaryAction::", "MarkCompleted"),
    concat!("mark", "_packed"),
    concat!("mark", "_completed"),
    concat!("AppTextKey::", "OrdersStatus", "Packed"),
    concat!("AppTextKey::", "OrdersAction", "MarkPacked"),
    concat!("AppTextKey::", "OrdersAction", "MarkCompleted"),
    concat!("orders.status.", "packed"),
    concat!("orders.action.", "mark_packed"),
    concat!("orders.action.", "mark_completed"),
];

const FORBIDDEN_PAYMENT_DEFERRAL_COPY_PATTERNS: &[&str] = &[
    "payments are deferred",
    "payment is deferred",
    "payment deferred",
    "payments deferred",
    "deferred payment",
    "deferred payments",
    "checkout unavailable",
    "figure it out",
    "payment handling outside the app",
    "refund outside the app",
    "handle any refund outside the app",
    "settle outside the app",
];

const FORBIDDEN_PAYMENT_ACTION_COPY_TERMS: &[&str] = &[
    "checkout",
    "pay",
    "refund",
    "settlement",
    "wallet",
    "invoice",
    "bank",
    "card",
    "processor",
    "provider",
    "payment-provider",
    "payment provider",
];

const FORBIDDEN_PRODUCTION_EVENT_KIND_LITERALS: &[(&str, &str)] = &[
    ("30340", "KIND_FARM"),
    ("30402", "KIND_LISTING"),
    ("30403", "KIND_LISTING_DRAFT"),
    ("3422", "KIND_ORDER_REQUEST"),
    ("3423", "KIND_ORDER_DECISION"),
    ("3424", "KIND_ORDER_REVISION_PROPOSAL"),
    ("3425", "KIND_ORDER_REVISION_DECISION"),
    ("3426", "KIND_TRADE_QUESTION"),
    ("3427", "KIND_TRADE_ANSWER"),
    ("3428", "KIND_TRADE_DISCOUNT_REQUEST"),
    ("3429", "KIND_TRADE_DISCOUNT_OFFER"),
    ("3430", "KIND_TRADE_DISCOUNT_ACCEPT"),
    ("3431", "RESERVED_ORDER_KIND_3431"),
    ("3432", "KIND_ORDER_CANCELLATION"),
    ("3433", "KIND_ORDER_FULFILLMENT_UPDATE"),
    ("3434", "KIND_ORDER_RECEIPT"),
    ("3435", "KIND_ORDER_PAYMENT_RECORD"),
    ("3436", "KIND_ORDER_SETTLEMENT_DECISION"),
    ("3440", "KIND_TRADE_VALIDATION_RECEIPT"),
];

struct SdkBoundaryForbiddenPattern {
    pattern: &'static str,
    reason: &'static str,
}

struct SdkBoundaryExceptionEntry {
    path: &'static str,
    pattern: &'static str,
    owner: &'static str,
    reason: &'static str,
    removal_condition: &'static str,
}

struct SdkBoundaryFinding {
    pattern: &'static str,
    reason: &'static str,
    line: usize,
}

const STRICT_SDK_BOUNDARY_FORBIDDEN_PATTERNS: &[SdkBoundaryForbiddenPattern] = &[
    SdkBoundaryForbiddenPattern {
        pattern: "SdkDirectRelayAppSyncTransport",
        reason: "app production sources must use AppSdkRuntime instead of direct relay sync transport",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsSdkClient",
        reason: "app production sources must use the long-lived RadrootsClient runtime boundary",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsSdkConfig",
        reason: "app production sources must use AppSdkConfig-derived runtime construction",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "status_client(",
        reason: "app production sources must use canonical grouped SDK trade status workflows instead of removed SDK status clients",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "TradeStatusClient",
        reason: "app production sources must use canonical grouped SDK trade status workflows instead of removed SDK status handles",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "TradeValidationClient",
        reason: "app production sources must use AppSdkRuntime DVM methods instead of removed SDK validation handles",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "SdkTransportMode::RelayDirect",
        reason: "app production sources must not configure direct relay publish transport",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "SignerConfig::LocalIdentity",
        reason: "app production sources must not configure local direct-publish signing",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "PendingSyncOperation::from_publish_payload",
        reason: "app production sources must not enqueue app publish payloads outside SDK workflow APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: ".enqueue_pending_operation(",
        reason: "app production sources must not mutate the app outbox outside SDK workflow APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "INSERT INTO local_outbox",
        reason: "app production sources must not write local outbox rows outside SDK workflow APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "UPDATE local_outbox",
        reason: "app production sources must not mutate local outbox rows outside SDK workflow APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "DELETE FROM local_outbox",
        reason: "app production sources must not delete local outbox rows outside SDK workflow APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsOutbox",
        reason: "canonical SDK outbox access belongs inside the SDK crate",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "enqueue_signed_operation",
        reason: "canonical SDK outbox writes must go through SDK APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "claim_ready_events",
        reason: "canonical SDK outbox push claims must go through SDK APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "connected_client_from_identity",
        reason: "app production sources must not connect relay clients directly for publish",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_signed_event",
        reason: "app production sources must not publish signed events directly",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "radroots_nostr_build_event",
        reason: "app production sources must not build protocol events outside SDK-owned publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsIdentity::from_secret_key_str",
        reason: "app production sources must not parse direct signing keys",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RawSecretKey",
        reason: "app production sources must not import raw signing-key material",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "EncryptedSecretKey",
        reason: "app production sources must not import encrypted signing-key material",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_with_identity",
        reason: "app production sources must not call direct SDK publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_draft_with_identity",
        reason: "app production sources must not encode direct SDK publish targets",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_request_with_identity",
        reason: "app production sources must not call direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_decision_with_identity",
        reason: "app production sources must not call direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_revision_proposal_with_identity",
        reason: "app production sources must not call direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_revision_decision_with_identity",
        reason: "app production sources must not call direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_cancellation_with_identity",
        reason: "app production sources must not call direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_fulfillment_update_with_identity",
        reason: "app production sources must not call direct SDK fulfillment publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_buyer_receipt_with_identity",
        reason: "app production sources must not call direct SDK receipt publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "radroots_sdk::protocol::order",
        reason: "app production sources must not import SDK protocol order bypasses",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "AppSdkOrder",
        reason: "app production sources must use AppSdkTrade workflow request types",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "AppSdkMigration",
        reason: "app production sources must not keep retired SDK workflow scaffolding",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "sdk_migration",
        reason: "app production sources must not keep retired SDK workflow scaffolding",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "migration_receipt",
        reason: "app production sources must not keep retired SDK receipt scaffolding",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "migration_audit",
        reason: "app production sources must not keep retired SDK audit scaffolding",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "ORDER_SUBMIT_OPERATION_KIND",
        reason: "app production sources must use trade workflow operation kinds",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "ORDER_DECISION_OPERATION_KIND",
        reason: "app production sources must use trade workflow operation kinds",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "ORDER_REVISION_PROPOSAL_OPERATION_KIND",
        reason: "app production sources must use trade workflow operation kinds",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "ORDER_REVISION_DECISION_OPERATION_KIND",
        reason: "app production sources must use trade workflow operation kinds",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "ORDER_CANCELLATION_OPERATION_KIND",
        reason: "app production sources must use trade workflow operation kinds",
    },
];

const FORBIDDEN_SDK_ROOT_TRADE_ALIAS_NAMES: &[&str] = &[
    "trade_buyer",
    "trade_seller",
    "trade_status",
    "trade_resync",
    "trade_validation",
];

const SDK_BOUNDARY_EXCEPTIONS: &[SdkBoundaryExceptionEntry] = &[
    SdkBoundaryExceptionEntry {
        path: "crates/desktop/src/accounts.rs",
        pattern: "RadrootsIdentity::from_secret_key_str",
        owner: "rpv1-app-sdk-hardening.04",
        reason: "desktop account import still accepts local raw secret-key material for account bootstrap",
        removal_condition: "remove when local account import is mediated by protected signer adapters instead of raw key parsing",
    },
    SdkBoundaryExceptionEntry {
        path: "crates/desktop/src/accounts.rs",
        pattern: "RawSecretKey",
        owner: "rpv1-app-sdk-hardening.04",
        reason: "desktop account import still exposes a raw secret-key import mode for account bootstrap",
        removal_condition: "remove when local account import is mediated by protected signer adapters instead of raw key import modes",
    },
    SdkBoundaryExceptionEntry {
        path: "crates/desktop/src/accounts.rs",
        pattern: "EncryptedSecretKey",
        owner: "rpv1-app-sdk-hardening.04",
        reason: "desktop account import still exposes an encrypted secret-key import mode for account bootstrap",
        removal_condition: "remove when local account import is mediated by protected signer adapters instead of secret-key import modes",
    },
    SdkBoundaryExceptionEntry {
        path: "crates/signer/src/protocol.rs",
        pattern: "RadrootsIdentity::from_secret_key_str",
        owner: "rpv1-sdksign.5",
        reason: "remote signer startup custody still reloads the NIP-46 client identity before shared protocol transport execution",
        removal_condition: "remove when startup remote signer custody stores client identities through protected signer-session APIs",
    },
    SdkBoundaryExceptionEntry {
        path: "crates/store/src/lib.rs",
        pattern: ".enqueue_pending_operation(",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "store facade accepts app local_outbox publish operations for deferred workflows",
        removal_condition: "remove when app local_outbox enqueue is replaced by SDK canonical outbox enqueue APIs",
    },
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
fn desktop_window_source_uses_settings_width_theme_token() {
    let source = include_str!("window.rs");

    assert!(
        !source.contains("Some(560.0)"),
        "settings panel width caps must use APP_UI_THEME.shells.settings_panel_content_max_width_px"
    );
    assert!(
        source.contains("settings_panel_content_max_width_px"),
        "settings panel width token is not used by window.rs"
    );
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

#[test]
fn app_sources_use_publish_lifecycle_action_identifiers() {
    for (path, source) in seller_lifecycle_action_owner_sources() {
        for pattern in FORBIDDEN_STALE_SELLER_LIFECYCLE_PATTERNS {
            assert!(
                !source.contains(pattern),
                "{} still contains stale seller lifecycle action pattern `{pattern}`",
                path.display()
            );
        }
    }
}

#[test]
fn desktop_window_source_does_not_use_about_placeholder_copy() {
    let source = include_str!("window.rs");

    assert!(
        !source.contains("SettingsAboutPlaceholder"),
        "window.rs still references retired about placeholder copy"
    );
}

#[test]
fn desktop_sources_do_not_hardcode_workflow_ui_copy() {
    for (path, source) in launcher_source_files() {
        let literals = extract_string_literals(&source);
        for literal in literals {
            for forbidden_literal in FORBIDDEN_HARDCODED_WORKFLOW_UI_LITERALS {
                assert_ne!(
                    literal,
                    *forbidden_literal,
                    "{} hardcodes workflow UI copy `{forbidden_literal}`",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn desktop_sources_do_not_expose_reserved_payment_action_copy() {
    for (path, source) in launcher_source_files() {
        for literal in extract_string_literals(&source) {
            let normalized_literal = literal.to_lowercase();
            for pattern in FORBIDDEN_PAYMENT_DEFERRAL_COPY_PATTERNS {
                assert!(
                    !normalized_literal.contains(pattern),
                    "{} contains forbidden payment-deferral copy `{pattern}`",
                    path.display()
                );
            }
            for term in FORBIDDEN_PAYMENT_ACTION_COPY_TERMS {
                assert!(
                    !contains_reserved_payment_action_term(&normalized_literal, term),
                    "{} contains reserved payment action copy `{term}` in `{literal}`",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn app_production_trade_event_kinds_use_shared_constants() {
    assert_production_source_omits_event_kind_literals(
        "crates/desktop/src/runtime.rs",
        include_str!("runtime.rs"),
    );

    let store_interop_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("desktop crate should live under app crates directory")
        .join("crates/store/src/interop.rs");
    let store_interop_source =
        fs::read_to_string(store_interop_path.as_path()).unwrap_or_else(|error| {
            panic!(
                "failed to read app store interop source {}: {error}",
                store_interop_path.display()
            )
        });

    assert_production_source_omits_event_kind_literals(
        "crates/store/src/interop.rs",
        store_interop_source.as_str(),
    );
}

#[test]
fn app_production_sdk_boundary_usage_is_exception_scoped() {
    for (relative_path, source) in app_rust_source_files() {
        let production_source = production_source_without_tests(relative_path.as_str(), &source)
            .unwrap_or_else(|error| {
                panic!("{} source classification failed: {error}", relative_path)
            });
        let findings =
            unexcepted_sdk_boundary_patterns(relative_path.as_str(), production_source.as_str());

        assert!(
            findings.is_empty(),
            "{}:{} contains unexcepted SDK boundary pattern `{}`: {}",
            relative_path,
            findings.first().map_or(0, |finding| finding.line),
            findings.first().map_or("", |finding| finding.pattern),
            findings.first().map_or("", |finding| finding.reason),
        );

        let root_alias_findings =
            sdk_root_trade_alias_findings(relative_path.as_str(), production_source.as_str());
        assert!(
            root_alias_findings.is_empty(),
            "{} contains removed SDK root trade alias usage:\n{}",
            relative_path,
            root_alias_findings.join("\n")
        );
    }
}

#[test]
fn app_store_current_schema_surfaces_reject_retired_terms() {
    let app_root = app_root();
    let mut paths = vec![
        app_root.join("crates/store/src/lib.rs"),
        app_root.join("crates/sync/src/publish.rs"),
    ];
    for entry in
        fs::read_dir(app_root.join("crates/store/migrations")).expect("read store migrations")
    {
        let path = entry.expect("migration entry").path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
            paths.push(path);
        }
    }

    let offenders = paths
        .iter()
        .flat_map(|path| {
            let source = read_source_path(path);
            let relative_path = path
                .strip_prefix(app_root.as_path())
                .expect("app relative path")
                .display()
                .to_string();
            APP_STORE_RETIRED_SCHEMA_TERMS
                .iter()
                .filter(move |term| source.contains(**term))
                .map(move |term| (relative_path.clone(), *term))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "app store current-schema surfaces retain retired terms: {offenders:?}"
    );
}

const APP_STORE_RETIRED_SCHEMA_TERMS: &[&str] = &[
    "legacy",
    "compat",
    "shim",
    "deprecated",
    "dual_read",
    "dual_write",
    "dual-read",
    "dual-write",
    "AppSdkMigration",
    "sdk_migration",
    "migration_receipt",
    "migration_audit",
    "migration_scaffold",
];

#[test]
fn strict_sdk_boundary_scanner_rejects_unexcepted_new_production_paths() {
    let findings = unexcepted_sdk_boundary_patterns(
        "crates/desktop/src/new_workflow.rs",
        "fn publish() { let _ = RadrootsSdkClient::from_config(config); }",
    );

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].pattern, "RadrootsSdkClient");
    let runtime_findings = unexcepted_sdk_boundary_patterns(
        "crates/desktop/src/runtime.rs",
        "fn publish() { let _ = RadrootsSdkClient::from_config(config); }",
    );
    assert_eq!(runtime_findings.len(), 1);
    assert_eq!(runtime_findings[0].pattern, "RadrootsSdkClient");
    let status_findings = unexcepted_sdk_boundary_patterns(
        "crates/desktop/src/runtime.rs",
        "fn status() { let _ = TradeStatusClient::new(root); root.status_client(); }",
    );
    assert_eq!(status_findings.len(), 2);
    assert!(
        status_findings
            .iter()
            .any(|finding| finding.pattern == "TradeStatusClient")
    );
    assert!(
        status_findings
            .iter()
            .any(|finding| finding.pattern == "status_client(")
    );
    assert!(
        unexcepted_sdk_boundary_patterns(
            "crates/desktop/src/accounts.rs",
            "fn import() { let _ = RawSecretKey; }",
        )
        .is_empty()
    );
}

#[test]
fn strict_sdk_boundary_scanner_rejects_removed_root_trade_alias_calls() {
    let allowed_findings = sdk_root_trade_alias_findings(
        "crates/runtime/src/sdk.rs",
        "fn trade_status_for_locator() {}",
    );
    assert!(allowed_findings.is_empty());

    let findings = sdk_root_trade_alias_findings(
        "crates/desktop/src/runtime.rs",
        "sdk.trade_status (request); RadrootsClient::trade_resync(&sdk);",
    );

    for alias in ["trade_status", "trade_resync"] {
        assert!(
            findings.iter().any(|finding| finding.contains(alias)),
            "strict SDK boundary scanner must reject `{alias}`"
        );
    }
}

#[test]
fn production_source_scanner_strips_inline_cfg_test_modules() {
    let source = concat!(
        "fn production() {}\n",
        "#[cfg(test)] mod tests { fn test_only() { sdk.trade_status(request); } }\n",
        "fn after_tests() {}\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");

    assert!(!production_source.contains("sdk.trade_status"));
    assert!(production_source.contains("fn production()"));
    assert!(production_source.contains("fn after_tests()"));
    assert!(sdk_root_trade_alias_findings("fixture.rs", production_source.as_str()).is_empty());
}

#[test]
fn production_source_scanner_strips_multiline_cfg_test_modules() {
    let source = concat!(
        "fn production() {}\n",
        "#[cfg(test)]\n",
        "#[allow(dead_code)]\n",
        "mod tests {\n",
        "    fn test_only() { let _ = TradeStatusClient::new(root); }\n",
        "    const BRACE: &str = \"}\";\n",
        "}\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");

    assert!(!production_source.contains("TradeStatusClient"));
    assert!(production_source.contains("fn production()"));
    assert!(unexcepted_sdk_boundary_patterns("fixture.rs", production_source.as_str()).is_empty());
}

#[test]
fn production_source_scanner_strips_cfg_test_functions() {
    let source = concat!(
        "fn production() {}\n",
        "#[cfg(test)]\n",
        "fn test_only() { let _ = TradeValidationClient::new(root); }\n",
        "fn after_tests() {}\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");

    assert!(!production_source.contains("TradeValidationClient"));
    assert!(production_source.contains("fn production()"));
    assert!(production_source.contains("fn after_tests()"));
    assert!(unexcepted_sdk_boundary_patterns("fixture.rs", production_source.as_str()).is_empty());
}

#[test]
fn production_source_scanner_strips_cfg_test_fragments() {
    let source = concat!(
        "enum Provider {",
        "#[cfg(test)] TestOnly,",
        "Production",
        "}\n",
        "fn production(provider: Provider) { match provider {",
        "#[cfg(test)] Provider::TestOnly => sdk.trade_status(request),",
        "Provider::Production => {}",
        "} }\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");

    assert!(!production_source.contains("TestOnly"));
    assert!(sdk_root_trade_alias_findings("fixture.rs", production_source.as_str()).is_empty());
}

#[test]
fn production_source_scanner_reports_malformed_cfg_test_items() {
    let source = concat!(
        "fn production() {}\n",
        "#[cfg(test)]\n",
        "mod tests { fn hidden() { let _ = TradeValidationClient::new(root); }\n",
        "fn after_tests() { sdk.trade_status(request); }\n",
    );
    let error = production_source_without_tests("fixture.rs", source).expect_err("classification");

    assert!(error.contains("fixture.rs:2"));
    assert!(error.contains("cfg(test) item is not closed"));
}

#[test]
fn production_surface_scanners_ignore_comments_and_literals() {
    let source = concat!(
        "fn production() {\n",
        "    let literal = \"TradeStatusClient\";\n",
        "    let raw = r#\"RadrootsClient::trade_resync(&sdk)\"#;\n",
        "    let character = 'x';\n",
        "}\n",
        "// RadrootsSdkClient::from_config(config)\n",
        "/* sdk.trade_validation(request) */\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");

    assert!(unexcepted_sdk_boundary_patterns("fixture.rs", production_source.as_str()).is_empty());
    assert!(sdk_root_trade_alias_findings("fixture.rs", production_source.as_str()).is_empty());
}

#[test]
fn production_source_scanner_keeps_production_violations() {
    let source = concat!(
        "fn production() { sdk.trade_resync(request); }\n",
        "#[cfg(test)]\n",
        "mod tests { fn test_only() { sdk.trade_status(request); } }\n",
    );
    let production_source =
        production_source_without_tests("fixture.rs", source).expect("production source");
    let findings = sdk_root_trade_alias_findings("fixture.rs", production_source.as_str());

    assert!(
        findings
            .iter()
            .any(|finding| finding.contains("trade_resync"))
    );
    assert!(
        !findings
            .iter()
            .any(|finding| finding.contains("trade_status"))
    );
}

#[test]
fn app_sdk_boundary_exception_entries_are_complete_and_current() {
    let app_root = app_root();
    let mut entries = BTreeSet::new();

    for entry in SDK_BOUNDARY_EXCEPTIONS {
        assert!(
            entries.insert((entry.path, entry.pattern)),
            "duplicate SDK boundary exception entry {} `{}`",
            entry.path,
            entry.pattern
        );
        assert!(
            !entry.owner.trim().is_empty(),
            "{} `{}` is missing an owner",
            entry.path,
            entry.pattern
        );
        assert!(
            !entry.reason.trim().is_empty(),
            "{} `{}` is missing a reason",
            entry.path,
            entry.pattern
        );
        assert!(
            !entry.removal_condition.trim().is_empty(),
            "{} `{}` is missing a removal condition",
            entry.path,
            entry.pattern
        );

        let source_path = app_root.join(entry.path);
        let source = read_source_path(source_path.as_path());
        let production_source = production_source_without_tests(entry.path, &source)
            .unwrap_or_else(|error| panic!("{} source classification failed: {error}", entry.path));
        assert!(
            production_source.as_str().contains(entry.pattern),
            "{} declares SDK boundary exception pattern `{}` that is no longer present",
            entry.path,
            entry.pattern
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

fn assert_production_source_omits_event_kind_literals(path: &str, source: &str) {
    let production_source =
        production_source_without_tests(path, source).expect("production source");
    for (literal, constant_name) in FORBIDDEN_PRODUCTION_EVENT_KIND_LITERALS {
        assert!(
            !contains_numeric_token(production_source.as_str(), literal),
            "{path} uses raw event kind {literal}; use shared {constant_name} instead"
        );
    }
}

fn production_source_without_tests(path: &str, source: &str) -> Result<String, String> {
    let code_source = rust_code_without_non_code(path, source)?;
    let mut production_source = String::with_capacity(code_source.len());
    let mut cursor = 0;

    while let Some((attribute_start, attribute_end)) =
        find_cfg_test_attribute(code_source.as_str(), cursor)
    {
        let item_end =
            find_cfg_test_item_end(path, code_source.as_str(), attribute_start, attribute_end)?;

        production_source.push_str(&code_source[cursor..attribute_start]);
        push_masked_source(
            &mut production_source,
            &code_source[attribute_start..item_end],
        );
        cursor = item_end;
    }

    production_source.push_str(&code_source[cursor..]);
    Ok(production_source)
}

fn rust_code_without_non_code(path: &str, source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut code = String::with_capacity(source.len());
    let mut cursor = 0;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => {
                let end = skip_quoted_rust_literal(source, cursor, b'"').ok_or_else(|| {
                    classification_error(path, source, cursor, "unterminated string literal")
                })?;
                push_masked_source(&mut code, &source[cursor..end]);
                cursor = end;
            }
            b'\'' => {
                if let Some(end) = skip_rust_char_literal(source, cursor) {
                    push_masked_source(&mut code, &source[cursor..end]);
                    cursor = end;
                } else {
                    let character = source[cursor..].chars().next().expect("quote");
                    code.push(character);
                    cursor += character.len_utf8();
                }
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'/') => {
                let end = source[cursor..]
                    .find('\n')
                    .map_or(source.len(), |newline| cursor + newline);
                push_masked_source(&mut code, &source[cursor..end]);
                cursor = end;
            }
            b'/' if bytes.get(cursor + 1) == Some(&b'*') => {
                let end = skip_rust_block_comment(source, cursor).ok_or_else(|| {
                    classification_error(path, source, cursor, "unterminated block comment")
                })?;
                push_masked_source(&mut code, &source[cursor..end]);
                cursor = end;
            }
            b'r' => {
                if let Some(end) = skip_raw_rust_string(source, cursor) {
                    push_masked_source(&mut code, &source[cursor..end]);
                    cursor = end;
                } else {
                    code.push('r');
                    cursor += 1;
                }
            }
            _ => {
                let character = source[cursor..].chars().next().expect("source character");
                code.push(character);
                cursor += character.len_utf8();
            }
        }
    }

    Ok(code)
}

fn push_masked_source(output: &mut String, source: &str) {
    for character in source.chars() {
        if character == '\n' {
            output.push('\n');
        } else {
            output.push(' ');
        }
    }
}

fn find_cfg_test_attribute(source: &str, start: usize) -> Option<(usize, usize)> {
    let mut cursor = start;
    while let Some(relative_start) = source[cursor..].find("#[") {
        let attribute_start = cursor + relative_start;
        let content_start = attribute_start + 2;
        let content_end = source[content_start..]
            .find(']')
            .map(|relative_end| content_start + relative_end)?;
        let normalized = source[content_start..content_end]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let attribute_end = content_end + 1;
        if normalized == "cfg(test)" {
            return Some((attribute_start, attribute_end));
        }
        cursor = attribute_end;
    }
    None
}

fn find_cfg_test_item_end(
    path: &str,
    source: &str,
    attribute_start: usize,
    attribute_end: usize,
) -> Result<usize, String> {
    let mut cursor = skip_rust_whitespace(source, attribute_end);
    while source[cursor..].starts_with("#[") {
        let attribute_end = source[cursor..]
            .find(']')
            .map(|end| cursor + end + 1)
            .ok_or_else(|| classification_error(path, source, cursor, "unterminated attribute"))?;
        cursor = skip_rust_whitespace(source, attribute_end);
    }

    cursor = skip_optional_visibility(path, source, cursor)?;
    find_rust_item_end(path, source, attribute_start, cursor)
}

fn skip_optional_visibility(path: &str, source: &str, cursor: usize) -> Result<usize, String> {
    if !starts_with_rust_keyword(source, cursor, "pub") {
        return Ok(cursor);
    }

    let mut cursor = skip_rust_whitespace(source, cursor + "pub".len());
    if source[cursor..].starts_with('(') {
        cursor = skip_balanced_parentheses(source, cursor)
            .ok_or_else(|| classification_error(path, source, cursor, "malformed visibility"))?;
        cursor = skip_rust_whitespace(source, cursor);
    }
    Ok(cursor)
}

fn find_rust_item_end(
    path: &str,
    source: &str,
    attribute_start: usize,
    item_start: usize,
) -> Result<usize, String> {
    let bytes = source.as_bytes();
    let mut cursor = item_start;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut saw_brace = false;

    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' => paren_depth += 1,
            b')' => {
                paren_depth = paren_depth.checked_sub(1).ok_or_else(|| {
                    classification_error(path, source, cursor, "unbalanced closing parenthesis")
                })?;
            }
            b'[' => bracket_depth += 1,
            b']' => {
                bracket_depth = bracket_depth.checked_sub(1).ok_or_else(|| {
                    classification_error(path, source, cursor, "unbalanced closing bracket")
                })?;
            }
            b'{' if paren_depth == 0 && bracket_depth == 0 => {
                brace_depth += 1;
                saw_brace = true;
            }
            b'}' if paren_depth == 0 && bracket_depth == 0 => {
                brace_depth = brace_depth.checked_sub(1).ok_or_else(|| {
                    classification_error(path, source, cursor, "unbalanced closing brace")
                })?;
                cursor += 1;
                if saw_brace && brace_depth == 0 {
                    return Ok(cursor);
                }
                continue;
            }
            b';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Ok(cursor + 1);
            }
            b',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                return Ok(cursor + 1);
            }
            _ => {}
        }
        cursor += 1;
    }

    Err(classification_error(
        path,
        source,
        attribute_start,
        "cfg(test) item is not closed",
    ))
}

fn skip_balanced_parentheses(source: &str, open_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative_index, character) in source[open_index..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open_index + relative_index + character.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn skip_rust_whitespace(source: &str, mut cursor: usize) -> usize {
    while cursor < source.len() {
        let Some(character) = source[cursor..].chars().next() else {
            return cursor;
        };
        if !character.is_whitespace() {
            return cursor;
        }
        cursor += character.len_utf8();
    }
    cursor
}

fn starts_with_rust_keyword(source: &str, cursor: usize, keyword: &str) -> bool {
    source[cursor..].starts_with(keyword)
        && source[cursor + keyword.len()..]
            .chars()
            .next()
            .is_none_or(|character| !is_rust_identifier_character(character))
}

fn skip_rust_block_comment(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start;
    let mut depth = 0usize;

    while cursor + 1 < bytes.len() {
        if bytes[cursor] == b'/' && bytes[cursor + 1] == b'*' {
            depth += 1;
            cursor += 2;
            continue;
        }
        if bytes[cursor] == b'*' && bytes[cursor + 1] == b'/' {
            depth = depth.checked_sub(1)?;
            cursor += 2;
            if depth == 0 {
                return Some(cursor);
            }
            continue;
        }
        cursor += 1;
    }

    None
}

fn skip_quoted_rust_literal(source: &str, start: usize, delimiter: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    let mut escaped = false;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == delimiter {
            return Some(cursor + 1);
        }
        cursor += 1;
    }
    None
}

fn skip_rust_char_literal(source: &str, start: usize) -> Option<usize> {
    let end = skip_quoted_rust_literal(source, start, b'\'')?;
    let literal = &source[start + 1..end - 1];
    if literal.starts_with('\\') || literal.chars().count() == 1 {
        Some(end)
    } else {
        None
    }
}

fn skip_raw_rust_string(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    let mut hashes = 0usize;

    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }

    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    cursor += 1;

    while cursor < bytes.len() {
        if bytes[cursor] == b'"' {
            let mut matched = true;
            for offset in 0..hashes {
                if bytes.get(cursor + 1 + offset) != Some(&b'#') {
                    matched = false;
                    break;
                }
            }
            if matched {
                return Some(cursor + 1 + hashes);
            }
        }
        cursor += 1;
    }

    None
}

fn unexcepted_sdk_boundary_patterns(
    path: &str,
    production_source: &str,
) -> Vec<SdkBoundaryFinding> {
    STRICT_SDK_BOUNDARY_FORBIDDEN_PATTERNS
        .iter()
        .flat_map(|forbidden| {
            production_source
                .match_indices(forbidden.pattern)
                .map(move |(index, _)| (forbidden, index))
        })
        .filter(|(forbidden, _)| !sdk_boundary_exception_contains(path, forbidden.pattern))
        .map(|(forbidden, index)| SdkBoundaryFinding {
            pattern: forbidden.pattern,
            reason: forbidden.reason,
            line: line_number(production_source, index),
        })
        .collect()
}

fn sdk_boundary_exception_contains(path: &str, pattern: &str) -> bool {
    SDK_BOUNDARY_EXCEPTIONS
        .iter()
        .any(|entry| entry.path == path && entry.pattern == pattern)
}

fn sdk_root_trade_alias_findings(path: &str, production_source: &str) -> Vec<String> {
    let mut findings = Vec::new();

    for alias in FORBIDDEN_SDK_ROOT_TRADE_ALIAS_NAMES {
        for (index, _) in production_source.match_indices(alias) {
            let before = production_source[..index].chars().next_back();
            let after_index = index + alias.len();
            let after = production_source[after_index..].chars().next();

            if before.is_some_and(is_rust_identifier_character)
                || after.is_some_and(is_rust_identifier_character)
            {
                continue;
            }

            if production_source[after_index..]
                .chars()
                .find(|character| !character.is_whitespace())
                != Some('(')
            {
                continue;
            }

            let prefix = production_source[..index].trim_end();
            if prefix.ends_with('.') || prefix.ends_with("::") {
                findings.push(format!(
                    "{path}:{} uses removed SDK root trade alias `{alias}`",
                    line_number(production_source, index)
                ));
            }
        }
    }

    findings
}

fn read_source_path(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read source {}: {error}", path.display()))
}

fn app_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("desktop crate should live under app crates directory")
        .to_path_buf()
}

fn app_rust_source_files() -> Vec<(String, String)> {
    let app_root = app_root();
    let mut paths = Vec::new();
    collect_rust_source_files(app_root.join("crates").as_path(), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("source_guards.rs"))
        .map(|path| {
            let relative_path = path
                .strip_prefix(app_root.as_path())
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to derive app-relative source path {}: {error}",
                        path.display()
                    )
                })
                .to_string_lossy()
                .replace('\\', "/");
            let source = read_source_path(path.as_path());
            (relative_path, source)
        })
        .collect()
}

fn contains_numeric_token(source: &str, literal: &str) -> bool {
    source.match_indices(literal).any(|(start, _)| {
        let end = start + literal.len();
        let before_ok = start == 0 || !is_rust_identifier_byte(source.as_bytes()[start - 1]);
        let after_ok = end == source.len() || !source.as_bytes()[end].is_ascii_digit();
        before_ok && after_ok
    })
}

fn is_rust_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_rust_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn line_number(source: &str, index: usize) -> usize {
    source[..index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn classification_error(path: &str, source: &str, index: usize, reason: &str) -> String {
    format!(
        "{path}:{} source classification failed: {reason}",
        line_number(source, index)
    )
}

fn contains_reserved_payment_action_term(value: &str, term: &str) -> bool {
    if term.contains(' ') || term.contains('-') {
        return value.contains(term);
    }

    value.match_indices(term).any(|(start, _)| {
        let end = start + term.len();
        is_reserved_payment_term_boundary_before(value, start)
            && is_reserved_payment_term_boundary_after(value, end)
    })
}

fn is_reserved_payment_term_boundary_before(value: &str, index: usize) -> bool {
    if index == 0 {
        return true;
    }

    is_reserved_payment_term_boundary_byte(value.as_bytes()[index - 1])
}

fn is_reserved_payment_term_boundary_after(value: &str, index: usize) -> bool {
    if index == value.len() {
        return true;
    }

    is_reserved_payment_term_boundary_byte(value.as_bytes()[index])
}

fn is_reserved_payment_term_boundary_byte(byte: u8) -> bool {
    !byte.is_ascii_alphanumeric() && byte != b'_' && byte != b'-'
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

fn seller_lifecycle_action_owner_sources() -> Vec<(PathBuf, String)> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("desktop crate should live under app crates directory");
    [
        manifest_dir.join("src/window.rs"),
        manifest_dir.join("src/runtime.rs"),
        app_root.join("crates/view/src/lib.rs"),
        app_root.join("crates/store/src/repo/orders.rs"),
        app_root.join("crates/i18n/src/keys.rs"),
        app_root.join("crates/i18n/src/lib.rs"),
        app_root.join("i18n/locales/en/messages.json"),
    ]
    .into_iter()
    .map(|path| {
        let source = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "failed to read seller lifecycle source {}: {error}",
                path.display()
            )
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
