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
    "14",
    "14.5",
    "2 bags",
    "2222222222222222222222222222222222222222222222222222222222222222",
    "3333333333333333333333333333333333333333333333333333333333333333",
    "2026-04-23T15:00:00Z",
    "6",
    "6.",
    "6.5",
    "6.50",
    "6.500",
    "Salad mix",
    "USD",
    "/tmp/radroots/data/apps/app",
    "/tmp/radroots/logs/apps/app",
    "{}.{:02}",
    "abc",
    "app.sqlite3",
    "account-add",
    "account-open-workspace",
    "account-log-out",
    "account-more",
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
    "buyer-order-keep-current",
    "buyer-order-keep-order",
    "buyer-order-mark-received",
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
    "buyer.order_receipt_failed",
    "buyer.order_revision_accept_failed",
    "buyer.order_revision_decline_failed",
    "buyer.repeat_demand_failed",
    "buyer.section_select_failed",
    "buyer_notice",
    "bunker://466d7fcae563e5cb09a0d1870bb580344804617879a14949cf22285f1bae3f27?relay=wss%3A%2F%2Frelay.radroots.example",
    "buyer.fulfillment_filter_update_failed",
    "buyer.search_query_update_failed",
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
    "today-reminder-chip",
    "https://auth.example/challenge",
    "identity",
    "npub1",
    "guest",
    "finder unavailable",
    "orders",
    "orders-reminders",
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
    "shell-account-label",
    "shell-mode-farm",
    "shell-mode-marketplace",
    "shell.switch_farm_failed",
    "shell.switch_marketplace_failed",
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
    "settings-about-conflict-action",
    "settings-about-refresh-sync",
    "settings-about-refresh-sync-disabled",
    "settings-remove-blackout-period",
    "settings-remove-fulfillment-window",
    "settings-use-media-servers",
    "settings-use-nip05",
    "settings.farm.load_failed",
    "settings.farm.save_failed",
    "settings.about.sync_refresh_failed",
    "settings.about.conflict_resolution_failed",
    "failed to refresh sync from the about panel",
    "failed to resolve sync conflict from the about panel",
    "switch_relays",
    "startup-title-radroots",
    "startup-title-starting",
    "wss://relay.example",
    "wss://relay.radroots.example",
    "{currency_code} {dollars}.{cents:02}",
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
    "AppTextKey::PersonalOrdersDetailEmptyBody",
    "AppTextKey::PersonalOrdersDetailFarmLabel",
    "AppTextKey::PersonalOrdersDetailFulfillmentLabel",
    "AppTextKey::PersonalOrdersDetailTotalLabel",
    "AppTextKey::PersonalOrdersDetailNoteLabel",
    "AppTextKey::PersonalOrdersDetailItemsTitle",
    "AppTextKey::PersonalOrdersActionCancel",
    "AppTextKey::PersonalOrdersActionAcceptChange",
    "AppTextKey::PersonalOrdersActionKeepOrder",
    "AppTextKey::PersonalOrdersActionMarkReceived",
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
    "AppTextKey::OrdersDetailEmptyBody",
    "AppTextKey::OrdersDetailItemsTitle",
    "AppTextKey::OrdersDetailCustomerLabel",
    "AppTextKey::OrdersDetailWindowLabel",
    "AppTextKey::OrdersDetailPickupLabel",
    "AppTextKey::OrdersDetailTotalLabel",
    "AppTextKey::TradeWorkflowAxisAgreement",
    "AppTextKey::TradeWorkflowAxisRevision",
    "AppTextKey::TradeWorkflowAxisFulfillment",
    "AppTextKey::TradeWorkflowAxisInventory",
    "AppTextKey::TradeWorkflowAxisPayment",
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
    "AppTextKey::ProductsEditorFieldStatus",
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
    "AppTextKey::SettingsAccountMoreActions",
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
    ("3422", "KIND_TRADE_ORDER_REQUEST"),
    ("3423", "KIND_TRADE_ORDER_DECISION"),
    ("3424", "KIND_TRADE_ORDER_REVISION"),
    ("3425", "KIND_TRADE_ORDER_REVISION_RESPONSE"),
    ("3426", "KIND_TRADE_QUESTION"),
    ("3427", "KIND_TRADE_ANSWER"),
    ("3428", "KIND_TRADE_DISCOUNT_REQUEST"),
    ("3429", "KIND_TRADE_DISCOUNT_OFFER"),
    ("3430", "KIND_TRADE_DISCOUNT_ACCEPT"),
    ("3431", "KIND_TRADE_FORBIDDEN_3431"),
    ("3432", "KIND_TRADE_CANCEL"),
    ("3433", "KIND_TRADE_FULFILLMENT_UPDATE"),
    ("3434", "KIND_TRADE_RECEIPT"),
    ("3435", "KIND_TRADE_PAYMENT_RECORDED"),
    ("3436", "KIND_TRADE_SETTLEMENT_DECISION"),
    ("3440", "KIND_TRADE_VALIDATION_RECEIPT"),
];

const TEST_MODULE_SENTINEL: &str = "\n#[cfg(test)]\nmod tests {";

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
