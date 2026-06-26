use radroots_studio_app_view::{
    OrderId, TradeAgreementStatus, TradeEconomicsProjection, TradeInventoryStatus,
    TradeProvenanceProjection, TradeRevisionStatus, TradeWorkflowProjection, TradeWorkflowSource,
};

use crate::AppSqliteError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StoredTradeWorkflowSnapshot {
    pub order_id: OrderId,
    pub revision: TradeRevisionStatus,
    pub economics: TradeEconomicsProjection,
    pub agreement: String,
    pub inventory: String,
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
        economics: snapshot.economics,
        inventory: parse_trade_inventory_status("orders.workflow_inventory", snapshot.inventory)?,
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
        "requested" => Ok(TradeAgreementStatus::Requested),
        "revision_proposed" => Ok(TradeAgreementStatus::RevisionProposed),
        "agreed_pending_rhi" => Ok(TradeAgreementStatus::AgreedPendingRhi),
        "committed" => Ok(TradeAgreementStatus::Committed),
        "declined" => Ok(TradeAgreementStatus::Declined),
        "cancelled" => Ok(TradeAgreementStatus::Cancelled),
        "invalid" => Ok(TradeAgreementStatus::Invalid),
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
