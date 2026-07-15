use radroots_sdk::{
    FARM_PUBLISH_OPERATION_KIND, LISTING_PUBLISH_OPERATION_KIND, TRADE_CANCELLATION_OPERATION_KIND,
    TRADE_DECISION_OPERATION_KIND, TRADE_SUBMIT_OPERATION_KIND,
};
use radroots_studio_app_view::{
    FarmId, FarmReadiness, FulfillmentWindowId, OrderId, ProductId, ProductStatus,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef, SyncOperationKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppPublishWorkKind {
    FarmProfile,
    Listing,
    OrderRequest,
    OrderDecision,
    OrderCancellation,
}

impl AppPublishWorkKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::FarmProfile => "farm_profile",
            Self::Listing => "listing",
            Self::OrderRequest => "order_request",
            Self::OrderDecision => "order_decision",
            Self::OrderCancellation => "order_cancellation",
        }
    }

    pub const fn sdk_operation(self) -> &'static str {
        match self {
            Self::FarmProfile => FARM_PUBLISH_OPERATION_KIND,
            Self::Listing => LISTING_PUBLISH_OPERATION_KIND,
            Self::OrderRequest => TRADE_SUBMIT_OPERATION_KIND,
            Self::OrderDecision => TRADE_DECISION_OPERATION_KIND,
            Self::OrderCancellation => TRADE_CANCELLATION_OPERATION_KIND,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppPublishContext {
    pub account_id: String,
    pub source: String,
    pub source_local_event_id: Option<String>,
}

impl AppPublishContext {
    pub fn new(account_id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            account_id: account_id.into(),
            source: source.into(),
            source_local_event_id: None,
        }
    }

    pub fn with_source_local_event_id(mut self, source_local_event_id: impl Into<String>) -> Self {
        self.source_local_event_id = Some(source_local_event_id.into());
        self
    }

    fn validation_failures(&self, failures: &mut Vec<AppPublishValidationFailure>) {
        if self.account_id.trim().is_empty() {
            failures.push(AppPublishValidationFailure::MissingAccountId);
        }

        if self.source.trim().is_empty() {
            failures.push(AppPublishValidationFailure::MissingSource);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppFarmProfilePublishPayload {
    pub context: AppPublishContext,
    pub farm_id: FarmId,
    pub display_name: String,
    pub readiness: Option<FarmReadiness>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppListingPublishPayload {
    pub context: AppPublishContext,
    pub product_id: ProductId,
    pub listing_d_tag: Option<String>,
    pub farm_id: Option<FarmId>,
    pub farm_pubkey: Option<String>,
    pub farm_d_tag: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: Option<String>,
    pub unit_label: String,
    pub price_minor_units: Option<u32>,
    pub price_currency: String,
    pub stock_quantity: Option<u32>,
    pub availability_window_id: Option<FulfillmentWindowId>,
    pub availability_starts_at: Option<String>,
    pub availability_ends_at: Option<String>,
    pub fulfillment_method: Option<String>,
    pub fulfillment_location: Option<String>,
    pub status: ProductStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppOrderRequestItemPayload {
    pub product_id: ProductId,
    pub quantity: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppOrderRequestPublishPayload {
    pub context: AppPublishContext,
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub status: Option<String>,
    pub order_document_json: Option<serde_json::Value>,
    pub listing_addr: Option<String>,
    pub listing_event_id: Option<String>,
    pub listing_relays: Vec<String>,
    pub buyer_pubkey: Option<String>,
    pub seller_pubkey: Option<String>,
    pub items: Vec<AppOrderRequestItemPayload>,
    pub currency_code: Option<String>,
    pub total_minor_units: Option<u32>,
    pub note: Option<String>,
    pub confirm_public_note: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppOrderDecisionInventoryCommitment {
    pub bin_id: String,
    pub bin_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum AppOrderDecisionPayload {
    Accepted {
        inventory_commitments: Vec<AppOrderDecisionInventoryCommitment>,
    },
    Declined {
        reason: String,
    },
}

impl AppOrderDecisionPayload {
    pub const fn storage_key(&self) -> &'static str {
        match self {
            Self::Accepted { .. } => "accepted",
            Self::Declined { .. } => "declined",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppOrderDecisionPublishPayload {
    pub context: AppPublishContext,
    pub app_order_id: OrderId,
    pub farm_id: FarmId,
    pub trade_order_id: String,
    pub request_event_id: String,
    pub listing_event_id: Option<String>,
    pub listing_addr: String,
    pub buyer_pubkey: String,
    pub seller_pubkey: String,
    pub decision: AppOrderDecisionPayload,
    pub confirm_public_note: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppOrderCancellationPublishPayload {
    pub context: AppPublishContext,
    pub app_order_id: OrderId,
    pub farm_id: FarmId,
    pub trade_order_id: String,
    pub request_event_id: String,
    pub listing_addr: String,
    pub buyer_pubkey: String,
    pub seller_pubkey: String,
    pub reason: String,
    pub confirm_public_note: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "publish_kind", content = "payload", rename_all = "snake_case")]
pub enum AppPublishPayload {
    FarmProfile(AppFarmProfilePublishPayload),
    Listing(AppListingPublishPayload),
    OrderRequest(AppOrderRequestPublishPayload),
    OrderDecision(AppOrderDecisionPublishPayload),
    OrderCancellation(AppOrderCancellationPublishPayload),
}

impl AppPublishPayload {
    pub const fn work_kind(&self) -> AppPublishWorkKind {
        match self {
            Self::FarmProfile(_) => AppPublishWorkKind::FarmProfile,
            Self::Listing(_) => AppPublishWorkKind::Listing,
            Self::OrderRequest(_) => AppPublishWorkKind::OrderRequest,
            Self::OrderDecision(_) => AppPublishWorkKind::OrderDecision,
            Self::OrderCancellation(_) => AppPublishWorkKind::OrderCancellation,
        }
    }

    pub const fn operation_kind(&self) -> SyncOperationKind {
        SyncOperationKind::Upsert
    }

    pub fn aggregate_ref(&self) -> SyncAggregateRef {
        match self {
            Self::FarmProfile(payload) => SyncAggregateRef::Farm(payload.farm_id),
            Self::Listing(payload) => SyncAggregateRef::Product(payload.product_id),
            Self::OrderRequest(payload) => SyncAggregateRef::Order(payload.order_id),
            Self::OrderDecision(payload) => SyncAggregateRef::Order(payload.app_order_id),
            Self::OrderCancellation(payload) => SyncAggregateRef::Order(payload.app_order_id),
        }
    }

    pub fn validation_failures(&self) -> Vec<AppPublishValidationFailure> {
        let mut failures = Vec::new();

        match self {
            Self::FarmProfile(payload) => {
                payload.context.validation_failures(&mut failures);
                if payload.display_name.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingFarmDisplayName);
                }
            }
            Self::Listing(payload) => {
                payload.context.validation_failures(&mut failures);
                if payload.farm_id.is_none() {
                    failures.push(AppPublishValidationFailure::MissingListingFarmId);
                }
                if payload
                    .farm_pubkey
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingFarmPubkey);
                }
                if payload
                    .category
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingCategory);
                }
                if payload.title.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingListingTitle);
                }
                if payload.unit_label.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingListingUnit);
                }
                if payload.price_minor_units.is_none_or(|value| value == 0) {
                    failures.push(AppPublishValidationFailure::MissingListingPrice);
                }
                if payload.price_currency.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingListingCurrency);
                }
                if payload.availability_window_id.is_none()
                    || payload
                        .availability_starts_at
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    || payload
                        .availability_ends_at
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingAvailability);
                }
                if payload.stock_quantity.is_none() {
                    failures.push(AppPublishValidationFailure::MissingListingStock);
                }
                if payload
                    .fulfillment_method
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingFulfillmentMethod);
                }
                if payload
                    .fulfillment_location
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingFulfillmentLocation);
                }
            }
            Self::OrderRequest(payload) => {
                payload.context.validation_failures(&mut failures);
                if payload.order_document_json.is_none() {
                    failures.push(AppPublishValidationFailure::MissingOrderDocument);
                }
                if payload
                    .listing_addr
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderListingAddress);
                }
                if payload
                    .listing_event_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderListingEventId);
                }
                if payload
                    .listing_relays
                    .iter()
                    .all(|relay| relay.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderListingRelay);
                }
                if payload
                    .buyer_pubkey
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderBuyerPubkey);
                }
                if payload
                    .seller_pubkey
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderSellerPubkey);
                }
                if payload.items.is_empty() || payload.items.iter().any(|item| item.quantity == 0) {
                    failures.push(AppPublishValidationFailure::MissingOrderItems);
                }
                if payload
                    .currency_code
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingOrderCurrency);
                }
                if payload.total_minor_units.is_none() {
                    failures.push(AppPublishValidationFailure::MissingOrderTotal);
                }
            }
            Self::OrderDecision(payload) => {
                payload.context.validation_failures(&mut failures);
                if payload.trade_order_id.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderTradeOrderId);
                }
                if payload.request_event_id.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderRequestEventId);
                }
                if payload.listing_addr.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderListingAddress);
                }
                if payload.buyer_pubkey.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderBuyerPubkey);
                }
                if payload.seller_pubkey.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderSellerPubkey);
                }
                match &payload.decision {
                    AppOrderDecisionPayload::Accepted {
                        inventory_commitments,
                    } => {
                        if inventory_commitments.is_empty()
                            || inventory_commitments.iter().any(|commitment| {
                                commitment.bin_id.trim().is_empty() || commitment.bin_count == 0
                            })
                        {
                            failures
                                .push(AppPublishValidationFailure::MissingOrderDecisionInventory);
                        }
                    }
                    AppOrderDecisionPayload::Declined { reason } => {
                        if reason.trim().is_empty() {
                            failures.push(AppPublishValidationFailure::MissingOrderDeclineReason);
                        }
                    }
                }
            }
            Self::OrderCancellation(payload) => {
                validate_lifecycle_order_fields(
                    &payload.context,
                    payload.trade_order_id.as_str(),
                    payload.request_event_id.as_str(),
                    payload.listing_addr.as_str(),
                    payload.buyer_pubkey.as_str(),
                    payload.seller_pubkey.as_str(),
                    &mut failures,
                );
                if payload.reason.trim().is_empty() {
                    failures.push(AppPublishValidationFailure::MissingOrderCancellationReason);
                }
            }
        }

        failures
    }

    pub fn validate(&self) -> Result<(), AppPublishValidationFailureSet> {
        let reason_codes = self.validation_failures();
        if reason_codes.is_empty() {
            Ok(())
        } else {
            Err(AppPublishValidationFailureSet { reason_codes })
        }
    }

    pub fn to_payload_json(&self) -> Result<String, AppPublishPayloadJsonError> {
        serde_json::to_string(self).map_err(|source| AppPublishPayloadJsonError::Serialize {
            message: source.to_string(),
        })
    }
}

fn validate_lifecycle_order_fields(
    context: &AppPublishContext,
    trade_order_id: &str,
    request_event_id: &str,
    listing_addr: &str,
    buyer_pubkey: &str,
    seller_pubkey: &str,
    failures: &mut Vec<AppPublishValidationFailure>,
) {
    context.validation_failures(failures);
    if trade_order_id.trim().is_empty() {
        failures.push(AppPublishValidationFailure::MissingOrderTradeOrderId);
    }
    if request_event_id.trim().is_empty() {
        failures.push(AppPublishValidationFailure::MissingOrderRequestEventId);
    }
    if listing_addr.trim().is_empty() {
        failures.push(AppPublishValidationFailure::MissingOrderListingAddress);
    }
    if buyer_pubkey.trim().is_empty() {
        failures.push(AppPublishValidationFailure::MissingOrderBuyerPubkey);
    }
    if seller_pubkey.trim().is_empty() {
        failures.push(AppPublishValidationFailure::MissingOrderSellerPubkey);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppPublishValidationFailure {
    MissingAccountId,
    MissingSource,
    MissingFarmDisplayName,
    MissingListingFarmId,
    MissingListingFarmPubkey,
    MissingListingCategory,
    MissingListingTitle,
    MissingListingUnit,
    MissingListingPrice,
    MissingListingCurrency,
    MissingListingAvailability,
    MissingListingStock,
    MissingListingFulfillmentMethod,
    MissingListingFulfillmentLocation,
    MissingOrderDocument,
    MissingOrderListingAddress,
    MissingOrderListingEventId,
    MissingOrderListingRelay,
    MissingOrderBuyerPubkey,
    MissingOrderSellerPubkey,
    MissingOrderItems,
    MissingOrderCurrency,
    MissingOrderTotal,
    MissingOrderTradeOrderId,
    MissingOrderRequestEventId,
    MissingOrderDecisionInventory,
    MissingOrderDeclineReason,
    MissingOrderCancellationReason,
}

impl AppPublishValidationFailure {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::MissingAccountId => "missing_account_id",
            Self::MissingSource => "missing_source",
            Self::MissingFarmDisplayName => "missing_farm_display_name",
            Self::MissingListingFarmId => "missing_listing_farm_id",
            Self::MissingListingFarmPubkey => "missing_listing_farm_pubkey",
            Self::MissingListingCategory => "missing_listing_category",
            Self::MissingListingTitle => "missing_listing_title",
            Self::MissingListingUnit => "missing_listing_unit",
            Self::MissingListingPrice => "missing_listing_price",
            Self::MissingListingCurrency => "missing_listing_currency",
            Self::MissingListingAvailability => "missing_listing_availability",
            Self::MissingListingStock => "missing_listing_stock",
            Self::MissingListingFulfillmentMethod => "missing_listing_fulfillment_method",
            Self::MissingListingFulfillmentLocation => "missing_listing_fulfillment_location",
            Self::MissingOrderDocument => "missing_order_document",
            Self::MissingOrderListingAddress => "missing_order_listing_address",
            Self::MissingOrderListingEventId => "missing_order_listing_event_id",
            Self::MissingOrderListingRelay => "missing_order_listing_relay",
            Self::MissingOrderBuyerPubkey => "missing_order_buyer_pubkey",
            Self::MissingOrderSellerPubkey => "missing_order_seller_pubkey",
            Self::MissingOrderItems => "missing_order_items",
            Self::MissingOrderCurrency => "missing_order_currency",
            Self::MissingOrderTotal => "missing_order_total",
            Self::MissingOrderTradeOrderId => "missing_order_trade_order_id",
            Self::MissingOrderRequestEventId => "missing_order_request_event_id",
            Self::MissingOrderDecisionInventory => "missing_order_decision_inventory",
            Self::MissingOrderDeclineReason => "missing_order_decline_reason",
            Self::MissingOrderCancellationReason => "missing_order_cancellation_reason",
        }
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("app publish payload is invalid: {reason_codes:?}")]
pub struct AppPublishValidationFailureSet {
    pub reason_codes: Vec<AppPublishValidationFailure>,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AppPublishPayloadJsonError {
    #[error("app publish payload serialization failed: {message}")]
    Serialize { message: String },
    #[error("app publish payload json is invalid: {message}")]
    Deserialize { message: String },
}

impl PendingSyncOperation {
    pub fn from_publish_payload(
        payload: AppPublishPayload,
        created_at: impl Into<String>,
    ) -> Result<Self, AppPublishPayloadJsonError> {
        let created_at = created_at.into();
        let aggregate = payload.aggregate_ref();
        let operation = payload.operation_kind();
        Ok(Self {
            operation_key: PendingSyncOperation::deterministic_operation_key(&aggregate, operation),
            aggregate,
            operation,
            payload_json: payload.to_payload_json()?,
            created_at: created_at.clone(),
            available_at: created_at,
            attempt_count: 0,
            state: PendingSyncOperationState::Pending,
            last_error_message: None,
        })
    }

    pub fn publish_payload(&self) -> Result<AppPublishPayload, AppPublishPayloadJsonError> {
        serde_json::from_str(self.payload_json.as_str()).map_err(|source| {
            AppPublishPayloadJsonError::Deserialize {
                message: source.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppFarmProfilePublishPayload, AppListingPublishPayload, AppOrderCancellationPublishPayload,
        AppOrderDecisionPayload, AppOrderDecisionPublishPayload, AppOrderRequestItemPayload,
        AppOrderRequestPublishPayload, AppPublishContext, AppPublishPayload,
        AppPublishValidationFailure, AppPublishWorkKind, FARM_PUBLISH_OPERATION_KIND,
        TRADE_CANCELLATION_OPERATION_KIND, TRADE_DECISION_OPERATION_KIND,
    };
    use crate::{
        PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef, SyncOperationKind,
    };
    use radroots_studio_app_view::{FarmId, FarmReadiness, OrderId, ProductId, ProductStatus};

    #[test]
    fn publish_payload_serializes_with_stable_kind_and_sdk_target() {
        let farm_id = FarmId::generate();
        let payload = AppPublishPayload::FarmProfile(AppFarmProfilePublishPayload {
            context: AppPublishContext::new("acct_local", "farm_setup")
                .with_source_local_event_id("local-event-1"),
            farm_id,
            display_name: "North Farm".to_owned(),
            readiness: Some(FarmReadiness::Ready),
        });

        assert_eq!(payload.work_kind().storage_key(), "farm_profile");
        assert_eq!(
            payload.work_kind().sdk_operation(),
            FARM_PUBLISH_OPERATION_KIND
        );
        assert_eq!(payload.validation_failures(), Vec::new());

        let operation =
            PendingSyncOperation::from_publish_payload(payload.clone(), "2026-04-20T18:00:00Z")
                .expect("typed publish payload should serialize");

        assert_eq!(operation.aggregate, SyncAggregateRef::Farm(farm_id));
        assert_eq!(operation.operation_key, format!("farm:{farm_id}:upsert"));
        assert_eq!(operation.operation, SyncOperationKind::Upsert);
        assert_eq!(operation.state, PendingSyncOperationState::Pending);
        assert_eq!(operation.last_error_message, None);
        assert_eq!(operation.created_at, operation.available_at);
        assert_eq!(
            operation.publish_payload().expect("payload should parse"),
            payload
        );
    }

    #[test]
    fn publish_work_kinds_are_current_agreement_surface() {
        let work_kinds = [
            AppPublishWorkKind::FarmProfile,
            AppPublishWorkKind::Listing,
            AppPublishWorkKind::OrderRequest,
            AppPublishWorkKind::OrderDecision,
            AppPublishWorkKind::OrderCancellation,
        ];

        assert_eq!(work_kinds.len(), 5);
        assert_eq!(work_kinds[0].storage_key(), "farm_profile");
        assert_eq!(work_kinds[1].storage_key(), "listing");
        assert_eq!(work_kinds[2].storage_key(), "order_request");
        assert_eq!(work_kinds[3].storage_key(), "order_decision");
        assert_eq!(work_kinds[4].storage_key(), "order_cancellation");
    }

    #[test]
    fn listing_publish_payload_reports_stable_validation_reason_codes() {
        let payload = AppPublishPayload::Listing(AppListingPublishPayload {
            context: AppPublishContext::new("", ""),
            product_id: ProductId::generate(),
            listing_d_tag: None,
            farm_id: None,
            farm_pubkey: None,
            farm_d_tag: None,
            title: " ".to_owned(),
            subtitle: None,
            category: None,
            unit_label: String::new(),
            price_minor_units: Some(0),
            price_currency: String::new(),
            stock_quantity: Some(4),
            availability_window_id: None,
            availability_starts_at: None,
            availability_ends_at: None,
            fulfillment_method: None,
            fulfillment_location: None,
            status: ProductStatus::Published,
        });

        let reason_codes: Vec<&str> = payload
            .validation_failures()
            .into_iter()
            .map(AppPublishValidationFailure::storage_key)
            .collect();

        assert_eq!(
            reason_codes,
            vec![
                "missing_account_id",
                "missing_source",
                "missing_listing_farm_id",
                "missing_listing_farm_pubkey",
                "missing_listing_category",
                "missing_listing_title",
                "missing_listing_unit",
                "missing_listing_price",
                "missing_listing_currency",
                "missing_listing_availability",
                "missing_listing_fulfillment_method",
                "missing_listing_fulfillment_location",
            ]
        );
        assert!(payload.validate().is_err());
    }

    #[test]
    fn order_request_publish_payload_requires_sdk_publish_inputs() {
        let payload = AppPublishPayload::OrderRequest(AppOrderRequestPublishPayload {
            context: AppPublishContext::new("acct_buyer", "place_personal_order"),
            order_id: OrderId::generate(),
            farm_id: FarmId::generate(),
            status: Some("needs_action".to_owned()),
            order_document_json: None,
            listing_addr: Some(String::new()),
            listing_event_id: None,
            listing_relays: vec![],
            buyer_pubkey: None,
            seller_pubkey: Some(" ".to_owned()),
            items: vec![AppOrderRequestItemPayload {
                product_id: ProductId::generate(),
                quantity: 0,
            }],
            currency_code: None,
            total_minor_units: None,
            note: None,
            confirm_public_note: false,
        });

        let reason_codes: Vec<&str> = payload
            .validation_failures()
            .into_iter()
            .map(AppPublishValidationFailure::storage_key)
            .collect();

        assert_eq!(
            reason_codes,
            vec![
                "missing_order_document",
                "missing_order_listing_address",
                "missing_order_listing_event_id",
                "missing_order_listing_relay",
                "missing_order_buyer_pubkey",
                "missing_order_seller_pubkey",
                "missing_order_items",
                "missing_order_currency",
                "missing_order_total",
            ]
        );
    }

    #[test]
    fn order_decision_publish_payload_reports_stable_validation_reason_codes() {
        let payload = AppPublishPayload::OrderDecision(AppOrderDecisionPublishPayload {
            context: AppPublishContext::new("", ""),
            app_order_id: OrderId::generate(),
            farm_id: FarmId::generate(),
            trade_order_id: " ".to_owned(),
            request_event_id: String::new(),
            listing_event_id: None,
            listing_addr: String::new(),
            buyer_pubkey: String::new(),
            seller_pubkey: String::new(),
            decision: AppOrderDecisionPayload::Declined {
                reason: " ".to_owned(),
            },
            confirm_public_note: false,
        });

        assert_eq!(payload.work_kind().storage_key(), "order_decision");
        assert_eq!(
            payload.work_kind().sdk_operation(),
            TRADE_DECISION_OPERATION_KIND
        );
        let reason_codes: Vec<&str> = payload
            .validation_failures()
            .into_iter()
            .map(AppPublishValidationFailure::storage_key)
            .collect();

        assert_eq!(
            reason_codes,
            vec![
                "missing_account_id",
                "missing_source",
                "missing_order_trade_order_id",
                "missing_order_request_event_id",
                "missing_order_listing_address",
                "missing_order_buyer_pubkey",
                "missing_order_seller_pubkey",
                "missing_order_decline_reason",
            ]
        );
    }

    #[test]
    fn cancellation_publish_payload_reports_stable_validation_reason_codes() {
        let order_id = OrderId::generate();
        let farm_id = FarmId::generate();
        let cancellation =
            AppPublishPayload::OrderCancellation(AppOrderCancellationPublishPayload {
                context: AppPublishContext::new("", ""),
                app_order_id: order_id,
                farm_id,
                trade_order_id: " ".to_owned(),
                request_event_id: String::new(),
                listing_addr: String::new(),
                buyer_pubkey: String::new(),
                seller_pubkey: String::new(),
                reason: " ".to_owned(),
                confirm_public_note: false,
            });

        assert_eq!(
            cancellation.work_kind().sdk_operation(),
            TRADE_CANCELLATION_OPERATION_KIND
        );

        let cancellation_reason_codes: Vec<&str> = cancellation
            .validation_failures()
            .into_iter()
            .map(AppPublishValidationFailure::storage_key)
            .collect();

        assert_eq!(
            cancellation_reason_codes,
            vec![
                "missing_account_id",
                "missing_source",
                "missing_order_trade_order_id",
                "missing_order_request_event_id",
                "missing_order_listing_address",
                "missing_order_buyer_pubkey",
                "missing_order_seller_pubkey",
                "missing_order_cancellation_reason",
            ]
        );

        let operation = PendingSyncOperation::from_publish_payload(
            AppPublishPayload::OrderCancellation(AppOrderCancellationPublishPayload {
                context: AppPublishContext::new("acct_local", "buyer_order_cancellation"),
                app_order_id: order_id,
                farm_id,
                trade_order_id: "order-1".to_owned(),
                request_event_id: "request-event-1".to_owned(),
                listing_addr: "30402:seller:listing".to_owned(),
                buyer_pubkey: "buyer".to_owned(),
                seller_pubkey: "seller".to_owned(),
                reason: "buyer cancelled order".to_owned(),
                confirm_public_note: false,
            }),
            "2026-04-20T18:00:00Z",
        )
        .expect("typed lifecycle payload should serialize");

        assert_eq!(operation.aggregate, SyncAggregateRef::Order(order_id));
        assert_eq!(operation.operation_key, format!("order:{order_id}:upsert"));
        assert_eq!(operation.operation, SyncOperationKind::Upsert);
        assert_eq!(
            operation.publish_payload().expect("payload should parse"),
            AppPublishPayload::OrderCancellation(AppOrderCancellationPublishPayload {
                context: AppPublishContext::new("acct_local", "buyer_order_cancellation"),
                app_order_id: order_id,
                farm_id,
                trade_order_id: "order-1".to_owned(),
                request_event_id: "request-event-1".to_owned(),
                listing_addr: "30402:seller:listing".to_owned(),
                buyer_pubkey: "buyer".to_owned(),
                seller_pubkey: "seller".to_owned(),
                reason: "buyer cancelled order".to_owned(),
                confirm_public_note: false,
            })
        );
    }

    #[test]
    fn existing_raw_payload_outbox_work_rejects_publish_payload() {
        let pending_operation = PendingSyncOperation {
            operation_key: "product:greens:upsert".to_owned(),
            aggregate: SyncAggregateRef::Product(ProductId::generate()),
            operation: SyncOperationKind::Upsert,
            payload_json: "{\"title\":\"greens\"}".to_owned(),
            created_at: "2026-04-17T19:32:00Z".to_owned(),
            available_at: "2026-04-17T19:32:00Z".to_owned(),
            attempt_count: 0,
            state: PendingSyncOperationState::Pending,
            last_error_message: None,
        };

        assert!(!pending_operation.is_retry());
        assert!(pending_operation.publish_payload().is_err());
    }
}
