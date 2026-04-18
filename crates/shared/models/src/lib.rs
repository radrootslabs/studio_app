#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveSurface {
    #[default]
    Farmer,
    Personal,
}

impl ActiveSurface {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Farmer => "farmer",
            Self::Personal => "personal",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmerSection {
    #[default]
    Today,
    Products,
    Orders,
    PackDay,
    Farm,
}

impl FarmerSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Today => "farmer.today",
            Self::Products => "farmer.products",
            Self::Orders => "farmer.orders",
            Self::PackDay => "farmer.pack_day",
            Self::Farm => "farmer.farm",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection {
    #[default]
    Account,
    Settings,
    About,
}

impl SettingsSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Account => "settings.account",
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "surface", content = "section", rename_all = "snake_case")]
pub enum ShellSection {
    #[default]
    Home,
    Farmer(FarmerSection),
    Settings(SettingsSection),
}

impl ShellSection {
    pub const fn surface(self) -> Option<ActiveSurface> {
        match self {
            Self::Home | Self::Settings(_) => None,
            Self::Farmer(_) => Some(ActiveSurface::Farmer),
        }
    }

    pub const fn default_for_surface(surface: ActiveSurface) -> Self {
        match surface {
            ActiveSurface::Personal => Self::Home,
            ActiveSurface::Farmer => Self::Farmer(FarmerSection::Today),
        }
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Farmer(section) => section.storage_key(),
            Self::Settings(section) => section.storage_key(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseShellSectionError;

impl fmt::Display for ParseShellSectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid shell section key")
    }
}

impl Error for ParseShellSectionError {}

impl FromStr for ShellSection {
    type Err = ParseShellSectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "home" => Ok(Self::Home),
            "farmer.today" => Ok(Self::Farmer(FarmerSection::Today)),
            "farmer.products" => Ok(Self::Farmer(FarmerSection::Products)),
            "farmer.orders" => Ok(Self::Farmer(FarmerSection::Orders)),
            "farmer.pack_day" => Ok(Self::Farmer(FarmerSection::PackDay)),
            "farmer.farm" => Ok(Self::Farmer(FarmerSection::Farm)),
            "settings.account" => Ok(Self::Settings(SettingsSection::Account)),
            "settings.settings" => Ok(Self::Settings(SettingsSection::Settings)),
            "settings.about" => Ok(Self::Settings(SettingsSection::About)),
            _ => Err(ParseShellSectionError),
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
            pub fn new() -> Self {
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
typed_id!(ProductId);
typed_id!(OrderId);
typed_id!(FulfillmentWindowId);
typed_id!(ActivityEventId);

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
pub enum IdentityBlockedReason {
    RuntimeUnavailable,
    HostVaultUnavailable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum IdentityReadiness {
    #[default]
    MissingAccount,
    Ready,
    Blocked(IdentityBlockedReason),
}

impl IdentityReadiness {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::MissingAccount => "missing_account",
            Self::Ready => "ready",
            Self::Blocked(IdentityBlockedReason::RuntimeUnavailable) => "runtime_unavailable",
            Self::Blocked(IdentityBlockedReason::HostVaultUnavailable) => "host_vault_unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SelectedSurfaceProjection {
    pub active_surface: ActiveSurface,
}

impl Default for SelectedSurfaceProjection {
    fn default() -> Self {
        Self::new(ActiveSurface::Personal)
    }
}

impl SelectedSurfaceProjection {
    pub const fn new(active_surface: ActiveSurface) -> Self {
        Self { active_surface }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmerActivationProjection {
    pub farm_id: Option<FarmId>,
}

impl FarmerActivationProjection {
    pub const fn inactive() -> Self {
        Self { farm_id: None }
    }

    pub fn active(farm_id: FarmId) -> Self {
        Self {
            farm_id: Some(farm_id),
        }
    }

    pub const fn is_active(&self) -> bool {
        self.farm_id.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountSummary {
    pub account_id: String,
    pub npub: String,
    pub label: Option<String>,
    pub custody: AccountCustody,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountSurfaceActivationProjection {
    pub account_id: String,
    pub selected_surface: SelectedSurfaceProjection,
    pub farmer_activation: FarmerActivationProjection,
}

impl AccountSurfaceActivationProjection {
    pub fn new(
        account_id: impl Into<String>,
        selected_surface: SelectedSurfaceProjection,
        farmer_activation: FarmerActivationProjection,
    ) -> Self {
        let active_surface = if farmer_activation.is_active() {
            selected_surface.active_surface
        } else {
            ActiveSurface::Personal
        };

        Self {
            account_id: account_id.into(),
            selected_surface: SelectedSurfaceProjection::new(active_surface),
            farmer_activation,
        }
    }

    pub const fn active_surface(&self) -> ActiveSurface {
        self.selected_surface.active_surface
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SelectedAccountProjection {
    pub account: AccountSummary,
    pub selected_surface: SelectedSurfaceProjection,
    pub farmer_activation: FarmerActivationProjection,
}

impl SelectedAccountProjection {
    pub fn new(
        account: AccountSummary,
        selected_surface: SelectedSurfaceProjection,
        farmer_activation: FarmerActivationProjection,
    ) -> Self {
        let active_surface = if farmer_activation.is_active() {
            selected_surface.active_surface
        } else {
            ActiveSurface::Personal
        };

        Self {
            account,
            selected_surface: SelectedSurfaceProjection::new(active_surface),
            farmer_activation,
        }
    }

    pub fn from_surface_activation(
        account: AccountSummary,
        activation: AccountSurfaceActivationProjection,
    ) -> Self {
        Self::new(
            account,
            activation.selected_surface,
            activation.farmer_activation,
        )
    }

    pub const fn active_surface(&self) -> ActiveSurface {
        self.selected_surface.active_surface
    }
}

impl From<&SelectedAccountProjection> for AccountSurfaceActivationProjection {
    fn from(value: &SelectedAccountProjection) -> Self {
        Self::new(
            value.account.account_id.clone(),
            value.selected_surface,
            value.farmer_activation.clone(),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStartupGate {
    Blocked,
    #[default]
    SetupRequired,
    Personal,
    Farmer,
}

impl AppStartupGate {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::SetupRequired => "setup_required",
            Self::Personal => "personal",
            Self::Farmer => "farmer",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppIdentityProjection {
    pub readiness: IdentityReadiness,
    pub roster: Vec<AccountSummary>,
    pub selected_account: Option<SelectedAccountProjection>,
}

impl AppIdentityProjection {
    pub fn missing() -> Self {
        Self::with_readiness(IdentityReadiness::MissingAccount, Vec::new(), None)
    }

    pub fn missing_with_roster(roster: Vec<AccountSummary>) -> Self {
        Self::with_readiness(IdentityReadiness::MissingAccount, roster, None)
    }

    pub fn blocked(reason: IdentityBlockedReason) -> Self {
        Self::with_readiness(IdentityReadiness::Blocked(reason), Vec::new(), None)
    }

    pub fn blocked_with_selection(
        reason: IdentityBlockedReason,
        roster: Vec<AccountSummary>,
        selected_account: Option<SelectedAccountProjection>,
    ) -> Self {
        Self::with_readiness(IdentityReadiness::Blocked(reason), roster, selected_account)
    }

    pub fn ready(roster: Vec<AccountSummary>, selected_account: SelectedAccountProjection) -> Self {
        Self::with_readiness(IdentityReadiness::Ready, roster, Some(selected_account))
    }

    pub fn with_readiness(
        readiness: IdentityReadiness,
        mut roster: Vec<AccountSummary>,
        selected_account: Option<SelectedAccountProjection>,
    ) -> Self {
        if let Some(selected_account) = selected_account.as_ref()
            && !roster
                .iter()
                .any(|account| account.account_id == selected_account.account.account_id)
        {
            roster.insert(0, selected_account.account.clone());
        }

        Self {
            readiness,
            roster,
            selected_account,
        }
    }

    pub fn startup_gate(&self) -> AppStartupGate {
        match self.readiness {
            IdentityReadiness::MissingAccount => AppStartupGate::SetupRequired,
            IdentityReadiness::Blocked(_) => AppStartupGate::Blocked,
            IdentityReadiness::Ready => self
                .selected_account
                .as_ref()
                .map(|account| {
                    if account.farmer_activation.is_active()
                        && account.active_surface() == ActiveSurface::Farmer
                    {
                        AppStartupGate::Farmer
                    } else {
                        AppStartupGate::Personal
                    }
                })
                .unwrap_or(AppStartupGate::SetupRequired),
        }
    }

    pub fn settings_account(&self) -> SettingsAccountProjection {
        self.into()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettingsAccountProjection {
    pub readiness: IdentityReadiness,
    pub roster: Vec<AccountSummary>,
    pub selected_account: Option<SelectedAccountProjection>,
}

impl From<&AppIdentityProjection> for SettingsAccountProjection {
    fn from(value: &AppIdentityProjection) -> Self {
        Self {
            readiness: value.readiness,
            roster: value.roster.clone(),
            selected_account: value.selected_account.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmReadiness {
    Incomplete,
    Ready,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductsFilter {
    #[default]
    All,
    Live,
    Drafts,
    NeedAttention,
    Paused,
    Archived,
}

impl ProductsFilter {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Live => "live",
            Self::Drafts => "drafts",
            Self::NeedAttention => "need_attention",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductsSort {
    #[default]
    Updated,
    Name,
    Availability,
    Stock,
    Price,
}

impl ProductsSort {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Name => "name",
            Self::Availability => "availability",
            Self::Stock => "stock",
            Self::Price => "price",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductAttentionState {
    #[default]
    Healthy,
    LowStock,
    SoldOut,
    MissingAvailability,
    NoFutureAvailability,
    MissingDetails,
}

impl ProductAttentionState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::LowStock => "low_stock",
            Self::SoldOut => "sold_out",
            Self::MissingAvailability => "missing_availability",
            Self::NoFutureAvailability => "no_future_availability",
            Self::MissingDetails => "missing_details",
        }
    }

    pub const fn requires_attention(self) -> bool {
        !matches!(self, Self::Healthy)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductAvailabilityState {
    Scheduled,
    Open,
    MissingWindow,
    NoFutureWindow,
}

impl ProductAvailabilityState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Open => "open",
            Self::MissingWindow => "missing_window",
            Self::NoFutureWindow => "no_future_window",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductAvailabilitySummary {
    pub state: ProductAvailabilityState,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStockState {
    Unset,
    InStock,
    LowStock,
    SoldOut,
}

impl ProductStockState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::InStock => "in_stock",
            Self::LowStock => "low_stock",
            Self::SoldOut => "sold_out",
        }
    }

    pub const fn requires_attention(self) -> bool {
        matches!(self, Self::LowStock | Self::SoldOut)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductStockSummary {
    pub quantity: Option<u32>,
    pub unit_label: Option<String>,
    pub state: ProductStockState,
}

impl ProductStockSummary {
    pub const fn requires_attention(&self) -> bool {
        self.state.requires_attention()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductPricePresentation {
    pub amount_minor_units: u32,
    pub currency_code: String,
    pub unit_label: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductsListSummary {
    pub total_products: u32,
    pub live_products: u32,
    pub draft_products: u32,
    pub need_attention_products: u32,
}

impl ProductsListSummary {
    pub const fn has_products(&self) -> bool {
        self.total_products > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductsListRow {
    pub product_id: ProductId,
    pub farm_id: FarmId,
    pub title: String,
    pub subtitle: Option<String>,
    pub status: ProductStatus,
    pub attention_state: ProductAttentionState,
    pub availability: ProductAvailabilitySummary,
    pub stock: ProductStockSummary,
    pub price: Option<ProductPricePresentation>,
    pub updated_at: String,
}

impl ProductsListRow {
    pub const fn requires_attention(&self) -> bool {
        self.attention_state.requires_attention() || self.stock.requires_attention()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductsListProjection {
    pub summary: ProductsListSummary,
    pub rows: Vec<ProductsListRow>,
}

impl ProductsListProjection {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductPublishBlocker {
    AddProductName,
    ChooseUnit,
    SetPrice,
    AttachAvailability,
}

impl ProductPublishBlocker {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::AddProductName => "add_product_name",
            Self::ChooseUnit => "choose_unit",
            Self::SetPrice => "set_price",
            Self::AttachAvailability => "attach_availability",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductEditorDraft {
    pub title: String,
    pub subtitle: String,
    pub unit_label: String,
    pub price_minor_units: Option<u32>,
    pub price_currency: String,
    pub stock_quantity: Option<u32>,
    pub availability_window_id: Option<FulfillmentWindowId>,
    pub status: ProductStatus,
}

impl Default for ProductEditorDraft {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: String::new(),
            unit_label: String::new(),
            price_minor_units: None,
            price_currency: "USD".to_owned(),
            stock_quantity: None,
            availability_window_id: None,
            status: ProductStatus::Draft,
        }
    }
}

impl ProductEditorDraft {
    pub fn publish_blockers(&self) -> Vec<ProductPublishBlocker> {
        let mut blockers = Vec::new();

        if self.title.trim().is_empty() {
            blockers.push(ProductPublishBlocker::AddProductName);
        }

        if self.unit_label.trim().is_empty() {
            blockers.push(ProductPublishBlocker::ChooseUnit);
        }

        if self.price_minor_units.is_none_or(|value| value == 0) {
            blockers.push(ProductPublishBlocker::SetPrice);
        }

        if self.availability_window_id.is_none() {
            blockers.push(ProductPublishBlocker::AttachAvailability);
        }

        blockers
    }

    pub fn is_publish_ready(&self) -> bool {
        self.publish_blockers().is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    NeedsAction,
    Scheduled,
    Packed,
    Completed,
    Refunded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmSummary {
    pub farm_id: FarmId,
    pub display_name: String,
    pub readiness: FarmReadiness,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmSetupReadiness {
    #[default]
    NotStarted,
    InProgress,
    Ready,
}

impl FarmSetupReadiness {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Ready => "ready",
        }
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
pub enum FarmSetupSection {
    Farm,
    Location,
    OrderMethods,
}

impl FarmSetupSection {
    pub const fn ordered() -> [Self; 3] {
        [Self::Farm, Self::Location, Self::OrderMethods]
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Farm => "farm",
            Self::Location => "location",
            Self::OrderMethods => "order_methods",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmSetupBlocker {
    AddFarmName,
    AddLocationOrServiceArea,
    ChooseOrderMethod,
}

impl FarmSetupBlocker {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::AddFarmName => "add_farm_name",
            Self::AddLocationOrServiceArea => "add_location_or_service_area",
            Self::ChooseOrderMethod => "choose_order_method",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmSetupDraft {
    pub farm_name: String,
    pub location_or_service_area: String,
    pub order_methods: BTreeSet<FarmOrderMethod>,
}

impl FarmSetupDraft {
    pub fn new(
        farm_name: impl Into<String>,
        location_or_service_area: impl Into<String>,
        order_methods: impl IntoIterator<Item = FarmOrderMethod>,
    ) -> Self {
        Self {
            farm_name: farm_name.into(),
            location_or_service_area: location_or_service_area.into(),
            order_methods: order_methods.into_iter().collect(),
        }
    }

    pub fn blockers(&self) -> Vec<FarmSetupBlocker> {
        let mut blockers = Vec::new();

        if self.farm_name.trim().is_empty() {
            blockers.push(FarmSetupBlocker::AddFarmName);
        }

        if self.location_or_service_area.trim().is_empty() {
            blockers.push(FarmSetupBlocker::AddLocationOrServiceArea);
        }

        if self.order_methods.is_empty() {
            blockers.push(FarmSetupBlocker::ChooseOrderMethod);
        }

        blockers
    }

    pub fn readiness(&self) -> FarmSetupReadiness {
        let blockers = self.blockers();
        if blockers.is_empty() {
            FarmSetupReadiness::Ready
        } else if self.is_empty() {
            FarmSetupReadiness::NotStarted
        } else {
            FarmSetupReadiness::InProgress
        }
    }

    pub fn is_empty(&self) -> bool {
        self.farm_name.trim().is_empty()
            && self.location_or_service_area.trim().is_empty()
            && self.order_methods.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmSetupProjection {
    pub draft: FarmSetupDraft,
    pub saved_farm: Option<FarmSummary>,
    pub readiness: FarmSetupReadiness,
    pub blockers: Vec<FarmSetupBlocker>,
}

impl Default for FarmSetupProjection {
    fn default() -> Self {
        Self::not_started()
    }
}

impl FarmSetupProjection {
    pub fn new(draft: FarmSetupDraft, saved_farm: Option<FarmSummary>) -> Self {
        match saved_farm {
            Some(saved_farm) => Self {
                draft,
                saved_farm: Some(saved_farm),
                readiness: FarmSetupReadiness::Ready,
                blockers: Vec::new(),
            },
            None => Self::from_draft(draft),
        }
    }

    pub fn not_started() -> Self {
        Self::from_draft(FarmSetupDraft::default())
    }

    pub fn from_draft(draft: FarmSetupDraft) -> Self {
        let readiness = draft.readiness();
        let blockers = draft.blockers();

        Self {
            draft,
            saved_farm: None,
            readiness,
            blockers,
        }
    }

    pub fn from_saved_farm(saved_farm: FarmSummary) -> Self {
        Self {
            draft: FarmSetupDraft::default(),
            saved_farm: Some(saved_farm),
            readiness: FarmSetupReadiness::Ready,
            blockers: Vec::new(),
        }
    }

    pub const fn has_saved_farm(&self) -> bool {
        self.saved_farm.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FulfillmentWindowSummary {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub farm_id: FarmId,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodaySummary {
    pub farm_id: FarmId,
    pub orders_needing_action: u32,
    pub low_stock_products: u32,
    pub draft_products: u32,
}

impl TodaySummary {
    pub const fn has_attention_items(&self) -> bool {
        self.orders_needing_action > 0 || self.low_stock_products > 0 || self.draft_products > 0
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductListRow {
    pub product_id: ProductId,
    pub farm_id: FarmId,
    pub title: String,
    pub status: ProductStatus,
    pub stock_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderListRow {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub order_number: String,
    pub customer_display_name: String,
    pub status: OrderStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodaySetupTaskKind {
    AddFulfillmentWindow,
    PublishProduct,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodaySetupTask {
    pub kind: TodaySetupTaskKind,
    pub is_complete: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TodayAgendaProjection {
    pub farm: Option<FarmSummary>,
    pub summary: Option<TodaySummary>,
    pub orders_needing_action: Vec<OrderListRow>,
    pub low_stock_products: Vec<ProductListRow>,
    pub draft_products: Vec<ProductListRow>,
    pub next_fulfillment_window: Option<FulfillmentWindowSummary>,
    pub setup_checklist: Vec<TodaySetupTask>,
}

impl TodayAgendaProjection {
    pub fn has_attention_items(&self) -> bool {
        self.summary
            .as_ref()
            .is_some_and(TodaySummary::has_attention_items)
            || !self.orders_needing_action.is_empty()
            || !self.low_stock_products.is_empty()
            || !self.draft_products.is_empty()
    }

    pub fn needs_setup(&self) -> bool {
        self.setup_checklist.iter().any(|item| !item.is_complete)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AccountCustody, AccountSummary, AccountSurfaceActivationProjection, ActiveSurface,
        ActivityEventId, AppActivityContext, AppActivityEvent, AppActivityKind,
        AppIdentityProjection, AppStartupGate, FarmId, FarmOrderMethod, FarmSetupBlocker,
        FarmSetupDraft, FarmSetupProjection, FarmSetupReadiness, FarmSetupSection,
        FarmerActivationProjection, FarmerSection, IdentityBlockedReason, IdentityReadiness,
        OrderListRow, ProductAttentionState, ProductAvailabilityState, ProductAvailabilitySummary,
        ProductEditorDraft, ProductListRow, ProductPricePresentation, ProductPublishBlocker,
        ProductStatus, ProductStockState, ProductStockSummary, ProductsFilter,
        ProductsListProjection, ProductsListRow, ProductsListSummary, ProductsSort,
        SelectedAccountProjection, SelectedSurfaceProjection, SettingsPreference, SettingsSection,
        ShellSection, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind, TodaySummary,
    };
    use std::{collections::BTreeSet, str::FromStr};
    use uuid::Uuid;

    #[test]
    fn shell_section_storage_keys_are_unique_and_round_trip() {
        let sections = [
            ShellSection::Home,
            ShellSection::Farmer(FarmerSection::Today),
            ShellSection::Farmer(FarmerSection::Products),
            ShellSection::Farmer(FarmerSection::Orders),
            ShellSection::Farmer(FarmerSection::PackDay),
            ShellSection::Farmer(FarmerSection::Farm),
            ShellSection::Settings(SettingsSection::Account),
            ShellSection::Settings(SettingsSection::Settings),
            ShellSection::Settings(SettingsSection::About),
        ];
        let keys = sections
            .into_iter()
            .map(ShellSection::storage_key)
            .collect::<BTreeSet<_>>();

        assert_eq!(keys.len(), sections.len());

        for section in sections {
            let parsed =
                ShellSection::from_str(section.storage_key()).expect("section should parse");
            assert_eq!(parsed, section);
        }
    }

    #[test]
    fn shell_section_surface_is_explicit_only_for_farmer_routes() {
        assert_eq!(ShellSection::Home.surface(), None);
        assert_eq!(
            ShellSection::Farmer(FarmerSection::Today).surface(),
            Some(ActiveSurface::Farmer)
        );
        assert_eq!(
            ShellSection::Settings(SettingsSection::Settings).surface(),
            None
        );
    }

    #[test]
    fn shell_section_default_for_surface_preserves_current_farmer_entry() {
        assert_eq!(
            ShellSection::default_for_surface(ActiveSurface::Personal),
            ShellSection::Home
        );
        assert_eq!(
            ShellSection::default_for_surface(ActiveSurface::Farmer),
            ShellSection::Farmer(FarmerSection::Today)
        );
    }

    #[test]
    fn selected_surface_defaults_to_personal() {
        assert_eq!(
            SelectedSurfaceProjection::default().active_surface,
            ActiveSurface::Personal
        );
    }

    #[test]
    fn selected_account_without_farmer_activation_falls_back_to_personal_surface() {
        let projection = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_01".to_owned(),
                npub: "npub1example".to_owned(),
                label: Some("North field".to_owned()),
                custody: AccountCustody::LocalManaged,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::inactive(),
        );

        assert_eq!(projection.active_surface(), ActiveSurface::Personal);
        assert!(!projection.farmer_activation.is_active());
    }

    #[test]
    fn account_surface_activation_projection_normalizes_to_personal_without_farm_binding() {
        let projection = AccountSurfaceActivationProjection::new(
            "acct_04",
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::inactive(),
        );

        assert_eq!(projection.account_id, "acct_04");
        assert_eq!(projection.active_surface(), ActiveSurface::Personal);
        assert!(!projection.farmer_activation.is_active());
    }

    #[test]
    fn selected_account_projection_round_trips_through_surface_activation_state() {
        let selected_account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_roundtrip".to_owned(),
                npub: "npub1roundtrip".to_owned(),
                label: Some("Roundtrip".to_owned()),
                custody: AccountCustody::LocalManaged,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::active(FarmId::new()),
        );
        let activation = AccountSurfaceActivationProjection::from(&selected_account);
        let restored = SelectedAccountProjection::from_surface_activation(
            selected_account.account.clone(),
            activation,
        );

        assert_eq!(restored, selected_account);
    }

    #[test]
    fn startup_gate_tracks_setup_personal_farmer_and_blocked_states() {
        let farmer_identity = AppIdentityProjection::ready(
            Vec::new(),
            SelectedAccountProjection::new(
                AccountSummary {
                    account_id: "acct_02".to_owned(),
                    npub: "npub1farmer".to_owned(),
                    label: None,
                    custody: AccountCustody::LocalManaged,
                },
                SelectedSurfaceProjection::new(ActiveSurface::Farmer),
                FarmerActivationProjection::active(FarmId::new()),
            ),
        );
        let personal_identity = AppIdentityProjection::ready(
            Vec::new(),
            SelectedAccountProjection::new(
                AccountSummary {
                    account_id: "acct_03".to_owned(),
                    npub: "npub1personal".to_owned(),
                    label: None,
                    custody: AccountCustody::LocalManaged,
                },
                SelectedSurfaceProjection::new(ActiveSurface::Personal),
                FarmerActivationProjection::inactive(),
            ),
        );

        assert_eq!(
            AppIdentityProjection::missing().startup_gate(),
            AppStartupGate::SetupRequired
        );
        assert_eq!(personal_identity.startup_gate(), AppStartupGate::Personal);
        assert_eq!(farmer_identity.startup_gate(), AppStartupGate::Farmer);
        assert_eq!(
            AppIdentityProjection::blocked(IdentityBlockedReason::HostVaultUnavailable)
                .startup_gate(),
            AppStartupGate::Blocked
        );
    }

    #[test]
    fn ready_identity_keeps_selected_account_visible_in_roster() {
        let selected_account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_selected".to_owned(),
                npub: "npub1selected".to_owned(),
                label: None,
                custody: AccountCustody::RemoteSigner,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Personal),
            FarmerActivationProjection::inactive(),
        );
        let projection = AppIdentityProjection::ready(Vec::new(), selected_account.clone());

        assert_eq!(projection.readiness.storage_key(), "ready");
        assert_eq!(projection.roster.len(), 1);
        assert_eq!(projection.roster[0], selected_account.account);
        assert_eq!(projection.selected_account, Some(selected_account));
    }

    #[test]
    fn blocked_identity_keeps_selected_account_visible_in_roster() {
        let selected_account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_blocked".to_owned(),
                npub: "npub1blocked".to_owned(),
                label: Some("Blocked account".to_owned()),
                custody: AccountCustody::LocalManaged,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Personal),
            FarmerActivationProjection::inactive(),
        );
        let projection = AppIdentityProjection::blocked_with_selection(
            IdentityBlockedReason::HostVaultUnavailable,
            Vec::new(),
            Some(selected_account.clone()),
        );

        assert_eq!(
            projection.readiness,
            IdentityReadiness::Blocked(IdentityBlockedReason::HostVaultUnavailable)
        );
        assert_eq!(projection.roster, vec![selected_account.account.clone()]);
        assert_eq!(projection.selected_account, Some(selected_account));
        assert_eq!(projection.startup_gate(), AppStartupGate::Blocked);
    }

    #[test]
    fn missing_identity_can_keep_roster_visible_without_selected_account() {
        let roster = vec![AccountSummary {
            account_id: "acct_waiting".to_owned(),
            npub: "npub1waiting".to_owned(),
            label: Some("Waiting".to_owned()),
            custody: AccountCustody::LocalManaged,
        }];
        let projection = AppIdentityProjection::missing_with_roster(roster.clone());

        assert_eq!(projection.readiness, IdentityReadiness::MissingAccount);
        assert_eq!(projection.roster, roster);
        assert!(projection.selected_account.is_none());
        assert_eq!(projection.startup_gate(), AppStartupGate::SetupRequired);
    }

    #[test]
    fn typed_ids_round_trip_through_strings() {
        let uuid = Uuid::parse_str("018f4d61-19b0-7cc4-9d4e-6d0df7c0aa11")
            .expect("test uuid should parse");
        let farm_id = FarmId::from(uuid);
        let parsed = FarmId::from_str(&farm_id.to_string()).expect("farm id should parse");

        assert_eq!(parsed, farm_id);
        assert_eq!(parsed.as_uuid(), uuid);
    }

    #[test]
    fn product_status_filter_and_sort_storage_keys_are_stable() {
        assert_eq!(ProductStatus::Draft.storage_key(), "draft");
        assert_eq!(ProductStatus::Published.storage_key(), "published");
        assert_eq!(ProductStatus::Paused.storage_key(), "paused");
        assert_eq!(ProductStatus::Archived.storage_key(), "archived");
        assert!(ProductStatus::Published.is_live());
        assert!(!ProductStatus::Draft.is_live());

        assert_eq!(ProductsFilter::default(), ProductsFilter::All);
        assert_eq!(ProductsFilter::All.storage_key(), "all");
        assert_eq!(ProductsFilter::Live.storage_key(), "live");
        assert_eq!(ProductsFilter::Drafts.storage_key(), "drafts");
        assert_eq!(
            ProductsFilter::NeedAttention.storage_key(),
            "need_attention"
        );
        assert_eq!(ProductsFilter::Paused.storage_key(), "paused");
        assert_eq!(ProductsFilter::Archived.storage_key(), "archived");

        assert_eq!(ProductsSort::default(), ProductsSort::Updated);
        assert_eq!(ProductsSort::Updated.storage_key(), "updated");
        assert_eq!(ProductsSort::Name.storage_key(), "name");
        assert_eq!(ProductsSort::Availability.storage_key(), "availability");
        assert_eq!(ProductsSort::Stock.storage_key(), "stock");
        assert_eq!(ProductsSort::Price.storage_key(), "price");
    }

    #[test]
    fn product_attention_stock_and_projection_states_are_explicit() {
        let row = ProductsListRow {
            product_id: super::ProductId::new(),
            farm_id: FarmId::new(),
            title: "Pea shoots".to_owned(),
            subtitle: Some("Tray-grown".to_owned()),
            status: ProductStatus::Draft,
            attention_state: ProductAttentionState::MissingAvailability,
            availability: ProductAvailabilitySummary {
                state: ProductAvailabilityState::MissingWindow,
                label: "Missing window".to_owned(),
            },
            stock: ProductStockSummary {
                quantity: None,
                unit_label: None,
                state: ProductStockState::Unset,
            },
            price: Some(ProductPricePresentation {
                amount_minor_units: 300,
                currency_code: "USD".to_owned(),
                unit_label: "bag".to_owned(),
            }),
            updated_at: "2026-04-18T10:00:00Z".to_owned(),
        };
        let summary = ProductsListSummary {
            total_products: 1,
            live_products: 0,
            draft_products: 1,
            need_attention_products: 1,
        };
        let projection = ProductsListProjection {
            summary: summary.clone(),
            rows: vec![row.clone()],
        };

        assert_eq!(ProductAttentionState::LowStock.storage_key(), "low_stock");
        assert!(ProductAttentionState::LowStock.requires_attention());
        assert!(!ProductAttentionState::Healthy.requires_attention());
        assert_eq!(
            ProductAvailabilityState::MissingWindow.storage_key(),
            "missing_window"
        );
        assert_eq!(ProductStockState::SoldOut.storage_key(), "sold_out");
        assert!(ProductStockState::SoldOut.requires_attention());
        assert!(!ProductStockState::InStock.requires_attention());
        assert!(row.requires_attention());
        assert!(summary.has_products());
        assert!(!projection.is_empty());
        assert_eq!(projection.rows[0].availability.label, "Missing window");
    }

    #[test]
    fn product_editor_publish_blockers_are_explicit_and_minimal() {
        let empty_draft = ProductEditorDraft::default();
        let ready_draft = ProductEditorDraft {
            title: "Heirloom tomatoes".to_owned(),
            subtitle: "Brandywine".to_owned(),
            unit_label: "lb".to_owned(),
            price_minor_units: Some(450),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(12),
            availability_window_id: Some(super::FulfillmentWindowId::new()),
            status: ProductStatus::Draft,
        };

        assert_eq!(
            empty_draft.publish_blockers(),
            vec![
                ProductPublishBlocker::AddProductName,
                ProductPublishBlocker::ChooseUnit,
                ProductPublishBlocker::SetPrice,
                ProductPublishBlocker::AttachAvailability,
            ]
        );
        assert_eq!(
            ProductPublishBlocker::AttachAvailability.storage_key(),
            "attach_availability"
        );
        assert_eq!(empty_draft.price_currency, "USD");
        assert!(!empty_draft.is_publish_ready());
        assert!(ready_draft.is_publish_ready());
        assert!(ready_draft.publish_blockers().is_empty());
    }

    #[test]
    fn today_summary_attention_state_is_explicit() {
        let quiet = TodaySummary {
            farm_id: FarmId::new(),
            orders_needing_action: 0,
            low_stock_products: 0,
            draft_products: 0,
        };
        let busy = TodaySummary {
            farm_id: FarmId::new(),
            orders_needing_action: 1,
            low_stock_products: 0,
            draft_products: 0,
        };

        assert!(!quiet.has_attention_items());
        assert!(busy.has_attention_items());
    }

    #[test]
    fn today_agenda_projection_tracks_attention_and_setup_independently() {
        let calm = TodayAgendaProjection::default();
        let with_attention = TodayAgendaProjection {
            draft_products: vec![ProductListRow {
                product_id: super::ProductId::new(),
                farm_id: FarmId::new(),
                title: "Spring onions".to_owned(),
                status: super::ProductStatus::Draft,
                stock_count: 0,
            }],
            ..TodayAgendaProjection::default()
        };
        let with_setup = TodayAgendaProjection {
            setup_checklist: vec![TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: false,
            }],
            ..TodayAgendaProjection::default()
        };

        assert!(!calm.has_attention_items());
        assert!(!calm.needs_setup());
        assert!(with_attention.has_attention_items());
        assert!(!with_attention.needs_setup());
        assert!(!with_setup.has_attention_items());
        assert!(with_setup.needs_setup());
    }

    #[test]
    fn today_agenda_projection_can_hold_truthful_lists() {
        let projection = TodayAgendaProjection {
            orders_needing_action: vec![OrderListRow {
                order_id: super::OrderId::new(),
                farm_id: FarmId::new(),
                fulfillment_window_id: Some(super::FulfillmentWindowId::new()),
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
                status: super::OrderStatus::NeedsAction,
            }],
            low_stock_products: vec![ProductListRow {
                product_id: super::ProductId::new(),
                farm_id: FarmId::new(),
                title: "Carrots".to_owned(),
                status: super::ProductStatus::Published,
                stock_count: 2,
            }],
            ..TodayAgendaProjection::default()
        };

        assert_eq!(projection.orders_needing_action.len(), 1);
        assert_eq!(projection.low_stock_products[0].stock_count, 2);
        assert!(projection.has_attention_items());
    }

    #[test]
    fn farm_setup_section_order_is_frozen() {
        assert_eq!(
            FarmSetupSection::ordered(),
            [
                FarmSetupSection::Farm,
                FarmSetupSection::Location,
                FarmSetupSection::OrderMethods,
            ]
        );
    }

    #[test]
    fn empty_farm_setup_draft_is_not_started_with_all_blockers() {
        let draft = FarmSetupDraft::default();

        assert!(draft.is_empty());
        assert_eq!(draft.readiness(), FarmSetupReadiness::NotStarted);
        assert_eq!(
            draft.blockers(),
            vec![
                FarmSetupBlocker::AddFarmName,
                FarmSetupBlocker::AddLocationOrServiceArea,
                FarmSetupBlocker::ChooseOrderMethod,
            ]
        );
    }

    #[test]
    fn partial_farm_setup_draft_is_in_progress() {
        let draft = FarmSetupDraft::new("North field farm", "", [FarmOrderMethod::Pickup]);

        assert_eq!(draft.readiness(), FarmSetupReadiness::InProgress);
        assert_eq!(
            draft.blockers(),
            vec![FarmSetupBlocker::AddLocationOrServiceArea]
        );
    }

    #[test]
    fn complete_farm_setup_draft_is_ready_and_deduplicates_order_methods() {
        let draft = FarmSetupDraft::new(
            "North field farm",
            "Asheville, NC",
            [
                FarmOrderMethod::Shipping,
                FarmOrderMethod::Pickup,
                FarmOrderMethod::Shipping,
            ],
        );

        assert_eq!(draft.readiness(), FarmSetupReadiness::Ready);
        assert_eq!(draft.blockers(), Vec::<FarmSetupBlocker>::new());
        assert_eq!(
            draft.order_methods,
            BTreeSet::from([FarmOrderMethod::Pickup, FarmOrderMethod::Shipping])
        );
    }

    #[test]
    fn saved_farm_projection_is_always_ready() {
        let saved_farm = super::FarmSummary {
            farm_id: FarmId::new(),
            display_name: "North field farm".to_owned(),
            readiness: super::FarmReadiness::Ready,
        };
        let projection = FarmSetupProjection::from_saved_farm(saved_farm.clone());

        assert_eq!(projection.saved_farm, Some(saved_farm));
        assert_eq!(projection.readiness, FarmSetupReadiness::Ready);
        assert!(projection.blockers.is_empty());
        assert!(projection.has_saved_farm());
    }

    #[test]
    fn settings_preference_storage_keys_are_stable() {
        assert_eq!(
            SettingsPreference::AllowRelayConnections.storage_key(),
            "allow_relay_connections"
        );
        assert_eq!(
            SettingsPreference::UseMediaServers.storage_key(),
            "use_media_servers"
        );
        assert_eq!(SettingsPreference::UseNip05.storage_key(), "use_nip05");
        assert_eq!(
            SettingsPreference::LaunchAtLogin.storage_key(),
            "launch_at_login"
        );
    }

    #[test]
    fn activity_kind_storage_keys_are_stable() {
        assert_eq!(AppActivityKind::HomeOpened.storage_key(), "home_opened");
        assert_eq!(
            AppActivityKind::SettingsOpened {
                section: SettingsSection::About,
            }
            .storage_key(),
            "settings_opened"
        );
        assert_eq!(
            AppActivityKind::SettingsSectionSelected {
                section: SettingsSection::Settings,
            }
            .storage_key(),
            "settings_section_selected"
        );
        assert_eq!(
            AppActivityKind::SettingsPreferenceUpdated {
                preference: SettingsPreference::LaunchAtLogin,
                enabled: true,
            }
            .storage_key(),
            "settings_preference_updated"
        );
    }

    #[test]
    fn activity_context_preserves_recent_event_order() {
        let first = AppActivityEvent {
            activity_event_id: ActivityEventId::new(),
            recorded_at: "2026-04-18T00:00:00.000Z".to_owned(),
            kind: AppActivityKind::HomeOpened,
        };
        let second = AppActivityEvent {
            activity_event_id: ActivityEventId::new(),
            recorded_at: "2026-04-18T00:01:00.000Z".to_owned(),
            kind: AppActivityKind::SettingsOpened {
                section: SettingsSection::About,
            },
        };
        let context = AppActivityContext::from_recent_events(vec![second.clone(), first.clone()]);

        assert_eq!(context.recent_events, vec![second, first]);
    }
}
