#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection {
    #[default]
    Account,
    Farm,
    Settings,
    About,
}

impl SettingsSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Account => "settings.account",
            Self::Farm => "settings.farm",
            Self::Settings => "settings.settings",
            Self::About => "settings.about",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsPreference {
    AllowRelayConnections,
    UseMediaServers,
    UseNip05,
    LaunchAtLogin,
}

impl SettingsPreference {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::AllowRelayConnections => "allow_relay_connections",
            Self::UseMediaServers => "use_media_servers",
            Self::UseNip05 => "use_nip05",
            Self::LaunchAtLogin => "launch_at_login",
        }
    }
}

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = uuid::Error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}

typed_id!(FarmId);

typed_id!(PickupLocationId);

typed_id!(BlackoutPeriodId);

typed_id!(ProductId);

typed_id!(OrderId);

typed_id!(FulfillmentWindowId);

typed_id!(PackDayExportInstanceId);

typed_id!(ActivityEventId);

typed_id!(ReminderId);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountCustody {
    LocalManaged,
    BrowserSigner,
    RemoteSigner,
}

impl AccountCustody {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::LocalManaged => "local_managed",
            Self::BrowserSigner => "browser_signer",
            Self::RemoteSigner => "remote_signer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmReadiness {
    Incomplete,
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmProfileRecord {
    pub farm_id: FarmId,
    pub display_name: String,
    pub timezone: String,
    pub currency_code: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmOperatingRulesRecord {
    pub farm_id: FarmId,
    pub promise_lead_hours: u16,
    pub substitution_policy: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PickupLocationRecord {
    pub pickup_location_id: PickupLocationId,
    pub farm_id: FarmId,
    pub label: String,
    pub address_line: String,
    pub directions: Option<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FulfillmentWindowRecord {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub farm_id: FarmId,
    pub pickup_location_id: PickupLocationId,
    pub label: String,
    pub starts_at: String,
    pub ends_at: String,
    pub order_cutoff_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlackoutPeriodRecord {
    pub blackout_period_id: BlackoutPeriodId,
    pub farm_id: FarmId,
    pub label: String,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmReadinessBlocker {
    MissingProfileBasics,
    MissingPickupLocation,
    MissingFulfillmentWindow,
    MissingOperatingRules,
}

impl FarmReadinessBlocker {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::MissingProfileBasics => "missing_profile_basics",
            Self::MissingPickupLocation => "missing_pickup_location",
            Self::MissingFulfillmentWindow => "missing_fulfillment_window",
            Self::MissingOperatingRules => "missing_operating_rules",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmTimingConflictKind {
    FulfillmentWindowEndsBeforeStart,
    FulfillmentWindowCutoffAfterStart,
    BlackoutPeriodEndsBeforeStart,
    BlackoutOverlapsFulfillmentWindow,
}

impl FarmTimingConflictKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::FulfillmentWindowEndsBeforeStart => "fulfillment_window_ends_before_start",
            Self::FulfillmentWindowCutoffAfterStart => "fulfillment_window_cutoff_after_start",
            Self::BlackoutPeriodEndsBeforeStart => "blackout_period_ends_before_start",
            Self::BlackoutOverlapsFulfillmentWindow => "blackout_overlaps_fulfillment_window",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmTimingConflict {
    pub kind: FarmTimingConflictKind,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub blackout_period_id: Option<BlackoutPeriodId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmRulesReadiness {
    pub blockers: Vec<FarmReadinessBlocker>,
    pub timing_conflicts: Vec<FarmTimingConflict>,
}

impl FarmRulesReadiness {
    pub fn ready() -> Self {
        Self {
            blockers: Vec::new(),
            timing_conflicts: Vec::new(),
        }
    }

    pub fn missing_v1_basics() -> Self {
        Self {
            blockers: vec![
                FarmReadinessBlocker::MissingProfileBasics,
                FarmReadinessBlocker::MissingPickupLocation,
                FarmReadinessBlocker::MissingFulfillmentWindow,
                FarmReadinessBlocker::MissingOperatingRules,
            ],
            timing_conflicts: Vec::new(),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.blockers.is_empty() && self.timing_conflicts.is_empty()
    }
}

impl Default for FarmRulesReadiness {
    fn default() -> Self {
        Self::ready()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    #[default]
    Draft,
    Published,
    Paused,
    Archived,
}

impl ProductStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }

    pub const fn is_live(self) -> bool {
        matches!(self, Self::Published)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductPublishBlocker {
    AddProductName,
    ChooseCategory,
    ChooseUnit,
    SetPrice,
    SetStock,
    AttachAvailability,
    CompleteFarmProfile,
    AddPickupLocation,
    AddOperatingRules,
    AddFulfillmentWindow,
    ResolveAvailabilityConflicts,
}

impl ProductPublishBlocker {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::AddProductName => "add_product_name",
            Self::ChooseCategory => "choose_category",
            Self::ChooseUnit => "choose_unit",
            Self::SetPrice => "set_price",
            Self::SetStock => "set_stock",
            Self::AttachAvailability => "attach_availability",
            Self::CompleteFarmProfile => "complete_farm_profile",
            Self::AddPickupLocation => "add_pickup_location",
            Self::AddOperatingRules => "add_operating_rules",
            Self::AddFulfillmentWindow => "add_fulfillment_window",
            Self::ResolveAvailabilityConflicts => "resolve_availability_conflicts",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    NeedsAction,
    Scheduled,
    Packed,
    Completed,
    Declined,
    NeedsReview,
}

impl OrderStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::NeedsAction => "needs_action",
            Self::Scheduled => "scheduled",
            Self::Packed => "packed",
            Self::Completed => "completed",
            Self::Declined => "declined",
            Self::NeedsReview => "needs_review",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuyerOrderStatus {
    Placed,
    Scheduled,
    Ready,
    Completed,
    Declined,
    NeedsReview,
}

impl BuyerOrderStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Placed => "placed",
            Self::Scheduled => "scheduled",
            Self::Ready => "ready",
            Self::Completed => "completed",
            Self::Declined => "declined",
            Self::NeedsReview => "needs_review",
        }
    }
}

impl From<OrderStatus> for BuyerOrderStatus {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::NeedsAction => Self::Placed,
            OrderStatus::Scheduled => Self::Scheduled,
            OrderStatus::Packed => Self::Ready,
            OrderStatus::Completed => Self::Completed,
            OrderStatus::Declined => Self::Declined,
            OrderStatus::NeedsReview => Self::NeedsReview,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayExportArtifactKind {
    PackSheet,
    PickupRoster,
    CustomerLabels,
}

impl PackDayExportArtifactKind {
    pub const fn all_v1() -> [Self; 3] {
        [Self::PackSheet, Self::PickupRoster, Self::CustomerLabels]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::PackSheet => "pack_sheet",
            Self::PickupRoster => "pickup_roster",
            Self::CustomerLabels => "customer_labels",
        }
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::PackSheet => "pack_sheet.txt",
            Self::PickupRoster => "pickup_roster.txt",
            Self::CustomerLabels => "customer_labels.txt",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayPrintKind {
    PrintPackSheet,
    PrintPickupRoster,
    PrintCustomerLabels,
}

impl PackDayPrintKind {
    pub const fn all_v1() -> [Self; 3] {
        [
            Self::PrintPackSheet,
            Self::PrintPickupRoster,
            Self::PrintCustomerLabels,
        ]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::PrintPackSheet => "print_pack_sheet",
            Self::PrintPickupRoster => "print_pickup_roster",
            Self::PrintCustomerLabels => "print_customer_labels",
        }
    }

    pub const fn artifact_kind(self) -> PackDayExportArtifactKind {
        match self {
            Self::PrintPackSheet => PackDayExportArtifactKind::PackSheet,
            Self::PrintPickupRoster => PackDayExportArtifactKind::PickupRoster,
            Self::PrintCustomerLabels => PackDayExportArtifactKind::CustomerLabels,
        }
    }

    pub const fn label_stock(self) -> Option<PackDayPrintLabelStock> {
        match self {
            Self::PrintPackSheet | Self::PrintPickupRoster => None,
            Self::PrintCustomerLabels => Some(PackDayPrintLabelStock::Avery5160Letter30Up),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayPrintLabelStock {
    Avery5160Letter30Up,
}

impl PackDayPrintLabelStock {
    pub const fn all_v1() -> [Self; 1] {
        [Self::Avery5160Letter30Up]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Avery5160Letter30Up => "avery_5160_letter_30_up",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayPrintFailureKind {
    CustomerLabelsAvery5160Overflow,
}

impl PackDayPrintFailureKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::CustomerLabelsAvery5160Overflow => "customer_labels_avery_5160_overflow",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayBatchPrintArtifact {
    pub print_kind: PackDayPrintKind,
    pub artifact_kind: PackDayExportArtifactKind,
    pub label_stock: Option<PackDayPrintLabelStock>,
}

impl PackDayBatchPrintArtifact {
    pub const fn all_v1() -> [Self; 3] {
        [
            Self::from_print_kind(PackDayPrintKind::PrintPackSheet),
            Self::from_print_kind(PackDayPrintKind::PrintPickupRoster),
            Self::from_print_kind(PackDayPrintKind::PrintCustomerLabels),
        ]
    }

    pub const fn from_print_kind(print_kind: PackDayPrintKind) -> Self {
        Self {
            print_kind,
            artifact_kind: print_kind.artifact_kind(),
            label_stock: print_kind.label_stock(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayBatchPrintFailureKind {
    Preflight,
    QueueLaunch,
    QueueExit,
    CustomerLabelsAvery5160Overflow,
}

impl PackDayBatchPrintFailureKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::QueueLaunch => "queue_launch",
            Self::QueueExit => "queue_exit",
            Self::CustomerLabelsAvery5160Overflow => "customer_labels_avery_5160_overflow",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayBatchPrintStatus {
    #[default]
    Idle,
    Running,
    Succeeded,
    Failed,
}

impl PackDayBatchPrintStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayPrintStatus {
    #[default]
    Idle,
    Running,
    Succeeded,
    Failed,
}

impl PackDayPrintStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayHostHandoffKind {
    RevealBundle,
    OpenPackSheet,
    OpenPickupRoster,
    OpenCustomerLabels,
}

impl PackDayHostHandoffKind {
    pub const fn all_v1() -> [Self; 4] {
        [
            Self::RevealBundle,
            Self::OpenPackSheet,
            Self::OpenPickupRoster,
            Self::OpenCustomerLabels,
        ]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::RevealBundle => "reveal_bundle",
            Self::OpenPackSheet => "open_pack_sheet",
            Self::OpenPickupRoster => "open_pickup_roster",
            Self::OpenCustomerLabels => "open_customer_labels",
        }
    }

    pub const fn artifact_kind(self) -> Option<PackDayExportArtifactKind> {
        match self {
            Self::RevealBundle => None,
            Self::OpenPackSheet => Some(PackDayExportArtifactKind::PackSheet),
            Self::OpenPickupRoster => Some(PackDayExportArtifactKind::PickupRoster),
            Self::OpenCustomerLabels => Some(PackDayExportArtifactKind::CustomerLabels),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayExportStatus {
    #[default]
    Idle,
    Running,
    Succeeded,
    Failed,
}

impl PackDayExportStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayHostHandoffStatus {
    #[default]
    Idle,
    Running,
    Succeeded,
    Failed,
}

impl PackDayHostHandoffStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDayOutputOrderState {
    NeedsAction,
    Scheduled,
    Packed,
}

impl PackDayOutputOrderState {
    pub const fn all_v1() -> [Self; 3] {
        [Self::NeedsAction, Self::Scheduled, Self::Packed]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::NeedsAction => "needs_action",
            Self::Scheduled => "scheduled",
            Self::Packed => "packed",
        }
    }

    pub const fn from_order_status(status: OrderStatus) -> Option<Self> {
        match status {
            OrderStatus::NeedsAction => Some(Self::NeedsAction),
            OrderStatus::Scheduled => Some(Self::Scheduled),
            OrderStatus::Packed => Some(Self::Packed),
            OrderStatus::Completed | OrderStatus::Declined | OrderStatus::NeedsReview => None,
        }
    }
}

impl From<PackDayOutputOrderState> for OrderStatus {
    fn from(value: PackDayOutputOrderState) -> Self {
        match value {
            PackDayOutputOrderState::NeedsAction => Self::NeedsAction,
            PackDayOutputOrderState::Scheduled => Self::Scheduled,
            PackDayOutputOrderState::Packed => Self::Packed,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputQuantity {
    pub value: u32,
    pub unit_label: String,
}

impl PackDayOutputQuantity {
    pub fn new(value: u32, unit_label: impl Into<String>) -> Self {
        Self {
            value,
            unit_label: unit_label.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputWindow {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub farm_id: FarmId,
    pub farm_display_name: String,
    pub pickup_location_label: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputProductTotal {
    pub title: String,
    pub quantity: PackDayOutputQuantity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputPackListEntry {
    pub order_id: OrderId,
    pub order_number: String,
    pub customer_display_name: String,
    pub order_state: PackDayOutputOrderState,
    pub title: String,
    pub quantity: PackDayOutputQuantity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputCustomerOrder {
    pub order_id: OrderId,
    pub order_number: String,
    pub customer_display_name: String,
    pub order_state: PackDayOutputOrderState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayOutputSource {
    pub fulfillment_window: PackDayOutputWindow,
    pub totals_by_product: Vec<PackDayOutputProductTotal>,
    pub pack_list: Vec<PackDayOutputPackListEntry>,
    pub pickup_roster: Vec<PackDayOutputCustomerOrder>,
}

impl PackDayOutputSource {
    pub fn is_empty(&self) -> bool {
        self.totals_by_product.is_empty()
            && self.pack_list.is_empty()
            && self.pickup_roster.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayExportArtifact {
    pub kind: PackDayExportArtifactKind,
    pub relative_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayExportBundle {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub export_instance_id: PackDayExportInstanceId,
    pub generated_at_utc: String,
    pub bundle_directory: String,
    pub artifacts: Vec<PackDayExportArtifact>,
}

impl PackDayExportBundle {
    pub fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    pub fn includes_artifact(&self, kind: PackDayExportArtifactKind) -> bool {
        self.artifacts.iter().any(|artifact| artifact.kind == kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmOrderMethod {
    Pickup,
    Delivery,
    Shipping,
}

impl FarmOrderMethod {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Pickup => "pickup",
            Self::Delivery => "delivery",
            Self::Shipping => "shipping",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderSurface {
    Today,
    Orders,
    PackDay,
}

impl ReminderSurface {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Orders => "orders",
            Self::PackDay => "pack_day",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderKind {
    FulfillmentWindow,
    OrderAction,
    SyncImpact,
}

impl ReminderKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::FulfillmentWindow => "fulfillment_window",
            Self::OrderAction => "order_action",
            Self::SyncImpact => "sync_impact",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderUrgency {
    Upcoming,
    DueSoon,
    Overdue,
    Blocking,
}

impl ReminderUrgency {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Upcoming => "upcoming",
            Self::DueSoon => "due_soon",
            Self::Overdue => "overdue",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderDeliveryState {
    Scheduled,
    Presented,
    Acknowledged,
    Resolved,
}

impl ReminderDeliveryState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Presented => "presented",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatDemandEligibility {
    Eligible,
    Partial,
    Unavailable,
}

impl RepeatDemandEligibility {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppActivityKind {
    HomeOpened,
    SettingsOpened {
        section: SettingsSection,
    },
    SettingsSectionSelected {
        section: SettingsSection,
    },
    SettingsPreferenceUpdated {
        preference: SettingsPreference,
        enabled: bool,
    },
}

impl AppActivityKind {
    pub const fn storage_key(&self) -> &'static str {
        match self {
            Self::HomeOpened => "home_opened",
            Self::SettingsOpened { .. } => "settings_opened",
            Self::SettingsSectionSelected { .. } => "settings_section_selected",
            Self::SettingsPreferenceUpdated { .. } => "settings_preference_updated",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppActivityEvent {
    pub activity_event_id: ActivityEventId,
    pub recorded_at: String,
    pub kind: AppActivityKind,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppActivityContext {
    pub recent_events: Vec<AppActivityEvent>,
}

impl AppActivityContext {
    pub fn from_recent_events(recent_events: Vec<AppActivityEvent>) -> Self {
        Self { recent_events }
    }
}
