#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, str::FromStr};
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

    pub const fn active_surface(&self) -> ActiveSurface {
        self.selected_surface.active_surface
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
        Self::default()
    }

    pub fn blocked(reason: IdentityBlockedReason) -> Self {
        Self {
            readiness: IdentityReadiness::Blocked(reason),
            ..Self::default()
        }
    }

    pub fn ready(
        mut roster: Vec<AccountSummary>,
        selected_account: SelectedAccountProjection,
    ) -> Self {
        if !roster
            .iter()
            .any(|account| account.account_id == selected_account.account.account_id)
        {
            roster.insert(0, selected_account.account.clone());
        }

        Self {
            readiness: IdentityReadiness::Ready,
            roster,
            selected_account: Some(selected_account),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Draft,
    Published,
    Paused,
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
        AccountCustody, AccountSummary, ActiveSurface, ActivityEventId, AppActivityContext,
        AppActivityEvent, AppActivityKind, AppIdentityProjection, AppStartupGate, FarmId,
        FarmerActivationProjection, FarmerSection, IdentityBlockedReason, OrderListRow,
        ProductListRow, SelectedAccountProjection, SelectedSurfaceProjection, SettingsPreference,
        SettingsSection, ShellSection, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
        TodaySummary,
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
    fn typed_ids_round_trip_through_strings() {
        let uuid = Uuid::parse_str("018f4d61-19b0-7cc4-9d4e-6d0df7c0aa11")
            .expect("test uuid should parse");
        let farm_id = FarmId::from(uuid);
        let parsed = FarmId::from_str(&farm_id.to_string()).expect("farm id should parse");

        assert_eq!(parsed, farm_id);
        assert_eq!(parsed.as_uuid(), uuid);
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
