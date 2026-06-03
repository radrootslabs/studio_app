pub(crate) mod activation;
pub(crate) mod activity;
pub(crate) mod buyer;
pub(crate) mod farm_rules;
pub(crate) mod farm_setup;
pub(crate) mod order_detail;
pub(crate) mod orders;
pub(crate) mod products;
pub(crate) mod reminders;
pub(crate) mod today;

use radroots_studio_app_view::TradeRevisionStatus;

use crate::AppSqliteError;

pub use activation::AppActivationRepository;
pub use activity::{
    APP_ACTIVITY_CONTEXT_LIMIT, APP_ACTIVITY_RETENTION_LIMIT, AppActivityRepository,
};
pub use buyer::{
    AppBuyerRepository, BuyerOrderCoordinationRecord, BuyerOrderCoordinationState,
    BuyerOrderLocalEventExport, BuyerOrderLocalEventLine, BuyerRepeatDemandApplyOutcome,
};
pub use farm_rules::{AppFarmRulesRepository, derive_farm_rules_readiness};
pub use farm_setup::AppFarmSetupRepository;
pub use orders::{AppOrdersRepository, SellerOrderDecisionExport, SellerOrderDecisionLineExport};
pub use products::AppProductsRepository;
pub use reminders::AppRemindersRepository;
pub use today::{
    AppTodayAgendaRepository, TODAY_AGENDA_LIST_LIMIT, TODAY_AGENDA_LOW_STOCK_THRESHOLD,
};

pub(crate) fn parse_trade_revision_status(
    field: &'static str,
    value: String,
) -> Result<TradeRevisionStatus, AppSqliteError> {
    TradeRevisionStatus::try_from_storage_key(value.as_str())
        .map_err(|_| AppSqliteError::DecodeEnum { field, value })
}
