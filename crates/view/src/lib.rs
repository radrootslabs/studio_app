#![forbid(unsafe_code)]

pub use radroots_studio_app_types::*;

use radroots_core::RadrootsCoreMoney;
use radroots_event::order::RadrootsOrderEconomics;
use radroots_trade::validation_receipt::{
    RadrootsValidationReceiptProofSystem, RadrootsValidationReceiptResult,
    RadrootsValidationReceiptType,
};
use radroots_trade::{order::RadrootsOrderProjection, workflow::RadrootsTradeWorkflowState};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt, str::FromStr};
use url::Url;

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
pub enum PersonalSection {
    #[default]
    Browse,
    Search,
    Cart,
    Orders,
}

impl PersonalSection {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Browse => "personal.browse",
            Self::Search => "personal.search",
            Self::Cart => "personal.cart",
            Self::Orders => "personal.orders",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "surface", content = "section", rename_all = "snake_case")]
pub enum ShellSection {
    #[default]
    Home,
    Account,
    Personal(PersonalSection),
    Farmer(FarmerSection),
    Settings(SettingsSection),
}

impl ShellSection {
    pub const fn surface(self) -> Option<ActiveSurface> {
        match self {
            Self::Home | Self::Account | Self::Settings(_) => None,
            Self::Personal(_) => Some(ActiveSurface::Personal),
            Self::Farmer(_) => Some(ActiveSurface::Farmer),
        }
    }

    pub const fn default_for_surface(surface: ActiveSurface) -> Self {
        match surface {
            ActiveSurface::Personal => Self::Personal(PersonalSection::Browse),
            ActiveSurface::Farmer => Self::Farmer(FarmerSection::Today),
        }
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Account => "account",
            Self::Personal(section) => section.storage_key(),
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
            "account" => Ok(Self::Account),
            "personal.browse" => Ok(Self::Personal(PersonalSection::Browse)),
            "personal.search" => Ok(Self::Personal(PersonalSection::Search)),
            "personal.cart" => Ok(Self::Personal(PersonalSection::Cart)),
            "personal.orders" => Ok(Self::Personal(PersonalSection::Orders)),
            "farmer.today" => Ok(Self::Farmer(FarmerSection::Today)),
            "farmer.products" => Ok(Self::Farmer(FarmerSection::Products)),
            "farmer.orders" => Ok(Self::Farmer(FarmerSection::Orders)),
            "farmer.pack_day" => Ok(Self::Farmer(FarmerSection::PackDay)),
            "farmer.farm" => Ok(Self::Farmer(FarmerSection::Farm)),
            "settings.account" => Ok(Self::Settings(SettingsSection::Account)),
            "settings.farm" => Ok(Self::Settings(SettingsSection::Farm)),
            "settings.settings" => Ok(Self::Settings(SettingsSection::Settings)),
            "settings.about" => Ok(Self::Settings(SettingsSection::About)),
            _ => Err(ParseShellSectionError),
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoggedOutStartupPhase {
    #[default]
    ContinuePrompt,
    IdentityChoice,
    GenerateKeyStarting,
    SignerEntry,
}

impl LoggedOutStartupPhase {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::ContinuePrompt => "continue_prompt",
            Self::IdentityChoice => "identity_choice",
            Self::GenerateKeyStarting => "generate_key_starting",
            Self::SignerEntry => "signer_entry",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupSignerSourceKind {
    BunkerUri,
    DiscoveryUrl,
}

impl StartupSignerSourceKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::BunkerUri => "bunker_uri",
            Self::DiscoveryUrl => "discovery_url",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseStartupSignerSourceError {
    EmptyInput,
    UnsupportedClientUri,
    UnsupportedSource,
    MissingDiscoveryUri,
}

impl fmt::Display for ParseStartupSignerSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("signer source input must not be empty"),
            Self::UnsupportedClientUri => formatter.write_str(
                "client nostrconnect URIs are not accepted by the app signer entry flow",
            ),
            Self::UnsupportedSource => {
                formatter.write_str("signer source input must be a bunker URI or discovery URL")
            }
            Self::MissingDiscoveryUri => {
                formatter.write_str("discovery URL must include a non-empty uri query parameter")
            }
        }
    }
}

impl Error for ParseStartupSignerSourceError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum StartupSignerSource {
    BunkerUri(String),
    DiscoveryUrl(String),
}

impl StartupSignerSource {
    pub const fn kind(&self) -> StartupSignerSourceKind {
        match self {
            Self::BunkerUri(_) => StartupSignerSourceKind::BunkerUri,
            Self::DiscoveryUrl(_) => StartupSignerSourceKind::DiscoveryUrl,
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::BunkerUri(value) | Self::DiscoveryUrl(value) => value,
        }
    }
}

impl FromStr for StartupSignerSource {
    type Err = ParseStartupSignerSourceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ParseStartupSignerSourceError::EmptyInput);
        }

        if trimmed.starts_with("nostrconnect://") {
            return Err(ParseStartupSignerSourceError::UnsupportedClientUri);
        }

        if trimmed.starts_with("bunker://") {
            return Ok(Self::BunkerUri(trimmed.to_owned()));
        }

        let url =
            Url::parse(trimmed).map_err(|_| ParseStartupSignerSourceError::UnsupportedSource)?;
        let has_discovery_uri = url
            .query_pairs()
            .any(|(key, value)| key == "uri" && !value.trim().is_empty());

        if !has_discovery_uri {
            return Err(ParseStartupSignerSourceError::MissingDiscoveryUri);
        }

        Ok(Self::DiscoveryUrl(trimmed.to_owned()))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StartupSignerEntryProjection {
    pub source_input: String,
}

impl StartupSignerEntryProjection {
    pub fn new(source_input: impl Into<String>) -> Self {
        Self {
            source_input: source_input.into(),
        }
    }

    pub fn parsed_source(&self) -> Result<StartupSignerSource, ParseStartupSignerSourceError> {
        self.source_input.parse()
    }

    pub fn set_source_input(&mut self, source_input: impl Into<String>) {
        self.source_input = source_input.into();
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoggedOutStartupProjection {
    pub phase: LoggedOutStartupPhase,
    pub signer_entry: StartupSignerEntryProjection,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "account_id", rename_all = "snake_case")]
pub enum BuyerContext {
    #[default]
    Guest,
    Account(String),
}

impl BuyerContext {
    pub const fn guest() -> Self {
        Self::Guest
    }

    pub fn account(account_id: impl Into<String>) -> Self {
        Self::Account(account_id.into())
    }

    pub fn storage_key(&self) -> String {
        match self {
            Self::Guest => "guest".to_owned(),
            Self::Account(account_id) => format!("account:{account_id}"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalEntryState {
    Blocked,
    #[default]
    Guest,
    SignedIn,
}

impl PersonalEntryState {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Guest => "guest",
            Self::SignedIn => "signed_in",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersonalEntryProjection {
    pub state: PersonalEntryState,
    pub selected_account: Option<SelectedAccountProjection>,
    pub can_enter_farmer_workspace: bool,
}

impl PersonalEntryProjection {
    pub fn blocked(selected_account: Option<SelectedAccountProjection>) -> Self {
        let can_enter_farmer_workspace = selected_account
            .as_ref()
            .is_some_and(|account| account.farmer_activation.is_active());

        Self {
            state: PersonalEntryState::Blocked,
            selected_account,
            can_enter_farmer_workspace,
        }
    }

    pub const fn guest() -> Self {
        Self {
            state: PersonalEntryState::Guest,
            selected_account: None,
            can_enter_farmer_workspace: false,
        }
    }

    pub fn signed_in(selected_account: SelectedAccountProjection) -> Self {
        let can_enter_farmer_workspace = selected_account.farmer_activation.is_active();

        Self {
            state: PersonalEntryState::SignedIn,
            selected_account: Some(selected_account),
            can_enter_farmer_workspace,
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

    pub fn personal_entry(&self) -> PersonalEntryProjection {
        match self.readiness {
            IdentityReadiness::MissingAccount => PersonalEntryProjection::guest(),
            IdentityReadiness::Blocked(_) => {
                PersonalEntryProjection::blocked(self.selected_account.clone())
            }
            IdentityReadiness::Ready => self
                .selected_account
                .clone()
                .map(PersonalEntryProjection::signed_in)
                .unwrap_or_else(PersonalEntryProjection::guest),
        }
    }

    pub fn buyer_context(&self) -> BuyerContext {
        self.selected_account
            .as_ref()
            .map(|account| BuyerContext::account(account.account.account_id.clone()))
            .unwrap_or_default()
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FarmRulesProjection {
    pub farm_profile: Option<FarmProfileRecord>,
    pub pickup_locations: Vec<PickupLocationRecord>,
    pub operating_rules: Option<FarmOperatingRulesRecord>,
    pub fulfillment_windows: Vec<FulfillmentWindowRecord>,
    pub blackout_periods: Vec<BlackoutPeriodRecord>,
    pub readiness: FarmRulesReadiness,
}

impl Default for FarmRulesProjection {
    fn default() -> Self {
        Self {
            farm_profile: None,
            pickup_locations: Vec::new(),
            operating_rules: None,
            fulfillment_windows: Vec::new(),
            blackout_periods: Vec::new(),
            readiness: FarmRulesReadiness::missing_v1_basics(),
        }
    }
}

impl FarmRulesProjection {
    pub fn is_ready(&self) -> bool {
        self.readiness.is_ready()
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductEditorDraft {
    pub title: String,
    pub subtitle: String,
    pub category: String,
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
            category: String::new(),
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

        if self.category.trim().is_empty() {
            blockers.push(ProductPublishBlocker::ChooseCategory);
        }

        if self.unit_label.trim().is_empty() {
            blockers.push(ProductPublishBlocker::ChooseUnit);
        }

        if self.price_minor_units.is_none_or(|value| value == 0) {
            blockers.push(ProductPublishBlocker::SetPrice);
        }

        if self.stock_quantity.is_none() {
            blockers.push(ProductPublishBlocker::SetStock);
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerListingRow {
    pub product_id: ProductId,
    pub farm_id: FarmId,
    pub farm_display_name: String,
    pub listing_relays: Vec<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub price: ProductPricePresentation,
    pub availability: ProductAvailabilitySummary,
    pub stock: ProductStockSummary,
    pub fulfillment_methods: BTreeSet<FarmOrderMethod>,
    pub next_fulfillment_window_label: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerListingsProjection {
    pub rows: Vec<BuyerListingRow>,
}

impl BuyerListingsProjection {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerProductDetailProjection {
    pub listing: BuyerListingRow,
    pub detail_text: Option<String>,
    pub selected_quantity: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerCartLineProjection {
    pub product_id: ProductId,
    pub farm_id: FarmId,
    pub farm_display_name: String,
    pub title: String,
    pub quantity: u32,
    pub unit_price: ProductPricePresentation,
    pub line_total_minor_units: u32,
    pub fulfillment_summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerCartReplaceConfirmationProjection {
    pub current_farm_display_name: String,
    pub incoming_farm_display_name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerCartProjection {
    pub farm_id: Option<FarmId>,
    pub farm_display_name: Option<String>,
    pub lines: Vec<BuyerCartLineProjection>,
    pub subtotal_minor_units: Option<u32>,
    pub currency_code: Option<String>,
    pub replace_confirmation: Option<BuyerCartReplaceConfirmationProjection>,
}

impl BuyerCartProjection {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrderReviewDraft {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub order_note: String,
    pub confirm_public_note: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrderReviewSummaryProjection {
    pub farm_display_name: Option<String>,
    pub fulfillment_summary: Option<String>,
    pub line_count: u32,
    pub subtotal_minor_units: Option<u32>,
    pub currency_code: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuyerOrderReviewDisabledReason {
    EmptyCart,
    MissingFulfillment,
    MissingName,
    MissingEmail,
    PublicNoteConfirmationRequired,
    AccountRequired,
}

impl BuyerOrderReviewDisabledReason {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::EmptyCart => "empty_cart",
            Self::MissingFulfillment => "missing_fulfillment",
            Self::MissingName => "missing_name",
            Self::MissingEmail => "missing_email",
            Self::PublicNoteConfirmationRequired => "public_note_confirmation_required",
            Self::AccountRequired => "account_required",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrderReviewProjection {
    pub draft: BuyerOrderReviewDraft,
    pub summary: BuyerOrderReviewSummaryProjection,
    pub can_place_order: bool,
    pub place_order_disabled_reason: Option<BuyerOrderReviewDisabledReason>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAgreementStatus {
    #[default]
    Requested,
    RevisionProposed,
    AgreedPendingRhi,
    Committed,
    Declined,
    Cancelled,
    Invalid,
}

impl TradeAgreementStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::RevisionProposed => "revision_proposed",
            Self::AgreedPendingRhi => "agreed_pending_rhi",
            Self::Committed => "committed",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Invalid => "invalid",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::Requested => "messages.trade.workflow.agreement.requested",
            Self::RevisionProposed => "messages.trade.workflow.agreement.revision_proposed",
            Self::AgreedPendingRhi => "messages.trade.workflow.agreement.agreed_pending_rhi",
            Self::Committed => "messages.trade.workflow.agreement.committed",
            Self::Declined => "messages.trade.workflow.agreement.declined",
            Self::Cancelled => "messages.trade.workflow.agreement.cancelled",
            Self::Invalid => "messages.trade.workflow.agreement.invalid",
        }
    }

    pub const fn from_active_order_status(status: &RadrootsTradeWorkflowState) -> Self {
        match status {
            RadrootsTradeWorkflowState::Missing => Self::Invalid,
            RadrootsTradeWorkflowState::Requested => Self::Requested,
            RadrootsTradeWorkflowState::RevisionProposed => Self::RevisionProposed,
            RadrootsTradeWorkflowState::AgreedPendingRhi => Self::AgreedPendingRhi,
            RadrootsTradeWorkflowState::Committed => Self::Committed,
            RadrootsTradeWorkflowState::Declined => Self::Declined,
            RadrootsTradeWorkflowState::Cancelled => Self::Cancelled,
            RadrootsTradeWorkflowState::Invalid => Self::Invalid,
        }
    }
}

impl From<&RadrootsTradeWorkflowState> for TradeAgreementStatus {
    fn from(status: &RadrootsTradeWorkflowState) -> Self {
        Self::from_active_order_status(status)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeRevisionStatus {
    #[default]
    None,
    ChangeProposed,
    Updated,
    KeptAsPlaced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseTradeRevisionStatusError {
    value: String,
}

impl ParseTradeRevisionStatusError {
    pub fn value(&self) -> &str {
        self.value.as_str()
    }
}

impl fmt::Display for ParseTradeRevisionStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid trade revision status `{}`", self.value)
    }
}

impl Error for ParseTradeRevisionStatusError {}

impl TradeRevisionStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ChangeProposed => "change_proposed",
            Self::Updated => "updated",
            Self::KeptAsPlaced => "kept_as_placed",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::None => "messages.trade.workflow.revision.none",
            Self::ChangeProposed => "messages.trade.workflow.revision.change_proposed",
            Self::Updated => "messages.trade.workflow.revision.updated",
            Self::KeptAsPlaced => "messages.trade.workflow.revision.kept_as_placed",
        }
    }

    pub fn try_from_storage_key(value: &str) -> Result<Self, ParseTradeRevisionStatusError> {
        match value {
            "none" => Ok(Self::None),
            "change_proposed" => Ok(Self::ChangeProposed),
            "updated" => Ok(Self::Updated),
            "kept_as_placed" => Ok(Self::KeptAsPlaced),
            _ => Err(ParseTradeRevisionStatusError {
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeInventoryStatus {
    Available,
    Reserved,
    SoldOut,
    #[default]
    NeedsReview,
}

impl TradeInventoryStatus {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Reserved => "reserved",
            Self::SoldOut => "sold_out",
            Self::NeedsReview => "needs_review",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::Available => "messages.trade.workflow.inventory.available",
            Self::Reserved => "messages.trade.workflow.inventory.reserved",
            Self::SoldOut => "messages.trade.workflow.inventory.sold_out",
            Self::NeedsReview => "messages.trade.workflow.inventory.needs_review",
        }
    }

    pub fn from_active_order_projection(projection: &RadrootsOrderProjection) -> Self {
        match projection.status {
            RadrootsTradeWorkflowState::Requested
            | RadrootsTradeWorkflowState::RevisionProposed => Self::NeedsReview,
            RadrootsTradeWorkflowState::AgreedPendingRhi
            | RadrootsTradeWorkflowState::Committed => Self::Reserved,
            RadrootsTradeWorkflowState::Declined | RadrootsTradeWorkflowState::Cancelled => {
                Self::Available
            }
            RadrootsTradeWorkflowState::Missing | RadrootsTradeWorkflowState::Invalid => {
                Self::NeedsReview
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeWorkflowSource {
    App,
    Cli,
    Relay,
    RuntimeStore,
    #[default]
    Unknown,
}

impl TradeWorkflowSource {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Cli => "cli",
            Self::Relay => "relay",
            Self::RuntimeStore => "runtime_store",
            Self::Unknown => "unknown",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::App => "messages.trade.workflow.provenance.app",
            Self::Cli => "messages.trade.workflow.provenance.cli",
            Self::Relay => "messages.trade.workflow.provenance.relay",
            Self::RuntimeStore => "messages.trade.workflow.provenance.runtime_store",
            Self::Unknown => "messages.trade.workflow.provenance.unknown",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeEconomicsProjection {
    pub subtotal_minor_units: Option<u32>,
    pub discount_total_minor_units: Option<u32>,
    pub adjustment_total_minor_units: Option<u32>,
    pub total_minor_units: Option<u32>,
    pub currency_code: Option<String>,
}

impl TradeEconomicsProjection {
    pub fn from_trade_order_economics(economics: &RadrootsOrderEconomics) -> Self {
        Self {
            subtotal_minor_units: money_minor_units(&economics.subtotal),
            discount_total_minor_units: money_minor_units(&economics.discount_total),
            adjustment_total_minor_units: money_minor_units(&economics.adjustment_total),
            total_minor_units: money_minor_units(&economics.total),
            currency_code: Some(economics.currency.to_string()),
        }
    }
}

impl From<&RadrootsOrderEconomics> for TradeEconomicsProjection {
    fn from(economics: &RadrootsOrderEconomics) -> Self {
        Self::from_trade_order_economics(economics)
    }
}

fn money_minor_units(money: &RadrootsCoreMoney) -> Option<u32> {
    money.to_minor_units_u32_exact().ok()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeProvenanceProjection {
    pub primary_source: TradeWorkflowSource,
    pub sources: BTreeSet<TradeWorkflowSource>,
    pub last_event_id: Option<String>,
}

impl TradeProvenanceProjection {
    pub fn new(
        primary_source: TradeWorkflowSource,
        sources: impl IntoIterator<Item = TradeWorkflowSource>,
    ) -> Self {
        let mut sources = sources.into_iter().collect::<BTreeSet<_>>();
        sources.insert(primary_source);
        Self {
            primary_source,
            sources,
            last_event_id: None,
        }
    }

    pub fn from_primary_source(primary_source: TradeWorkflowSource) -> Self {
        Self::new(primary_source, [primary_source])
    }

    pub fn with_last_event_id(mut self, last_event_id: Option<String>) -> Self {
        self.last_event_id = last_event_id;
        self
    }
}

impl Default for TradeProvenanceProjection {
    fn default() -> Self {
        Self::from_primary_source(TradeWorkflowSource::Unknown)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeValidationReceiptResult {
    #[default]
    Valid,
    NeedsReview,
}

impl TradeValidationReceiptResult {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::NeedsReview => "needs_review",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::Valid => "messages.trade.validation.result.valid",
            Self::NeedsReview => "messages.trade.validation.result.needs_review",
        }
    }

    pub const fn from_validation_receipt_result(result: RadrootsValidationReceiptResult) -> Self {
        match result {
            RadrootsValidationReceiptResult::Valid => Self::Valid,
            RadrootsValidationReceiptResult::Invalid => Self::NeedsReview,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeValidationReceiptType {
    ListingValidation,
    #[default]
    TradeTransition,
    InventoryState,
    StateCheckpoint,
}

impl TradeValidationReceiptType {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::ListingValidation => "listing_validation",
            Self::TradeTransition => "trade_transition",
            Self::InventoryState => "inventory_state",
            Self::StateCheckpoint => "state_checkpoint",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::ListingValidation => "messages.trade.validation.type.listing_validation",
            Self::TradeTransition => "messages.trade.validation.type.trade_transition",
            Self::InventoryState => "messages.trade.validation.type.inventory_state",
            Self::StateCheckpoint => "messages.trade.validation.type.state_checkpoint",
        }
    }

    pub const fn from_validation_receipt_type(receipt_type: RadrootsValidationReceiptType) -> Self {
        match receipt_type {
            RadrootsValidationReceiptType::ListingValidation => Self::ListingValidation,
            RadrootsValidationReceiptType::TradeTransition => Self::TradeTransition,
            RadrootsValidationReceiptType::InventoryState => Self::InventoryState,
            RadrootsValidationReceiptType::StateCheckpoint => Self::StateCheckpoint,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeValidationReceiptProofSystem {
    #[default]
    None,
    Sp1Core,
    Sp1Compressed,
    Sp1Groth16,
    Sp1Plonk,
}

impl TradeValidationReceiptProofSystem {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sp1Core => "sp1_core",
            Self::Sp1Compressed => "sp1_compressed",
            Self::Sp1Groth16 => "sp1_groth16",
            Self::Sp1Plonk => "sp1_plonk",
        }
    }

    pub const fn label_key_id(self) -> &'static str {
        match self {
            Self::None => "messages.trade.validation.proof.none",
            Self::Sp1Core => "messages.trade.validation.proof.sp1_core",
            Self::Sp1Compressed => "messages.trade.validation.proof.sp1_compressed",
            Self::Sp1Groth16 => "messages.trade.validation.proof.sp1_groth16",
            Self::Sp1Plonk => "messages.trade.validation.proof.sp1_plonk",
        }
    }

    pub const fn from_validation_receipt_proof_system(
        proof_system: RadrootsValidationReceiptProofSystem,
    ) -> Self {
        match proof_system {
            RadrootsValidationReceiptProofSystem::None => Self::None,
            RadrootsValidationReceiptProofSystem::Sp1Core => Self::Sp1Core,
            RadrootsValidationReceiptProofSystem::Sp1Compressed => Self::Sp1Compressed,
            RadrootsValidationReceiptProofSystem::Sp1Groth16 => Self::Sp1Groth16,
            RadrootsValidationReceiptProofSystem::Sp1Plonk => Self::Sp1Plonk,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeValidationReceiptProjection {
    pub event_id: String,
    pub result: TradeValidationReceiptResult,
    pub receipt_type: TradeValidationReceiptType,
    pub proof_system: TradeValidationReceiptProofSystem,
    pub event_set_root: String,
    pub reducer_output_root: String,
    pub public_values_hash: String,
    pub target_event_id: String,
    pub recorded_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeWorkflowProjection {
    pub order_id: OrderId,
    pub agreement: TradeAgreementStatus,
    pub revision: TradeRevisionStatus,
    pub economics: TradeEconomicsProjection,
    pub inventory: TradeInventoryStatus,
    pub provenance: TradeProvenanceProjection,
}

impl TradeWorkflowProjection {
    pub fn new(order_id: OrderId, agreement: TradeAgreementStatus) -> Self {
        Self {
            order_id,
            agreement,
            revision: TradeRevisionStatus::None,
            economics: TradeEconomicsProjection::default(),
            inventory: TradeInventoryStatus::NeedsReview,
            provenance: TradeProvenanceProjection::default(),
        }
    }

    pub fn from_active_order_projection(
        order_id: OrderId,
        projection: &RadrootsOrderProjection,
        revision: TradeRevisionStatus,
        provenance: TradeProvenanceProjection,
    ) -> Self {
        let mut workflow = Self::new(
            order_id,
            TradeAgreementStatus::from_active_order_status(&projection.status),
        );
        workflow.revision = revision;
        workflow.economics = projection
            .economics
            .as_ref()
            .map(TradeEconomicsProjection::from_trade_order_economics)
            .unwrap_or_default();
        workflow.inventory = TradeInventoryStatus::from_active_order_projection(projection);
        workflow.provenance = provenance
            .with_last_event_id(projection.last_event_id.as_ref().map(ToString::to_string));
        workflow
    }

    pub fn from_order_status(order_id: OrderId, status: OrderStatus) -> Self {
        let mut projection = match status {
            OrderStatus::NeedsAction => Self::new(order_id, TradeAgreementStatus::Requested),
            OrderStatus::Scheduled => Self::new(order_id, TradeAgreementStatus::Committed),
            OrderStatus::Packed => Self::new(order_id, TradeAgreementStatus::Committed),
            OrderStatus::Completed => Self::new(order_id, TradeAgreementStatus::Committed),
            OrderStatus::Declined => Self::new(order_id, TradeAgreementStatus::Declined),
            OrderStatus::NeedsReview => Self::new(order_id, TradeAgreementStatus::Invalid),
        };

        match status {
            OrderStatus::NeedsAction => {}
            OrderStatus::Scheduled => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            OrderStatus::Packed => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            OrderStatus::Completed => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            OrderStatus::Declined => {
                projection.inventory = TradeInventoryStatus::Available;
            }
            OrderStatus::NeedsReview => {}
        }

        projection
    }

    pub fn from_buyer_order_status(order_id: OrderId, status: BuyerOrderStatus) -> Self {
        let mut projection = match status {
            BuyerOrderStatus::Placed => Self::new(order_id, TradeAgreementStatus::Requested),
            BuyerOrderStatus::Scheduled => Self::new(order_id, TradeAgreementStatus::Committed),
            BuyerOrderStatus::Ready => Self::new(order_id, TradeAgreementStatus::Committed),
            BuyerOrderStatus::Completed => Self::new(order_id, TradeAgreementStatus::Committed),
            BuyerOrderStatus::Declined => Self::new(order_id, TradeAgreementStatus::Declined),
            BuyerOrderStatus::NeedsReview => Self::new(order_id, TradeAgreementStatus::Invalid),
        };

        match status {
            BuyerOrderStatus::Placed => {}
            BuyerOrderStatus::Scheduled => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            BuyerOrderStatus::Ready => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            BuyerOrderStatus::Completed => {
                projection.inventory = TradeInventoryStatus::Reserved;
            }
            BuyerOrderStatus::Declined => {
                projection.inventory = TradeInventoryStatus::Available;
            }
            BuyerOrderStatus::NeedsReview => {}
        }

        projection
    }

    pub fn with_economics(mut self, economics: TradeEconomicsProjection) -> Self {
        self.economics = economics;
        self
    }

    pub fn with_revision(mut self, revision: TradeRevisionStatus) -> Self {
        self.revision = revision;
        self
    }
}

pub fn order_status_from_active_order_projection(
    projection: &RadrootsOrderProjection,
) -> Option<OrderStatus> {
    match projection.status {
        RadrootsTradeWorkflowState::Missing => None,
        RadrootsTradeWorkflowState::Requested
        | RadrootsTradeWorkflowState::RevisionProposed
        | RadrootsTradeWorkflowState::AgreedPendingRhi => Some(OrderStatus::NeedsAction),
        RadrootsTradeWorkflowState::Committed => Some(OrderStatus::Scheduled),
        RadrootsTradeWorkflowState::Declined | RadrootsTradeWorkflowState::Cancelled => {
            Some(OrderStatus::Declined)
        }
        RadrootsTradeWorkflowState::Invalid => Some(OrderStatus::NeedsAction),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdersFilter {
    All,
    #[default]
    NeedsAction,
    Scheduled,
    Packed,
    Completed,
}

impl OrdersFilter {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsAction => "needs_action",
            Self::Scheduled => "scheduled",
            Self::Packed => "packed",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrdersScreenQueryState {
    pub filter: OrdersFilter,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderPrimaryAction {
    Review,
}

impl OrderPrimaryAction {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Review => "review",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrdersListSummary {
    pub total_orders: u32,
    pub needs_action_orders: u32,
    pub scheduled_orders: u32,
    pub packed_orders: u32,
}

impl OrdersListSummary {
    pub const fn has_orders(&self) -> bool {
        self.total_orders > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrdersListRow {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub order_number: String,
    pub customer_display_name: String,
    pub fulfillment_window_label: Option<String>,
    pub pickup_location_label: Option<String>,
    pub status: OrderStatus,
    pub workflow: TradeWorkflowProjection,
    pub primary_action: Option<OrderPrimaryAction>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrdersListProjection {
    pub summary: OrdersListSummary,
    pub rows: Vec<OrdersListRow>,
}

impl OrdersListProjection {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderDetailItemRow {
    pub title: String,
    pub quantity_display: String,
    pub unit_price: Option<ProductPricePresentation>,
    pub line_total_minor_units: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderDetailProjection {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub order_number: String,
    pub customer_display_name: String,
    pub status: OrderStatus,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub fulfillment_window_label: Option<String>,
    pub pickup_location_label: Option<String>,
    pub items: Vec<OrderDetailItemRow>,
    pub economics: TradeEconomicsProjection,
    pub workflow: TradeWorkflowProjection,
    pub validation_receipts: Vec<TradeValidationReceiptProjection>,
    pub primary_action: Option<OrderPrimaryAction>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrdersListRow {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub order_number: String,
    pub farm_display_name: String,
    pub fulfillment_summary: String,
    pub status: BuyerOrderStatus,
    pub workflow: TradeWorkflowProjection,
    pub repeat_demand: Option<RepeatDemandHandoffProjection>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrdersProjection {
    pub rows: Vec<BuyerOrdersListRow>,
}

impl BuyerOrdersProjection {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerOrderDetailProjection {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub order_number: String,
    pub farm_display_name: String,
    pub fulfillment_summary: String,
    pub status: BuyerOrderStatus,
    pub items: Vec<OrderDetailItemRow>,
    pub economics: TradeEconomicsProjection,
    pub workflow: TradeWorkflowProjection,
    pub validation_receipts: Vec<TradeValidationReceiptProjection>,
    pub order_note: Option<String>,
    pub repeat_demand: Option<RepeatDemandHandoffProjection>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayScreenQueryState {
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayProductTotalRow {
    pub title: String,
    pub quantity_display: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayPackListRow {
    pub title: String,
    pub quantity_display: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayRosterRow {
    pub order_id: OrderId,
    pub order_number: String,
    pub customer_display_name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PackDayProjection {
    pub fulfillment_window: Option<FulfillmentWindowSummary>,
    pub reminders: ReminderFeedProjection,
    pub totals_by_product: Vec<PackDayProductTotalRow>,
    pub pack_list: Vec<PackDayPackListRow>,
    pub pickup_roster: Vec<PackDayRosterRow>,
}

impl PackDayProjection {
    pub fn is_empty(&self) -> bool {
        self.totals_by_product.is_empty()
            && self.pack_list.is_empty()
            && self.pickup_roster.is_empty()
    }
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
    pub reminders_due_soon: u32,
}

impl TodaySummary {
    pub const fn has_attention_items(&self) -> bool {
        self.orders_needing_action > 0
            || self.low_stock_products > 0
            || self.draft_products > 0
            || self.reminders_due_soon > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReminderDeadlineProjection {
    pub reminder_id: ReminderId,
    pub farm_id: FarmId,
    pub order_id: Option<OrderId>,
    pub fulfillment_window_id: Option<FulfillmentWindowId>,
    pub kind: ReminderKind,
    pub surface: ReminderSurface,
    pub urgency: ReminderUrgency,
    pub title: String,
    pub detail: String,
    pub deadline_at: String,
    pub action_label: Option<String>,
    pub delivery_state: ReminderDeliveryState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReminderFeedProjection {
    pub items: Vec<ReminderDeadlineProjection>,
}

impl ReminderFeedProjection {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn due_soon_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item.urgency,
                    ReminderUrgency::DueSoon | ReminderUrgency::Overdue | ReminderUrgency::Blocking
                )
            })
            .count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReminderLogEntryProjection {
    pub reminder_id: ReminderId,
    pub kind: ReminderKind,
    pub title: String,
    pub recorded_at: String,
    pub delivery_state: ReminderDeliveryState,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReminderLogProjection {
    pub entries: Vec<ReminderLogEntryProjection>,
}

impl ReminderLogProjection {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepeatDemandHandoffProjection {
    pub order_id: OrderId,
    pub farm_id: FarmId,
    pub eligibility: RepeatDemandEligibility,
    pub available_item_count: u32,
    pub unavailable_item_count: u32,
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
    CompleteFarmProfile,
    AddPickupLocation,
    AddOperatingRules,
    AddFulfillmentWindow,
    ResolveAvailabilityConflicts,
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
    pub reminders: ReminderFeedProjection,
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
            || !self.reminders.is_empty()
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
    use radroots_core::{
        RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreMoney, RadrootsCoreUnit,
    };
    use radroots_event::ids::{
        RadrootsEventId, RadrootsInventoryBinId, RadrootsListingAddress, RadrootsOrderId,
        RadrootsOrderQuoteId, RadrootsPublicKey,
    };
    use radroots_event::order::{
        RadrootsOrderEconomicItem, RadrootsOrderEconomics, RadrootsOrderPricingBasis,
    };
    use radroots_trade::validation_receipt::{
        RadrootsValidationReceiptProofSystem, RadrootsValidationReceiptResult,
        RadrootsValidationReceiptType,
    };
    use radroots_trade::{order::RadrootsOrderProjection, workflow::RadrootsTradeWorkflowState};

    use super::{
        AccountCustody, AccountSummary, AccountSurfaceActivationProjection, ActiveSurface,
        ActivityEventId, AppActivityContext, AppActivityEvent, AppActivityKind,
        AppIdentityProjection, AppStartupGate, BlackoutPeriodId, BuyerCartLineProjection,
        BuyerCartProjection, BuyerContext, BuyerListingRow, BuyerListingsProjection,
        BuyerOrderDetailProjection, BuyerOrderReviewDisabledReason, BuyerOrderReviewDraft,
        BuyerOrderReviewProjection, BuyerOrderReviewSummaryProjection, BuyerOrderStatus,
        BuyerOrdersListRow, BuyerOrdersProjection, FarmId, FarmOrderMethod, FarmReadinessBlocker,
        FarmRulesProjection, FarmRulesReadiness, FarmSetupBlocker, FarmSetupDraft,
        FarmSetupProjection, FarmSetupReadiness, FarmSetupSection, FarmTimingConflict,
        FarmTimingConflictKind, FarmerActivationProjection, FarmerSection, FulfillmentWindowId,
        IdentityBlockedReason, IdentityReadiness, LoggedOutStartupPhase,
        LoggedOutStartupProjection, OrderDetailItemRow, OrderDetailProjection, OrderId,
        OrderListRow, OrderPrimaryAction, OrderStatus, OrdersFilter, OrdersListProjection,
        OrdersListRow, OrdersListSummary, OrdersScreenQueryState, PackDayBatchPrintArtifact,
        PackDayBatchPrintFailureKind, PackDayBatchPrintStatus, PackDayExportArtifact,
        PackDayExportArtifactKind, PackDayExportBundle, PackDayExportInstanceId,
        PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus,
        PackDayOutputCustomerOrder, PackDayOutputOrderState, PackDayOutputPackListEntry,
        PackDayOutputProductTotal, PackDayOutputQuantity, PackDayOutputSource, PackDayOutputWindow,
        PackDayPackListRow, PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintLabelStock,
        PackDayPrintStatus, PackDayProductTotalRow, PackDayProjection, PackDayRosterRow,
        PackDayScreenQueryState, ParseStartupSignerSourceError, PersonalEntryProjection,
        PersonalEntryState, PersonalSection, PickupLocationId, ProductAttentionState,
        ProductAvailabilityState, ProductAvailabilitySummary, ProductEditorDraft, ProductListRow,
        ProductPricePresentation, ProductPublishBlocker, ProductStatus, ProductStockState,
        ProductStockSummary, ProductsFilter, ProductsListProjection, ProductsListRow,
        ProductsListSummary, ProductsSort, ReminderDeadlineProjection, ReminderDeliveryState,
        ReminderFeedProjection, ReminderId, ReminderKind, ReminderLogEntryProjection,
        ReminderLogProjection, ReminderSurface, ReminderUrgency, RepeatDemandEligibility,
        RepeatDemandHandoffProjection, SelectedAccountProjection, SelectedSurfaceProjection,
        SettingsPreference, SettingsSection, ShellSection, StartupSignerEntryProjection,
        StartupSignerSource, StartupSignerSourceKind, TodayAgendaProjection, TodaySetupTask,
        TodaySetupTaskKind, TodaySummary, TradeAgreementStatus, TradeEconomicsProjection,
        TradeInventoryStatus, TradeProvenanceProjection, TradeRevisionStatus,
        TradeValidationReceiptProofSystem, TradeValidationReceiptResult,
        TradeValidationReceiptType, TradeWorkflowProjection, TradeWorkflowSource,
        order_status_from_active_order_projection,
    };
    use std::{collections::BTreeSet, str::FromStr};
    use uuid::Uuid;

    #[test]
    fn shell_section_storage_keys_are_unique_and_round_trip() {
        let sections = [
            ShellSection::Home,
            ShellSection::Personal(PersonalSection::Browse),
            ShellSection::Personal(PersonalSection::Search),
            ShellSection::Personal(PersonalSection::Cart),
            ShellSection::Personal(PersonalSection::Orders),
            ShellSection::Account,
            ShellSection::Farmer(FarmerSection::Today),
            ShellSection::Farmer(FarmerSection::Products),
            ShellSection::Farmer(FarmerSection::Orders),
            ShellSection::Farmer(FarmerSection::PackDay),
            ShellSection::Farmer(FarmerSection::Farm),
            ShellSection::Settings(SettingsSection::Account),
            ShellSection::Settings(SettingsSection::Farm),
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
    fn shell_section_surface_is_explicit_for_surface_routes_only() {
        assert_eq!(ShellSection::Home.surface(), None);
        assert_eq!(ShellSection::Account.surface(), None);
        assert_eq!(
            ShellSection::Personal(PersonalSection::Browse).surface(),
            Some(ActiveSurface::Personal)
        );
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
            ShellSection::Personal(PersonalSection::Browse)
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
            FarmerActivationProjection::active(FarmId::generate()),
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
                FarmerActivationProjection::active(FarmId::generate()),
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
    fn personal_entry_projection_is_derived_from_identity_truth() {
        let guest_identity = AppIdentityProjection::missing();
        let selected_account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_farmer".to_owned(),
                npub: "npub1farmer".to_owned(),
                label: Some("Field stand".to_owned()),
                custody: AccountCustody::LocalManaged,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Farmer),
            FarmerActivationProjection::active(FarmId::generate()),
        );
        let signed_in_identity = AppIdentityProjection::ready(Vec::new(), selected_account.clone());
        let blocked_identity = AppIdentityProjection::blocked_with_selection(
            IdentityBlockedReason::HostVaultUnavailable,
            Vec::new(),
            Some(selected_account.clone()),
        );

        assert_eq!(
            guest_identity.personal_entry(),
            PersonalEntryProjection::guest()
        );
        assert_eq!(
            guest_identity.personal_entry().state.storage_key(),
            PersonalEntryState::Guest.storage_key()
        );
        assert_eq!(
            signed_in_identity.personal_entry(),
            PersonalEntryProjection::signed_in(selected_account.clone())
        );
        assert!(
            signed_in_identity
                .personal_entry()
                .can_enter_farmer_workspace
        );
        assert_eq!(
            blocked_identity.personal_entry(),
            PersonalEntryProjection::blocked(Some(selected_account))
        );
    }

    #[test]
    fn buyer_context_defaults_to_guest_and_tracks_selected_account() {
        let selected_account = SelectedAccountProjection::new(
            AccountSummary {
                account_id: "acct_buyer".to_owned(),
                npub: "npub1buyer".to_owned(),
                label: Some("Buyer".to_owned()),
                custody: AccountCustody::LocalManaged,
            },
            SelectedSurfaceProjection::new(ActiveSurface::Personal),
            FarmerActivationProjection::inactive(),
        );
        let ready_identity = AppIdentityProjection::ready(Vec::new(), selected_account);

        assert_eq!(BuyerContext::guest().storage_key(), "guest");
        assert_eq!(
            BuyerContext::account("acct_buyer").storage_key(),
            "account:acct_buyer"
        );
        assert_eq!(
            AppIdentityProjection::missing().buyer_context(),
            BuyerContext::Guest
        );
        assert_eq!(
            ready_identity.buyer_context(),
            BuyerContext::account("acct_buyer")
        );
    }

    #[test]
    fn logged_out_startup_defaults_to_continue_prompt_with_empty_signer_entry() {
        assert_eq!(
            LoggedOutStartupProjection::default(),
            LoggedOutStartupProjection {
                phase: LoggedOutStartupPhase::ContinuePrompt,
                signer_entry: StartupSignerEntryProjection::default(),
            }
        );
    }

    #[test]
    fn logged_out_startup_phase_and_signer_source_kind_storage_keys_are_stable() {
        assert_eq!(
            LoggedOutStartupPhase::ContinuePrompt.storage_key(),
            "continue_prompt"
        );
        assert_eq!(
            LoggedOutStartupPhase::IdentityChoice.storage_key(),
            "identity_choice"
        );
        assert_eq!(
            LoggedOutStartupPhase::GenerateKeyStarting.storage_key(),
            "generate_key_starting"
        );
        assert_eq!(
            LoggedOutStartupPhase::SignerEntry.storage_key(),
            "signer_entry"
        );
        assert_eq!(
            StartupSignerSourceKind::BunkerUri.storage_key(),
            "bunker_uri"
        );
        assert_eq!(
            StartupSignerSourceKind::DiscoveryUrl.storage_key(),
            "discovery_url"
        );
    }

    #[test]
    fn startup_signer_source_parses_direct_bunker_uri_and_discovery_url() {
        let bunker_uri =
            "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example&secret=test-secret";
        let discovery_url =
            format!("https://signer.radroots.example/connect?uri={bunker_uri}&label=field");

        let bunker_source = bunker_uri
            .parse::<StartupSignerSource>()
            .expect("bunker uri should parse");
        let discovery_source = discovery_url
            .parse::<StartupSignerSource>()
            .expect("discovery url should parse");

        assert_eq!(
            bunker_source,
            StartupSignerSource::BunkerUri(bunker_uri.to_owned())
        );
        assert_eq!(bunker_source.kind(), StartupSignerSourceKind::BunkerUri);
        assert_eq!(bunker_source.value(), bunker_uri);
        assert_eq!(
            discovery_source,
            StartupSignerSource::DiscoveryUrl(discovery_url.clone())
        );
        assert_eq!(
            discovery_source.kind(),
            StartupSignerSourceKind::DiscoveryUrl
        );
        assert_eq!(discovery_source.value(), discovery_url);
    }

    #[test]
    fn startup_signer_source_rejects_empty_client_uri_and_missing_discovery_uri() {
        assert_eq!(
            "".parse::<StartupSignerSource>(),
            Err(ParseStartupSignerSourceError::EmptyInput)
        );
        assert_eq!(
            "nostrconnect://npub1client?relay=wss%3A%2F%2Frelay.radroots.example&secret=test"
                .parse::<StartupSignerSource>(),
            Err(ParseStartupSignerSourceError::UnsupportedClientUri)
        );
        assert_eq!(
            "https://signer.radroots.example/connect".parse::<StartupSignerSource>(),
            Err(ParseStartupSignerSourceError::MissingDiscoveryUri)
        );
        assert_eq!(
            "not a signer source".parse::<StartupSignerSource>(),
            Err(ParseStartupSignerSourceError::UnsupportedSource)
        );
    }

    #[test]
    fn signer_entry_projection_exposes_the_typed_source_contract() {
        let mut projection = StartupSignerEntryProjection::new(
            " bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example ",
        );

        assert_eq!(
            projection.parsed_source(),
            Ok(StartupSignerSource::BunkerUri(
                "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example".to_owned()
            ))
        );

        projection.set_source_input("https://signer.radroots.example/connect?uri=bunker://npub1");
        assert_eq!(
            projection.parsed_source(),
            Ok(StartupSignerSource::DiscoveryUrl(
                "https://signer.radroots.example/connect?uri=bunker://npub1".to_owned()
            ))
        );
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
    fn buyer_order_review_disabled_reason_storage_keys_are_stable() {
        assert_eq!(
            BuyerOrderReviewDisabledReason::EmptyCart.storage_key(),
            "empty_cart"
        );
        assert_eq!(
            BuyerOrderReviewDisabledReason::MissingFulfillment.storage_key(),
            "missing_fulfillment"
        );
        assert_eq!(
            BuyerOrderReviewDisabledReason::MissingName.storage_key(),
            "missing_name"
        );
        assert_eq!(
            BuyerOrderReviewDisabledReason::MissingEmail.storage_key(),
            "missing_email"
        );
        assert_eq!(
            BuyerOrderReviewDisabledReason::PublicNoteConfirmationRequired.storage_key(),
            "public_note_confirmation_required"
        );
        assert_eq!(
            BuyerOrderReviewDisabledReason::AccountRequired.storage_key(),
            "account_required"
        );
    }

    #[test]
    fn product_attention_stock_and_projection_states_are_explicit() {
        let row = ProductsListRow {
            product_id: super::ProductId::generate(),
            farm_id: FarmId::generate(),
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
            category: "vegetables".to_owned(),
            unit_label: "lb".to_owned(),
            price_minor_units: Some(450),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(12),
            availability_window_id: Some(super::FulfillmentWindowId::generate()),
            status: ProductStatus::Draft,
        };

        assert_eq!(
            empty_draft.publish_blockers(),
            vec![
                ProductPublishBlocker::AddProductName,
                ProductPublishBlocker::ChooseCategory,
                ProductPublishBlocker::ChooseUnit,
                ProductPublishBlocker::SetPrice,
                ProductPublishBlocker::SetStock,
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
    fn order_status_filter_and_primary_action_storage_keys_are_stable() {
        assert_eq!(OrderStatus::NeedsAction.storage_key(), "needs_action");
        assert_eq!(OrderStatus::Scheduled.storage_key(), "scheduled");
        assert_eq!(OrderStatus::Packed.storage_key(), "packed");
        assert_eq!(OrderStatus::Completed.storage_key(), "completed");
        assert_eq!(OrderStatus::Declined.storage_key(), "declined");
        assert_eq!(OrderStatus::NeedsReview.storage_key(), "needs_review");
        assert_eq!(BuyerOrderStatus::Placed.storage_key(), "placed");
        assert_eq!(BuyerOrderStatus::Scheduled.storage_key(), "scheduled");
        assert_eq!(BuyerOrderStatus::Ready.storage_key(), "ready");
        assert_eq!(BuyerOrderStatus::Completed.storage_key(), "completed");
        assert_eq!(BuyerOrderStatus::Declined.storage_key(), "declined");
        assert_eq!(BuyerOrderStatus::NeedsReview.storage_key(), "needs_review");
        assert_eq!(
            BuyerOrderStatus::from(OrderStatus::NeedsAction),
            BuyerOrderStatus::Placed
        );
        assert_eq!(
            BuyerOrderStatus::from(OrderStatus::Packed),
            BuyerOrderStatus::Ready
        );
        assert_eq!(
            BuyerOrderStatus::from(OrderStatus::Declined),
            BuyerOrderStatus::Declined
        );
        assert_eq!(
            BuyerOrderStatus::from(OrderStatus::NeedsReview),
            BuyerOrderStatus::NeedsReview
        );

        assert_eq!(OrdersFilter::default(), OrdersFilter::NeedsAction);
        assert_eq!(OrdersFilter::All.storage_key(), "all");
        assert_eq!(OrdersFilter::NeedsAction.storage_key(), "needs_action");
        assert_eq!(OrdersFilter::Scheduled.storage_key(), "scheduled");
        assert_eq!(OrdersFilter::Packed.storage_key(), "packed");
        assert_eq!(OrdersFilter::Completed.storage_key(), "completed");

        assert_eq!(OrderPrimaryAction::Review.storage_key(), "review");
    }

    fn test_decimal(raw: &str) -> RadrootsCoreDecimal {
        raw.parse().expect("test decimal should parse")
    }

    fn test_usd(raw: &str) -> RadrootsCoreMoney {
        RadrootsCoreMoney::new(test_decimal(raw), RadrootsCoreCurrency::USD)
    }

    fn test_order_id(raw: &str) -> RadrootsOrderId {
        raw.parse().expect("test order id should parse")
    }

    fn test_quote_id(raw: &str) -> RadrootsOrderQuoteId {
        raw.parse().expect("test quote id should parse")
    }

    fn test_bin_id(raw: &str) -> RadrootsInventoryBinId {
        raw.parse().expect("test bin id should parse")
    }

    fn test_event_id(raw: &str) -> RadrootsEventId {
        raw.parse().expect("test event id should parse")
    }

    fn test_pubkey(raw: &str) -> RadrootsPublicKey {
        raw.parse().expect("test pubkey should parse")
    }

    fn test_listing_addr(raw: &str) -> RadrootsListingAddress {
        raw.parse().expect("test listing address should parse")
    }

    fn test_trade_economics() -> RadrootsOrderEconomics {
        RadrootsOrderEconomics {
            quote_id: test_quote_id("quote-1"),
            quote_version: 2,
            pricing_basis: RadrootsOrderPricingBasis::ListingEvent,
            currency: RadrootsCoreCurrency::USD,
            items: vec![RadrootsOrderEconomicItem {
                bin_id: test_bin_id("bin-1"),
                bin_count: 2,
                quantity_amount: test_decimal("1"),
                quantity_unit: RadrootsCoreUnit::Each,
                unit_price_amount: test_decimal("6.17"),
                unit_price_currency: RadrootsCoreCurrency::USD,
                line_subtotal: test_usd("12.34"),
            }],
            discounts: Vec::new(),
            adjustments: Vec::new(),
            subtotal: test_usd("12.34"),
            discount_total: test_usd("0"),
            adjustment_total: test_usd("0"),
            total: test_usd("12.34"),
        }
    }

    fn test_active_order_projection(status: RadrootsTradeWorkflowState) -> RadrootsOrderProjection {
        RadrootsOrderProjection {
            order_id: test_order_id("ord_AAAAAAAAAAAAAAAAAAAAAg"),
            status,
            request_event_id: Some(test_event_id(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )),
            decision_event_id: Some(test_event_id(
                "2222222222222222222222222222222222222222222222222222222222222222",
            )),
            validation_receipt_event_id: None,
            cancellation_event_id: None,
            lifecycle_terminal: false,
            economics: Some(test_trade_economics()),
            agreement_event_id: Some(test_event_id(
                "2222222222222222222222222222222222222222222222222222222222222222",
            )),
            pending_revision_event_id: None,
            pending_inventory_reservations: Vec::new(),
            committed_inventory_reservations: Vec::new(),
            listing_addr: Some(test_listing_addr(
                "30402:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb:AAAAAAAAAAAAAAAAAAAAAg",
            )),
            buyer_pubkey: Some(test_pubkey(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )),
            seller_pubkey: Some(test_pubkey(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )),
            last_event_id: Some(test_event_id(
                "3333333333333333333333333333333333333333333333333333333333333333",
            )),
            issues: Vec::new(),
        }
    }

    #[test]
    fn trade_workflow_projection_maps_shared_active_order_projection_to_product_axes() {
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(&RadrootsTradeWorkflowState::Requested),
            TradeAgreementStatus::Requested
        );
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(
                &RadrootsTradeWorkflowState::RevisionProposed
            ),
            TradeAgreementStatus::RevisionProposed
        );
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(
                &RadrootsTradeWorkflowState::AgreedPendingRhi
            ),
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(&RadrootsTradeWorkflowState::Committed),
            TradeAgreementStatus::Committed
        );
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(&RadrootsTradeWorkflowState::Invalid),
            TradeAgreementStatus::Invalid
        );
        assert_eq!(
            TradeRevisionStatus::try_from_storage_key("none"),
            Ok(TradeRevisionStatus::None)
        );
        assert_eq!(
            TradeRevisionStatus::try_from_storage_key("change_proposed"),
            Ok(TradeRevisionStatus::ChangeProposed)
        );
        assert_eq!(
            TradeRevisionStatus::try_from_storage_key("updated"),
            Ok(TradeRevisionStatus::Updated)
        );
        assert_eq!(
            TradeRevisionStatus::try_from_storage_key("kept_as_placed"),
            Ok(TradeRevisionStatus::KeptAsPlaced)
        );
        assert_eq!(
            TradeRevisionStatus::try_from_storage_key("proposed")
                .expect_err("shared reducer key should not parse as app revision key")
                .value(),
            "proposed"
        );
        assert!(
            TradeRevisionStatus::try_from_storage_key(" none ").is_err(),
            "storage keys must parse exactly"
        );

        let order_id = OrderId::generate();
        let active_order = test_active_order_projection(RadrootsTradeWorkflowState::Committed);
        let projection = TradeWorkflowProjection::from_active_order_projection(
            order_id,
            &active_order,
            TradeRevisionStatus::Updated,
            TradeProvenanceProjection::from_primary_source(TradeWorkflowSource::RuntimeStore),
        );
        assert_eq!(projection.order_id, order_id);
        assert_eq!(projection.agreement, TradeAgreementStatus::Committed);
        assert_eq!(projection.revision, TradeRevisionStatus::Updated);
        assert_eq!(projection.inventory, TradeInventoryStatus::Reserved);
        assert_eq!(projection.economics.total_minor_units, Some(1234));
        assert_eq!(projection.economics.currency_code.as_deref(), Some("USD"));
        assert_eq!(
            projection.provenance,
            TradeProvenanceProjection::from_primary_source(TradeWorkflowSource::RuntimeStore)
                .with_last_event_id(Some(
                    "3333333333333333333333333333333333333333333333333333333333333333".to_owned()
                ))
        );
        assert_eq!(
            order_status_from_active_order_projection(&active_order),
            Some(OrderStatus::Scheduled)
        );

        let pending_rhi_order =
            test_active_order_projection(RadrootsTradeWorkflowState::AgreedPendingRhi);
        let pending_rhi_projection = TradeWorkflowProjection::from_active_order_projection(
            order_id,
            &pending_rhi_order,
            TradeRevisionStatus::None,
            TradeProvenanceProjection::from_primary_source(TradeWorkflowSource::RuntimeStore),
        );
        assert_eq!(
            pending_rhi_projection.agreement,
            TradeAgreementStatus::AgreedPendingRhi
        );
        assert_eq!(
            pending_rhi_projection.inventory,
            TradeInventoryStatus::Reserved
        );
        assert_eq!(
            order_status_from_active_order_projection(&pending_rhi_order),
            Some(OrderStatus::NeedsAction)
        );

        let requested_order = test_active_order_projection(RadrootsTradeWorkflowState::Requested);
        let requested_projection = TradeWorkflowProjection::from_active_order_projection(
            order_id,
            &requested_order,
            TradeRevisionStatus::None,
            TradeProvenanceProjection::from_primary_source(TradeWorkflowSource::RuntimeStore),
        );
        assert_eq!(
            requested_projection.agreement,
            TradeAgreementStatus::Requested
        );
        assert_eq!(
            requested_projection.inventory,
            TradeInventoryStatus::NeedsReview
        );
    }

    #[test]
    fn validation_receipt_projection_maps_shared_receipt_metadata_passively() {
        assert_eq!(
            TradeValidationReceiptResult::from_validation_receipt_result(
                RadrootsValidationReceiptResult::Valid
            ),
            TradeValidationReceiptResult::Valid
        );
        assert_eq!(
            TradeValidationReceiptResult::from_validation_receipt_result(
                RadrootsValidationReceiptResult::Invalid
            ),
            TradeValidationReceiptResult::NeedsReview
        );
        assert_eq!(
            TradeValidationReceiptType::from_validation_receipt_type(
                RadrootsValidationReceiptType::TradeTransition
            ),
            TradeValidationReceiptType::TradeTransition
        );
        assert_eq!(
            TradeValidationReceiptProofSystem::from_validation_receipt_proof_system(
                RadrootsValidationReceiptProofSystem::Sp1Compressed
            ),
            TradeValidationReceiptProofSystem::Sp1Compressed
        );
        assert_eq!(TradeValidationReceiptResult::Valid.storage_key(), "valid");
        assert_eq!(
            TradeValidationReceiptResult::NeedsReview.storage_key(),
            "needs_review"
        );
        assert_eq!(
            TradeValidationReceiptType::TradeTransition.storage_key(),
            "trade_transition"
        );
        assert_eq!(
            TradeValidationReceiptProofSystem::Sp1Compressed.storage_key(),
            "sp1_compressed"
        );
        assert_eq!(
            TradeValidationReceiptResult::NeedsReview.label_key_id(),
            "messages.trade.validation.result.needs_review"
        );
        assert_eq!(
            TradeValidationReceiptType::InventoryState.label_key_id(),
            "messages.trade.validation.type.inventory_state"
        );
        assert_eq!(
            TradeValidationReceiptProofSystem::Sp1Compressed.label_key_id(),
            "messages.trade.validation.proof.sp1_compressed"
        );
    }

    #[test]
    fn trade_workflow_projection_uses_localization_key_ids_for_visible_status_labels() {
        assert_eq!(
            TradeAgreementStatus::from_active_order_status(&RadrootsTradeWorkflowState::Requested)
                .storage_key(),
            "requested"
        );
        assert_eq!(TradeAgreementStatus::Requested.storage_key(), "requested");
        assert_eq!(
            TradeAgreementStatus::RevisionProposed.storage_key(),
            "revision_proposed"
        );
        assert_eq!(
            TradeAgreementStatus::AgreedPendingRhi.storage_key(),
            "agreed_pending_rhi"
        );
        assert_eq!(TradeAgreementStatus::Committed.storage_key(), "committed");
        assert_eq!(TradeAgreementStatus::Invalid.storage_key(), "invalid");
        assert_eq!(
            TradeRevisionStatus::KeptAsPlaced.storage_key(),
            "kept_as_placed"
        );
        assert_eq!(TradeInventoryStatus::Reserved.storage_key(), "reserved");
        assert_eq!(
            TradeWorkflowSource::RuntimeStore.storage_key(),
            "runtime_store"
        );

        assert_eq!(
            TradeAgreementStatus::Requested.label_key_id(),
            "messages.trade.workflow.agreement.requested"
        );
        assert_eq!(
            TradeAgreementStatus::RevisionProposed.label_key_id(),
            "messages.trade.workflow.agreement.revision_proposed"
        );
        assert_eq!(
            TradeAgreementStatus::AgreedPendingRhi.label_key_id(),
            "messages.trade.workflow.agreement.agreed_pending_rhi"
        );
        assert_eq!(
            TradeAgreementStatus::Invalid.label_key_id(),
            "messages.trade.workflow.agreement.invalid"
        );
        assert_eq!(
            TradeRevisionStatus::ChangeProposed.label_key_id(),
            "messages.trade.workflow.revision.change_proposed"
        );
        assert_eq!(
            TradeInventoryStatus::SoldOut.label_key_id(),
            "messages.trade.workflow.inventory.sold_out"
        );
        assert_eq!(
            TradeWorkflowSource::Cli.label_key_id(),
            "messages.trade.workflow.provenance.cli"
        );
    }

    #[test]
    fn orders_and_pack_day_query_state_defaults_are_frozen() {
        assert_eq!(
            OrdersScreenQueryState::default(),
            OrdersScreenQueryState {
                filter: OrdersFilter::NeedsAction,
                fulfillment_window_id: None,
            }
        );
        assert_eq!(
            PackDayScreenQueryState::default(),
            PackDayScreenQueryState {
                fulfillment_window_id: None,
            }
        );
    }

    #[test]
    fn pack_day_export_print_and_host_handoff_contracts_are_frozen_for_v1() {
        assert_eq!(
            PackDayExportArtifactKind::all_v1(),
            [
                PackDayExportArtifactKind::PackSheet,
                PackDayExportArtifactKind::PickupRoster,
                PackDayExportArtifactKind::CustomerLabels,
            ]
        );
        assert_eq!(
            PackDayExportArtifactKind::PackSheet.storage_key(),
            "pack_sheet"
        );
        assert_eq!(
            PackDayExportArtifactKind::PackSheet.file_name(),
            "pack_sheet.txt"
        );
        assert_eq!(
            PackDayExportArtifactKind::PickupRoster.file_name(),
            "pickup_roster.txt"
        );
        assert_eq!(
            PackDayExportArtifactKind::CustomerLabels.file_name(),
            "customer_labels.txt"
        );
        assert_eq!(PackDayExportStatus::default(), PackDayExportStatus::Idle);
        assert_eq!(PackDayExportStatus::Running.storage_key(), "running");
        assert_eq!(PackDayExportStatus::Succeeded.storage_key(), "succeeded");
        assert_eq!(PackDayExportStatus::Failed.storage_key(), "failed");
        assert_eq!(
            PackDayPrintKind::all_v1(),
            [
                PackDayPrintKind::PrintPackSheet,
                PackDayPrintKind::PrintPickupRoster,
                PackDayPrintKind::PrintCustomerLabels,
            ]
        );
        assert_eq!(
            PackDayPrintKind::PrintPackSheet.storage_key(),
            "print_pack_sheet"
        );
        assert_eq!(
            PackDayPrintKind::PrintPickupRoster.storage_key(),
            "print_pickup_roster"
        );
        assert_eq!(
            PackDayPrintKind::PrintCustomerLabels.storage_key(),
            "print_customer_labels"
        );
        assert_eq!(
            PackDayPrintKind::PrintPackSheet.artifact_kind(),
            PackDayExportArtifactKind::PackSheet
        );
        assert_eq!(
            PackDayPrintKind::PrintPickupRoster.artifact_kind(),
            PackDayExportArtifactKind::PickupRoster
        );
        assert_eq!(
            PackDayPrintKind::PrintCustomerLabels.artifact_kind(),
            PackDayExportArtifactKind::CustomerLabels
        );
        assert_eq!(PackDayPrintKind::PrintPackSheet.label_stock(), None);
        assert_eq!(PackDayPrintKind::PrintPickupRoster.label_stock(), None);
        assert_eq!(
            PackDayPrintKind::PrintCustomerLabels.label_stock(),
            Some(PackDayPrintLabelStock::Avery5160Letter30Up)
        );
        assert_eq!(
            PackDayPrintLabelStock::all_v1(),
            [PackDayPrintLabelStock::Avery5160Letter30Up]
        );
        assert_eq!(
            PackDayPrintLabelStock::Avery5160Letter30Up.storage_key(),
            "avery_5160_letter_30_up"
        );
        assert_eq!(
            PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow.storage_key(),
            "customer_labels_avery_5160_overflow"
        );
        assert_eq!(
            PackDayBatchPrintArtifact::all_v1(),
            [
                PackDayBatchPrintArtifact {
                    print_kind: PackDayPrintKind::PrintPackSheet,
                    artifact_kind: PackDayExportArtifactKind::PackSheet,
                    label_stock: None,
                },
                PackDayBatchPrintArtifact {
                    print_kind: PackDayPrintKind::PrintPickupRoster,
                    artifact_kind: PackDayExportArtifactKind::PickupRoster,
                    label_stock: None,
                },
                PackDayBatchPrintArtifact {
                    print_kind: PackDayPrintKind::PrintCustomerLabels,
                    artifact_kind: PackDayExportArtifactKind::CustomerLabels,
                    label_stock: Some(PackDayPrintLabelStock::Avery5160Letter30Up),
                },
            ]
        );
        assert_eq!(
            PackDayBatchPrintArtifact::from_print_kind(PackDayPrintKind::PrintCustomerLabels),
            PackDayBatchPrintArtifact {
                print_kind: PackDayPrintKind::PrintCustomerLabels,
                artifact_kind: PackDayExportArtifactKind::CustomerLabels,
                label_stock: Some(PackDayPrintLabelStock::Avery5160Letter30Up),
            }
        );
        assert_eq!(
            PackDayBatchPrintFailureKind::Preflight.storage_key(),
            "preflight"
        );
        assert_eq!(
            PackDayBatchPrintFailureKind::QueueLaunch.storage_key(),
            "queue_launch"
        );
        assert_eq!(
            PackDayBatchPrintFailureKind::QueueExit.storage_key(),
            "queue_exit"
        );
        assert_eq!(
            PackDayBatchPrintFailureKind::CustomerLabelsAvery5160Overflow.storage_key(),
            "customer_labels_avery_5160_overflow"
        );
        assert_eq!(
            PackDayBatchPrintStatus::default(),
            PackDayBatchPrintStatus::Idle
        );
        assert_eq!(PackDayBatchPrintStatus::Running.storage_key(), "running");
        assert_eq!(
            PackDayBatchPrintStatus::Succeeded.storage_key(),
            "succeeded"
        );
        assert_eq!(PackDayBatchPrintStatus::Failed.storage_key(), "failed");
        assert_eq!(PackDayPrintStatus::default(), PackDayPrintStatus::Idle);
        assert_eq!(PackDayPrintStatus::Running.storage_key(), "running");
        assert_eq!(PackDayPrintStatus::Succeeded.storage_key(), "succeeded");
        assert_eq!(PackDayPrintStatus::Failed.storage_key(), "failed");
        assert_eq!(
            PackDayHostHandoffKind::all_v1(),
            [
                PackDayHostHandoffKind::RevealBundle,
                PackDayHostHandoffKind::OpenPackSheet,
                PackDayHostHandoffKind::OpenPickupRoster,
                PackDayHostHandoffKind::OpenCustomerLabels,
            ]
        );
        assert_eq!(
            PackDayHostHandoffKind::RevealBundle.storage_key(),
            "reveal_bundle"
        );
        assert_eq!(
            PackDayHostHandoffKind::OpenPackSheet.storage_key(),
            "open_pack_sheet"
        );
        assert_eq!(
            PackDayHostHandoffKind::OpenPickupRoster.storage_key(),
            "open_pickup_roster"
        );
        assert_eq!(
            PackDayHostHandoffKind::OpenCustomerLabels.storage_key(),
            "open_customer_labels"
        );
        assert_eq!(PackDayHostHandoffKind::RevealBundle.artifact_kind(), None);
        assert_eq!(
            PackDayHostHandoffKind::OpenPackSheet.artifact_kind(),
            Some(PackDayExportArtifactKind::PackSheet)
        );
        assert_eq!(
            PackDayHostHandoffKind::OpenPickupRoster.artifact_kind(),
            Some(PackDayExportArtifactKind::PickupRoster)
        );
        assert_eq!(
            PackDayHostHandoffKind::OpenCustomerLabels.artifact_kind(),
            Some(PackDayExportArtifactKind::CustomerLabels)
        );
        assert_eq!(
            PackDayHostHandoffStatus::default(),
            PackDayHostHandoffStatus::Idle
        );
        assert_eq!(PackDayHostHandoffStatus::Running.storage_key(), "running");
        assert_eq!(
            PackDayHostHandoffStatus::Succeeded.storage_key(),
            "succeeded"
        );
        assert_eq!(PackDayHostHandoffStatus::Failed.storage_key(), "failed");
    }

    #[test]
    fn pack_day_output_order_state_freezes_the_v1_status_subset() {
        assert_eq!(
            PackDayOutputOrderState::all_v1(),
            [
                PackDayOutputOrderState::NeedsAction,
                PackDayOutputOrderState::Scheduled,
                PackDayOutputOrderState::Packed,
            ]
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::NeedsAction),
            Some(PackDayOutputOrderState::NeedsAction)
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::Scheduled),
            Some(PackDayOutputOrderState::Scheduled)
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::Packed),
            Some(PackDayOutputOrderState::Packed)
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::Completed),
            None
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::Declined),
            None
        );
        assert_eq!(
            PackDayOutputOrderState::from_order_status(OrderStatus::NeedsReview),
            None
        );
        assert_eq!(
            OrderStatus::from(PackDayOutputOrderState::Packed),
            OrderStatus::Packed
        );
    }

    #[test]
    fn pack_day_output_source_keeps_export_truth_out_of_ui_display_strings() {
        let farm_id = FarmId::generate();
        let fulfillment_window_id = FulfillmentWindowId::generate();
        let order_id = OrderId::generate();
        let screen_row = PackDayPackListRow {
            title: "Salad mix".to_owned(),
            quantity_display: "Casey: 2 bags".to_owned(),
        };
        let source = PackDayOutputSource {
            fulfillment_window: PackDayOutputWindow {
                fulfillment_window_id,
                farm_id,
                farm_display_name: "Willow farm".to_owned(),
                pickup_location_label: Some("North barn".to_owned()),
                starts_at: "2026-04-23T16:00:00Z".to_owned(),
                ends_at: "2026-04-23T19:00:00Z".to_owned(),
            },
            totals_by_product: vec![PackDayOutputProductTotal {
                title: "Salad mix".to_owned(),
                quantity: PackDayOutputQuantity::new(2, "bags"),
            }],
            pack_list: vec![PackDayOutputPackListEntry {
                order_id,
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
                order_state: PackDayOutputOrderState::Scheduled,
                title: "Salad mix".to_owned(),
                quantity: PackDayOutputQuantity::new(2, "bags"),
            }],
            pickup_roster: vec![PackDayOutputCustomerOrder {
                order_id,
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
                order_state: PackDayOutputOrderState::Scheduled,
            }],
        };

        assert_eq!(screen_row.quantity_display, "Casey: 2 bags");
        assert!(!source.is_empty());
        assert_eq!(source.pack_list[0].customer_display_name, "Casey");
        assert_eq!(source.pack_list[0].quantity.value, 2);
        assert_eq!(source.pack_list[0].quantity.unit_label, "bags");
        assert_eq!(
            source.pickup_roster[0].order_state.storage_key(),
            "scheduled"
        );
    }

    #[test]
    fn pack_day_export_bundle_tracks_output_directory_and_artifacts() {
        let fulfillment_window_id = FulfillmentWindowId::generate();
        let bundle = PackDayExportBundle {
            fulfillment_window_id,
            export_instance_id: PackDayExportInstanceId::generate(),
            generated_at_utc: "2026-04-23T15:00:00Z".to_owned(),
            bundle_directory: "exports/pack_day/window-1/20260423T150000Z".to_owned(),
            artifacts: vec![
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PackSheet,
                    relative_path: "pack_sheet.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PickupRoster,
                    relative_path: "pickup_roster.txt".to_owned(),
                },
            ],
        };

        assert_eq!(bundle.fulfillment_window_id, fulfillment_window_id);
        assert_eq!(bundle.artifact_count(), 2);
        assert!(bundle.includes_artifact(PackDayExportArtifactKind::PackSheet));
        assert!(bundle.includes_artifact(PackDayExportArtifactKind::PickupRoster));
        assert!(!bundle.includes_artifact(PackDayExportArtifactKind::CustomerLabels));
    }

    #[test]
    fn orders_and_pack_day_projections_hold_truthful_execution_data() {
        let fulfillment_window_id = super::FulfillmentWindowId::generate();
        let farm_id = FarmId::generate();
        let order_id = super::OrderId::generate();
        let order_economics = TradeEconomicsProjection {
            subtotal_minor_units: Some(1300),
            total_minor_units: Some(1300),
            currency_code: Some("USD".to_owned()),
            ..TradeEconomicsProjection::default()
        };
        let orders_list = OrdersListProjection {
            summary: OrdersListSummary {
                total_orders: 3,
                needs_action_orders: 1,
                scheduled_orders: 1,
                packed_orders: 1,
            },
            rows: vec![OrdersListRow {
                order_id,
                farm_id,
                fulfillment_window_id: Some(fulfillment_window_id),
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
                fulfillment_window_label: Some("Wednesday pickup".to_owned()),
                pickup_location_label: Some("North barn".to_owned()),
                status: OrderStatus::Scheduled,
                workflow: TradeWorkflowProjection::from_order_status(
                    order_id,
                    OrderStatus::Scheduled,
                ),
                primary_action: None,
            }],
        };
        let order_detail = OrderDetailProjection {
            order_id,
            farm_id,
            order_number: "R-1001".to_owned(),
            customer_display_name: "Casey".to_owned(),
            status: OrderStatus::Scheduled,
            fulfillment_window_id: Some(fulfillment_window_id),
            fulfillment_window_label: Some("Wednesday pickup".to_owned()),
            pickup_location_label: Some("North barn".to_owned()),
            items: vec![OrderDetailItemRow {
                title: "Salad mix".to_owned(),
                quantity_display: "2 bags".to_owned(),
                unit_price: Some(ProductPricePresentation {
                    amount_minor_units: 650,
                    currency_code: "USD".to_owned(),
                    unit_label: "bag".to_owned(),
                }),
                line_total_minor_units: Some(1300),
            }],
            economics: order_economics.clone(),
            workflow: TradeWorkflowProjection::from_order_status(order_id, OrderStatus::Scheduled)
                .with_economics(order_economics),
            validation_receipts: Vec::new(),
            primary_action: None,
        };
        let pack_day = PackDayProjection {
            fulfillment_window: Some(super::FulfillmentWindowSummary {
                fulfillment_window_id,
                farm_id,
                starts_at: "2026-04-23T16:00:00Z".to_owned(),
                ends_at: "2026-04-23T19:00:00Z".to_owned(),
            }),
            totals_by_product: vec![PackDayProductTotalRow {
                title: "Salad mix".to_owned(),
                quantity_display: "8 bags".to_owned(),
            }],
            pack_list: vec![PackDayPackListRow {
                title: "Salad mix".to_owned(),
                quantity_display: "Casey: 2 bags".to_owned(),
            }],
            pickup_roster: vec![PackDayRosterRow {
                order_id,
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
            }],
            reminders: ReminderFeedProjection::default(),
        };

        assert!(orders_list.summary.has_orders());
        assert!(!orders_list.is_empty());
        assert_eq!(orders_list.rows[0].primary_action, None);
        assert_eq!(
            orders_list.rows[0].workflow.agreement,
            TradeAgreementStatus::Committed
        );
        assert_eq!(order_detail.items[0].quantity_display, "2 bags");
        assert_eq!(
            order_detail.workflow.inventory,
            TradeInventoryStatus::Reserved
        );
        assert!(!pack_day.is_empty());
        assert_eq!(pack_day.pickup_roster[0].order_number, "R-1001");
    }

    #[test]
    fn buyer_marketplace_projections_hold_guest_capable_contract_data() {
        let farm_id = FarmId::generate();
        let product_id = super::ProductId::generate();
        let order_id = super::OrderId::generate();
        let buyer_order_economics = TradeEconomicsProjection {
            subtotal_minor_units: Some(1300),
            total_minor_units: Some(1300),
            currency_code: Some("USD".to_owned()),
            ..TradeEconomicsProjection::default()
        };
        let listing = BuyerListingRow {
            product_id,
            farm_id,
            farm_display_name: "Cedar Grove Farm".to_owned(),
            listing_relays: vec!["wss://relay.example".to_owned()],
            title: "Spring salad mix".to_owned(),
            subtitle: Some("Tender leaves".to_owned()),
            price: ProductPricePresentation {
                amount_minor_units: 650,
                currency_code: "USD".to_owned(),
                unit_label: "bag".to_owned(),
            },
            availability: ProductAvailabilitySummary {
                state: ProductAvailabilityState::Scheduled,
                label: "Thursday pickup".to_owned(),
            },
            stock: ProductStockSummary {
                quantity: Some(8),
                unit_label: Some("bag".to_owned()),
                state: ProductStockState::InStock,
            },
            fulfillment_methods: BTreeSet::from([FarmOrderMethod::Pickup]),
            next_fulfillment_window_label: Some("Thursday pickup".to_owned()),
        };
        let listings = BuyerListingsProjection {
            rows: vec![listing.clone()],
        };
        let cart = BuyerCartProjection {
            farm_id: Some(farm_id),
            farm_display_name: Some("Cedar Grove Farm".to_owned()),
            lines: vec![BuyerCartLineProjection {
                product_id,
                farm_id,
                farm_display_name: "Cedar Grove Farm".to_owned(),
                title: "Spring salad mix".to_owned(),
                quantity: 2,
                unit_price: ProductPricePresentation {
                    amount_minor_units: 650,
                    currency_code: "USD".to_owned(),
                    unit_label: "bag".to_owned(),
                },
                line_total_minor_units: 1300,
                fulfillment_summary: "Thursday pickup".to_owned(),
            }],
            subtotal_minor_units: Some(1300),
            currency_code: Some("USD".to_owned()),
            replace_confirmation: None,
        };
        let order_review = BuyerOrderReviewProjection {
            draft: BuyerOrderReviewDraft {
                name: "Casey Buyer".to_owned(),
                email: "casey@example.com".to_owned(),
                phone: String::new(),
                order_note: "Leave by the cooler".to_owned(),
                confirm_public_note: true,
            },
            summary: BuyerOrderReviewSummaryProjection {
                farm_display_name: Some("Cedar Grove Farm".to_owned()),
                fulfillment_summary: Some("Thursday pickup".to_owned()),
                line_count: 1,
                subtotal_minor_units: Some(1300),
                currency_code: Some("USD".to_owned()),
            },
            can_place_order: true,
            place_order_disabled_reason: None,
        };
        let orders = BuyerOrdersProjection {
            rows: vec![BuyerOrdersListRow {
                order_id,
                farm_id,
                order_number: "R-2001".to_owned(),
                farm_display_name: "Cedar Grove Farm".to_owned(),
                fulfillment_summary: "Thursday pickup".to_owned(),
                status: BuyerOrderStatus::Scheduled,
                workflow: TradeWorkflowProjection::from_buyer_order_status(
                    order_id,
                    BuyerOrderStatus::Scheduled,
                ),
                repeat_demand: None,
            }],
        };
        let order_detail = BuyerOrderDetailProjection {
            order_id,
            farm_id,
            order_number: "R-2001".to_owned(),
            farm_display_name: "Cedar Grove Farm".to_owned(),
            fulfillment_summary: "Thursday pickup".to_owned(),
            status: BuyerOrderStatus::Scheduled,
            items: vec![OrderDetailItemRow {
                title: "Spring salad mix".to_owned(),
                quantity_display: "2 bags".to_owned(),
                unit_price: Some(ProductPricePresentation {
                    amount_minor_units: 650,
                    currency_code: "USD".to_owned(),
                    unit_label: "bag".to_owned(),
                }),
                line_total_minor_units: Some(1300),
            }],
            economics: buyer_order_economics.clone(),
            workflow: TradeWorkflowProjection::from_buyer_order_status(
                order_id,
                BuyerOrderStatus::Scheduled,
            )
            .with_economics(buyer_order_economics),
            validation_receipts: Vec::new(),
            order_note: Some("Leave by the cooler".to_owned()),
            repeat_demand: None,
        };

        assert!(!listings.is_empty());
        assert!(!cart.is_empty());
        assert!(order_review.can_place_order);
        assert!(!orders.is_empty());
        assert_eq!(listing.fulfillment_methods.len(), 1);
        assert_eq!(order_detail.status, BuyerOrderStatus::Scheduled);
        assert_eq!(
            order_detail.workflow.agreement,
            TradeAgreementStatus::Committed
        );
    }

    #[test]
    fn today_agenda_stays_on_the_compact_order_row_contract() {
        let today = TodayAgendaProjection {
            orders_needing_action: vec![OrderListRow {
                order_id: super::OrderId::generate(),
                farm_id: FarmId::generate(),
                fulfillment_window_id: Some(super::FulfillmentWindowId::generate()),
                order_number: "R-1002".to_owned(),
                customer_display_name: "Morgan".to_owned(),
                status: OrderStatus::NeedsAction,
            }],
            ..TodayAgendaProjection::default()
        };
        let orders_row_id = super::OrderId::generate();
        let orders_row = OrdersListRow {
            order_id: orders_row_id,
            farm_id: FarmId::generate(),
            fulfillment_window_id: None,
            order_number: "R-2002".to_owned(),
            customer_display_name: "Robin".to_owned(),
            fulfillment_window_label: None,
            pickup_location_label: None,
            status: OrderStatus::Completed,
            workflow: TradeWorkflowProjection::from_order_status(
                orders_row_id,
                OrderStatus::Completed,
            ),
            primary_action: None,
        };

        assert_eq!(today.orders_needing_action.len(), 1);
        assert_eq!(
            today.orders_needing_action[0].status,
            OrderStatus::NeedsAction
        );
        assert_eq!(orders_row.primary_action, None);
        assert_eq!(orders_row.status, OrderStatus::Completed);
    }

    #[test]
    fn today_summary_attention_state_is_explicit() {
        let quiet = TodaySummary {
            farm_id: FarmId::generate(),
            orders_needing_action: 0,
            low_stock_products: 0,
            draft_products: 0,
            reminders_due_soon: 0,
        };
        let busy = TodaySummary {
            farm_id: FarmId::generate(),
            orders_needing_action: 1,
            low_stock_products: 0,
            draft_products: 0,
            reminders_due_soon: 0,
        };

        assert!(!quiet.has_attention_items());
        assert!(busy.has_attention_items());
    }

    #[test]
    fn reminder_and_repeat_demand_contracts_are_explicit() {
        let farm_id = FarmId::generate();
        let order_id = OrderId::generate();
        let fulfillment_window_id = FulfillmentWindowId::generate();
        let reminder = ReminderDeadlineProjection {
            reminder_id: ReminderId::generate(),
            farm_id,
            order_id: Some(order_id),
            fulfillment_window_id: Some(fulfillment_window_id),
            kind: ReminderKind::FulfillmentWindow,
            surface: ReminderSurface::Today,
            urgency: ReminderUrgency::DueSoon,
            title: "Pickup closes soon".to_owned(),
            detail: "Pack before the pickup window opens.".to_owned(),
            deadline_at: "2026-04-24T15:00:00Z".to_owned(),
            action_label: Some("Open pack day".to_owned()),
            delivery_state: ReminderDeliveryState::Scheduled,
        };
        let repeat_demand = RepeatDemandHandoffProjection {
            order_id,
            farm_id,
            eligibility: RepeatDemandEligibility::Partial,
            available_item_count: 2,
            unavailable_item_count: 1,
        };

        let reminder_feed = ReminderFeedProjection {
            items: vec![reminder.clone()],
        };
        let reminder_log = ReminderLogProjection {
            entries: vec![ReminderLogEntryProjection {
                reminder_id: reminder.reminder_id,
                kind: reminder.kind,
                title: reminder.title.clone(),
                recorded_at: "2026-04-24T14:00:00Z".to_owned(),
                delivery_state: ReminderDeliveryState::Presented,
                detail: Some(reminder.detail.clone()),
            }],
        };

        assert_eq!(ReminderSurface::PackDay.storage_key(), "pack_day");
        assert_eq!(ReminderUrgency::DueSoon.storage_key(), "due_soon");
        assert_eq!(
            ReminderDeliveryState::Acknowledged.storage_key(),
            "acknowledged"
        );
        assert_eq!(
            RepeatDemandEligibility::Unavailable.storage_key(),
            "unavailable"
        );
        assert_eq!(reminder_feed.due_soon_count(), 1);
        assert!(!reminder_log.is_empty());
        assert_eq!(repeat_demand.unavailable_item_count, 1);
    }

    #[test]
    fn today_agenda_projection_tracks_attention_and_setup_independently() {
        let calm = TodayAgendaProjection::default();
        let with_attention = TodayAgendaProjection {
            draft_products: vec![ProductListRow {
                product_id: super::ProductId::generate(),
                farm_id: FarmId::generate(),
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
                order_id: super::OrderId::generate(),
                farm_id: FarmId::generate(),
                fulfillment_window_id: Some(super::FulfillmentWindowId::generate()),
                order_number: "R-1001".to_owned(),
                customer_display_name: "Casey".to_owned(),
                status: super::OrderStatus::NeedsAction,
            }],
            low_stock_products: vec![ProductListRow {
                product_id: super::ProductId::generate(),
                farm_id: FarmId::generate(),
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
            farm_id: FarmId::generate(),
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
    fn farm_rules_projection_defaults_to_missing_v1_requirements() {
        let projection = FarmRulesProjection::default();

        assert!(projection.farm_profile.is_none());
        assert!(projection.pickup_locations.is_empty());
        assert!(projection.operating_rules.is_none());
        assert!(projection.fulfillment_windows.is_empty());
        assert!(projection.blackout_periods.is_empty());
        assert_eq!(
            projection.readiness,
            FarmRulesReadiness::missing_v1_basics()
        );
        assert!(!projection.is_ready());
    }

    #[test]
    fn farm_rules_readiness_and_timing_conflicts_are_explicit() {
        let readiness = FarmRulesReadiness {
            blockers: vec![FarmReadinessBlocker::MissingOperatingRules],
            timing_conflicts: vec![FarmTimingConflict {
                kind: FarmTimingConflictKind::BlackoutOverlapsFulfillmentWindow,
                fulfillment_window_id: Some(super::FulfillmentWindowId::generate()),
                blackout_period_id: Some(BlackoutPeriodId::generate()),
            }],
        };

        assert_eq!(
            FarmReadinessBlocker::MissingProfileBasics.storage_key(),
            "missing_profile_basics"
        );
        assert_eq!(
            FarmReadinessBlocker::MissingPickupLocation.storage_key(),
            "missing_pickup_location"
        );
        assert_eq!(
            FarmReadinessBlocker::MissingFulfillmentWindow.storage_key(),
            "missing_fulfillment_window"
        );
        assert_eq!(
            FarmReadinessBlocker::MissingOperatingRules.storage_key(),
            "missing_operating_rules"
        );
        assert_eq!(
            FarmTimingConflictKind::FulfillmentWindowEndsBeforeStart.storage_key(),
            "fulfillment_window_ends_before_start"
        );
        assert_eq!(
            FarmTimingConflictKind::FulfillmentWindowCutoffAfterStart.storage_key(),
            "fulfillment_window_cutoff_after_start"
        );
        assert_eq!(
            FarmTimingConflictKind::BlackoutPeriodEndsBeforeStart.storage_key(),
            "blackout_period_ends_before_start"
        );
        assert_eq!(
            FarmTimingConflictKind::BlackoutOverlapsFulfillmentWindow.storage_key(),
            "blackout_overlaps_fulfillment_window"
        );
        assert!(!readiness.is_ready());
        assert!(FarmRulesReadiness::ready().is_ready());
    }

    #[test]
    fn farm_rules_projection_represents_full_v1_inventory() {
        let farm_id = FarmId::generate();
        let pickup_location_id = PickupLocationId::generate();
        let fulfillment_window_id = super::FulfillmentWindowId::generate();
        let blackout_period_id = BlackoutPeriodId::generate();
        let projection = super::FarmRulesProjection {
            farm_profile: Some(super::FarmProfileRecord {
                farm_id,
                display_name: "North field farm".to_owned(),
                timezone: "UTC".to_owned(),
                currency_code: "USD".to_owned(),
            }),
            pickup_locations: vec![super::PickupLocationRecord {
                pickup_location_id,
                farm_id,
                label: "Barn pickup".to_owned(),
                address_line: "14 Orchard Lane".to_owned(),
                directions: Some("Drive to the red barn.".to_owned()),
                is_default: true,
            }],
            operating_rules: Some(super::FarmOperatingRulesRecord {
                farm_id,
                promise_lead_hours: 24,
                substitution_policy: "ask_customer".to_owned(),
            }),
            fulfillment_windows: vec![super::FulfillmentWindowRecord {
                fulfillment_window_id,
                farm_id,
                pickup_location_id,
                label: "Friday pickup".to_owned(),
                starts_at: "2026-04-25T14:00:00Z".to_owned(),
                ends_at: "2026-04-25T18:00:00Z".to_owned(),
                order_cutoff_at: "2026-04-24T18:00:00Z".to_owned(),
            }],
            blackout_periods: vec![super::BlackoutPeriodRecord {
                blackout_period_id,
                farm_id,
                label: "Spring break".to_owned(),
                starts_at: "2026-05-01T00:00:00Z".to_owned(),
                ends_at: "2026-05-03T23:59:59Z".to_owned(),
            }],
            readiness: FarmRulesReadiness::ready(),
        };
        let saved_farm = super::FarmSummary {
            farm_id,
            display_name: "North field farm".to_owned(),
            readiness: super::FarmReadiness::Ready,
        };

        assert!(projection.is_ready());
        assert_eq!(
            projection
                .farm_profile
                .as_ref()
                .map(|profile| profile.display_name.as_str()),
            Some(saved_farm.display_name.as_str())
        );
        assert_eq!(
            projection.pickup_locations[0].pickup_location_id,
            pickup_location_id
        );
        assert_eq!(
            projection.fulfillment_windows[0].pickup_location_id,
            pickup_location_id
        );
        assert_eq!(
            projection.blackout_periods[0].blackout_period_id,
            blackout_period_id
        );
        assert_eq!(saved_farm.readiness, super::FarmReadiness::Ready);
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
            activity_event_id: ActivityEventId::generate(),
            recorded_at: "2026-04-18T00:00:00.000Z".to_owned(),
            kind: AppActivityKind::HomeOpened,
        };
        let second = AppActivityEvent {
            activity_event_id: ActivityEventId::generate(),
            recorded_at: "2026-04-18T00:01:00.000Z".to_owned(),
            kind: AppActivityKind::SettingsOpened {
                section: SettingsSection::About,
            },
        };
        let context = AppActivityContext::from_recent_events(vec![second.clone(), first.clone()]);

        assert_eq!(context.recent_events, vec![second, first]);
    }
}
