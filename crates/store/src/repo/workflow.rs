use radroots_studio_app_view::{
    OrderId, TradeAgreementStatus, TradeEconomicsProjection, TradeFulfillmentStatus,
    TradeInventoryStatus, TradePaymentDisplayStatus, TradeProvenanceProjection,
    TradeRevisionStatus, TradeWorkflowProjection, TradeWorkflowSource,
};

use crate::AppSqliteError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StoredTradeWorkflowSnapshot {
    pub order_id: OrderId,
    pub revision: TradeRevisionStatus,
    pub economics: TradeEconomicsProjection,
    pub agreement: String,
    pub fulfillment: Option<String>,
    pub inventory: String,
    pub payment: String,
    pub provenance_source: String,
    pub provenance_last_event_id: Option<String>,
}

pub(super) fn trade_workflow_projection_from_storage(
    snapshot: StoredTradeWorkflowSnapshot,
) -> Result<TradeWorkflowProjection, AppSqliteError> {
    Ok(TradeWorkflowProjection {
        order_id: snapshot.order_id,
        agreement: parse_trade_agreement_status("orders.workflow_agreement", snapshot.agreement)?,
        revision: snapshot.revision,
        fulfillment: snapshot
            .fulfillment
            .map(|value| parse_trade_fulfillment_status("orders.workflow_fulfillment", value))
            .transpose()?,
        economics: snapshot.economics,
        inventory: parse_trade_inventory_status("orders.workflow_inventory", snapshot.inventory)?,
        payment: parse_trade_payment_display_status("orders.workflow_payment", snapshot.payment)?,
        provenance: TradeProvenanceProjection::from_primary_source(parse_trade_workflow_source(
            "orders.workflow_provenance_source",
            snapshot.provenance_source,
        )?)
        .with_last_event_id(snapshot.provenance_last_event_id),
    })
}

fn parse_trade_agreement_status(
    field: &'static str,
    value: String,
) -> Result<TradeAgreementStatus, AppSqliteError> {
    match value.as_str() {
        "ordered" => Ok(TradeAgreementStatus::Ordered),
        "confirmed" => Ok(TradeAgreementStatus::Confirmed),
        "declined" => Ok(TradeAgreementStatus::Declined),
        "cancelled" => Ok(TradeAgreementStatus::Cancelled),
        "completed" => Ok(TradeAgreementStatus::Completed),
        "needs_review" => Ok(TradeAgreementStatus::NeedsReview),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_trade_fulfillment_status(
    field: &'static str,
    value: String,
) -> Result<TradeFulfillmentStatus, AppSqliteError> {
    match value.as_str() {
        "confirmed" => Ok(TradeFulfillmentStatus::Confirmed),
        "preparing" => Ok(TradeFulfillmentStatus::Preparing),
        "ready_for_pickup" => Ok(TradeFulfillmentStatus::ReadyForPickup),
        "out_for_delivery" => Ok(TradeFulfillmentStatus::OutForDelivery),
        "delivered" => Ok(TradeFulfillmentStatus::Delivered),
        "cancelled" => Ok(TradeFulfillmentStatus::Cancelled),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_trade_inventory_status(
    field: &'static str,
    value: String,
) -> Result<TradeInventoryStatus, AppSqliteError> {
    match value.as_str() {
        "available" => Ok(TradeInventoryStatus::Available),
        "reserved" => Ok(TradeInventoryStatus::Reserved),
        "sold_out" => Ok(TradeInventoryStatus::SoldOut),
        "needs_review" => Ok(TradeInventoryStatus::NeedsReview),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_trade_payment_display_status(
    field: &'static str,
    value: String,
) -> Result<TradePaymentDisplayStatus, AppSqliteError> {
    match value.as_str() {
        "not_recorded" => Ok(TradePaymentDisplayStatus::NotRecorded),
        "pending" => Ok(TradePaymentDisplayStatus::Pending),
        "recorded" => Ok(TradePaymentDisplayStatus::Recorded),
        "settled" => Ok(TradePaymentDisplayStatus::Settled),
        "needs_review" => Ok(TradePaymentDisplayStatus::NeedsReview),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

fn parse_trade_workflow_source(
    field: &'static str,
    value: String,
) -> Result<TradeWorkflowSource, AppSqliteError> {
    match value.as_str() {
        "app" => Ok(TradeWorkflowSource::App),
        "cli" => Ok(TradeWorkflowSource::Cli),
        "relay" => Ok(TradeWorkflowSource::Relay),
        "local_events" => Ok(TradeWorkflowSource::LocalEvents),
        "unknown" => Ok(TradeWorkflowSource::Unknown),
        _ => Err(AppSqliteError::DecodeEnum { field, value }),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_trade_payment_display_status;
    use crate::AppSqliteError;
    use radroots_studio_app_view::TradePaymentDisplayStatus;

    #[test]
    fn workflow_payment_display_parser_accepts_all_payment_states() {
        for (stored, expected) in [
            ("not_recorded", TradePaymentDisplayStatus::NotRecorded),
            ("pending", TradePaymentDisplayStatus::Pending),
            ("recorded", TradePaymentDisplayStatus::Recorded),
            ("settled", TradePaymentDisplayStatus::Settled),
            ("needs_review", TradePaymentDisplayStatus::NeedsReview),
        ] {
            assert_eq!(
                parse_trade_payment_display_status("orders.workflow_payment", stored.to_owned())
                    .expect("workflow payment should parse"),
                expected
            );
        }
    }

    #[test]
    fn workflow_payment_display_parser_rejects_unknown_payment_state() {
        let error =
            parse_trade_payment_display_status("orders.workflow_payment", "unknown".to_owned())
                .expect_err("unknown workflow payment should reject");

        match error {
            AppSqliteError::DecodeEnum { field, value } => {
                assert_eq!(field, "orders.workflow_payment");
                assert_eq!(value, "unknown");
            }
            other => panic!("expected DecodeEnum error, got {other:?}"),
        }
    }
}
