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

    const RESERVED_PAYMENT_ACTION_TERMS: &[&str] = &[
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

    const FORBIDDEN_TRADE_WORKFLOW_LEAKAGE_PATTERNS: &[&str] = &[
        "state machine",
        "reducer",
        "event kind",
        "nostr",
        "protocol",
        "checkout",
        "payment provider",
        "payment-provider",
    ];

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
            "Admin Console"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountImportFileAction),
            "Import from file"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountImportDatabaseAction),
            "Import from database"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAccountConnectRemoteBunkerAction),
            "Connect remote bunker"
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
            "Admin Console"
        );
    }

    #[test]
    fn english_reminder_copy_matches_the_seller_surface_contract() {
        assert_eq!(app_text(AppTextKey::HomeTodayRemindersTitle), "Coming up");
        assert_eq!(app_text(AppTextKey::OrdersRemindersTitle), "Reminders");
        assert_eq!(
            app_text(AppTextKey::OrdersReminderLogTitle),
            "Reminder activity"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayRemindersTitle),
            "Before this window"
        );
        assert_eq!(app_text(AppTextKey::ReminderDeadlineLabel), "Due");
        assert_eq!(app_text(AppTextKey::ReminderUrgencyDueSoon), "Due soon");
        assert_eq!(app_text(AppTextKey::ReminderUrgencyBlocking), "Blocking");
        assert_eq!(
            app_text(AppTextKey::ReminderPresentationTitle),
            "Needs attention now"
        );
        assert_eq!(
            app_text(AppTextKey::ReminderPresentationDismissAction),
            "Dismiss"
        );
        assert_eq!(
            app_text(AppTextKey::ReminderDeliveryStatePresented),
            "Presented"
        );
        assert_eq!(
            app_text(AppTextKey::ReminderDeliveryStateResolved),
            "Resolved"
        );
    }

    #[test]
    fn english_about_copy_matches_the_runtime_status_contract() {
        assert_eq!(
            app_text(AppTextKey::SettingsAboutCompanyName),
            "Radroots, Inc."
        );
        assert_eq!(app_text(AppTextKey::SettingsAboutVersionLabel), "Version");
        assert_eq!(
            app_text(AppTextKey::SettingsAboutVariantLabel),
            "Standalone local app"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutAcknowledgementsAction),
            "Acknowledgements"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutPrivacyPolicyAction),
            "Privacy Policy"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutTermsAction),
            "Terms of Service"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutReportIssueAction),
            "Report an Issue..."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutCopyrightNotice),
            "© 2026 Radroots, Inc. All rights reserved."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutTrademarkNotice),
            "Radroots is a trademark of Radroots, Inc."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutStatusSectionLabel),
            "Status"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictReviewSectionLabel),
            "Conflict review"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutRuntimeSectionLabel),
            "Runtime"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictReviewUnavailable),
            "Conflict review becomes available after you select an account."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictReviewBlocking),
            "Blocking conflicts pause sync until you resolve them."
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutRefreshAction),
            "Refresh sync"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictAcceptLocalAction),
            "Accept local"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictAcceptRemoteAction),
            "Accept remote"
        );
        assert_eq!(
            app_text(AppTextKey::SettingsAboutConflictDismissAction),
            "Dismiss"
        );
        assert_eq!(
            app_text(AppTextKey::MetadataSyncPendingWriteCount),
            "pending writes"
        );
        assert_eq!(
            app_text(AppTextKey::MetadataSyncBlockingConflictCount),
            "blocking conflict count"
        );
        assert_eq!(
            app_text(AppTextKey::MetadataSyncConflictAggregate),
            "aggregate"
        );
        assert_eq!(app_text(AppTextKey::MetadataSyncConflictKind), "kind");
        assert_eq!(
            app_text(AppTextKey::MetadataSyncConflictSeverity),
            "severity"
        );
        assert_eq!(
            app_text(AppTextKey::MetadataSyncConflictDetectedAt),
            "detected"
        );
        assert_eq!(
            app_text(AppTextKey::MetadataSyncConflictResolution),
            "resolution"
        );
        assert_eq!(
            app_text(AppTextKey::ValueSyncConflictAggregateFulfillmentWindow),
            "Fulfillment window"
        );
        assert_eq!(
            app_text(AppTextKey::ValueSyncConflictKindRevisionMismatch),
            "Revision mismatch"
        );
        assert_eq!(
            app_text(AppTextKey::ValueSyncConflictSeverityBlocking),
            "Blocking"
        );
        assert_eq!(
            app_text(AppTextKey::ValueSyncConflictResolutionAcceptedRemote),
            "Accepted remote"
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
        assert_eq!(app_text(AppTextKey::OrdersStatusDeclined), "Declined");
        assert_eq!(app_text(AppTextKey::OrdersStatusInHandoff), "In handoff");
        assert_eq!(
            app_text(AppTextKey::OrdersStatusNeedsReview),
            "Needs review"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersActionReadyForPickup),
            "Ready for pickup"
        );
        assert_eq!(app_text(AppTextKey::OrdersActionPreparing), "Preparing");
        assert_eq!(
            app_text(AppTextKey::OrdersActionOutForDelivery),
            "Out for delivery"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersActionMarkDelivered),
            "Mark delivered"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersActionCancelFulfillment),
            "Cancel fulfillment"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersActionUpdateFulfillment),
            "Update"
        );
        assert_eq!(app_text(AppTextKey::OrdersDetailTitle), "Order detail");
        assert_eq!(app_text(AppTextKey::OrdersRecoverySectionTitle), "Recovery");
        assert_eq!(
            app_text(AppTextKey::OrdersRecoveryMissedPickupTitle),
            "Missed pickup"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersRecoveryRefundFollowUpTitle),
            "Payment status"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersRecoveryRefundFollowUpBody),
            "Track the recorded payment state for this order."
        );
        assert_eq!(
            app_text(AppTextKey::OrdersRecoveryActionResolve),
            "Mark resolved"
        );
        assert_eq!(
            app_text(AppTextKey::OrdersRecoveryStateInReview),
            "In review"
        );
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
        assert_eq!(
            app_text(AppTextKey::PersonalMarketplaceRefreshFailedNotice),
            "Couldn't refresh marketplace listings. Your saved local state is still here; try again in a moment."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalDetailOpenFailedNotice),
            "Couldn't open that listing. Refresh the marketplace and try again."
        );
    }

    #[test]
    fn english_marketplace_order_review_copy_matches_the_local_order_contract() {
        assert_eq!(
            app_text(AppTextKey::PersonalCartReviewOrderAction),
            "Review order"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrderReviewTitle),
            "Order review"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrderReviewPlaceOrderAction),
            "Place order"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrderReviewLocalOnlyBody),
            "Review the details before placing the order."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrderPlaceFailedNotice),
            "Couldn't place that order. Nothing was sent; check the order and try again."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrderCoordinationFailedNotice),
            "Order saved locally. It still needs to be shared with your order tools; open Orders and try again."
        );
    }

    #[test]
    fn english_payment_action_copy_remains_unspoken_for_reserved_workflow() {
        let action_keys = AppTextKey::ALL
            .iter()
            .copied()
            .filter(|key| is_visible_action_text_key(*key))
            .collect::<Vec<_>>();

        assert!(action_keys.contains(&AppTextKey::PersonalCartReviewOrderAction));
        assert!(action_keys.contains(&AppTextKey::PersonalOrderReviewPlaceOrderAction));
        assert!(action_keys.contains(&AppTextKey::OrdersRecoveryActionResolve));

        for key in action_keys {
            let copy = app_text(key).to_lowercase();
            for term in RESERVED_PAYMENT_ACTION_TERMS {
                assert!(
                    !contains_reserved_payment_action_term(&copy, term),
                    "{} contains reserved payment action term `{term}`",
                    key.id()
                );
            }
        }
    }

    #[test]
    fn english_visible_copy_does_not_explain_payment_deferral() {
        for key in AppTextKey::ALL {
            let normalized_copy = app_text(*key).to_lowercase();
            for pattern in FORBIDDEN_PAYMENT_DEFERRAL_COPY_PATTERNS {
                assert!(
                    !normalized_copy.contains(pattern),
                    "{} contains forbidden payment-deferral copy `{pattern}`",
                    key.id()
                );
            }
        }
    }

    #[test]
    fn english_buyer_visible_copy_does_not_use_checkout_wording() {
        for key in AppTextKey::ALL
            .iter()
            .copied()
            .filter(|key| is_buyer_visible_text_key(*key))
        {
            let normalized_copy = app_text(key).to_lowercase();
            assert!(
                !contains_reserved_payment_action_term(&normalized_copy, "checkout"),
                "{} contains buyer-visible checkout wording",
                key.id()
            );
        }
    }

    #[test]
    fn english_trade_workflow_copy_stays_compact_and_product_facing() {
        for key in AppTextKey::ALL
            .iter()
            .copied()
            .filter(|key| is_trade_workflow_text_key(*key))
        {
            let copy = app_text(key);
            let normalized_copy = copy.to_lowercase();
            assert!(
                copy.split_whitespace().count() <= 4,
                "{} is too long for a compact workflow badge",
                key.id()
            );
            for pattern in FORBIDDEN_TRADE_WORKFLOW_LEAKAGE_PATTERNS {
                assert!(
                    !normalized_copy.contains(pattern),
                    "{} contains workflow implementation copy `{pattern}`",
                    key.id()
                );
            }
        }
    }

    #[test]
    fn english_trade_workflow_copy_matches_the_projection_contract() {
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowAxisAgreement),
            "Agreement"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowAxisFulfillment),
            "Fulfillment"
        );
        assert_eq!(app_text(AppTextKey::TradeWorkflowAxisPayment), "Payment");
        assert_eq!(app_text(AppTextKey::TradeWorkflowAxisReceipt), "Receipt");
        assert_eq!(app_text(AppTextKey::TradeWorkflowAxisSource), "Source");
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowAgreementOrdered),
            "Ordered"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowAgreementConfirmed),
            "Confirmed"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowAgreementNeedsReview),
            "Needs review"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowRevisionChangeProposed),
            "Change proposed"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowRevisionKeptAsPlaced),
            "Kept as placed"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowFulfillmentReadyForPickup),
            "Ready for pickup"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowInventoryReserved),
            "Reserved"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowPaymentNotRecorded),
            "Not recorded"
        );
        assert_eq!(app_text(AppTextKey::TradeWorkflowPaymentPending), "Pending");
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowPaymentRecorded),
            "Recorded"
        );
        assert_eq!(app_text(AppTextKey::TradeWorkflowPaymentSettled), "Settled");
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowReceiptReceived),
            "Received"
        );
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowReceiptNeedsReview),
            "Needs review"
        );
        assert_eq!(app_text(AppTextKey::TradeWorkflowProvenanceCli), "CLI");
        assert_eq!(
            app_text(AppTextKey::TradeWorkflowProvenanceLocalEvents),
            "Local events"
        );
    }

    #[test]
    fn payment_workflow_copy_covers_passive_statuses() {
        for (key, expected) in [
            (AppTextKey::TradeWorkflowPaymentNotRecorded, "Not recorded"),
            (AppTextKey::TradeWorkflowPaymentPending, "Pending"),
            (AppTextKey::TradeWorkflowPaymentRecorded, "Recorded"),
            (AppTextKey::TradeWorkflowPaymentSettled, "Settled"),
            (AppTextKey::TradeWorkflowPaymentNeedsReview, "Needs review"),
        ] {
            assert_eq!(app_text(key), expected);
        }
    }

    #[test]
    fn validation_receipt_copy_covers_passive_evidence() {
        for (key, expected) in [
            (AppTextKey::TradeValidationReceiptSectionLabel, "Validation"),
            (AppTextKey::TradeValidationReceiptEventLabel, "Receipt"),
            (AppTextKey::TradeValidationReceiptTargetLabel, "Target"),
            (
                AppTextKey::TradeValidationReceiptEventSetRootLabel,
                "Evidence set",
            ),
            (
                AppTextKey::TradeValidationReceiptReducerOutputRootLabel,
                "Review output",
            ),
            (
                AppTextKey::TradeValidationReceiptPublicValuesHashLabel,
                "Verification values",
            ),
            (
                AppTextKey::TradeValidationReceiptRecordedAtLabel,
                "Recorded",
            ),
            (AppTextKey::TradeValidationReceiptResultValid, "Valid"),
            (
                AppTextKey::TradeValidationReceiptResultNeedsReview,
                "Needs review",
            ),
            (
                AppTextKey::TradeValidationReceiptTypeListingValidation,
                "Listing",
            ),
            (
                AppTextKey::TradeValidationReceiptTypeTradeTransition,
                "Trade",
            ),
            (
                AppTextKey::TradeValidationReceiptTypeInventoryState,
                "Stock",
            ),
            (
                AppTextKey::TradeValidationReceiptTypeStateCheckpoint,
                "State",
            ),
            (AppTextKey::TradeValidationReceiptProofNone, "None"),
            (AppTextKey::TradeValidationReceiptProofSp1Core, "Core proof"),
            (
                AppTextKey::TradeValidationReceiptProofSp1Compressed,
                "Compressed proof",
            ),
            (
                AppTextKey::TradeValidationReceiptProofSp1Groth16,
                "Groth16 proof",
            ),
            (
                AppTextKey::TradeValidationReceiptProofSp1Plonk,
                "Plonk proof",
            ),
        ] {
            assert_eq!(app_text(key), expected);
            assert!(
                app_text(key).split_whitespace().count() <= 3,
                "{} is too long for compact validation receipt evidence",
                key.id()
            );
        }
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
            app_text(AppTextKey::PersonalOrdersStatusDeclined),
            "Declined"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersStatusRefunded),
            "Refunded"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersStatusNeedsReview),
            "Needs review"
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
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersDetailReceiptLabel),
            "Receipt"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionCancel),
            "Cancel order"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionAcceptChange),
            "Accept change"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionKeepOrder),
            "Keep order"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionMarkReceived),
            "Mark received"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionReportIssue),
            "Report issue"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersActionSendReceiptIssue),
            "Send update"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersReceiptIssuePlaceholder),
            "What needs review"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandTitle),
            "Reorder"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandActionEligible),
            "Reorder"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandActionPartial),
            "Reorder available items"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandNotePartialSingle),
            "One item from this order is currently unavailable to reorder."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandNotePartialMultiple),
            "Some items from this order are currently unavailable to reorder."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersRepeatDemandNoteUnavailable),
            "Items from this order are currently unavailable to reorder."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersCoordinationRetryTitle),
            "Finish sharing saved orders"
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersCoordinationRetryBody),
            "A saved order still needs to be shared with your order tools."
        );
        assert_eq!(
            app_text(AppTextKey::PersonalOrdersCoordinationRetryAction),
            "Try sharing again"
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
        assert_eq!(app_text(AppTextKey::PackDayExportTitle), "Export pack day");
        assert_eq!(
            app_text(AppTextKey::PackDayExportReadyTitle),
            "Ready to save locally"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayExportUnavailableTitle),
            "Not ready yet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayExportRunningTitle),
            "Saving locally"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayExportSucceededTitle),
            "Saved locally"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayExportFailedTitle),
            "Couldn't save export"
        );
        assert_eq!(app_text(AppTextKey::PackDayExportAction), "Export pack day");
        assert_eq!(
            app_text(AppTextKey::PackDayExportActionRunning),
            "Exporting..."
        );
        assert_eq!(app_text(AppTextKey::PackDayExportFolderLabel), "Folder");
        assert_eq!(app_text(AppTextKey::PackDayExportFilesLabel), "Files");
        assert_eq!(app_text(AppTextKey::PackDayExportErrorLabel), "Error");
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPackSheetAction),
            "Print pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPackSheetActionRunning),
            "Printing pack sheet..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPickupRosterAction),
            "Print pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPickupRosterActionRunning),
            "Printing pickup roster..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsAction),
            "Print customer labels (Avery 5160)"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsActionRunning),
            "Printing customer labels (Avery 5160)..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintUnavailableTitle),
            "Print not available yet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintUnavailableBody),
            "Print actions become available after pack day files are saved locally."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPackSheetQueuedTitle),
            "Queueing pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPackSheetSubmittedTitle),
            "Sent pack sheet to the printer"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPackSheetFailedTitle),
            "Couldn't print pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPickupRosterQueuedTitle),
            "Queueing pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPickupRosterSubmittedTitle),
            "Sent pickup roster to the printer"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintPickupRosterFailedTitle),
            "Couldn't print pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsQueuedTitle),
            "Queueing customer labels"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsSubmittedTitle),
            "Sent customer labels to the printer"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsFailedTitle),
            "Couldn't print customer labels"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayPrintCustomerLabelsAvery5160OverflowFailedTitle),
            "Customer labels do not fit Avery 5160"
        );
        assert_eq!(app_text(AppTextKey::PackDayBatchPrintAction), "Print all");
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintActionRunning),
            "Printing all..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintQueuedTitle),
            "Queueing pack day print run"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintSucceededTitle),
            "Sent all pack day files to the printer"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintFailedTitle),
            "Couldn't print all pack day files"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintFailedPreflightTitle),
            "Pack day files are not ready to print"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintFailedQueueLaunchTitle),
            "Couldn't start the print queue"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintFailedQueueExitTitle),
            "Print queue stopped before the run finished"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayBatchPrintCustomerLabelsAvery5160OverflowFailedTitle),
            "Customer labels do not fit Avery 5160"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffRevealAction),
            "Show in Finder"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffRevealActionRunning),
            "Showing in Finder..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPackSheetAction),
            "Open pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPackSheetActionRunning),
            "Opening pack sheet..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPickupRosterAction),
            "Open pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPickupRosterActionRunning),
            "Opening pickup roster..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenCustomerLabelsAction),
            "Open customer labels"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenCustomerLabelsActionRunning),
            "Opening customer labels..."
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffRevealSucceededTitle),
            "Shown in Finder"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPackSheetSucceededTitle),
            "Opened pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPickupRosterSucceededTitle),
            "Opened pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenCustomerLabelsSucceededTitle),
            "Opened customer labels"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffRevealFailedTitle),
            "Couldn't show in Finder"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPackSheetFailedTitle),
            "Couldn't open pack sheet"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenPickupRosterFailedTitle),
            "Couldn't open pickup roster"
        );
        assert_eq!(
            app_text(AppTextKey::PackDayHostHandoffOpenCustomerLabelsFailedTitle),
            "Couldn't open customer labels"
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
            "HomeSetupIssueUnavailableBody => \"home.setup.issue.unavailable_body\"",
            "HomeSetupErrorStartupFailed => \"home.setup.error.startup_failed\"",
            "HomeSetupSignerSourceValueBunkerUri => \"home.setup.signer.source_value.bunker_uri\"",
            "HomeSetupSignerSourceValueDiscoveryUrl => \"home.setup.signer.source_value.discovery_url\"",
            "HomeSetupSignerPermissionSignEventKind1 => \"home.setup.signer.permission.sign_event_kind_1\"",
            "HomeSetupSignerPermissionSwitchRelays => \"home.setup.signer.permission.switch_relays\"",
            "HomeSetupSignerPermissionAdditional => \"home.setup.signer.permission.additional\"",
            "HomeSetupSignerErrorEnterSource => \"home.setup.signer.error.enter_source\"",
            "HomeSetupSignerErrorUseSignerUri => \"home.setup.signer.error.use_signer_uri\"",
            "HomeSetupSignerErrorMissingDiscoveryUri => \"home.setup.signer.error.missing_discovery_uri\"",
            "HomeSetupSignerErrorInvalidDiscoveryUrl => \"home.setup.signer.error.invalid_discovery_url\"",
            "HomeSetupSignerErrorInvalidRemoteSignerUri => \"home.setup.signer.error.invalid_remote_signer_uri\"",
            "HomeSetupSignerErrorPendingApprovalExists => \"home.setup.signer.error.pending_approval_exists\"",
            "HomeSetupSignerErrorConnectionFailed => \"home.setup.signer.error.connection_failed\"",
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
        assert_eq!(
            app_text(AppTextKey::HomeSetupIssueUnavailableBody),
            "Radroots couldn't start normally on this device. Check the local setup and try again."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupErrorStartupFailed),
            "Couldn't finish startup right now. Check the connection and try again."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerSourceValueBunkerUri),
            "Bunker URI"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerSourceValueDiscoveryUrl),
            "Discovery URL"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerPermissionSignEventKind1),
            "Sign notes"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerPermissionSwitchRelays),
            "Switch relays"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerPermissionAdditional),
            "Additional permission"
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorEnterSource),
            "Paste a bunker URI or discovery URL from your signer to continue."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorUseSignerUri),
            "Use a bunker URI or discovery URL from your signer."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorMissingDiscoveryUri),
            "The discovery URL is missing the signer address."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorInvalidDiscoveryUrl),
            "That discovery URL isn't valid. Check it and try again."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorInvalidRemoteSignerUri),
            "That signer address isn't valid. Check it and try again."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorPendingApprovalExists),
            "A signer connection is already waiting for approval."
        );
        assert_eq!(
            app_text(AppTextKey::HomeSetupSignerErrorConnectionFailed),
            "Couldn't continue with the signer. Check the signer and try again."
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
        assert_eq!(
            app_text(AppTextKey::ProductsEditorFieldCategory),
            "Category"
        );
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
            app_text(AppTextKey::ProductsEditorBlockerChooseCategory),
            "Choose a category."
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
            app_text(AppTextKey::ProductsEditorBlockerSetStock),
            "Set available stock."
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

    fn is_visible_action_text_key(key: AppTextKey) -> bool {
        let id = key.id();
        id.contains(".action") || id.contains("_action")
    }

    fn is_buyer_visible_text_key(key: AppTextKey) -> bool {
        key.id().starts_with("messages.personal.")
    }

    fn is_trade_workflow_text_key(key: AppTextKey) -> bool {
        key.id().starts_with("messages.trade.workflow.")
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
}
