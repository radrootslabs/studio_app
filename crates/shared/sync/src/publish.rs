use radroots_studio_app_models::{
    FarmId, FarmReadiness, FulfillmentWindowId, OrderId, ProductId, ProductStatus,
};
use radroots_sdk::SdkTransportMode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef, SyncOperationKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppPublishWorkKind {
    FarmProfile,
    Listing,
    OrderRequest,
}

impl AppPublishWorkKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::FarmProfile => "farm_profile",
            Self::Listing => "listing",
            Self::OrderRequest => "order_request",
        }
    }

    pub const fn sdk_operation(self) -> &'static str {
        match self {
            Self::FarmProfile => "farm.publish_draft_with_identity",
            Self::Listing => "listing.publish_draft_with_identity",
            Self::OrderRequest => "trade.publish_order_request_with_identity",
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "publish_kind", content = "payload", rename_all = "snake_case")]
pub enum AppPublishPayload {
    FarmProfile(AppFarmProfilePublishPayload),
    Listing(AppListingPublishPayload),
    OrderRequest(AppOrderRequestPublishPayload),
}

impl AppPublishPayload {
    pub const fn work_kind(&self) -> AppPublishWorkKind {
        match self {
            Self::FarmProfile(_) => AppPublishWorkKind::FarmProfile,
            Self::Listing(_) => AppPublishWorkKind::Listing,
            Self::OrderRequest(_) => AppPublishWorkKind::OrderRequest,
        }
    }

    pub const fn sdk_transport_mode(&self) -> SdkTransportMode {
        SdkTransportMode::RelayDirect
    }

    pub const fn operation_kind(&self) -> SyncOperationKind {
        SyncOperationKind::Upsert
    }

    pub fn aggregate_ref(&self) -> SyncAggregateRef {
        match self {
            Self::FarmProfile(payload) => SyncAggregateRef::Farm(payload.farm_id),
            Self::Listing(payload) => SyncAggregateRef::Product(payload.product_id),
            Self::OrderRequest(payload) => SyncAggregateRef::Order(payload.order_id),
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
                if payload.availability_window_id.is_none() {
                    failures.push(AppPublishValidationFailure::MissingListingAvailability);
                }
                if payload
                    .fulfillment_method
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    failures.push(AppPublishValidationFailure::MissingListingFulfillmentMethod);
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
    MissingListingFulfillmentMethod,
    MissingOrderDocument,
    MissingOrderListingAddress,
    MissingOrderListingEventId,
    MissingOrderListingRelay,
    MissingOrderBuyerPubkey,
    MissingOrderSellerPubkey,
    MissingOrderItems,
    MissingOrderCurrency,
    MissingOrderTotal,
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
            Self::MissingListingFulfillmentMethod => "missing_listing_fulfillment_method",
            Self::MissingOrderDocument => "missing_order_document",
            Self::MissingOrderListingAddress => "missing_order_listing_address",
            Self::MissingOrderListingEventId => "missing_order_listing_event_id",
            Self::MissingOrderListingRelay => "missing_order_listing_relay",
            Self::MissingOrderBuyerPubkey => "missing_order_buyer_pubkey",
            Self::MissingOrderSellerPubkey => "missing_order_seller_pubkey",
            Self::MissingOrderItems => "missing_order_items",
            Self::MissingOrderCurrency => "missing_order_currency",
            Self::MissingOrderTotal => "missing_order_total",
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
        AppFarmProfilePublishPayload, AppListingPublishPayload, AppOrderRequestItemPayload,
        AppOrderRequestPublishPayload, AppPublishContext, AppPublishPayload,
        AppPublishValidationFailure,
    };
    use crate::{
        PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef, SyncOperationKind,
    };
    use radroots_studio_app_models::{FarmId, FarmReadiness, OrderId, ProductId, ProductStatus};

    #[test]
    fn publish_payload_serializes_with_stable_kind_and_sdk_target() {
        let farm_id = FarmId::new();
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
            "farm.publish_draft_with_identity"
        );
        assert_eq!(
            payload.sdk_transport_mode(),
            radroots_sdk::SdkTransportMode::RelayDirect
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
    fn listing_publish_payload_reports_stable_validation_reason_codes() {
        let payload = AppPublishPayload::Listing(AppListingPublishPayload {
            context: AppPublishContext::new("", ""),
            product_id: ProductId::new(),
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
            ]
        );
        assert!(payload.validate().is_err());
    }

    #[test]
    fn order_request_publish_payload_requires_sdk_publish_inputs() {
        let payload = AppPublishPayload::OrderRequest(AppOrderRequestPublishPayload {
            context: AppPublishContext::new("acct_buyer", "place_personal_order"),
            order_id: OrderId::new(),
            farm_id: FarmId::new(),
            status: Some("needs_action".to_owned()),
            order_document_json: None,
            listing_addr: Some(String::new()),
            listing_event_id: None,
            listing_relays: vec![],
            buyer_pubkey: None,
            seller_pubkey: Some(" ".to_owned()),
            items: vec![AppOrderRequestItemPayload {
                product_id: ProductId::new(),
                quantity: 0,
            }],
            currency_code: None,
            total_minor_units: None,
            note: None,
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
    fn existing_raw_payload_outbox_work_remains_local_save_compatible() {
        let pending_operation = PendingSyncOperation {
            operation_key: "product:greens:upsert".to_owned(),
            aggregate: SyncAggregateRef::Product(ProductId::new()),
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
