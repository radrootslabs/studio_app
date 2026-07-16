use radroots_event::trade::{RadrootsTradeMutationBodyV1, RadrootsTradeMutationEnvelopeV1};
use radroots_sdk::{
    FARM_PUBLISH_OPERATION_KIND, LISTING_PUBLISH_OPERATION_KIND, TRADE_CANCEL_OPERATION_KIND,
    TRADE_DECIDE_CANDIDATE_OPERATION_KIND, TRADE_PROPOSE_REVISION_OPERATION_KIND,
    TRADE_SUBMIT_PROPOSAL_OPERATION_KIND,
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
    TradeProposal,
    TradeRevisionProposal,
    TradeCandidateDecision,
    TradeCancellation,
}

impl AppPublishWorkKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::FarmProfile => "farm_profile",
            Self::Listing => "listing",
            Self::TradeProposal => "trade_proposal",
            Self::TradeRevisionProposal => "trade_revision_proposal",
            Self::TradeCandidateDecision => "trade_candidate_decision",
            Self::TradeCancellation => "trade_cancellation",
        }
    }

    pub const fn sdk_operation(self) -> &'static str {
        match self {
            Self::FarmProfile => FARM_PUBLISH_OPERATION_KIND,
            Self::Listing => LISTING_PUBLISH_OPERATION_KIND,
            Self::TradeProposal => TRADE_SUBMIT_PROPOSAL_OPERATION_KIND,
            Self::TradeRevisionProposal => TRADE_PROPOSE_REVISION_OPERATION_KIND,
            Self::TradeCandidateDecision => TRADE_DECIDE_CANDIDATE_OPERATION_KIND,
            Self::TradeCancellation => TRADE_CANCEL_OPERATION_KIND,
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
pub struct AppTradeProposalPublishPayload {
    pub context: AppPublishContext,
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub envelope: RadrootsTradeMutationEnvelopeV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppTradeRevisionProposalPublishPayload {
    pub context: AppPublishContext,
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub envelope: RadrootsTradeMutationEnvelopeV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppTradeCandidateDecisionPublishPayload {
    pub context: AppPublishContext,
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub envelope: RadrootsTradeMutationEnvelopeV1,
    pub acknowledge_private_terms: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppTradeCancellationPublishPayload {
    pub context: AppPublishContext,
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub envelope: RadrootsTradeMutationEnvelopeV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "publish_kind", content = "payload", rename_all = "snake_case")]
pub enum AppPublishPayload {
    FarmProfile(AppFarmProfilePublishPayload),
    Listing(AppListingPublishPayload),
    TradeProposal(AppTradeProposalPublishPayload),
    TradeRevisionProposal(AppTradeRevisionProposalPublishPayload),
    TradeCandidateDecision(AppTradeCandidateDecisionPublishPayload),
    TradeCancellation(AppTradeCancellationPublishPayload),
}

impl AppPublishPayload {
    pub const fn work_kind(&self) -> AppPublishWorkKind {
        match self {
            Self::FarmProfile(_) => AppPublishWorkKind::FarmProfile,
            Self::Listing(_) => AppPublishWorkKind::Listing,
            Self::TradeProposal(_) => AppPublishWorkKind::TradeProposal,
            Self::TradeRevisionProposal(_) => AppPublishWorkKind::TradeRevisionProposal,
            Self::TradeCandidateDecision(_) => AppPublishWorkKind::TradeCandidateDecision,
            Self::TradeCancellation(_) => AppPublishWorkKind::TradeCancellation,
        }
    }

    pub const fn operation_kind(&self) -> SyncOperationKind {
        SyncOperationKind::Upsert
    }

    pub fn aggregate_ref(&self) -> SyncAggregateRef {
        match self {
            Self::FarmProfile(payload) => SyncAggregateRef::Farm(payload.farm_id),
            Self::Listing(payload) => SyncAggregateRef::Product(payload.product_id),
            Self::TradeProposal(payload) => SyncAggregateRef::Order(payload.order_id),
            Self::TradeRevisionProposal(payload) => SyncAggregateRef::Order(payload.order_id),
            Self::TradeCandidateDecision(payload) => SyncAggregateRef::Order(payload.order_id),
            Self::TradeCancellation(payload) => SyncAggregateRef::Order(payload.order_id),
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
            Self::TradeProposal(payload) => {
                payload.context.validation_failures(&mut failures);
                validate_trade_envelope_kind(
                    &payload.envelope,
                    TradeMutationPayloadKind::Proposal,
                    &mut failures,
                );
            }
            Self::TradeRevisionProposal(payload) => {
                payload.context.validation_failures(&mut failures);
                validate_trade_envelope_kind(
                    &payload.envelope,
                    TradeMutationPayloadKind::RevisionProposal,
                    &mut failures,
                );
            }
            Self::TradeCandidateDecision(payload) => {
                payload.context.validation_failures(&mut failures);
                validate_trade_envelope_kind(
                    &payload.envelope,
                    TradeMutationPayloadKind::CandidateDecision,
                    &mut failures,
                );
            }
            Self::TradeCancellation(payload) => {
                payload.context.validation_failures(&mut failures);
                validate_trade_envelope_kind(
                    &payload.envelope,
                    TradeMutationPayloadKind::Cancellation,
                    &mut failures,
                );
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TradeMutationPayloadKind {
    Proposal,
    RevisionProposal,
    CandidateDecision,
    Cancellation,
}

fn validate_trade_envelope_kind(
    envelope: &RadrootsTradeMutationEnvelopeV1,
    expected: TradeMutationPayloadKind,
    failures: &mut Vec<AppPublishValidationFailure>,
) {
    if envelope.validate().is_err() {
        failures.push(AppPublishValidationFailure::InvalidTradeMutationEnvelope);
        return;
    }
    let actual = match &envelope.body {
        RadrootsTradeMutationBodyV1::Proposal { .. } => TradeMutationPayloadKind::Proposal,
        RadrootsTradeMutationBodyV1::RevisionProposal { .. } => {
            TradeMutationPayloadKind::RevisionProposal
        }
        RadrootsTradeMutationBodyV1::Decision { .. }
        | RadrootsTradeMutationBodyV1::RevisionDecision { .. } => {
            TradeMutationPayloadKind::CandidateDecision
        }
        RadrootsTradeMutationBodyV1::Cancellation { .. } => TradeMutationPayloadKind::Cancellation,
    };
    if actual != expected {
        failures.push(AppPublishValidationFailure::WrongTradeMutationKind);
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
    InvalidTradeMutationEnvelope,
    WrongTradeMutationKind,
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
            Self::InvalidTradeMutationEnvelope => "invalid_trade_mutation_envelope",
            Self::WrongTradeMutationKind => "wrong_trade_mutation_kind",
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
        AppFarmProfilePublishPayload, AppListingPublishPayload, AppPublishContext,
        AppPublishPayload, AppPublishValidationFailure, AppPublishWorkKind,
        AppTradeCancellationPublishPayload, AppTradeCandidateDecisionPublishPayload,
        AppTradeProposalPublishPayload, AppTradeRevisionProposalPublishPayload,
        FARM_PUBLISH_OPERATION_KIND, TRADE_CANCEL_OPERATION_KIND,
        TRADE_DECIDE_CANDIDATE_OPERATION_KIND, TRADE_PROPOSE_REVISION_OPERATION_KIND,
    };
    use crate::{
        PendingSyncOperation, PendingSyncOperationState, SyncAggregateRef, SyncOperationKind,
    };
    use radroots_event::{
        ids::{
            RadrootsAddressableCoordinate, RadrootsDTag, RadrootsEventId, RadrootsInventoryBinId,
            RadrootsPublicKey, RadrootsTradeId,
        },
        trade::{
            RADROOTS_TRADE_CANCELLATION_CONTRACT_ID, RADROOTS_TRADE_DECISION_CONTRACT_ID,
            RADROOTS_TRADE_PROPOSAL_CONTRACT_ID, RADROOTS_TRADE_REVISION_PROPOSAL_CONTRACT_ID,
            RADROOTS_TRADE_SCHEMA_VERSION, RadrootsFulfillmentProfileV1,
            RadrootsTradeCancellationProfileV1, RadrootsTradeCandidateLineV1,
            RadrootsTradeCandidateTermsV1, RadrootsTradeDecisionV1,
            RadrootsTradeEconomicsProfileV1, RadrootsTradeMutationBodyV1,
            RadrootsTradeMutationEnvelopeV1, canonical_trade_mutation_content,
        },
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
            AppPublishWorkKind::TradeProposal,
            AppPublishWorkKind::TradeRevisionProposal,
            AppPublishWorkKind::TradeCandidateDecision,
            AppPublishWorkKind::TradeCancellation,
        ];

        assert_eq!(work_kinds.len(), 6);
        assert_eq!(work_kinds[0].storage_key(), "farm_profile");
        assert_eq!(work_kinds[1].storage_key(), "listing");
        assert_eq!(work_kinds[2].storage_key(), "trade_proposal");
        assert_eq!(work_kinds[3].storage_key(), "trade_revision_proposal");
        assert_eq!(work_kinds[4].storage_key(), "trade_candidate_decision");
        assert_eq!(work_kinds[5].storage_key(), "trade_cancellation");
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
    fn trade_proposal_publish_payload_uses_sdk_envelope() {
        let order_id = OrderId::generate();
        let proposal = proposal_envelope();
        let payload = AppPublishPayload::TradeProposal(AppTradeProposalPublishPayload {
            context: AppPublishContext::new("acct_buyer", "trade_proposal"),
            order_id,
            farm_id: FarmId::generate(),
            envelope: proposal.clone(),
        });

        assert_eq!(payload.validation_failures(), Vec::new());
        assert_eq!(payload.work_kind().storage_key(), "trade_proposal");
        assert_eq!(
            payload.work_kind().sdk_operation(),
            super::TRADE_SUBMIT_PROPOSAL_OPERATION_KIND
        );

        let operation =
            PendingSyncOperation::from_publish_payload(payload.clone(), "2026-04-20T18:00:00Z")
                .expect("typed trade payload should serialize");

        assert_eq!(operation.aggregate, SyncAggregateRef::Order(order_id));
        assert_eq!(operation.operation_key, format!("order:{order_id}:upsert"));
        assert_eq!(
            operation.publish_payload().expect("payload should parse"),
            AppPublishPayload::TradeProposal(AppTradeProposalPublishPayload {
                context: AppPublishContext::new("acct_buyer", "trade_proposal"),
                order_id,
                farm_id: match payload {
                    AppPublishPayload::TradeProposal(payload) => payload.farm_id,
                    _ => unreachable!(),
                },
                envelope: proposal,
            })
        );
    }

    #[test]
    fn trade_revision_and_decision_payloads_validate_expected_mutation_kind() {
        let order_id = OrderId::generate();
        let farm_id = FarmId::generate();
        let proposal = proposal_envelope();
        let revision = revision_proposal_envelope(&proposal);
        let decision = decision_envelope(&revision);

        let revision_payload =
            AppPublishPayload::TradeRevisionProposal(AppTradeRevisionProposalPublishPayload {
                context: AppPublishContext::new("acct_buyer", "trade_revision_proposal"),
                order_id,
                farm_id,
                envelope: revision,
            });
        assert_eq!(revision_payload.validation_failures(), Vec::new());
        assert_eq!(
            revision_payload.work_kind().sdk_operation(),
            TRADE_PROPOSE_REVISION_OPERATION_KIND
        );

        let decision_payload =
            AppPublishPayload::TradeCandidateDecision(AppTradeCandidateDecisionPublishPayload {
                context: AppPublishContext::new("acct_seller", "trade_candidate_decision"),
                order_id,
                farm_id,
                envelope: decision,
                acknowledge_private_terms: true,
            });
        assert_eq!(decision_payload.validation_failures(), Vec::new());
        assert_eq!(
            decision_payload.work_kind().sdk_operation(),
            TRADE_DECIDE_CANDIDATE_OPERATION_KIND
        );

        let wrong_kind =
            AppPublishPayload::TradeCandidateDecision(AppTradeCandidateDecisionPublishPayload {
                context: AppPublishContext::new("acct_seller", "trade_candidate_decision"),
                order_id,
                farm_id,
                envelope: proposal_envelope(),
                acknowledge_private_terms: false,
            });

        assert_eq!(
            wrong_kind
                .validation_failures()
                .into_iter()
                .map(AppPublishValidationFailure::storage_key)
                .collect::<Vec<_>>(),
            vec!["wrong_trade_mutation_kind"]
        );
    }

    #[test]
    fn trade_payload_reports_stable_invalid_envelope_reason_code() {
        let mut envelope = proposal_envelope();
        envelope.contract_id = RADROOTS_TRADE_DECISION_CONTRACT_ID.to_owned();
        let payload = AppPublishPayload::TradeProposal(AppTradeProposalPublishPayload {
            context: AppPublishContext::new("", ""),
            order_id: OrderId::generate(),
            farm_id: FarmId::generate(),
            envelope,
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
                "invalid_trade_mutation_envelope",
            ]
        );
    }

    #[test]
    fn cancellation_publish_payload_round_trips_semantic_envelope() {
        let order_id = OrderId::generate();
        let farm_id = FarmId::generate();
        let proposal = proposal_envelope();
        let cancellation_envelope = cancellation_envelope(&proposal);
        let cancellation =
            AppPublishPayload::TradeCancellation(AppTradeCancellationPublishPayload {
                context: AppPublishContext::new("acct_buyer", "trade_cancellation"),
                order_id,
                farm_id,
                envelope: cancellation_envelope.clone(),
            });

        assert_eq!(
            cancellation.work_kind().sdk_operation(),
            TRADE_CANCEL_OPERATION_KIND
        );
        assert_eq!(cancellation.validation_failures(), Vec::new());

        let operation =
            PendingSyncOperation::from_publish_payload(cancellation, "2026-04-20T18:00:00Z")
                .expect("typed lifecycle payload should serialize");

        assert_eq!(operation.aggregate, SyncAggregateRef::Order(order_id));
        assert_eq!(operation.operation_key, format!("order:{order_id}:upsert"));
        assert_eq!(operation.operation, SyncOperationKind::Upsert);
        assert_eq!(
            operation.publish_payload().expect("payload should parse"),
            AppPublishPayload::TradeCancellation(AppTradeCancellationPublishPayload {
                context: AppPublishContext::new("acct_buyer", "trade_cancellation"),
                order_id,
                farm_id,
                envelope: cancellation_envelope,
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

    fn hex_64(character: char) -> String {
        core::iter::repeat_n(character, 64).collect()
    }

    fn hex_32(character: char) -> String {
        core::iter::repeat_n(character, 32).collect()
    }

    fn pubkey(character: char) -> RadrootsPublicKey {
        RadrootsPublicKey::parse(hex_64(character)).expect("pubkey")
    }

    fn event_id(character: char) -> RadrootsEventId {
        RadrootsEventId::parse(hex_64(character)).expect("event id")
    }

    fn trade_id() -> RadrootsTradeId {
        RadrootsTradeId::parse(hex_32('1')).expect("trade id")
    }

    fn candidate() -> RadrootsTradeCandidateTermsV1 {
        RadrootsTradeCandidateTermsV1 {
            candidate_id: None,
            schema_version: RADROOTS_TRADE_SCHEMA_VERSION,
            base_candidate_id: None,
            supersession_intent: None,
            buyer_pubkey: pubkey('a'),
            seller_pubkey: pubkey('b'),
            farm_id: RadrootsDTag::parse("farm-1").expect("farm d tag"),
            lines: vec![RadrootsTradeCandidateLineV1 {
                line_id: RadrootsDTag::parse("line-1").expect("line d tag"),
                listing_addr: RadrootsAddressableCoordinate::parse(format!(
                    "30402:{}:listing-1",
                    hex_64('b')
                ))
                .expect("listing addr"),
                listing_event_id: event_id('c'),
                listing_snapshot_sha256: hex_64('d'),
                product_id: "carrots".to_owned(),
                option_id: None,
                bin_id: RadrootsInventoryBinId::parse("bin-1").expect("inventory bin"),
                quantity_mantissa: "2".to_owned(),
                quantity_scale: 0,
                unit_code: "count".to_owned(),
                unit_profile: "mvp-count".to_owned(),
                unit_price_mantissa: "500".to_owned(),
                currency_code: "USD".to_owned(),
                line_subtotal_mantissa: "1000".to_owned(),
                replaces_line_id: None,
            }],
            line_tombstones: Vec::new(),
            economics: RadrootsTradeEconomicsProfileV1 {
                profile_id: "mvp-fixed".to_owned(),
                currency_code: "USD".to_owned(),
                currency_exponent: 2,
                rounding_profile: "half-even".to_owned(),
                subtotal_mantissa: "1000".to_owned(),
                discount_total_mantissa: "0".to_owned(),
                adjustment_total_mantissa: "0".to_owned(),
                total_mantissa: "1000".to_owned(),
                adjustments: Vec::new(),
            },
            fulfillment: RadrootsFulfillmentProfileV1 {
                profile_id: "market-pickup".to_owned(),
                method: "pickup".to_owned(),
                starts_at_unix_s: 1_800_000_000,
                ends_at_unix_s: 1_800_003_600,
                timezone: "America/New_York".to_owned(),
                utc_offset_seconds: -18_000,
                fold: 0,
                location_class: "farmstand".to_owned(),
                requires_private_terms: false,
            },
            cancellation: RadrootsTradeCancellationProfileV1 {
                profile_id: "buyer-pre-agreement".to_owned(),
                buyer_pre_agreement: true,
                post_agreement_cutoff_unix_s: None,
            },
            private_terms: None,
            proposal_expires_at_unix_s: 1_799_999_000,
        }
    }

    fn proposal_envelope() -> RadrootsTradeMutationEnvelopeV1 {
        canonical_trade_mutation_content(RadrootsTradeMutationEnvelopeV1 {
            mutation_id: None,
            contract_id: RADROOTS_TRADE_PROPOSAL_CONTRACT_ID.to_owned(),
            schema_version: RADROOTS_TRADE_SCHEMA_VERSION,
            trade_id: trade_id(),
            root_mutation_id: None,
            buyer_pubkey: pubkey('a'),
            seller_pubkey: pubkey('b'),
            farm_id: RadrootsDTag::parse("farm-1").expect("farm d tag"),
            parent_mutation_ids: Vec::new(),
            author_pubkey: pubkey('a'),
            counterparty_pubkey: pubkey('b'),
            authored_at_unix_s: 1_799_000_000,
            body: RadrootsTradeMutationBodyV1::Proposal {
                candidate: candidate(),
            },
        })
        .expect("canonical proposal")
        .envelope
    }

    fn revision_proposal_envelope(
        root: &RadrootsTradeMutationEnvelopeV1,
    ) -> RadrootsTradeMutationEnvelopeV1 {
        canonical_trade_mutation_content(RadrootsTradeMutationEnvelopeV1 {
            mutation_id: None,
            contract_id: RADROOTS_TRADE_REVISION_PROPOSAL_CONTRACT_ID.to_owned(),
            schema_version: RADROOTS_TRADE_SCHEMA_VERSION,
            trade_id: root.trade_id.clone(),
            root_mutation_id: root.mutation_id.clone(),
            buyer_pubkey: pubkey('a'),
            seller_pubkey: pubkey('b'),
            farm_id: RadrootsDTag::parse("farm-1").expect("farm d tag"),
            parent_mutation_ids: vec![root.mutation_id.clone().expect("root mutation id")],
            author_pubkey: pubkey('a'),
            counterparty_pubkey: pubkey('b'),
            authored_at_unix_s: 1_799_000_100,
            body: RadrootsTradeMutationBodyV1::RevisionProposal {
                candidate: candidate(),
            },
        })
        .expect("canonical revision")
        .envelope
    }

    fn decision_envelope(
        proposal: &RadrootsTradeMutationEnvelopeV1,
    ) -> RadrootsTradeMutationEnvelopeV1 {
        let proposal_mutation_id = proposal.mutation_id.clone().expect("proposal mutation id");
        let candidate_id = match &proposal.body {
            RadrootsTradeMutationBodyV1::Proposal { candidate }
            | RadrootsTradeMutationBodyV1::RevisionProposal { candidate } => {
                candidate.candidate_id.clone().expect("candidate id")
            }
            _ => unreachable!(),
        };
        canonical_trade_mutation_content(RadrootsTradeMutationEnvelopeV1 {
            mutation_id: None,
            contract_id: RADROOTS_TRADE_DECISION_CONTRACT_ID.to_owned(),
            schema_version: RADROOTS_TRADE_SCHEMA_VERSION,
            trade_id: proposal.trade_id.clone(),
            root_mutation_id: proposal
                .root_mutation_id
                .clone()
                .or(proposal.mutation_id.clone()),
            buyer_pubkey: pubkey('a'),
            seller_pubkey: pubkey('b'),
            farm_id: RadrootsDTag::parse("farm-1").expect("farm d tag"),
            parent_mutation_ids: vec![proposal_mutation_id.clone()],
            author_pubkey: pubkey('b'),
            counterparty_pubkey: pubkey('a'),
            authored_at_unix_s: 1_799_000_200,
            body: RadrootsTradeMutationBodyV1::Decision {
                proposal_mutation_id,
                candidate_id,
                decision: RadrootsTradeDecisionV1::Declined {
                    reason: "not available".to_owned(),
                },
            },
        })
        .expect("canonical decision")
        .envelope
    }

    fn cancellation_envelope(
        root: &RadrootsTradeMutationEnvelopeV1,
    ) -> RadrootsTradeMutationEnvelopeV1 {
        let root_mutation_id = root.mutation_id.clone().expect("root mutation id");
        let candidate_id = match &root.body {
            RadrootsTradeMutationBodyV1::Proposal { candidate } => {
                candidate.candidate_id.clone().expect("candidate id")
            }
            _ => unreachable!(),
        };
        canonical_trade_mutation_content(RadrootsTradeMutationEnvelopeV1 {
            mutation_id: None,
            contract_id: RADROOTS_TRADE_CANCELLATION_CONTRACT_ID.to_owned(),
            schema_version: RADROOTS_TRADE_SCHEMA_VERSION,
            trade_id: root.trade_id.clone(),
            root_mutation_id: Some(root_mutation_id.clone()),
            buyer_pubkey: pubkey('a'),
            seller_pubkey: pubkey('b'),
            farm_id: RadrootsDTag::parse("farm-1").expect("farm d tag"),
            parent_mutation_ids: vec![root_mutation_id],
            author_pubkey: pubkey('a'),
            counterparty_pubkey: pubkey('b'),
            authored_at_unix_s: 1_799_000_300,
            body: RadrootsTradeMutationBodyV1::Cancellation {
                target_candidate_id: Some(candidate_id),
                target_claim_mutation_id: None,
                reason: "buyer cancelled".to_owned(),
            },
        })
        .expect("canonical cancellation")
        .envelope
    }
}
