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
    "buyer-order-close-issue",
    "buyer-order-mark-received",
    "buyer-order-report-issue",
    "buyer-order-repeat-demand",
    "buyer-order-send-issue",
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
    "buyer.order_issue_receipt_failed",
    "buyer.order_receipt_failed",
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
    "customer_labels.txt",
    "desktop runtime paths should resolve",
    "desktop runtime roots require HOME for macos",
    "disk unavailable",
    "eggs",
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
    "failed to mark buyer order received",
    "failed to report buyer order issue",
    "failed to publish order fulfillment update",
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
    "failed to start order recovery",
    "failed to review order recovery",
    "failed to reopen order recovery",
    "failed to resolve order recovery",
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
    "items need review",
    "localhost",
    "npub1",
    "npub1qqqqq...qqqqqq",
    "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq",
    "npub1sxczr...5lkheq",
    "npub1sxczrq2dp4jtehcm8mtemj975u5ytf2d7mc6dpuuq3rzkjzr76ls5lkheq",
    "receipt-clean",
    "receipt-issue",
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
    "orders-detail-publish-delivered",
    "orders-detail-publish-out-for-delivery",
    "orders-detail-publish-preparing",
    "orders-detail-publish-ready-for-pickup",
    "orders-detail-publish-seller-cancelled",
    "orders-filter-all",
    "orders-filter-completed",
    "orders-filter-needs-action",
    "orders-filter-packed",
    "orders-filter-refunded",
    "orders-filter-scheduled",
    "orders-recovery-open",
    "orders-recovery-review",
    "orders-recovery-reopen",
    "orders-recovery-resolve",
    "orders-row-action-publish-fulfillment",
    "orders-row-action-review",
    "orders-row-open",
    "orders.detail_open_failed",
    "orders.filter_update_failed",
    "orders.fulfillment_publish_failed",
    "orders.recovery_reopen_failed",
    "orders.recovery_resolve_failed",
    "orders.recovery_review_failed",
    "orders.recovery_start_failed",
    "orders.route_failed",
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
    "a remote signer connection is already pending approval",
    "raw nostrconnect client uris are signer-side only",
    "remote signer",
    "remote signer connection failed: relay refused the request",
    "remote signer did not respond yet",
    "runtime unavailable",
    "radroots_home_view_{label}_{suffix}",
    "sign_event:kind:1",
    "shell",
    "shell-account-entry",
    "shell-mode-farm",
    "shell-mode-marketplace",
    "shell.switch_farm_failed",
    "shell.switch_marketplace_failed",
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
    "AppTextKey::PersonalOrdersDetailReceiptLabel",
    "AppTextKey::PersonalOrdersDetailItemsTitle",
    "AppTextKey::PersonalOrdersActionCancel",
    "AppTextKey::PersonalOrdersActionAcceptChange",
    "AppTextKey::PersonalOrdersActionKeepOrder",
    "AppTextKey::PersonalOrdersActionMarkReceived",
    "AppTextKey::PersonalOrdersActionReportIssue",
    "AppTextKey::PersonalOrdersActionSendReceiptIssue",
    "AppTextKey::PersonalOrdersActionCloseReceiptIssue",
    "AppTextKey::PersonalOrdersReceiptIssueLabel",
    "AppTextKey::PersonalOrdersReceiptIssuePlaceholder",
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
    "AppTextKey::OrdersStatusRefunded",
    "AppTextKey::OrdersTableTitle",
    "AppTextKey::OrdersColumnOrder",
    "AppTextKey::OrdersColumnStatus",
    "AppTextKey::OrdersColumnWindow",
    "AppTextKey::OrdersColumnPickup",
    "AppTextKey::OrdersColumnAction",
    "AppTextKey::OrdersActionReview",
    "AppTextKey::OrdersActionPreparing",
    "AppTextKey::OrdersActionReadyForPickup",
    "AppTextKey::OrdersActionOutForDelivery",
    "AppTextKey::OrdersActionMarkDelivered",
    "AppTextKey::OrdersActionCancelFulfillment",
    "AppTextKey::OrdersActionUpdateFulfillment",
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
    "AppTextKey::TradeWorkflowAxisFulfillment",
    "AppTextKey::TradeWorkflowAxisInventory",
    "AppTextKey::TradeWorkflowAxisPayment",
    "AppTextKey::TradeWorkflowAxisReceipt",
    "AppTextKey::TradeWorkflowAxisSource",
    "AppTextKey::TradeWorkflowAgreementOrdered",
    "AppTextKey::TradeWorkflowAgreementConfirmed",
    "AppTextKey::TradeWorkflowAgreementDeclined",
    "AppTextKey::TradeWorkflowAgreementCancelled",
    "AppTextKey::TradeWorkflowAgreementCompleted",
    "AppTextKey::TradeWorkflowAgreementNeedsReview",
    "AppTextKey::TradeWorkflowRevisionNone",
    "AppTextKey::TradeWorkflowRevisionChangeProposed",
    "AppTextKey::TradeWorkflowRevisionUpdated",
    "AppTextKey::TradeWorkflowRevisionKeptAsPlaced",
    "AppTextKey::TradeWorkflowFulfillmentConfirmed",
    "AppTextKey::TradeWorkflowFulfillmentPreparing",
    "AppTextKey::TradeWorkflowFulfillmentReadyForPickup",
    "AppTextKey::TradeWorkflowFulfillmentOutForDelivery",
    "AppTextKey::TradeWorkflowFulfillmentDelivered",
    "AppTextKey::TradeWorkflowFulfillmentCancelled",
    "AppTextKey::TradeWorkflowInventoryAvailable",
    "AppTextKey::TradeWorkflowInventoryReserved",
    "AppTextKey::TradeWorkflowInventorySoldOut",
    "AppTextKey::TradeWorkflowInventoryNeedsReview",
    "AppTextKey::TradeWorkflowPaymentNotRecorded",
    "AppTextKey::TradeWorkflowPaymentPending",
    "AppTextKey::TradeWorkflowPaymentRecorded",
    "AppTextKey::TradeWorkflowPaymentSettled",
    "AppTextKey::TradeWorkflowPaymentNeedsReview",
    "AppTextKey::TradeWorkflowReceiptReceived",
    "AppTextKey::TradeWorkflowReceiptNeedsReview",
    "AppTextKey::TradeWorkflowProvenanceApp",
    "AppTextKey::TradeWorkflowProvenanceCli",
    "AppTextKey::TradeWorkflowProvenanceRelay",
    "AppTextKey::TradeWorkflowProvenanceLocalEvents",
    "AppTextKey::TradeWorkflowProvenanceUnknown",
    "AppTextKey::OrdersRecoverySectionTitle",
    "AppTextKey::OrdersRecoveryMissedPickupTitle",
    "AppTextKey::OrdersRecoveryMissedPickupBody",
    "AppTextKey::OrdersRecoveryRefundFollowUpTitle",
    "AppTextKey::OrdersRecoveryRefundFollowUpBody",
    "AppTextKey::OrdersRecoveryLastUpdatedLabel",
    "AppTextKey::OrdersRecoveryActionOpenFollowUp",
    "AppTextKey::OrdersRecoveryActionStartReview",
    "AppTextKey::OrdersRecoveryActionMarkOpen",
    "AppTextKey::OrdersRecoveryActionResolve",
    "AppTextKey::OrdersRecoveryStateOpen",
    "AppTextKey::OrdersRecoveryStateInReview",
    "AppTextKey::OrdersRecoveryStateResolved",
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

struct LegacySdkBoundaryAllowlistEntry {
    path: &'static str,
    pattern: &'static str,
    owner: &'static str,
    reason: &'static str,
    removal_condition: &'static str,
}

const TEST_MODULE_SENTINEL: &str = "\n#[cfg(test)]\nmod tests {";

const SDK_MIGRATED_SOURCE_PATHS: &[&str] = &[
    "crates/runtime/src/sdk.rs",
    "crates/store/src/migration_audit.rs",
];

const FORBIDDEN_SDK_MIGRATED_BOUNDARY_PATTERNS: &[SdkBoundaryForbiddenPattern] = &[
    SdkBoundaryForbiddenPattern {
        pattern: "SdkDirectRelayAppSyncTransport",
        reason: "migrated paths must use AppSdkRuntime instead of the legacy direct relay sync transport",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsSdkClient",
        reason: "migrated paths must use the long-lived RadrootsSdk runtime boundary",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsSdkConfig",
        reason: "migrated paths must use AppSdkConfig-derived runtime construction",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "SdkTransportMode::RelayDirect",
        reason: "migrated paths must not configure direct relay publish transport",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "SignerConfig::LocalIdentity",
        reason: "migrated paths must not configure local direct-publish signing",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "PendingSyncOperation::from_publish_payload",
        reason: "migrated paths must not enqueue legacy app publish payloads",
    },
    SdkBoundaryForbiddenPattern {
        pattern: ".enqueue_pending_operation(",
        reason: "migrated paths must not mutate the legacy app outbox",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "INSERT INTO local_outbox",
        reason: "migrated paths must not write legacy local outbox rows",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "UPDATE local_outbox",
        reason: "migrated paths must not mutate legacy local outbox rows",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "DELETE FROM local_outbox",
        reason: "migrated paths must not delete legacy local outbox rows",
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
        reason: "migrated paths must not connect relay clients directly for publish",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_signed_event",
        reason: "migrated paths must not publish signed events directly",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "radroots_nostr_build_event",
        reason: "migrated paths must not build protocol events outside SDK-owned publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RadrootsIdentity::from_secret_key_str",
        reason: "migrated paths must not parse direct signing keys",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "RawSecretKey",
        reason: "migrated paths must not import raw signing-key material",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "EncryptedSecretKey",
        reason: "migrated paths must not import encrypted signing-key material",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_with_identity",
        reason: "migrated paths must not call legacy direct SDK publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_draft_with_identity",
        reason: "migrated paths must not encode legacy direct SDK publish targets",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_request_with_identity",
        reason: "migrated paths must not call legacy direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_decision_with_identity",
        reason: "migrated paths must not call legacy direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_revision_proposal_with_identity",
        reason: "migrated paths must not call legacy direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_revision_decision_with_identity",
        reason: "migrated paths must not call legacy direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_order_cancellation_with_identity",
        reason: "migrated paths must not call legacy direct SDK order publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_fulfillment_update_with_identity",
        reason: "migrated paths must not call legacy direct SDK fulfillment publish APIs",
    },
    SdkBoundaryForbiddenPattern {
        pattern: "publish_buyer_receipt_with_identity",
        reason: "migrated paths must not call legacy direct SDK receipt publish APIs",
    },
];

const LEGACY_SDK_BOUNDARY_PATTERNS: &[&str] = &[
    "SdkDirectRelayAppSyncTransport",
    "RadrootsSdkClient",
    "RadrootsSdkConfig",
    "SdkTransportMode::RelayDirect",
    "SignerConfig::LocalIdentity",
    "PendingSyncOperation::from_publish_payload",
    ".enqueue_pending_operation(",
    "INSERT INTO local_outbox",
    "UPDATE local_outbox",
    "DELETE FROM local_outbox",
    "publish_with_identity",
    "publish_draft_with_identity",
    "publish_order_request_with_identity",
    "publish_order_decision_with_identity",
    "publish_order_revision_proposal_with_identity",
    "publish_order_revision_decision_with_identity",
    "publish_order_cancellation_with_identity",
    "publish_fulfillment_update_with_identity",
    "publish_buyer_receipt_with_identity",
];

const LEGACY_SDK_BOUNDARY_ALLOWLIST: &[LegacySdkBoundaryAllowlistEntry] = &[
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "SdkDirectRelayAppSyncTransport",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still owns deferred direct relay publish transport for unmigrated publish workflows",
        removal_condition: "remove when farm, listing, order, fulfillment, and receipt publish workflows enqueue through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "RadrootsSdkClient",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still constructs the legacy direct publish client for unmigrated publish workflows",
        removal_condition: "remove when direct publish workflows no longer construct SDK clients outside AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "RadrootsSdkConfig",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still configures the legacy direct publish client for unmigrated publish workflows",
        removal_condition: "remove when direct publish workflows no longer configure SDK clients outside AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "SdkTransportMode::RelayDirect",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still uses relay direct publish transport for deferred workflow migration",
        removal_condition: "remove when all publish workflows route through SDK canonical outbox and sync APIs",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "SignerConfig::LocalIdentity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still configures direct local signing for deferred workflow migration",
        removal_condition: "remove when publish signing is mediated by AppSdkRuntime and SDK signer adapters",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "PendingSyncOperation::from_publish_payload",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still creates legacy local outbox publish work for unmigrated workflows",
        removal_condition: "remove when app publish workflows write SDK canonical outbox requests instead of app local_outbox operations",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK farm and listing publish APIs",
        removal_condition: "remove when farm profile and listing publish workflows enqueue through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_order_request_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK order request publish APIs",
        removal_condition: "remove when buyer order request publish workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_order_decision_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK order decision publish APIs",
        removal_condition: "remove when seller order decision publish workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_order_revision_proposal_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK order revision proposal publish APIs",
        removal_condition: "remove when seller order revision proposal workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_order_revision_decision_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK order revision decision publish APIs",
        removal_condition: "remove when buyer order revision decision workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_order_cancellation_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK order cancellation publish APIs",
        removal_condition: "remove when buyer order cancellation workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_fulfillment_update_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK fulfillment publish APIs",
        removal_condition: "remove when seller fulfillment workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/desktop/src/runtime.rs",
        pattern: "publish_buyer_receipt_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "desktop runtime still calls legacy direct SDK receipt publish APIs",
        removal_condition: "remove when buyer receipt workflow enqueues through AppSdkRuntime",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "SdkTransportMode::RelayDirect",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still marks legacy app local outbox publish work as relay direct",
        removal_condition: "remove when app sync publish payloads are replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_draft_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy farm and listing SDK publish operations",
        removal_condition: "remove when farm and listing publish payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_order_request_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy order request SDK publish operations",
        removal_condition: "remove when buyer order request publish payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_order_decision_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy order decision SDK publish operations",
        removal_condition: "remove when seller order decision publish payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_order_revision_proposal_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy order revision proposal SDK publish operations",
        removal_condition: "remove when order revision proposal payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_order_revision_decision_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy order revision decision SDK publish operations",
        removal_condition: "remove when order revision decision payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_order_cancellation_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy order cancellation SDK publish operations",
        removal_condition: "remove when order cancellation payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_fulfillment_update_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy fulfillment SDK publish operations",
        removal_condition: "remove when fulfillment payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/sync/src/publish.rs",
        pattern: "publish_buyer_receipt_with_identity",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "sync payload metadata still names legacy receipt SDK publish operations",
        removal_condition: "remove when receipt payload metadata is replaced by SDK canonical outbox requests",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/store/src/lib.rs",
        pattern: ".enqueue_pending_operation(",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "store facade still accepts legacy app local_outbox publish operations for deferred workflows",
        removal_condition: "remove when app local_outbox enqueue is replaced by SDK canonical outbox enqueue APIs",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/store/src/sync.rs",
        pattern: "INSERT INTO local_outbox",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "store sync implementation still writes legacy app local_outbox rows for deferred workflows",
        removal_condition: "remove when app local_outbox storage is retired after SDK canonical outbox migration",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/store/src/sync.rs",
        pattern: "UPDATE local_outbox",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "store sync implementation still updates legacy app local_outbox rows for deferred workflows",
        removal_condition: "remove when app local_outbox storage is retired after SDK canonical outbox migration",
    },
    LegacySdkBoundaryAllowlistEntry {
        path: "crates/store/src/sync.rs",
        pattern: "DELETE FROM local_outbox",
        owner: "rpv1-app-sdk-refactor.07",
        reason: "store sync implementation still deletes legacy app local_outbox rows for deferred workflows",
        removal_condition: "remove when app local_outbox storage is retired after SDK canonical outbox migration",
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
fn app_migrated_sdk_paths_do_not_use_direct_publish_boundaries() {
    let app_root = app_root();

    for relative_path in SDK_MIGRATED_SOURCE_PATHS {
        let path = app_root.join(relative_path);
        let source = read_source_path(path.as_path());
        let production_source = production_source_without_tests(&source);

        for forbidden in FORBIDDEN_SDK_MIGRATED_BOUNDARY_PATTERNS {
            assert!(
                !production_source.contains(forbidden.pattern),
                "{relative_path} contains forbidden SDK boundary pattern `{}`: {}",
                forbidden.pattern,
                forbidden.reason
            );
        }
    }
}

#[test]
fn app_legacy_sdk_boundary_allowlist_entries_are_complete_and_current() {
    let app_root = app_root();
    let mut entries = BTreeSet::new();

    for entry in LEGACY_SDK_BOUNDARY_ALLOWLIST {
        assert!(
            entries.insert((entry.path, entry.pattern)),
            "duplicate legacy SDK boundary allowlist entry {} `{}`",
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
        let production_source = production_source_without_tests(&source);
        assert!(
            production_source.contains(entry.pattern),
            "{} allowlists legacy SDK boundary pattern `{}` that is no longer present",
            entry.path,
            entry.pattern
        );
    }
}

#[test]
fn app_legacy_sdk_boundary_usage_is_allowlisted() {
    for (relative_path, source) in app_rust_source_files() {
        let production_source = production_source_without_tests(&source);

        for pattern in LEGACY_SDK_BOUNDARY_PATTERNS {
            if production_source.contains(pattern) {
                assert!(
                    legacy_sdk_boundary_allowlist_contains(relative_path.as_str(), pattern),
                    "{} contains unallowlisted legacy SDK boundary pattern `{pattern}`",
                    relative_path
                );
            }
        }
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
    let production_source = production_source_without_tests(source);
    for (literal, constant_name) in FORBIDDEN_PRODUCTION_EVENT_KIND_LITERALS {
        assert!(
            !contains_numeric_token(production_source, literal),
            "{path} uses raw event kind {literal}; use shared {constant_name} instead"
        );
    }
}

fn production_source_without_tests(source: &str) -> &str {
    source
        .split_once(TEST_MODULE_SENTINEL)
        .map_or(source, |(production_source, _)| production_source)
}

fn legacy_sdk_boundary_allowlist_contains(path: &str, pattern: &str) -> bool {
    LEGACY_SDK_BOUNDARY_ALLOWLIST
        .iter()
        .any(|entry| entry.path == path && entry.pattern == pattern)
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
