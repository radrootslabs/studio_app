#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use radroots_studio_app_models::{
    ActiveSurface, AppIdentityProjection, AppStartupGate, BuyerCartProjection,
    BuyerCheckoutProjection, BuyerListingsProjection, BuyerOrderDetailProjection,
    BuyerOrdersProjection, BuyerProductDetailProjection, FarmOrderMethod, FarmReadiness,
    FarmReadinessBlocker, FarmRulesProjection, FarmSetupBlocker, FarmSetupProjection,
    FarmSetupReadiness, FarmTimingConflict, FulfillmentWindowId, LoggedOutStartupPhase,
    LoggedOutStartupProjection, OrderDetailProjection, OrderId, OrdersFilter, OrdersListProjection,
    OrdersScreenQueryState, PackDayExportArtifactKind, PackDayExportBundle,
    PackDayExportInstanceId, PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus,
    PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintLabelStock, PackDayPrintStatus,
    PackDayProjection, PackDayScreenQueryState, PersonalEntryProjection, ProductEditorDraft,
    ProductId, ProductPublishBlocker, ProductsFilter, ProductsListProjection, ProductsSort,
    RecoveryQueueProjection, ReminderFeedProjection, ReminderLogProjection,
    SelectedSurfaceProjection, SettingsAccountProjection, SettingsPreference, SettingsSection,
    ShellSection, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
};
use radroots_studio_app_sync::{
    AppSyncProjection, AppSyncRunStatus, SyncCheckpointState, SyncCheckpointStatus, SyncConflict,
    SyncConflictStatus,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneralSettingsProjection {
    pub allow_relay_connections: bool,
    pub use_media_servers: bool,
    pub use_nip05: bool,
    pub launch_at_login: bool,
}

impl Default for GeneralSettingsProjection {
    fn default() -> Self {
        Self {
            allow_relay_connections: true,
            use_media_servers: true,
            use_nip05: true,
            launch_at_login: false,
        }
    }
}

impl GeneralSettingsProjection {
    fn set_preference(&mut self, preference: SettingsPreference, enabled: bool) {
        match preference {
            SettingsPreference::AllowRelayConnections => {
                self.allow_relay_connections = enabled;
            }
            SettingsPreference::UseMediaServers => {
                self.use_media_servers = enabled;
            }
            SettingsPreference::UseNip05 => {
                self.use_nip05 = enabled;
            }
            SettingsPreference::LaunchAtLogin => {
                self.launch_at_login = enabled;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsShellProjection {
    pub selected_section: SettingsSection,
    pub general: GeneralSettingsProjection,
}

impl Default for SettingsShellProjection {
    fn default() -> Self {
        Self::new(SettingsSection::default())
    }
}

impl SettingsShellProjection {
    pub fn new(selected_section: SettingsSection) -> Self {
        Self {
            selected_section,
            general: GeneralSettingsProjection::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BuyerSearchScreenQueryState {
    pub search_query: String,
    pub fulfillment_methods: BTreeSet<FarmOrderMethod>,
}

impl BuyerSearchScreenQueryState {
    pub fn new(
        search_query: impl Into<String>,
        fulfillment_methods: impl IntoIterator<Item = FarmOrderMethod>,
    ) -> Self {
        Self {
            search_query: search_query.into(),
            fulfillment_methods: fulfillment_methods.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuyerBrowseScreenProjection {
    pub listings: BuyerListingsProjection,
    pub detail: Option<BuyerProductDetailProjection>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuyerSearchScreenProjection {
    pub query: BuyerSearchScreenQueryState,
    pub listings: BuyerListingsProjection,
    pub detail: Option<BuyerProductDetailProjection>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuyerCartScreenProjection {
    pub cart: BuyerCartProjection,
    pub checkout: BuyerCheckoutProjection,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuyerOrdersScreenProjection {
    pub list: BuyerOrdersProjection,
    pub detail: Option<BuyerOrderDetailProjection>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PersonalWorkspaceProjection {
    pub entry: PersonalEntryProjection,
    pub browse: BuyerBrowseScreenProjection,
    pub search: BuyerSearchScreenProjection,
    pub cart: BuyerCartScreenProjection,
    pub orders: BuyerOrdersScreenProjection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductsScreenQueryState {
    pub search_query: String,
    pub filter: ProductsFilter,
    pub sort: ProductsSort,
}

impl Default for ProductsScreenQueryState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            filter: ProductsFilter::default(),
            sort: ProductsSort::default(),
        }
    }
}

impl ProductsScreenQueryState {
    pub fn new(
        search_query: impl Into<String>,
        filter: ProductsFilter,
        sort: ProductsSort,
    ) -> Self {
        Self {
            search_query: search_query.into(),
            filter,
            sort,
        }
    }

    fn set_search_query(&mut self, search_query: impl Into<String>) {
        self.search_query = search_query.into();
    }

    fn select_filter(&mut self, filter: ProductsFilter) {
        self.filter = filter;
    }

    fn select_sort(&mut self, sort: ProductsSort) {
        self.sort = sort;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductEditorSession {
    pub selected_product_id: Option<ProductId>,
    pub draft: ProductEditorDraft,
    pub publish_blockers: Vec<ProductPublishBlocker>,
}

impl ProductEditorSession {
    fn new_draft(farm_readiness: &FarmWorkspaceReadinessProjection) -> Self {
        Self::from_selection(None, ProductEditorDraft::default(), farm_readiness)
    }

    fn existing(
        product_id: ProductId,
        draft: ProductEditorDraft,
        farm_readiness: &FarmWorkspaceReadinessProjection,
    ) -> Self {
        Self::from_selection(Some(product_id), draft, farm_readiness)
    }

    fn from_selection(
        selected_product_id: Option<ProductId>,
        draft: ProductEditorDraft,
        farm_readiness: &FarmWorkspaceReadinessProjection,
    ) -> Self {
        let publish_blockers = derive_product_publish_blockers(&draft, farm_readiness);

        Self {
            selected_product_id,
            draft,
            publish_blockers,
        }
    }

    fn replace_draft(
        &mut self,
        draft: ProductEditorDraft,
        farm_readiness: &FarmWorkspaceReadinessProjection,
    ) {
        self.publish_blockers = derive_product_publish_blockers(&draft, farm_readiness);
        self.draft = draft;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductEditorState {
    Closed,
    Open(ProductEditorSession),
}

impl Default for ProductEditorState {
    fn default() -> Self {
        Self::Closed
    }
}

impl ProductEditorState {
    fn open_new_draft(&mut self, farm_readiness: &FarmWorkspaceReadinessProjection) {
        *self = Self::Open(ProductEditorSession::new_draft(farm_readiness));
    }

    fn open_existing(
        &mut self,
        product_id: ProductId,
        draft: ProductEditorDraft,
        farm_readiness: &FarmWorkspaceReadinessProjection,
    ) {
        *self = Self::Open(ProductEditorSession::existing(
            product_id,
            draft,
            farm_readiness,
        ));
    }

    fn replace_draft(
        &mut self,
        draft: ProductEditorDraft,
        farm_readiness: &FarmWorkspaceReadinessProjection,
    ) {
        if let Self::Open(session) = self {
            session.replace_draft(draft, farm_readiness);
        }
    }

    fn close(&mut self) {
        *self = Self::Closed;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductsScreenProjection {
    pub list: ProductsListProjection,
    pub query: ProductsScreenQueryState,
    pub editor: ProductEditorState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrdersScreenProjection {
    pub list: OrdersListProjection,
    pub query: OrdersScreenQueryState,
    pub reminders: ReminderFeedProjection,
    pub recovery_queue: RecoveryQueueProjection,
    pub detail: Option<OrderDetailProjection>,
}

impl OrdersScreenProjection {
    fn select_filter(&mut self, filter: OrdersFilter) {
        self.query.filter = filter;
        self.detail = None;
    }

    fn select_fulfillment_window(&mut self, fulfillment_window_id: Option<FulfillmentWindowId>) {
        self.query.fulfillment_window_id = fulfillment_window_id;
        self.detail = None;
    }

    fn replace_detail(&mut self, detail: Option<OrderDetailProjection>) {
        self.detail = detail;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackDayScreenProjection {
    pub query: PackDayScreenQueryState,
    pub projection: PackDayProjection,
    pub export: PackDayExportProjection,
    pub print: PackDayPrintProjection,
    pub host_handoff: PackDayHostHandoffProjection,
}

impl PackDayScreenProjection {
    fn select_fulfillment_window(&mut self, fulfillment_window_id: Option<FulfillmentWindowId>) {
        if self.query.fulfillment_window_id != fulfillment_window_id {
            self.export = PackDayExportProjection::default();
            self.print = PackDayPrintProjection::default();
            self.host_handoff = PackDayHostHandoffProjection::default();
        }
        self.query.fulfillment_window_id = fulfillment_window_id;
    }

    fn replace_projection(&mut self, projection: PackDayProjection) {
        let previous_window_id = self
            .projection
            .fulfillment_window
            .as_ref()
            .map(|window| window.fulfillment_window_id);
        let next_window_id = projection
            .fulfillment_window
            .as_ref()
            .map(|window| window.fulfillment_window_id);

        if previous_window_id != next_window_id {
            self.export = PackDayExportProjection::default();
            self.print = PackDayPrintProjection::default();
            self.host_handoff = PackDayHostHandoffProjection::default();
        }

        self.projection = projection;
    }

    fn replace_export(&mut self, export: PackDayExportProjection) {
        if self.export != export {
            self.print = PackDayPrintProjection::default();
            self.host_handoff = PackDayHostHandoffProjection::default();
        }
        self.export = export;
    }

    fn replace_print(&mut self, print: PackDayPrintProjection) {
        self.print = print;
    }

    fn replace_host_handoff(&mut self, host_handoff: PackDayHostHandoffProjection) {
        self.host_handoff = host_handoff;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayExportRequest {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub artifact_kinds: Vec<PackDayExportArtifactKind>,
}

impl PackDayExportRequest {
    pub fn for_fulfillment_window(fulfillment_window_id: FulfillmentWindowId) -> Self {
        Self {
            fulfillment_window_id,
            artifact_kinds: Vec::from(PackDayExportArtifactKind::all_v1()),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackDayExportProjection {
    pub status: PackDayExportStatus,
    pub request: Option<PackDayExportRequest>,
    pub bundle: Option<PackDayExportBundle>,
    pub error_message: Option<String>,
}

impl PackDayExportProjection {
    pub fn running(request: PackDayExportRequest) -> Self {
        Self {
            status: PackDayExportStatus::Running,
            request: Some(request),
            bundle: None,
            error_message: None,
        }
    }

    pub fn succeeded(request: PackDayExportRequest, bundle: PackDayExportBundle) -> Self {
        Self {
            status: PackDayExportStatus::Succeeded,
            request: Some(request),
            bundle: Some(bundle),
            error_message: None,
        }
    }

    pub fn failed(request: PackDayExportRequest, message: impl Into<String>) -> Self {
        Self {
            status: PackDayExportStatus::Failed,
            request: Some(request),
            bundle: None,
            error_message: Some(message.into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayPrintRequest {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub export_instance_id: PackDayExportInstanceId,
    pub kind: PackDayPrintKind,
    pub label_stock: Option<PackDayPrintLabelStock>,
}

impl PackDayPrintRequest {
    pub fn for_bundle(kind: PackDayPrintKind, bundle: &PackDayExportBundle) -> Self {
        Self {
            fulfillment_window_id: bundle.fulfillment_window_id,
            export_instance_id: bundle.export_instance_id,
            kind,
            label_stock: kind.label_stock(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackDayPrintProjection {
    pub status: PackDayPrintStatus,
    pub request: Option<PackDayPrintRequest>,
    pub failure: Option<PackDayPrintFailureKind>,
}

impl PackDayPrintProjection {
    pub fn running(request: PackDayPrintRequest) -> Self {
        Self {
            status: PackDayPrintStatus::Running,
            request: Some(request),
            failure: None,
        }
    }

    pub fn succeeded(request: PackDayPrintRequest) -> Self {
        Self {
            status: PackDayPrintStatus::Succeeded,
            request: Some(request),
            failure: None,
        }
    }

    pub fn failed(request: PackDayPrintRequest) -> Self {
        Self::failed_with_failure(request, None)
    }

    pub fn failed_with_kind(
        request: PackDayPrintRequest,
        failure: PackDayPrintFailureKind,
    ) -> Self {
        Self::failed_with_failure(request, Some(failure))
    }

    fn failed_with_failure(
        request: PackDayPrintRequest,
        failure: Option<PackDayPrintFailureKind>,
    ) -> Self {
        Self {
            status: PackDayPrintStatus::Failed,
            request: Some(request),
            failure,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayHostHandoffRequest {
    pub fulfillment_window_id: FulfillmentWindowId,
    pub kind: PackDayHostHandoffKind,
    pub bundle_directory: String,
}

impl PackDayHostHandoffRequest {
    pub fn for_bundle(kind: PackDayHostHandoffKind, bundle: &PackDayExportBundle) -> Self {
        Self {
            fulfillment_window_id: bundle.fulfillment_window_id,
            kind,
            bundle_directory: bundle.bundle_directory.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackDayHostHandoffProjection {
    pub status: PackDayHostHandoffStatus,
    pub request: Option<PackDayHostHandoffRequest>,
    pub error_message: Option<String>,
}

impl PackDayHostHandoffProjection {
    pub fn running(request: PackDayHostHandoffRequest) -> Self {
        Self {
            status: PackDayHostHandoffStatus::Running,
            request: Some(request),
            error_message: None,
        }
    }

    pub fn succeeded(request: PackDayHostHandoffRequest) -> Self {
        Self {
            status: PackDayHostHandoffStatus::Succeeded,
            request: Some(request),
            error_message: None,
        }
    }

    pub fn failed(request: PackDayHostHandoffRequest, message: impl Into<String>) -> Self {
        Self {
            status: PackDayHostHandoffStatus::Failed,
            request: Some(request),
            error_message: Some(message.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FarmSetupFlowStage {
    #[default]
    Onboarding,
    Editing,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FarmWorkspaceStatus {
    #[default]
    NoFarm,
    SetupRequired,
    Ready,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FarmWorkspaceReadinessProjection {
    pub has_saved_farm: bool,
    pub status: FarmWorkspaceStatus,
    pub setup_blockers: Vec<FarmSetupBlocker>,
    pub rules_blockers: Vec<FarmReadinessBlocker>,
    pub timing_conflicts: Vec<FarmTimingConflict>,
}

impl FarmWorkspaceReadinessProjection {
    pub const fn needs_setup(&self) -> bool {
        matches!(self.status, FarmWorkspaceStatus::SetupRequired)
    }

    pub fn coarse_readiness(&self) -> Option<FarmReadiness> {
        self.has_saved_farm.then_some(if self.needs_setup() {
            FarmReadiness::Incomplete
        } else {
            FarmReadiness::Ready
        })
    }

    fn has_rules_blocker(&self, blocker: FarmReadinessBlocker) -> bool {
        self.rules_blockers.contains(&blocker)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HomeRoute {
    Blocked,
    SetupRequired,
    Personal,
    FarmSetupOnboarding,
    FarmSetupForm,
    Today,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppShellProjection {
    pub active_surface: ActiveSurface,
    pub selected_section: ShellSection,
    pub settings: SettingsShellProjection,
}

impl Default for AppShellProjection {
    fn default() -> Self {
        Self::new(ActiveSurface::Personal, ShellSection::Home)
    }
}

impl AppShellProjection {
    pub fn new(active_surface: ActiveSurface, selected_section: ShellSection) -> Self {
        let settings = match selected_section {
            ShellSection::Settings(section) => SettingsShellProjection::new(section),
            _ => SettingsShellProjection::default(),
        };

        Self {
            active_surface: selected_section.surface().unwrap_or(active_surface),
            selected_section,
            settings,
        }
    }

    pub fn for_surface(active_surface: ActiveSurface) -> Self {
        Self::new(
            active_surface,
            ShellSection::default_for_surface(active_surface),
        )
    }

    pub fn for_settings(active_surface: ActiveSurface, selected_section: SettingsSection) -> Self {
        Self::new(active_surface, ShellSection::Settings(selected_section))
    }

    fn select_section(&mut self, selected_section: ShellSection) {
        if let Some(active_surface) = selected_section.surface() {
            self.active_surface = active_surface;
        }
        self.selected_section = selected_section;

        if let ShellSection::Settings(settings_section) = selected_section {
            self.settings.selected_section = settings_section;
        }
    }

    fn select_active_surface(&mut self, active_surface: ActiveSurface) {
        if self.active_surface == active_surface {
            return;
        }

        self.active_surface = active_surface;
        match active_surface {
            ActiveSurface::Personal => {
                if matches!(self.selected_section, ShellSection::Farmer(_)) {
                    self.selected_section = ShellSection::default_for_surface(active_surface);
                }
            }
            ActiveSurface::Farmer => {
                if matches!(
                    self.selected_section,
                    ShellSection::Home | ShellSection::Personal(_)
                ) {
                    self.selected_section = ShellSection::default_for_surface(active_surface);
                }
            }
        }
    }

    fn select_settings_section(&mut self, selected_section: SettingsSection) {
        self.settings.selected_section = selected_section;

        if matches!(self.selected_section, ShellSection::Settings(_)) {
            self.selected_section = ShellSection::Settings(selected_section);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppProjection {
    pub shell: AppShellProjection,
    pub identity: AppIdentityProjection,
    pub startup_gate: AppStartupGate,
    pub sync: AppSyncProjection,
    pub logged_out_startup: LoggedOutStartupProjection,
    pub personal: PersonalWorkspaceProjection,
    pub today: TodayAgendaProjection,
    pub products: ProductsScreenProjection,
    pub orders: OrdersScreenProjection,
    pub pack_day: PackDayScreenProjection,
    pub reminder_log: ReminderLogProjection,
    pub farm_setup: FarmSetupProjection,
    pub farm_rules: FarmRulesProjection,
    pub farm_readiness: FarmWorkspaceReadinessProjection,
    pub farm_setup_flow_stage: FarmSetupFlowStage,
}

impl AppProjection {
    pub fn new(
        shell: AppShellProjection,
        identity: AppIdentityProjection,
        today: TodayAgendaProjection,
    ) -> Self {
        Self::with_farm_setup(shell, identity, today, FarmSetupProjection::default())
    }

    pub fn with_farm_setup(
        shell: AppShellProjection,
        identity: AppIdentityProjection,
        today: TodayAgendaProjection,
        farm_setup: FarmSetupProjection,
    ) -> Self {
        let mut projection = Self {
            shell,
            identity,
            startup_gate: AppStartupGate::default(),
            sync: AppSyncProjection::default(),
            logged_out_startup: LoggedOutStartupProjection::default(),
            personal: PersonalWorkspaceProjection::default(),
            today,
            products: ProductsScreenProjection::default(),
            orders: OrdersScreenProjection::default(),
            pack_day: PackDayScreenProjection::default(),
            reminder_log: ReminderLogProjection::default(),
            farm_setup,
            farm_rules: FarmRulesProjection::default(),
            farm_readiness: FarmWorkspaceReadinessProjection::default(),
            farm_setup_flow_stage: FarmSetupFlowStage::default(),
        };
        sync_projection(&mut projection);

        projection
    }

    pub fn home_route(&self) -> HomeRoute {
        match self.startup_gate {
            AppStartupGate::Blocked => HomeRoute::Blocked,
            AppStartupGate::SetupRequired => HomeRoute::SetupRequired,
            AppStartupGate::Personal => HomeRoute::Personal,
            AppStartupGate::Farmer if self.farm_setup.has_saved_farm() => HomeRoute::Today,
            AppStartupGate::Farmer
                if self.farm_setup.readiness == FarmSetupReadiness::NotStarted
                    && self.farm_setup_flow_stage == FarmSetupFlowStage::Onboarding =>
            {
                HomeRoute::FarmSetupOnboarding
            }
            AppStartupGate::Farmer => HomeRoute::FarmSetupForm,
        }
    }
}

impl Default for AppProjection {
    fn default() -> Self {
        Self::new(
            AppShellProjection::default(),
            AppIdentityProjection::default(),
            TodayAgendaProjection::default(),
        )
    }
}

pub const APP_STATE_FILE_NAME: &str = "state.json";
const APP_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedShellProjection {
    pub selected_section: ShellSection,
    pub settings_section: SettingsSection,
}

impl Default for PersistedShellProjection {
    fn default() -> Self {
        Self {
            selected_section: ShellSection::Home,
            settings_section: SettingsSection::default(),
        }
    }
}

impl PersistedShellProjection {
    fn from_shell(shell: &AppShellProjection) -> Self {
        Self {
            selected_section: shell.selected_section,
            settings_section: shell.settings.selected_section,
        }
    }

    fn to_shell_projection(&self) -> AppShellProjection {
        let mut shell = AppShellProjection::new(ActiveSurface::Personal, self.selected_section);
        shell.settings.selected_section = self.settings_section;
        if matches!(shell.selected_section, ShellSection::Settings(_)) {
            shell.selected_section = ShellSection::Settings(self.settings_section);
        }

        shell
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedBuyerProjection {
    pub search_query: BuyerSearchScreenQueryState,
    pub browse_detail_product_id: Option<ProductId>,
    pub search_detail_product_id: Option<ProductId>,
    pub orders_detail_order_id: Option<OrderId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedSellerProjection {
    pub products_query: ProductsScreenQueryState,
    pub product_editor_product_id: Option<ProductId>,
    pub orders_query: OrdersScreenQueryState,
    pub order_detail_order_id: Option<OrderId>,
    pub pack_day_query: PackDayScreenQueryState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PersistedAppState {
    pub shell: PersistedShellProjection,
    pub logged_out_startup: LoggedOutStartupProjection,
    pub buyer: PersistedBuyerProjection,
    pub seller: PersistedSellerProjection,
}

impl PersistedAppState {
    pub fn from_projection(projection: &AppProjection) -> Self {
        Self {
            shell: PersistedShellProjection::from_shell(&projection.shell),
            logged_out_startup: projection.logged_out_startup.clone(),
            buyer: PersistedBuyerProjection {
                search_query: projection.personal.search.query.clone(),
                browse_detail_product_id: projection
                    .personal
                    .browse
                    .detail
                    .as_ref()
                    .map(|detail| detail.listing.product_id),
                search_detail_product_id: projection
                    .personal
                    .search
                    .detail
                    .as_ref()
                    .map(|detail| detail.listing.product_id),
                orders_detail_order_id: projection
                    .personal
                    .orders
                    .detail
                    .as_ref()
                    .map(|detail| detail.order_id),
            },
            seller: PersistedSellerProjection {
                products_query: projection.products.query.clone(),
                product_editor_product_id: match &projection.products.editor {
                    ProductEditorState::Open(session) => session.selected_product_id,
                    ProductEditorState::Closed => None,
                },
                orders_query: projection.orders.query.clone(),
                order_detail_order_id: projection
                    .orders
                    .detail
                    .as_ref()
                    .map(|detail| detail.order_id),
                pack_day_query: projection.pack_day.query.clone(),
            },
        }
    }

    fn sanitized_for_restart(&self) -> Self {
        let mut state = self.clone();

        if state.logged_out_startup.phase == LoggedOutStartupPhase::GenerateKeyStarting {
            state.logged_out_startup.phase = LoggedOutStartupPhase::IdentityChoice;
        }

        state
    }

    fn to_projection(&self) -> AppProjection {
        let mut projection = AppProjection {
            shell: self.shell.to_shell_projection(),
            identity: AppIdentityProjection::default(),
            startup_gate: AppStartupGate::SetupRequired,
            sync: AppSyncProjection::default(),
            logged_out_startup: self.logged_out_startup.clone(),
            personal: PersonalWorkspaceProjection {
                entry: AppIdentityProjection::default().personal_entry(),
                search: BuyerSearchScreenProjection {
                    query: self.buyer.search_query.clone(),
                    ..BuyerSearchScreenProjection::default()
                },
                ..PersonalWorkspaceProjection::default()
            },
            today: TodayAgendaProjection::default(),
            products: ProductsScreenProjection {
                query: self.seller.products_query.clone(),
                ..ProductsScreenProjection::default()
            },
            orders: OrdersScreenProjection {
                query: self.seller.orders_query.clone(),
                ..OrdersScreenProjection::default()
            },
            pack_day: PackDayScreenProjection {
                query: self.seller.pack_day_query.clone(),
                ..PackDayScreenProjection::default()
            },
            reminder_log: ReminderLogProjection::default(),
            farm_setup: FarmSetupProjection::default(),
            farm_rules: FarmRulesProjection::default(),
            farm_readiness: FarmWorkspaceReadinessProjection::default(),
            farm_setup_flow_stage: FarmSetupFlowStage::default(),
        };
        sync_farm_setup_to_today(&mut projection.farm_setup, &projection.today);
        projection.farm_readiness =
            derive_farm_workspace_readiness(&projection.farm_setup, &projection.farm_rules);
        sync_coarse_farm_readiness(
            &mut projection.farm_setup,
            &mut projection.today,
            &projection.farm_readiness,
        );
        projection.today.setup_checklist =
            derive_today_setup_checklist(&projection.farm_readiness, &projection.products.list);
        sync_product_editor_publish_blockers(
            &mut projection.products.editor,
            &projection.farm_readiness,
        );
        projection.startup_gate = projection.identity.startup_gate();
        projection.personal.entry = projection.identity.personal_entry();
        sync_logged_out_startup(&mut projection.logged_out_startup, projection.startup_gate);
        sync_farm_setup_flow_stage(
            &mut projection.farm_setup_flow_stage,
            projection.startup_gate,
            projection.farm_setup.has_saved_farm(),
        );

        projection
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PersistedAppStateEnvelope {
    version: u32,
    state: PersistedAppState,
}

impl PersistedAppStateEnvelope {
    fn new(state: PersistedAppState) -> Self {
        Self {
            version: APP_STATE_SCHEMA_VERSION,
            state,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStateCommand {
    SelectActiveSurface(ActiveSurface),
    SelectSection(ShellSection),
    SelectSettingsSection(SettingsSection),
    ShowStartupIdentityChoice,
    BeginGenerateKeyStartup,
    ShowStartupSignerEntry,
    SetStartupSignerSourceInput(String),
    ResetLoggedOutStartup,
    ReplaceIdentityProjection(AppIdentityProjection),
    ReplaceSyncProjection(AppSyncProjection),
    ReplacePersonalProjection(PersonalWorkspaceProjection),
    ReplaceFarmSetupProjection(FarmSetupProjection),
    ReplaceFarmRulesProjection(FarmRulesProjection),
    SelectFarmSetupFlowStage(FarmSetupFlowStage),
    SetSettingsPreference {
        preference: SettingsPreference,
        enabled: bool,
    },
    ReplaceTodayAgenda(TodayAgendaProjection),
    SetProductsSearchQuery(String),
    SelectProductsFilter(ProductsFilter),
    SelectProductsSort(ProductsSort),
    ReplaceProductsList(ProductsListProjection),
    SelectOrdersFilter(OrdersFilter),
    SelectOrdersFulfillmentWindow(Option<FulfillmentWindowId>),
    ReplaceOrdersList(OrdersListProjection),
    ReplaceOrdersReminders(ReminderFeedProjection),
    ReplaceOrdersRecoveryQueue(RecoveryQueueProjection),
    ReplaceReminderLog(ReminderLogProjection),
    ReplaceOrderDetail(Option<OrderDetailProjection>),
    SetPackDayFulfillmentWindow(Option<FulfillmentWindowId>),
    ReplacePackDayProjection(PackDayProjection),
    BeginPackDayExport(PackDayExportRequest),
    SucceedPackDayExport {
        request: PackDayExportRequest,
        bundle: PackDayExportBundle,
    },
    FailPackDayExport {
        request: PackDayExportRequest,
        message: String,
    },
    ResetPackDayExport,
    BeginPackDayPrint(PackDayPrintRequest),
    SucceedPackDayPrint(PackDayPrintRequest),
    FailPackDayPrint(PackDayPrintRequest),
    FailPackDayPrintWithKind {
        request: PackDayPrintRequest,
        failure: PackDayPrintFailureKind,
    },
    ResetPackDayPrint,
    BeginPackDayHostHandoff(PackDayHostHandoffRequest),
    SucceedPackDayHostHandoff(PackDayHostHandoffRequest),
    FailPackDayHostHandoff {
        request: PackDayHostHandoffRequest,
        message: String,
    },
    ResetPackDayHostHandoff,
    OpenNewProductEditor,
    OpenExistingProductEditor {
        product_id: ProductId,
        draft: ProductEditorDraft,
    },
    ReplaceProductEditorDraft(ProductEditorDraft),
    CloseProductEditor,
}

impl AppStateCommand {
    pub const fn select_active_surface(surface: ActiveSurface) -> Self {
        Self::SelectActiveSurface(surface)
    }

    pub const fn select_settings_section(section: SettingsSection) -> Self {
        Self::SelectSettingsSection(section)
    }

    pub const fn show_startup_identity_choice() -> Self {
        Self::ShowStartupIdentityChoice
    }

    pub const fn begin_generate_key_startup() -> Self {
        Self::BeginGenerateKeyStartup
    }

    pub const fn show_startup_signer_entry() -> Self {
        Self::ShowStartupSignerEntry
    }

    pub fn set_startup_signer_source_input(source_input: impl Into<String>) -> Self {
        Self::SetStartupSignerSourceInput(source_input.into())
    }

    pub const fn reset_logged_out_startup() -> Self {
        Self::ResetLoggedOutStartup
    }

    pub fn replace_identity_projection(projection: AppIdentityProjection) -> Self {
        Self::ReplaceIdentityProjection(projection)
    }

    pub fn replace_sync_projection(projection: AppSyncProjection) -> Self {
        Self::ReplaceSyncProjection(projection)
    }

    pub fn replace_personal_projection(projection: PersonalWorkspaceProjection) -> Self {
        Self::ReplacePersonalProjection(projection)
    }

    pub fn replace_farm_setup_projection(projection: FarmSetupProjection) -> Self {
        Self::ReplaceFarmSetupProjection(projection)
    }

    pub fn replace_farm_rules_projection(projection: FarmRulesProjection) -> Self {
        Self::ReplaceFarmRulesProjection(projection)
    }

    pub const fn select_farm_setup_flow_stage(stage: FarmSetupFlowStage) -> Self {
        Self::SelectFarmSetupFlowStage(stage)
    }

    pub fn replace_today_agenda(projection: TodayAgendaProjection) -> Self {
        Self::ReplaceTodayAgenda(projection)
    }

    pub fn set_products_search_query(search_query: impl Into<String>) -> Self {
        Self::SetProductsSearchQuery(search_query.into())
    }

    pub const fn select_products_filter(filter: ProductsFilter) -> Self {
        Self::SelectProductsFilter(filter)
    }

    pub const fn select_products_sort(sort: ProductsSort) -> Self {
        Self::SelectProductsSort(sort)
    }

    pub fn replace_products_list(projection: ProductsListProjection) -> Self {
        Self::ReplaceProductsList(projection)
    }

    pub const fn select_orders_filter(filter: OrdersFilter) -> Self {
        Self::SelectOrdersFilter(filter)
    }

    pub fn select_orders_fulfillment_window(
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Self {
        Self::SelectOrdersFulfillmentWindow(fulfillment_window_id)
    }

    pub fn replace_orders_list(projection: OrdersListProjection) -> Self {
        Self::ReplaceOrdersList(projection)
    }

    pub fn replace_orders_reminders(projection: ReminderFeedProjection) -> Self {
        Self::ReplaceOrdersReminders(projection)
    }

    pub fn replace_orders_recovery_queue(projection: RecoveryQueueProjection) -> Self {
        Self::ReplaceOrdersRecoveryQueue(projection)
    }

    pub fn replace_reminder_log(projection: ReminderLogProjection) -> Self {
        Self::ReplaceReminderLog(projection)
    }

    pub fn replace_order_detail(projection: Option<OrderDetailProjection>) -> Self {
        Self::ReplaceOrderDetail(projection)
    }

    pub fn set_pack_day_fulfillment_window(
        fulfillment_window_id: Option<FulfillmentWindowId>,
    ) -> Self {
        Self::SetPackDayFulfillmentWindow(fulfillment_window_id)
    }

    pub fn replace_pack_day_projection(projection: PackDayProjection) -> Self {
        Self::ReplacePackDayProjection(projection)
    }

    pub fn begin_pack_day_export(request: PackDayExportRequest) -> Self {
        Self::BeginPackDayExport(request)
    }

    pub fn succeed_pack_day_export(
        request: PackDayExportRequest,
        bundle: PackDayExportBundle,
    ) -> Self {
        Self::SucceedPackDayExport { request, bundle }
    }

    pub fn fail_pack_day_export(request: PackDayExportRequest, message: impl Into<String>) -> Self {
        Self::FailPackDayExport {
            request,
            message: message.into(),
        }
    }

    pub const fn reset_pack_day_export() -> Self {
        Self::ResetPackDayExport
    }

    pub fn begin_pack_day_print(request: PackDayPrintRequest) -> Self {
        Self::BeginPackDayPrint(request)
    }

    pub fn succeed_pack_day_print(request: PackDayPrintRequest) -> Self {
        Self::SucceedPackDayPrint(request)
    }

    pub fn fail_pack_day_print(request: PackDayPrintRequest) -> Self {
        Self::FailPackDayPrint(request)
    }

    pub fn fail_pack_day_print_with_kind(
        request: PackDayPrintRequest,
        failure: PackDayPrintFailureKind,
    ) -> Self {
        Self::FailPackDayPrintWithKind { request, failure }
    }

    pub const fn reset_pack_day_print() -> Self {
        Self::ResetPackDayPrint
    }

    pub fn begin_pack_day_host_handoff(request: PackDayHostHandoffRequest) -> Self {
        Self::BeginPackDayHostHandoff(request)
    }

    pub fn succeed_pack_day_host_handoff(request: PackDayHostHandoffRequest) -> Self {
        Self::SucceedPackDayHostHandoff(request)
    }

    pub fn fail_pack_day_host_handoff(
        request: PackDayHostHandoffRequest,
        message: impl Into<String>,
    ) -> Self {
        Self::FailPackDayHostHandoff {
            request,
            message: message.into(),
        }
    }

    pub const fn reset_pack_day_host_handoff() -> Self {
        Self::ResetPackDayHostHandoff
    }

    pub const fn open_new_product_editor() -> Self {
        Self::OpenNewProductEditor
    }

    pub fn open_existing_product_editor(product_id: ProductId, draft: ProductEditorDraft) -> Self {
        Self::OpenExistingProductEditor { product_id, draft }
    }

    pub fn replace_product_editor_draft(draft: ProductEditorDraft) -> Self {
        Self::ReplaceProductEditorDraft(draft)
    }

    pub const fn close_product_editor() -> Self {
        Self::CloseProductEditor
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AppStateRepositoryError {
    #[error("app state repository load failed: {message}")]
    Load { message: String },
    #[error("app state repository save failed: {message}")]
    Save { message: String },
}

impl AppStateRepositoryError {
    pub fn load(message: impl Into<String>) -> Self {
        Self::Load {
            message: message.into(),
        }
    }

    pub fn save(message: impl Into<String>) -> Self {
        Self::Save {
            message: message.into(),
        }
    }
}

pub trait AppStateRepository {
    fn load_persisted_state(&self) -> Result<PersistedAppState, AppStateRepositoryError>;

    fn save_persisted_state(
        &mut self,
        state: &PersistedAppState,
    ) -> Result<(), AppStateRepositoryError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InMemoryAppStateRepository {
    state: PersistedAppState,
}

impl Default for InMemoryAppStateRepository {
    fn default() -> Self {
        Self::new(AppShellProjection::default())
    }
}

impl InMemoryAppStateRepository {
    pub fn new(projection: AppShellProjection) -> Self {
        let state = PersistedAppState {
            shell: PersistedShellProjection::from_shell(&projection),
            ..PersistedAppState::default()
        };

        Self { state }
    }

    pub fn from_persisted_state(state: PersistedAppState) -> Self {
        Self { state }
    }

    pub fn projection(&self) -> AppShellProjection {
        self.state.shell.to_shell_projection()
    }

    pub fn persisted_state(&self) -> &PersistedAppState {
        &self.state
    }

    pub fn overwrite(&mut self, state: PersistedAppState) {
        self.state = state;
    }
}

impl AppStateRepository for InMemoryAppStateRepository {
    fn load_persisted_state(&self) -> Result<PersistedAppState, AppStateRepositoryError> {
        Ok(self.state.clone())
    }

    fn save_persisted_state(
        &mut self,
        state: &PersistedAppState,
    ) -> Result<(), AppStateRepositoryError> {
        self.state = state.clone();
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileBackedAppStateRepository {
    path: PathBuf,
}

impl FileBackedAppStateRepository {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn write_state(&self, state: &PersistedAppState) -> Result<(), AppStateRepositoryError> {
        let Some(parent) = self.path.parent() else {
            return Err(AppStateRepositoryError::save(
                "app state path must have a parent directory",
            ));
        };
        fs::create_dir_all(parent)
            .map_err(|error| AppStateRepositoryError::save(error.to_string()))?;
        let payload = serde_json::to_vec_pretty(&PersistedAppStateEnvelope::new(state.clone()))
            .map_err(|error| AppStateRepositoryError::save(error.to_string()))?;
        let temporary_path = self.path.with_extension("tmp");
        let _ = fs::remove_file(&temporary_path);
        fs::write(&temporary_path, payload)
            .map_err(|error| AppStateRepositoryError::save(error.to_string()))?;
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|error| AppStateRepositoryError::save(error.to_string()))?;
        }
        fs::rename(&temporary_path, &self.path)
            .map_err(|error| AppStateRepositoryError::save(error.to_string()))
    }
}

impl AppStateRepository for FileBackedAppStateRepository {
    fn load_persisted_state(&self) -> Result<PersistedAppState, AppStateRepositoryError> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(PersistedAppState::default());
            }
            Err(error) => {
                return Err(AppStateRepositoryError::load(error.to_string()));
            }
        };

        let envelope = match serde_json::from_str::<PersistedAppStateEnvelope>(&contents) {
            Ok(envelope) if envelope.version == APP_STATE_SCHEMA_VERSION => envelope,
            Ok(_) | Err(_) => {
                let default_state = PersistedAppState::default();
                self.write_state(&default_state)?;
                return Ok(default_state);
            }
        };

        let sanitized = envelope.state.sanitized_for_restart();
        if sanitized != envelope.state {
            self.write_state(&sanitized)?;
        }

        Ok(sanitized)
    }

    fn save_persisted_state(
        &mut self,
        state: &PersistedAppState,
    ) -> Result<(), AppStateRepositoryError> {
        self.write_state(state)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStatePersistenceRepository {
    InMemory(InMemoryAppStateRepository),
    FileBacked(FileBackedAppStateRepository),
}

impl AppStatePersistenceRepository {
    pub fn in_memory() -> Self {
        Self::InMemory(InMemoryAppStateRepository::default())
    }

    pub fn file_backed(path: impl Into<PathBuf>) -> Self {
        Self::FileBacked(FileBackedAppStateRepository::new(path))
    }
}

impl AppStateRepository for AppStatePersistenceRepository {
    fn load_persisted_state(&self) -> Result<PersistedAppState, AppStateRepositoryError> {
        match self {
            Self::InMemory(repository) => repository.load_persisted_state(),
            Self::FileBacked(repository) => repository.load_persisted_state(),
        }
    }

    fn save_persisted_state(
        &mut self,
        state: &PersistedAppState,
    ) -> Result<(), AppStateRepositoryError> {
        match self {
            Self::InMemory(repository) => repository.save_persisted_state(state),
            Self::FileBacked(repository) => repository.save_persisted_state(state),
        }
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AppStateStoreError {
    #[error(transparent)]
    Repository(#[from] AppStateRepositoryError),
}

#[derive(Clone, Debug)]
pub struct AppStateStore<R> {
    repository: R,
    projection: AppProjection,
    persisted_state: PersistedAppState,
}

impl<R: AppStateRepository> AppStateStore<R> {
    pub fn load(repository: R) -> Result<Self, AppStateStoreError> {
        let persisted_state = repository.load_persisted_state()?;
        let projection = persisted_state.to_projection();

        Ok(Self {
            repository,
            projection,
            persisted_state,
        })
    }

    pub fn projection(&self) -> &AppProjection {
        &self.projection
    }

    pub fn shell_projection(&self) -> &AppShellProjection {
        &self.projection.shell
    }

    pub fn today_projection(&self) -> &TodayAgendaProjection {
        &self.projection.today
    }

    pub fn identity_projection(&self) -> &AppIdentityProjection {
        &self.projection.identity
    }

    pub fn farm_setup_projection(&self) -> &FarmSetupProjection {
        &self.projection.farm_setup
    }

    pub fn farm_rules_projection(&self) -> &FarmRulesProjection {
        &self.projection.farm_rules
    }

    pub fn farm_readiness_projection(&self) -> &FarmWorkspaceReadinessProjection {
        &self.projection.farm_readiness
    }

    pub fn logged_out_startup_projection(&self) -> &LoggedOutStartupProjection {
        &self.projection.logged_out_startup
    }

    pub fn personal_projection(&self) -> &PersonalWorkspaceProjection {
        &self.projection.personal
    }

    pub fn products_projection(&self) -> &ProductsScreenProjection {
        &self.projection.products
    }

    pub fn orders_projection(&self) -> &OrdersScreenProjection {
        &self.projection.orders
    }

    pub fn reminder_log_projection(&self) -> &ReminderLogProjection {
        &self.projection.reminder_log
    }

    pub fn pack_day_projection(&self) -> &PackDayScreenProjection {
        &self.projection.pack_day
    }

    pub fn home_route(&self) -> HomeRoute {
        self.projection.home_route()
    }

    pub fn settings_account_projection(&self) -> SettingsAccountProjection {
        self.projection.identity.settings_account()
    }

    pub fn startup_gate(&self) -> AppStartupGate {
        self.projection.startup_gate
    }

    pub fn sync_projection(&self) -> &AppSyncProjection {
        &self.projection.sync
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn persisted_state(&self) -> &PersistedAppState {
        &self.persisted_state
    }

    pub fn apply(&mut self, command: AppStateCommand) -> Result<bool, AppStateStoreError> {
        let mut next_projection = self.projection.clone();
        if matches!(
            apply_command(&mut next_projection, command),
            AppStateMutation::NoChange
        ) {
            return Ok(false);
        }

        let next_persisted_state = PersistedAppState::from_projection(&next_projection);
        if next_persisted_state != self.persisted_state {
            self.repository
                .save_persisted_state(&next_persisted_state)?;
        }
        self.persisted_state = next_persisted_state;
        self.projection = next_projection;

        Ok(true)
    }

    pub fn apply_in_memory(&mut self, command: AppStateCommand) -> bool {
        match self.apply(command) {
            Ok(changed) => changed,
            Err(error) => {
                error!(target: "app_state", error = %error, "failed to persist app state");
                false
            }
        }
    }
}

impl AppStateStore<InMemoryAppStateRepository> {
    pub fn in_memory(projection: AppShellProjection) -> Self {
        let repository = InMemoryAppStateRepository::new(projection.clone());
        let persisted_state = repository.persisted_state().clone();
        Self {
            repository,
            projection: persisted_state.to_projection(),
            persisted_state,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppStateMutation {
    NoChange,
    ShellChanged,
    FarmSetupChanged,
    StartupChanged,
    SyncChanged,
    PersonalChanged,
    TodayChanged,
    ProductsChanged,
    OrdersChanged,
    PackDayChanged,
}

fn apply_command(projection: &mut AppProjection, command: AppStateCommand) -> AppStateMutation {
    let before = projection.clone();

    match command {
        AppStateCommand::SelectActiveSurface(active_surface) => {
            projection.shell.select_active_surface(active_surface);
            if let Some(selected_account) = projection.identity.selected_account.as_mut() {
                let selected_surface = if selected_account.farmer_activation.is_active() {
                    active_surface
                } else {
                    ActiveSurface::Personal
                };
                selected_account.selected_surface =
                    SelectedSurfaceProjection::new(selected_surface);
            }
        }
        AppStateCommand::SelectSection(selected_section) => {
            projection.shell.select_section(selected_section);
        }
        AppStateCommand::SelectSettingsSection(selected_section) => {
            projection.shell.select_settings_section(selected_section);
        }
        AppStateCommand::ShowStartupIdentityChoice => {
            if projection.startup_gate == AppStartupGate::SetupRequired {
                projection.logged_out_startup.phase = LoggedOutStartupPhase::IdentityChoice;
            }
        }
        AppStateCommand::BeginGenerateKeyStartup => {
            if projection.startup_gate == AppStartupGate::SetupRequired {
                projection.logged_out_startup.phase = LoggedOutStartupPhase::GenerateKeyStarting;
            }
        }
        AppStateCommand::ShowStartupSignerEntry => {
            if projection.startup_gate == AppStartupGate::SetupRequired {
                projection.logged_out_startup.phase = LoggedOutStartupPhase::SignerEntry;
            }
        }
        AppStateCommand::SetStartupSignerSourceInput(source_input) => {
            if projection.startup_gate == AppStartupGate::SetupRequired {
                projection
                    .logged_out_startup
                    .signer_entry
                    .set_source_input(source_input);
            }
        }
        AppStateCommand::ResetLoggedOutStartup => {
            projection.logged_out_startup = LoggedOutStartupProjection::default();
        }
        AppStateCommand::ReplaceIdentityProjection(identity_projection) => {
            projection.identity = identity_projection;
        }
        AppStateCommand::ReplaceSyncProjection(sync_projection) => {
            projection.sync = sync_projection;
        }
        AppStateCommand::ReplacePersonalProjection(personal_projection) => {
            projection.personal = personal_projection;
        }
        AppStateCommand::ReplaceFarmSetupProjection(farm_setup_projection) => {
            projection.farm_setup = farm_setup_projection;
        }
        AppStateCommand::ReplaceFarmRulesProjection(farm_rules_projection) => {
            projection.farm_rules = farm_rules_projection;
        }
        AppStateCommand::SelectFarmSetupFlowStage(flow_stage) => {
            projection.farm_setup_flow_stage = flow_stage;
        }
        AppStateCommand::SetSettingsPreference {
            preference,
            enabled,
        } => {
            projection
                .shell
                .settings
                .general
                .set_preference(preference, enabled);
        }
        AppStateCommand::ReplaceTodayAgenda(today_projection) => {
            projection.today = today_projection;
        }
        AppStateCommand::SetProductsSearchQuery(search_query) => {
            projection.products.query.set_search_query(search_query);
        }
        AppStateCommand::SelectProductsFilter(filter) => {
            projection.products.query.select_filter(filter);
        }
        AppStateCommand::SelectProductsSort(sort) => {
            projection.products.query.select_sort(sort);
        }
        AppStateCommand::ReplaceProductsList(products_projection) => {
            projection.products.list = products_projection;
        }
        AppStateCommand::SelectOrdersFilter(filter) => {
            projection.orders.select_filter(filter);
        }
        AppStateCommand::SelectOrdersFulfillmentWindow(fulfillment_window_id) => {
            projection
                .orders
                .select_fulfillment_window(fulfillment_window_id);
        }
        AppStateCommand::ReplaceOrdersList(orders_projection) => {
            projection.orders.list = orders_projection;
        }
        AppStateCommand::ReplaceOrdersReminders(reminders_projection) => {
            projection.orders.reminders = reminders_projection;
        }
        AppStateCommand::ReplaceOrdersRecoveryQueue(recovery_queue_projection) => {
            projection.orders.recovery_queue = recovery_queue_projection;
        }
        AppStateCommand::ReplaceReminderLog(reminder_log_projection) => {
            projection.reminder_log = reminder_log_projection;
        }
        AppStateCommand::ReplaceOrderDetail(order_detail_projection) => {
            projection.orders.replace_detail(order_detail_projection);
        }
        AppStateCommand::SetPackDayFulfillmentWindow(fulfillment_window_id) => {
            projection
                .pack_day
                .select_fulfillment_window(fulfillment_window_id);
        }
        AppStateCommand::ReplacePackDayProjection(pack_day_projection) => {
            projection.pack_day.replace_projection(pack_day_projection);
        }
        AppStateCommand::BeginPackDayExport(request) => {
            projection
                .pack_day
                .replace_export(PackDayExportProjection::running(request));
        }
        AppStateCommand::SucceedPackDayExport { request, bundle } => {
            projection
                .pack_day
                .replace_export(PackDayExportProjection::succeeded(request, bundle));
        }
        AppStateCommand::FailPackDayExport { request, message } => {
            projection
                .pack_day
                .replace_export(PackDayExportProjection::failed(request, message));
        }
        AppStateCommand::ResetPackDayExport => {
            projection
                .pack_day
                .replace_export(PackDayExportProjection::default());
        }
        AppStateCommand::BeginPackDayPrint(request) => {
            projection
                .pack_day
                .replace_print(PackDayPrintProjection::running(request));
        }
        AppStateCommand::SucceedPackDayPrint(request) => {
            projection
                .pack_day
                .replace_print(PackDayPrintProjection::succeeded(request));
        }
        AppStateCommand::FailPackDayPrint(request) => {
            projection
                .pack_day
                .replace_print(PackDayPrintProjection::failed(request));
        }
        AppStateCommand::FailPackDayPrintWithKind { request, failure } => {
            projection
                .pack_day
                .replace_print(PackDayPrintProjection::failed_with_kind(request, failure));
        }
        AppStateCommand::ResetPackDayPrint => {
            projection
                .pack_day
                .replace_print(PackDayPrintProjection::default());
        }
        AppStateCommand::BeginPackDayHostHandoff(request) => {
            projection
                .pack_day
                .replace_host_handoff(PackDayHostHandoffProjection::running(request));
        }
        AppStateCommand::SucceedPackDayHostHandoff(request) => {
            projection
                .pack_day
                .replace_host_handoff(PackDayHostHandoffProjection::succeeded(request));
        }
        AppStateCommand::FailPackDayHostHandoff { request, message } => {
            projection
                .pack_day
                .replace_host_handoff(PackDayHostHandoffProjection::failed(request, message));
        }
        AppStateCommand::ResetPackDayHostHandoff => {
            projection
                .pack_day
                .replace_host_handoff(PackDayHostHandoffProjection::default());
        }
        AppStateCommand::OpenNewProductEditor => {
            projection
                .products
                .editor
                .open_new_draft(&projection.farm_readiness);
        }
        AppStateCommand::OpenExistingProductEditor { product_id, draft } => {
            projection
                .products
                .editor
                .open_existing(product_id, draft, &projection.farm_readiness);
        }
        AppStateCommand::ReplaceProductEditorDraft(draft) => {
            projection
                .products
                .editor
                .replace_draft(draft, &projection.farm_readiness);
        }
        AppStateCommand::CloseProductEditor => {
            projection.products.editor.close();
        }
    }

    sync_projection(projection);

    if *projection == before {
        AppStateMutation::NoChange
    } else if projection.shell != before.shell {
        AppStateMutation::ShellChanged
    } else if projection.farm_setup != before.farm_setup
        || projection.farm_rules != before.farm_rules
        || projection.farm_readiness != before.farm_readiness
        || projection.farm_setup_flow_stage != before.farm_setup_flow_stage
    {
        AppStateMutation::FarmSetupChanged
    } else if projection.logged_out_startup != before.logged_out_startup {
        AppStateMutation::StartupChanged
    } else if projection.sync != before.sync {
        AppStateMutation::SyncChanged
    } else if projection.personal != before.personal {
        AppStateMutation::PersonalChanged
    } else if projection.products != before.products {
        AppStateMutation::ProductsChanged
    } else if projection.orders != before.orders {
        AppStateMutation::OrdersChanged
    } else if projection.reminder_log != before.reminder_log {
        AppStateMutation::OrdersChanged
    } else if projection.pack_day != before.pack_day {
        AppStateMutation::PackDayChanged
    } else {
        AppStateMutation::TodayChanged
    }
}

fn sync_projection(projection: &mut AppProjection) {
    sync_shell_to_identity(&mut projection.shell, &projection.identity);
    sync_farm_setup_to_today(&mut projection.farm_setup, &projection.today);
    projection.farm_readiness =
        derive_farm_workspace_readiness(&projection.farm_setup, &projection.farm_rules);
    sync_coarse_farm_readiness(
        &mut projection.farm_setup,
        &mut projection.today,
        &projection.farm_readiness,
    );
    projection.today.setup_checklist =
        derive_today_setup_checklist(&projection.farm_readiness, &projection.products.list);
    sync_product_editor_publish_blockers(
        &mut projection.products.editor,
        &projection.farm_readiness,
    );
    projection.startup_gate = projection.identity.startup_gate();
    projection.personal.entry = projection.identity.personal_entry();
    sync_logged_out_startup(&mut projection.logged_out_startup, projection.startup_gate);
    sync_farm_setup_flow_stage(
        &mut projection.farm_setup_flow_stage,
        projection.startup_gate,
        projection.farm_setup.has_saved_farm(),
    );
}

fn sync_shell_to_identity(shell: &mut AppShellProjection, identity: &AppIdentityProjection) {
    match identity.startup_gate() {
        AppStartupGate::Blocked | AppStartupGate::SetupRequired => {
            shell.active_surface = ActiveSurface::Personal;
            if matches!(shell.selected_section, ShellSection::Farmer(_)) {
                shell.selected_section = ShellSection::Home;
            }
        }
        AppStartupGate::Personal => {
            shell.active_surface = ActiveSurface::Personal;
            if matches!(shell.selected_section, ShellSection::Farmer(_)) {
                shell.selected_section = ShellSection::default_for_surface(ActiveSurface::Personal);
            }
        }
        AppStartupGate::Farmer => {
            shell.active_surface = ActiveSurface::Farmer;
            if matches!(
                shell.selected_section,
                ShellSection::Home | ShellSection::Personal(_)
            ) {
                shell.selected_section = ShellSection::default_for_surface(ActiveSurface::Farmer);
            }
        }
    }
}

fn sync_farm_setup_to_today(farm_setup: &mut FarmSetupProjection, today: &TodayAgendaProjection) {
    if let Some(saved_farm) = today.farm.clone()
        && !farm_setup.has_saved_farm()
    {
        *farm_setup = FarmSetupProjection::from_saved_farm(saved_farm);
    }
}

fn sync_farm_setup_flow_stage(
    flow_stage: &mut FarmSetupFlowStage,
    startup_gate: AppStartupGate,
    has_saved_farm: bool,
) {
    if startup_gate != AppStartupGate::Farmer || has_saved_farm {
        *flow_stage = FarmSetupFlowStage::Onboarding;
    }
}

fn sync_logged_out_startup(
    logged_out_startup: &mut LoggedOutStartupProjection,
    startup_gate: AppStartupGate,
) {
    if startup_gate != AppStartupGate::SetupRequired {
        *logged_out_startup = LoggedOutStartupProjection::default();
    }
}

pub fn derive_farm_workspace_readiness(
    farm_setup: &FarmSetupProjection,
    farm_rules: &FarmRulesProjection,
) -> FarmWorkspaceReadinessProjection {
    if !farm_setup.has_saved_farm() {
        return FarmWorkspaceReadinessProjection {
            has_saved_farm: false,
            status: if farm_setup.readiness == FarmSetupReadiness::NotStarted {
                FarmWorkspaceStatus::NoFarm
            } else {
                FarmWorkspaceStatus::SetupRequired
            },
            setup_blockers: farm_setup.blockers.clone(),
            rules_blockers: Vec::new(),
            timing_conflicts: Vec::new(),
        };
    }

    let status = if farm_rules.is_ready() {
        FarmWorkspaceStatus::Ready
    } else {
        FarmWorkspaceStatus::SetupRequired
    };

    FarmWorkspaceReadinessProjection {
        has_saved_farm: true,
        status,
        setup_blockers: Vec::new(),
        rules_blockers: farm_rules.readiness.blockers.clone(),
        timing_conflicts: farm_rules.readiness.timing_conflicts.clone(),
    }
}

pub fn derive_today_setup_checklist(
    farm_readiness: &FarmWorkspaceReadinessProjection,
    products: &ProductsListProjection,
) -> Vec<TodaySetupTask> {
    if !farm_readiness.has_saved_farm {
        return Vec::new();
    }

    vec![
        TodaySetupTask {
            kind: TodaySetupTaskKind::CompleteFarmProfile,
            is_complete: !farm_readiness
                .has_rules_blocker(FarmReadinessBlocker::MissingProfileBasics),
        },
        TodaySetupTask {
            kind: TodaySetupTaskKind::AddPickupLocation,
            is_complete: !farm_readiness
                .has_rules_blocker(FarmReadinessBlocker::MissingPickupLocation),
        },
        TodaySetupTask {
            kind: TodaySetupTaskKind::AddOperatingRules,
            is_complete: !farm_readiness
                .has_rules_blocker(FarmReadinessBlocker::MissingOperatingRules),
        },
        TodaySetupTask {
            kind: TodaySetupTaskKind::AddFulfillmentWindow,
            is_complete: !farm_readiness
                .has_rules_blocker(FarmReadinessBlocker::MissingFulfillmentWindow),
        },
        TodaySetupTask {
            kind: TodaySetupTaskKind::ResolveAvailabilityConflicts,
            is_complete: farm_readiness.timing_conflicts.is_empty(),
        },
        TodaySetupTask {
            kind: TodaySetupTaskKind::PublishProduct,
            is_complete: products.summary.live_products > 0,
        },
    ]
}

pub fn derive_product_publish_blockers(
    draft: &ProductEditorDraft,
    farm_readiness: &FarmWorkspaceReadinessProjection,
) -> Vec<ProductPublishBlocker> {
    let mut blockers = draft.publish_blockers();

    if farm_readiness.has_saved_farm {
        replace_availability_blocker(&mut blockers, farm_readiness);

        if farm_readiness.has_rules_blocker(FarmReadinessBlocker::MissingProfileBasics) {
            push_unique_product_blocker(&mut blockers, ProductPublishBlocker::CompleteFarmProfile);
        }

        if farm_readiness.has_rules_blocker(FarmReadinessBlocker::MissingPickupLocation) {
            push_unique_product_blocker(&mut blockers, ProductPublishBlocker::AddPickupLocation);
        }

        if farm_readiness.has_rules_blocker(FarmReadinessBlocker::MissingOperatingRules) {
            push_unique_product_blocker(&mut blockers, ProductPublishBlocker::AddOperatingRules);
        }

        if farm_readiness.has_rules_blocker(FarmReadinessBlocker::MissingFulfillmentWindow) {
            push_unique_product_blocker(&mut blockers, ProductPublishBlocker::AddFulfillmentWindow);
        }

        if !farm_readiness.timing_conflicts.is_empty() {
            push_unique_product_blocker(
                &mut blockers,
                ProductPublishBlocker::ResolveAvailabilityConflicts,
            );
        }
    }

    blockers
}

fn sync_coarse_farm_readiness(
    farm_setup: &mut FarmSetupProjection,
    today: &mut TodayAgendaProjection,
    farm_readiness: &FarmWorkspaceReadinessProjection,
) {
    let Some(coarse_readiness) = farm_readiness.coarse_readiness() else {
        return;
    };

    if let Some(saved_farm) = farm_setup.saved_farm.as_mut() {
        saved_farm.readiness = coarse_readiness;
    }

    if let Some(saved_farm) = today.farm.as_mut() {
        saved_farm.readiness = coarse_readiness;
    }
}

fn sync_product_editor_publish_blockers(
    editor: &mut ProductEditorState,
    farm_readiness: &FarmWorkspaceReadinessProjection,
) {
    if let ProductEditorState::Open(session) = editor {
        session.publish_blockers = derive_product_publish_blockers(&session.draft, farm_readiness);
    }
}

fn replace_availability_blocker(
    blockers: &mut [ProductPublishBlocker],
    farm_readiness: &FarmWorkspaceReadinessProjection,
) {
    for blocker in blockers.iter_mut() {
        if *blocker != ProductPublishBlocker::AttachAvailability {
            continue;
        }

        *blocker = if !farm_readiness.timing_conflicts.is_empty() {
            ProductPublishBlocker::ResolveAvailabilityConflicts
        } else if farm_readiness.has_rules_blocker(FarmReadinessBlocker::MissingFulfillmentWindow) {
            ProductPublishBlocker::AddFulfillmentWindow
        } else {
            ProductPublishBlocker::AttachAvailability
        };
    }
}

fn push_unique_product_blocker(
    blockers: &mut Vec<ProductPublishBlocker>,
    blocker: ProductPublishBlocker,
) {
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

pub fn derive_sync_projection(
    checkpoint: &SyncCheckpointStatus,
    conflicts: &[SyncConflict],
) -> AppSyncProjection {
    let conflict_status = SyncConflictStatus::from_conflicts(conflicts);

    AppSyncProjection {
        run_status: derive_sync_run_status(checkpoint, &conflict_status),
        checkpoint: checkpoint.clone(),
        conflict_status,
    }
}

pub fn derive_sync_run_status(
    checkpoint: &SyncCheckpointStatus,
    conflict_status: &SyncConflictStatus,
) -> AppSyncRunStatus {
    if checkpoint.is_syncing() {
        AppSyncRunStatus::Syncing
    } else if checkpoint.is_failed() {
        AppSyncRunStatus::Failed
    } else if conflict_status.requires_attention() {
        AppSyncRunStatus::Conflicted
    } else if checkpoint.state == SyncCheckpointState::Current {
        AppSyncRunStatus::Succeeded
    } else {
        AppSyncRunStatus::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppProjection, AppShellProjection, AppStateCommand, AppStateRepository,
        AppStateRepositoryError, AppStateStore, AppStateStoreError, FarmSetupFlowStage, HomeRoute,
        InMemoryAppStateRepository, OrdersScreenProjection, PackDayExportProjection,
        PackDayExportRequest, PackDayHostHandoffProjection, PackDayHostHandoffRequest,
        PackDayPrintProjection, PackDayPrintRequest, PackDayScreenProjection, PersistedAppState,
        ProductEditorState, ProductsScreenProjection, ProductsScreenQueryState, SettingsPreference,
        derive_sync_projection, derive_sync_run_status,
    };
    use radroots_studio_app_models::{
        AccountCustody, AccountSummary, ActiveSurface, AppIdentityProjection, AppStartupGate,
        FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection,
        FarmerActivationProjection, FarmerSection, FulfillmentWindowId, LoggedOutStartupPhase,
        LoggedOutStartupProjection, OrderDetailItemRow, OrderDetailProjection, OrderId,
        OrderPrimaryAction, OrderStatus, OrdersFilter, OrdersListProjection, OrdersListRow,
        OrdersListSummary, OrdersScreenQueryState, PackDayExportArtifact,
        PackDayExportArtifactKind, PackDayExportBundle, PackDayExportInstanceId,
        PackDayExportStatus, PackDayHostHandoffKind, PackDayHostHandoffStatus, PackDayPackListRow,
        PackDayPrintFailureKind, PackDayPrintKind, PackDayPrintLabelStock, PackDayPrintStatus,
        PackDayProductTotalRow, PackDayProjection, PackDayRosterRow, PackDayScreenQueryState,
        PersonalEntryState, PersonalSection, ProductEditorDraft, ProductId, ProductPublishBlocker,
        ProductsFilter, ProductsListProjection, ProductsSort, ReminderDeliveryState,
        ReminderFeedProjection, ReminderKind, ReminderLogEntryProjection, ReminderLogProjection,
        SelectedAccountProjection, SelectedSurfaceProjection, SettingsSection, ShellSection,
        TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
    };
    use radroots_studio_app_sync::{
        AppSyncProjection, AppSyncRunStatus, SyncCheckpointState, SyncCheckpointStatus,
        SyncConflict, SyncConflictKind, SyncConflictResolutionStatus, SyncConflictSeverity,
        SyncConflictStatus,
    };

    struct FailingRepository;

    impl AppStateRepository for FailingRepository {
        fn load_persisted_state(&self) -> Result<PersistedAppState, AppStateRepositoryError> {
            Ok(PersistedAppState::default())
        }

        fn save_persisted_state(
            &mut self,
            _: &PersistedAppState,
        ) -> Result<(), AppStateRepositoryError> {
            Err(AppStateRepositoryError::save("disk unavailable"))
        }
    }

    fn ready_identity(surface: ActiveSurface) -> AppIdentityProjection {
        AppIdentityProjection::ready(
            Vec::new(),
            SelectedAccountProjection::new(
                AccountSummary {
                    account_id: "acct_surface".to_owned(),
                    npub: "npub1surface".to_owned(),
                    label: Some("North field".to_owned()),
                    custody: AccountCustody::LocalManaged,
                },
                SelectedSurfaceProjection::new(surface),
                FarmerActivationProjection::active(FarmId::new()),
            ),
        )
    }

    fn sample_pack_day_export_request(
        fulfillment_window_id: FulfillmentWindowId,
    ) -> PackDayExportRequest {
        PackDayExportRequest::for_fulfillment_window(fulfillment_window_id)
    }

    fn sample_pack_day_host_handoff_request(
        fulfillment_window_id: FulfillmentWindowId,
        kind: PackDayHostHandoffKind,
    ) -> PackDayHostHandoffRequest {
        let bundle = sample_pack_day_export_bundle(fulfillment_window_id);
        PackDayHostHandoffRequest::for_bundle(kind, &bundle)
    }

    fn sample_pack_day_print_request(
        fulfillment_window_id: FulfillmentWindowId,
        kind: PackDayPrintKind,
    ) -> PackDayPrintRequest {
        let bundle = sample_pack_day_export_bundle(fulfillment_window_id);
        PackDayPrintRequest::for_bundle(kind, &bundle)
    }

    fn sample_pack_day_export_bundle(
        fulfillment_window_id: FulfillmentWindowId,
    ) -> PackDayExportBundle {
        PackDayExportBundle {
            fulfillment_window_id,
            export_instance_id: PackDayExportInstanceId::new(),
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
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::CustomerLabels,
                    relative_path: "customer_labels.txt".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn default_projection_starts_on_personal_setup_gate() {
        let projection = AppProjection::default();

        assert_eq!(projection.shell.active_surface, ActiveSurface::Personal);
        assert_eq!(projection.shell.selected_section, ShellSection::Home);
        assert_eq!(projection.identity, AppIdentityProjection::default());
        assert_eq!(projection.startup_gate, AppStartupGate::SetupRequired);
        assert_eq!(projection.sync, AppSyncProjection::default());
        assert_eq!(
            projection.logged_out_startup,
            LoggedOutStartupProjection::default()
        );
        assert_eq!(
            projection.shell.settings.selected_section,
            SettingsSection::Account
        );
        assert!(projection.shell.settings.general.allow_relay_connections);
        assert!(projection.shell.settings.general.use_media_servers);
        assert!(projection.shell.settings.general.use_nip05);
        assert!(!projection.shell.settings.general.launch_at_login);
        assert_eq!(projection.today, TodayAgendaProjection::default());
        assert_eq!(projection.products, ProductsScreenProjection::default());
        assert_eq!(projection.orders, OrdersScreenProjection::default());
        assert_eq!(projection.pack_day, PackDayScreenProjection::default());
        assert_eq!(projection.personal.entry.state, PersonalEntryState::Guest);
        assert_eq!(projection.farm_setup, FarmSetupProjection::default());
        assert_eq!(
            projection.farm_setup_flow_stage,
            FarmSetupFlowStage::Onboarding
        );
        assert_eq!(projection.home_route(), HomeRoute::SetupRequired);
    }

    #[test]
    fn load_uses_repository_projection() {
        let repository = InMemoryAppStateRepository::new(AppShellProjection::for_settings(
            ActiveSurface::Farmer,
            SettingsSection::About,
        ));
        let store = AppStateStore::load(repository).expect("in-memory repository should load");

        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Personal
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Settings(SettingsSection::About)
        );
        assert_eq!(
            store.projection().shell.settings.selected_section,
            SettingsSection::About
        );
        assert_eq!(store.startup_gate(), AppStartupGate::SetupRequired);
        assert_eq!(store.sync_projection(), &AppSyncProjection::default());
        assert_eq!(
            store.logged_out_startup_projection(),
            &LoggedOutStartupProjection::default()
        );
        assert_eq!(store.projection().today, TodayAgendaProjection::default());
        assert_eq!(
            store.projection().products,
            ProductsScreenProjection::default()
        );
        assert_eq!(store.projection().orders, OrdersScreenProjection::default());
        assert_eq!(
            store.projection().pack_day,
            PackDayScreenProjection::default()
        );
        assert_eq!(
            store.personal_projection().entry.state,
            PersonalEntryState::Guest
        );
        assert_eq!(store.home_route(), HomeRoute::SetupRequired);
    }

    #[test]
    fn products_query_defaults_and_refreshes_are_local_app_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let products_list = ProductsListProjection {
            summary: radroots_studio_app_models::ProductsListSummary {
                total_products: 2,
                live_products: 1,
                draft_products: 1,
                need_attention_products: 1,
            },
            rows: Vec::new(),
        };

        assert_eq!(
            store.projection().products.query,
            ProductsScreenQueryState::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::set_products_search_query("pea")),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::select_products_filter(
                ProductsFilter::NeedAttention,
            )),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::select_products_sort(ProductsSort::Name)),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_products_list(
                products_list.clone()
            )),
            Ok(true)
        );
        assert_eq!(
            store.projection().products.query,
            ProductsScreenQueryState::new("pea", ProductsFilter::NeedAttention, ProductsSort::Name)
        );
        assert_eq!(store.projection().products.list, products_list);
        assert_eq!(
            store.repository().projection(),
            AppShellProjection::default()
        );
        assert_eq!(
            store.repository().persisted_state().seller.products_query,
            ProductsScreenQueryState::new("pea", ProductsFilter::NeedAttention, ProductsSort::Name)
        );
    }

    #[test]
    fn orders_and_pack_day_queries_refresh_as_local_app_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let farm_id = FarmId::new();
        let fulfillment_window_id = FulfillmentWindowId::new();
        let order_id = OrderId::new();
        let orders_list = OrdersListProjection {
            summary: OrdersListSummary {
                total_orders: 2,
                needs_action_orders: 1,
                scheduled_orders: 1,
                packed_orders: 0,
            },
            rows: vec![OrdersListRow {
                order_id,
                farm_id,
                fulfillment_window_id: Some(fulfillment_window_id),
                order_number: "R-100".to_owned(),
                customer_display_name: "Casey".to_owned(),
                fulfillment_window_label: Some("Friday pickup".to_owned()),
                pickup_location_label: Some("North barn".to_owned()),
                status: OrderStatus::NeedsAction,
                primary_action: Some(OrderPrimaryAction::Review),
            }],
        };
        let order_detail = OrderDetailProjection {
            order_id,
            farm_id,
            order_number: "R-100".to_owned(),
            customer_display_name: "Casey".to_owned(),
            status: OrderStatus::NeedsAction,
            fulfillment_window_id: Some(fulfillment_window_id),
            fulfillment_window_label: Some("Friday pickup".to_owned()),
            pickup_location_label: Some("North barn".to_owned()),
            items: vec![OrderDetailItemRow {
                title: "Salad mix".to_owned(),
                quantity_display: "2 bags".to_owned(),
            }],
            primary_action: Some(OrderPrimaryAction::Review),
            recoveries: Vec::new(),
        };
        let orders_reminders = ReminderFeedProjection {
            items: vec![radroots_studio_app_models::ReminderDeadlineProjection {
                reminder_id: radroots_studio_app_models::ReminderId::new(),
                farm_id,
                order_id: Some(order_id),
                fulfillment_window_id: Some(fulfillment_window_id),
                kind: radroots_studio_app_models::ReminderKind::OrderAction,
                surface: radroots_studio_app_models::ReminderSurface::Orders,
                urgency: radroots_studio_app_models::ReminderUrgency::DueSoon,
                title: "review order".to_owned(),
                detail: "Casey still needs confirmation.".to_owned(),
                deadline_at: "2026-04-18T15:00:00Z".to_owned(),
                action_label: Some("Review".to_owned()),
                delivery_state: radroots_studio_app_models::ReminderDeliveryState::Scheduled,
            }],
        };
        let recovery_queue = radroots_studio_app_models::RecoveryQueueProjection {
            items: vec![radroots_studio_app_models::OrderRecoveryProjection {
                recovery_record_id: radroots_studio_app_models::RecoveryRecordId::new(),
                order_id,
                kind: radroots_studio_app_models::RecoveryKind::MissedPickup,
                state: radroots_studio_app_models::RecoveryState::Open,
                summary: "Follow up on pickup".to_owned(),
                note: None,
                last_updated_at: "2026-04-18T19:00:00Z".to_owned(),
            }],
        };
        let reminder_log = ReminderLogProjection {
            entries: vec![ReminderLogEntryProjection {
                reminder_id: orders_reminders.items[0].reminder_id,
                kind: ReminderKind::OrderAction,
                title: "review order".to_owned(),
                recorded_at: "2026-04-18T14:30:00Z".to_owned(),
                delivery_state: ReminderDeliveryState::Presented,
                detail: Some("Casey still needs confirmation.".to_owned()),
            }],
        };
        let pack_day = PackDayProjection {
            fulfillment_window: Some(radroots_studio_app_models::FulfillmentWindowSummary {
                fulfillment_window_id,
                farm_id,
                starts_at: "2026-04-18T16:00:00Z".to_owned(),
                ends_at: "2026-04-18T18:00:00Z".to_owned(),
            }),
            totals_by_product: vec![PackDayProductTotalRow {
                title: "Salad mix".to_owned(),
                quantity_display: "2 bags".to_owned(),
            }],
            pack_list: vec![PackDayPackListRow {
                title: "Salad mix".to_owned(),
                quantity_display: "Casey: 2 bags".to_owned(),
            }],
            pickup_roster: vec![PackDayRosterRow {
                order_id,
                order_number: "R-100".to_owned(),
                customer_display_name: "Casey".to_owned(),
            }],
            reminders: ReminderFeedProjection::default(),
        };

        assert_eq!(
            store.projection().orders.query,
            OrdersScreenQueryState::default()
        );
        assert_eq!(
            store.projection().pack_day.query,
            PackDayScreenQueryState::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::select_orders_filter(OrdersFilter::Packed)),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::select_orders_fulfillment_window(Some(
                fulfillment_window_id,
            ))),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_orders_list(orders_list.clone())),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_orders_reminders(
                orders_reminders.clone()
            )),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_orders_recovery_queue(
                recovery_queue.clone()
            )),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_reminder_log(reminder_log.clone())),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_order_detail(Some(
                order_detail.clone()
            ))),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::set_pack_day_fulfillment_window(Some(
                fulfillment_window_id,
            ))),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_pack_day_projection(
                pack_day.clone()
            )),
            Ok(true)
        );
        assert_eq!(
            store.projection().orders.query,
            OrdersScreenQueryState {
                filter: OrdersFilter::Packed,
                fulfillment_window_id: Some(fulfillment_window_id),
            }
        );
        assert_eq!(store.projection().orders.list, orders_list);
        assert_eq!(store.projection().orders.reminders, orders_reminders);
        assert_eq!(store.projection().orders.recovery_queue, recovery_queue);
        assert_eq!(store.projection().reminder_log, reminder_log);
        assert_eq!(store.projection().orders.detail, Some(order_detail));
        assert_eq!(
            store.projection().pack_day.query,
            PackDayScreenQueryState {
                fulfillment_window_id: Some(fulfillment_window_id),
            }
        );
        assert_eq!(store.projection().pack_day.projection, pack_day);
        assert_eq!(
            store.apply(AppStateCommand::select_orders_filter(
                OrdersFilter::NeedsAction
            )),
            Ok(true)
        );
        assert_eq!(store.projection().orders.detail, None);
        assert_eq!(
            store.repository().projection(),
            AppShellProjection::default()
        );
        assert_eq!(
            store.repository().persisted_state().seller.orders_query,
            OrdersScreenQueryState {
                filter: OrdersFilter::NeedsAction,
                fulfillment_window_id: Some(fulfillment_window_id),
            }
        );
        assert_eq!(
            store
                .repository()
                .persisted_state()
                .seller
                .order_detail_order_id,
            None
        );
        assert_eq!(
            store.repository().persisted_state().seller.pack_day_query,
            PackDayScreenQueryState {
                fulfillment_window_id: Some(fulfillment_window_id),
            }
        );
        assert_eq!(
            store.projection().pack_day.export,
            PackDayExportProjection::default()
        );
    }

    #[test]
    fn pack_day_export_and_host_handoff_projections_default_to_idle() {
        assert_eq!(
            PackDayScreenProjection::default().export,
            PackDayExportProjection {
                status: PackDayExportStatus::Idle,
                request: None,
                bundle: None,
                error_message: None,
            }
        );
        assert_eq!(
            PackDayScreenProjection::default().print,
            PackDayPrintProjection {
                status: PackDayPrintStatus::Idle,
                request: None,
                failure: None,
            }
        );
        assert_eq!(
            PackDayScreenProjection::default().host_handoff,
            PackDayHostHandoffProjection {
                status: PackDayHostHandoffStatus::Idle,
                request: None,
                error_message: None,
            }
        );
    }

    #[test]
    fn pack_day_export_state_is_restart_ephemeral_and_skips_persistence() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_export_request(fulfillment_window_id);
        let bundle = sample_pack_day_export_bundle(fulfillment_window_id);

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_export(request.clone())),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().export,
            PackDayExportProjection::running(request.clone())
        );
        assert_eq!(
            store.persisted_state().seller.pack_day_query,
            PackDayScreenQueryState::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::succeed_pack_day_export(
                request.clone(),
                bundle.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().export,
            PackDayExportProjection::succeeded(request.clone(), bundle)
        );

        assert_eq!(
            store.apply(AppStateCommand::fail_pack_day_export(
                request.clone(),
                "disk unavailable",
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().export,
            PackDayExportProjection::failed(request, "disk unavailable")
        );

        assert_eq!(
            store.apply(AppStateCommand::reset_pack_day_export()),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().export,
            PackDayExportProjection::default()
        );
    }

    #[test]
    fn pack_day_print_state_is_restart_ephemeral_and_skips_persistence() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_print_request(
            fulfillment_window_id,
            PackDayPrintKind::PrintCustomerLabels,
        );

        assert_eq!(
            request.label_stock,
            Some(PackDayPrintLabelStock::Avery5160Letter30Up)
        );
        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_print(request.clone())),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::running(request.clone())
        );
        assert_eq!(
            store.persisted_state().seller.pack_day_query,
            PackDayScreenQueryState::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::succeed_pack_day_print(request.clone())),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::succeeded(request.clone())
        );

        assert_eq!(
            store.apply(AppStateCommand::fail_pack_day_print(request.clone())),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::failed(request.clone())
        );

        assert_eq!(
            store.apply(AppStateCommand::fail_pack_day_print_with_kind(
                request.clone(),
                PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow,
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::failed_with_kind(
                request,
                PackDayPrintFailureKind::CustomerLabelsAvery5160Overflow,
            )
        );

        assert_eq!(
            store.apply(AppStateCommand::reset_pack_day_print()),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::default()
        );
    }

    #[test]
    fn pack_day_host_handoff_state_is_restart_ephemeral_and_skips_persistence() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_host_handoff_request(
            fulfillment_window_id,
            PackDayHostHandoffKind::RevealBundle,
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_host_handoff(
                request.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::running(request.clone())
        );
        assert_eq!(
            store.persisted_state().seller.pack_day_query,
            PackDayScreenQueryState::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::succeed_pack_day_host_handoff(
                request.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::succeeded(request.clone())
        );

        assert_eq!(
            store.apply(AppStateCommand::fail_pack_day_host_handoff(
                request.clone(),
                "finder unavailable",
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::failed(request, "finder unavailable")
        );

        assert_eq!(
            store.apply(AppStateCommand::reset_pack_day_host_handoff()),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::default()
        );
    }

    #[test]
    fn changing_pack_day_window_clears_stale_export_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let next_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_export_request(fulfillment_window_id);

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_export(request)),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().export.status,
            PackDayExportStatus::Running
        );

        assert_eq!(
            store.apply(AppStateCommand::set_pack_day_fulfillment_window(Some(
                next_window_id,
            ))),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().query,
            PackDayScreenQueryState {
                fulfillment_window_id: Some(next_window_id),
            }
        );
        assert_eq!(
            store.pack_day_projection().export,
            PackDayExportProjection::default()
        );
    }

    #[test]
    fn changing_pack_day_window_clears_stale_host_handoff_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let next_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_host_handoff_request(
            fulfillment_window_id,
            PackDayHostHandoffKind::OpenPickupRoster,
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_host_handoff(request)),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff.status,
            PackDayHostHandoffStatus::Running
        );

        assert_eq!(
            store.apply(AppStateCommand::set_pack_day_fulfillment_window(Some(
                next_window_id,
            ))),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().query,
            PackDayScreenQueryState {
                fulfillment_window_id: Some(next_window_id),
            }
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::default()
        );
    }

    #[test]
    fn changing_pack_day_window_clears_stale_print_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let next_window_id = FulfillmentWindowId::new();
        let request =
            sample_pack_day_print_request(fulfillment_window_id, PackDayPrintKind::PrintPackSheet);

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_print(request)),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print.status,
            PackDayPrintStatus::Running
        );

        assert_eq!(
            store.apply(AppStateCommand::set_pack_day_fulfillment_window(Some(
                next_window_id,
            ))),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().query,
            PackDayScreenQueryState {
                fulfillment_window_id: Some(next_window_id),
            }
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::default()
        );
    }

    #[test]
    fn changing_pack_day_export_state_clears_stale_host_handoff_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let export_request = sample_pack_day_export_request(fulfillment_window_id);
        let host_handoff_request = sample_pack_day_host_handoff_request(
            fulfillment_window_id,
            PackDayHostHandoffKind::RevealBundle,
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_export(
                export_request.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_host_handoff(
                host_handoff_request,
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff.status,
            PackDayHostHandoffStatus::Running
        );

        assert_eq!(
            store.apply(AppStateCommand::succeed_pack_day_export(
                export_request,
                sample_pack_day_export_bundle(fulfillment_window_id),
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::default()
        );
    }

    #[test]
    fn changing_pack_day_export_state_clears_stale_print_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let fulfillment_window_id = FulfillmentWindowId::new();
        let export_request = sample_pack_day_export_request(fulfillment_window_id);
        let print_request = sample_pack_day_print_request(
            fulfillment_window_id,
            PackDayPrintKind::PrintPickupRoster,
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_export(
                export_request.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_print(print_request)),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print.status,
            PackDayPrintStatus::Running
        );

        assert_eq!(
            store.apply(AppStateCommand::succeed_pack_day_export(
                export_request,
                sample_pack_day_export_bundle(fulfillment_window_id),
            )),
            Ok(true)
        );
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::default()
        );
    }

    #[test]
    fn replacing_pack_day_projection_with_new_window_clears_stale_host_handoff_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let farm_id = FarmId::new();
        let current_window_id = FulfillmentWindowId::new();
        let next_window_id = FulfillmentWindowId::new();
        let request = sample_pack_day_host_handoff_request(
            current_window_id,
            PackDayHostHandoffKind::OpenCustomerLabels,
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_host_handoff(request)),
            Ok(true)
        );

        let next_pack_day = PackDayProjection {
            fulfillment_window: Some(radroots_studio_app_models::FulfillmentWindowSummary {
                fulfillment_window_id: next_window_id,
                farm_id,
                starts_at: "2026-04-25T16:00:00Z".to_owned(),
                ends_at: "2026-04-25T19:00:00Z".to_owned(),
            }),
            totals_by_product: Vec::new(),
            pack_list: Vec::new(),
            pickup_roster: Vec::new(),
            reminders: ReminderFeedProjection::default(),
        };

        assert_eq!(
            store.apply(AppStateCommand::replace_pack_day_projection(
                next_pack_day.clone(),
            )),
            Ok(true)
        );
        assert_eq!(store.pack_day_projection().projection, next_pack_day);
        assert_eq!(
            store.pack_day_projection().host_handoff,
            PackDayHostHandoffProjection::default()
        );
    }

    #[test]
    fn replacing_pack_day_projection_with_new_window_clears_stale_print_state() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let farm_id = FarmId::new();
        let current_window_id = FulfillmentWindowId::new();
        let next_window_id = FulfillmentWindowId::new();
        let request =
            sample_pack_day_print_request(current_window_id, PackDayPrintKind::PrintCustomerLabels);

        assert_eq!(
            store.apply(AppStateCommand::begin_pack_day_print(request)),
            Ok(true)
        );

        let next_pack_day = PackDayProjection {
            fulfillment_window: Some(radroots_studio_app_models::FulfillmentWindowSummary {
                fulfillment_window_id: next_window_id,
                farm_id,
                starts_at: "2026-04-25T16:00:00Z".to_owned(),
                ends_at: "2026-04-25T19:00:00Z".to_owned(),
            }),
            totals_by_product: Vec::new(),
            pack_list: Vec::new(),
            pickup_roster: Vec::new(),
            reminders: ReminderFeedProjection::default(),
        };

        assert_eq!(
            store.apply(AppStateCommand::replace_pack_day_projection(
                next_pack_day.clone(),
            )),
            Ok(true)
        );
        assert_eq!(store.pack_day_projection().projection, next_pack_day);
        assert_eq!(
            store.pack_day_projection().print,
            PackDayPrintProjection::default()
        );
    }

    #[test]
    fn startup_identity_choice_flow_is_explicit_and_in_memory_only() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        assert_eq!(
            store.logged_out_startup_projection(),
            &LoggedOutStartupProjection::default()
        );

        assert_eq!(
            store.apply(AppStateCommand::show_startup_identity_choice()),
            Ok(true)
        );
        assert_eq!(
            store.logged_out_startup_projection().phase,
            LoggedOutStartupPhase::IdentityChoice
        );

        assert_eq!(
            store.apply(AppStateCommand::show_startup_signer_entry()),
            Ok(true)
        );
        assert_eq!(
            store.logged_out_startup_projection().phase,
            LoggedOutStartupPhase::SignerEntry
        );

        assert_eq!(
            store.apply(AppStateCommand::set_startup_signer_source_input(
                "https://signer.radroots.example/connect?uri=bunker://npub1signer",
            )),
            Ok(true)
        );
        assert_eq!(
            store
                .logged_out_startup_projection()
                .signer_entry
                .source_input,
            "https://signer.radroots.example/connect?uri=bunker://npub1signer"
        );

        assert_eq!(
            store.apply(AppStateCommand::begin_generate_key_startup()),
            Ok(true)
        );
        assert_eq!(
            store.logged_out_startup_projection().phase,
            LoggedOutStartupPhase::GenerateKeyStarting
        );
        assert_eq!(
            store.repository().projection(),
            AppShellProjection::default()
        );
        assert_eq!(
            store
                .repository()
                .persisted_state()
                .logged_out_startup
                .phase,
            LoggedOutStartupPhase::GenerateKeyStarting
        );

        assert_eq!(
            store.apply(AppStateCommand::reset_logged_out_startup()),
            Ok(true)
        );
        assert_eq!(
            store.logged_out_startup_projection(),
            &LoggedOutStartupProjection::default()
        );
    }

    #[test]
    fn product_editor_state_transitions_are_explicit() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let product_id = ProductId::new();
        let ready_draft = ProductEditorDraft {
            title: "Heirloom tomatoes".to_owned(),
            subtitle: "Brandywine".to_owned(),
            unit_label: "lb".to_owned(),
            price_minor_units: Some(450),
            price_currency: "USD".to_owned(),
            stock_quantity: Some(12),
            availability_window_id: Some(FulfillmentWindowId::new()),
            status: radroots_studio_app_models::ProductStatus::Draft,
        };

        assert_eq!(
            store.apply(AppStateCommand::open_new_product_editor()),
            Ok(true)
        );
        assert_eq!(
            store.projection().products.editor,
            ProductEditorState::Open(super::ProductEditorSession {
                selected_product_id: None,
                draft: ProductEditorDraft::default(),
                publish_blockers: vec![
                    ProductPublishBlocker::AddProductName,
                    ProductPublishBlocker::ChooseUnit,
                    ProductPublishBlocker::SetPrice,
                    ProductPublishBlocker::AttachAvailability,
                ],
            })
        );

        assert_eq!(
            store.apply(AppStateCommand::replace_product_editor_draft(
                ready_draft.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.projection().products.editor,
            ProductEditorState::Open(super::ProductEditorSession {
                selected_product_id: None,
                draft: ready_draft.clone(),
                publish_blockers: Vec::new(),
            })
        );

        assert_eq!(
            store.apply(AppStateCommand::open_existing_product_editor(
                product_id,
                ready_draft.clone(),
            )),
            Ok(true)
        );
        assert_eq!(
            store.projection().products.editor,
            ProductEditorState::Open(super::ProductEditorSession {
                selected_product_id: Some(product_id),
                draft: ready_draft,
                publish_blockers: Vec::new(),
            })
        );

        assert_eq!(
            store.apply(AppStateCommand::close_product_editor()),
            Ok(true)
        );
        assert_eq!(
            store.projection().products.editor,
            ProductEditorState::Closed
        );
        assert_eq!(
            store.apply(AppStateCommand::replace_product_editor_draft(
                ProductEditorDraft::default(),
            )),
            Ok(false)
        );
    }

    #[test]
    fn select_settings_section_updates_shared_settings_without_clobbering_home() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::select_settings_section(
            SettingsSection::Settings,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Personal
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            store.projection().shell.settings.selected_section,
            SettingsSection::Settings
        );
        assert_eq!(
            store.repository().projection().selected_section,
            ShellSection::Home
        );
        assert_eq!(
            store.repository().projection().settings.selected_section,
            SettingsSection::Settings
        );
    }

    #[test]
    fn select_farmer_section_without_identity_gate_is_rejected() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::SelectSection(ShellSection::Farmer(
            FarmerSection::Products,
        )));

        assert_eq!(changed, Ok(false));
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Personal
        );
    }

    #[test]
    fn replacing_identity_projection_with_farmer_activation_moves_home_to_farmer_today() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::replace_identity_projection(
            ready_identity(ActiveSurface::Farmer),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.startup_gate(), AppStartupGate::Farmer);
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Farmer
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );
        assert_eq!(store.home_route(), HomeRoute::FarmSetupOnboarding);
    }

    #[test]
    fn replacing_identity_projection_makes_signed_in_personal_entry_explicit_without_rewriting_home()
     {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::replace_identity_projection(
            ready_identity(ActiveSurface::Personal),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.startup_gate(), AppStartupGate::Personal);
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Home
        );
        assert_eq!(
            store.personal_projection().entry.state,
            PersonalEntryState::SignedIn
        );
    }

    #[test]
    fn startup_identity_choice_state_resets_once_identity_leaves_setup_required() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        assert_eq!(
            store.apply(AppStateCommand::show_startup_identity_choice()),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::show_startup_signer_entry()),
            Ok(true)
        );
        assert_eq!(
            store.apply(AppStateCommand::set_startup_signer_source_input(
                "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example",
            )),
            Ok(true)
        );

        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Personal),
            )),
            Ok(true)
        );
        assert_eq!(store.startup_gate(), AppStartupGate::Personal);
        assert_eq!(
            store.logged_out_startup_projection(),
            &LoggedOutStartupProjection::default()
        );
    }

    #[test]
    fn startup_identity_choice_commands_are_rejected_after_setup_gate_is_cleared() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Personal),
            )),
            Ok(true)
        );

        assert_eq!(
            store.apply(AppStateCommand::show_startup_identity_choice()),
            Ok(false)
        );
        assert_eq!(
            store.apply(AppStateCommand::show_startup_signer_entry()),
            Ok(false)
        );
        assert_eq!(
            store.apply(AppStateCommand::begin_generate_key_startup()),
            Ok(false)
        );
        assert_eq!(
            store.apply(AppStateCommand::set_startup_signer_source_input(
                "bunker://npub1signer?relay=wss%3A%2F%2Frelay.radroots.example",
            )),
            Ok(false)
        );
        assert_eq!(
            store.logged_out_startup_projection(),
            &LoggedOutStartupProjection::default()
        );
    }

    #[test]
    fn select_active_surface_moves_personal_home_to_farmer_today() {
        let repository = InMemoryAppStateRepository::new(AppShellProjection::for_surface(
            ActiveSurface::Personal,
        ));
        let mut store = AppStateStore::load(repository).expect("in-memory repository should load");
        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Personal,)
            )),
            Ok(true)
        );

        let changed = store.apply(AppStateCommand::select_active_surface(
            ActiveSurface::Farmer,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Farmer
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Farmer(FarmerSection::Today)
        );
        assert_eq!(
            store
                .identity_projection()
                .selected_account
                .as_ref()
                .expect("selected account")
                .active_surface(),
            ActiveSurface::Farmer
        );
    }

    #[test]
    fn select_active_surface_moves_farmer_routes_back_to_home_for_personal() {
        let repository = InMemoryAppStateRepository::new(AppShellProjection::new(
            ActiveSurface::Farmer,
            ShellSection::Farmer(FarmerSection::Products),
        ));
        let mut store = AppStateStore::load(repository).expect("in-memory repository should load");
        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Farmer,)
            )),
            Ok(true)
        );

        let changed = store.apply(AppStateCommand::select_active_surface(
            ActiveSurface::Personal,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Personal
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Personal(PersonalSection::Browse)
        );
        assert_eq!(store.startup_gate(), AppStartupGate::Personal);
    }

    #[test]
    fn select_active_surface_preserves_settings_route() {
        let repository = InMemoryAppStateRepository::new(AppShellProjection::for_settings(
            ActiveSurface::Personal,
            SettingsSection::About,
        ));
        let mut store = AppStateStore::load(repository).expect("in-memory repository should load");
        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Personal,)
            )),
            Ok(true)
        );

        let changed = store.apply(AppStateCommand::select_active_surface(
            ActiveSurface::Farmer,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(
            store.projection().shell.active_surface,
            ActiveSurface::Farmer
        );
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Settings(SettingsSection::About)
        );
    }

    #[test]
    fn settings_preference_command_is_a_noop_when_value_is_unchanged() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::SetSettingsPreference {
            preference: SettingsPreference::UseNip05,
            enabled: true,
        });

        assert_eq!(changed, Ok(false));
        assert!(store.projection().shell.settings.general.use_nip05);
    }

    #[test]
    fn settings_preference_command_updates_projection_and_repository() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::SetSettingsPreference {
            preference: SettingsPreference::LaunchAtLogin,
            enabled: true,
        });

        assert_eq!(changed, Ok(true));
        assert!(store.projection().shell.settings.general.launch_at_login);
        assert!(
            !store
                .repository()
                .projection()
                .settings
                .general
                .launch_at_login
        );
    }

    #[test]
    fn repository_errors_bubble_out_of_the_store() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");

        let error = store
            .apply(AppStateCommand::select_settings_section(
                SettingsSection::About,
            ))
            .expect_err("save should fail");

        assert_eq!(
            error,
            AppStateStoreError::Repository(AppStateRepositoryError::save("disk unavailable"))
        );
    }

    #[test]
    fn replace_today_agenda_updates_in_memory_state_without_touching_repository() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let farm_id = FarmId::new();
        let today = TodayAgendaProjection {
            farm: Some(radroots_studio_app_models::FarmSummary {
                farm_id,
                display_name: "North field farm".to_owned(),
                readiness: FarmReadiness::Incomplete,
            }),
            setup_checklist: vec![TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: false,
            }],
            ..TodayAgendaProjection::default()
        };

        let changed = store.apply(AppStateCommand::replace_today_agenda(today.clone()));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.projection().today.farm, today.farm);
        assert_eq!(store.projection().today.setup_checklist.len(), 6);
        assert!(store.projection().today.needs_setup());
    }

    #[test]
    fn replace_farm_setup_projection_updates_in_memory_state_without_touching_repository() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let farm_setup = FarmSetupProjection::from_draft(FarmSetupDraft::new(
            "North field farm",
            "",
            [FarmOrderMethod::Pickup],
        ));

        let changed = store.apply(AppStateCommand::replace_farm_setup_projection(
            farm_setup.clone(),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.farm_setup_projection(), &farm_setup);
    }

    #[test]
    fn select_farm_setup_flow_stage_switches_farmer_home_into_form_route() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Farmer),
            )),
            Ok(true)
        );
        assert_eq!(store.home_route(), HomeRoute::FarmSetupOnboarding);

        let changed = store.apply(AppStateCommand::select_farm_setup_flow_stage(
            FarmSetupFlowStage::Editing,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.home_route(), HomeRoute::FarmSetupForm);
    }

    #[test]
    fn complete_draft_without_saved_farm_stays_on_farm_setup_form() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Farmer),
            )),
            Ok(true)
        );

        let changed = store.apply(AppStateCommand::replace_farm_setup_projection(
            FarmSetupProjection::from_draft(FarmSetupDraft::new(
                "North field farm",
                "Stockholm County",
                [FarmOrderMethod::Pickup],
            )),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.home_route(), HomeRoute::FarmSetupForm);
        assert_eq!(
            store.projection().farm_setup_flow_stage,
            FarmSetupFlowStage::Onboarding
        );
    }

    #[test]
    fn saved_farm_in_today_projection_synchronizes_ready_home_route() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");
        let farm_id = FarmId::new();

        assert_eq!(
            store.apply(AppStateCommand::replace_identity_projection(
                ready_identity(ActiveSurface::Farmer),
            )),
            Ok(true)
        );

        let changed = store.apply(AppStateCommand::replace_today_agenda(
            TodayAgendaProjection {
                farm: Some(radroots_studio_app_models::FarmSummary {
                    farm_id,
                    display_name: "North field farm".to_owned(),
                    readiness: FarmReadiness::Ready,
                }),
                ..TodayAgendaProjection::default()
            },
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.home_route(), HomeRoute::Today);
        assert_eq!(
            store
                .farm_setup_projection()
                .saved_farm
                .as_ref()
                .expect("saved farm")
                .farm_id,
            farm_id
        );
    }

    #[test]
    fn replacing_identity_projection_surfaces_settings_account_state_without_touching_repository() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");

        let changed = store.apply(AppStateCommand::replace_identity_projection(
            ready_identity(ActiveSurface::Personal),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.startup_gate(), AppStartupGate::Personal);
        assert_eq!(store.settings_account_projection().roster.len(), 1);
        assert_eq!(
            store
                .settings_account_projection()
                .selected_account
                .as_ref()
                .expect("selected account")
                .account
                .account_id,
            "acct_surface"
        );
    }

    #[test]
    fn replace_sync_projection_updates_in_memory_state_without_touching_repository() {
        let mut store =
            AppStateStore::load(FailingRepository).expect("failing repository should still load");
        let checkpoint = SyncCheckpointStatus::current(
            None,
            "2026-04-20T19:00:00Z",
            Some("cursor-1".to_owned()),
        );
        let sync_projection = derive_sync_projection(&checkpoint, &[]);

        let changed = store.apply(AppStateCommand::replace_sync_projection(
            sync_projection.clone(),
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.sync_projection(), &sync_projection);
    }

    #[test]
    fn derive_sync_run_status_prefers_syncing_failed_and_conflicted_states_explicitly() {
        assert_eq!(
            derive_sync_run_status(
                &SyncCheckpointStatus::syncing("2026-04-20T18:00:00Z", None),
                &SyncConflictStatus::clear(),
            ),
            AppSyncRunStatus::Syncing
        );
        assert_eq!(
            derive_sync_run_status(
                &SyncCheckpointStatus::failed(None, None, None, "relay unavailable"),
                &SyncConflictStatus::clear(),
            ),
            AppSyncRunStatus::Failed
        );
        assert_eq!(
            derive_sync_run_status(
                &SyncCheckpointStatus {
                    state: SyncCheckpointState::Current,
                    ..SyncCheckpointStatus::never_synced()
                },
                &SyncConflictStatus {
                    unresolved_count: 1,
                    blocking_count: 1,
                },
            ),
            AppSyncRunStatus::Conflicted
        );
        assert_eq!(
            derive_sync_run_status(
                &SyncCheckpointStatus::current(None, "2026-04-20T19:00:00Z", None),
                &SyncConflictStatus::clear(),
            ),
            AppSyncRunStatus::Succeeded
        );
        assert_eq!(
            derive_sync_run_status(
                &SyncCheckpointStatus::never_synced(),
                &SyncConflictStatus::clear(),
            ),
            AppSyncRunStatus::Idle
        );
    }

    #[test]
    fn derive_sync_projection_counts_unresolved_conflicts_from_typed_rows() {
        let checkpoint = SyncCheckpointStatus::current(
            None,
            "2026-04-20T19:00:00Z",
            Some("cursor-2".to_owned()),
        );
        let conflicts = vec![
            SyncConflict {
                aggregate: radroots_studio_app_sync::SyncAggregateRef::Farm(FarmId::new()),
                kind: SyncConflictKind::RevisionMismatch,
                severity: SyncConflictSeverity::Blocking,
                resolution: SyncConflictResolutionStatus::Unresolved,
                local_payload_json: "{\"farm\":\"local\"}".to_owned(),
                remote_payload_json: Some("{\"farm\":\"remote\"}".to_owned()),
                detected_at: "2026-04-20T19:01:00Z".to_owned(),
                resolved_at: None,
            },
            SyncConflict {
                aggregate: radroots_studio_app_sync::SyncAggregateRef::Farm(FarmId::new()),
                kind: SyncConflictKind::RemoteValidationReject,
                severity: SyncConflictSeverity::ReviewRequired,
                resolution: SyncConflictResolutionStatus::AcceptedRemote,
                local_payload_json: "{\"farm\":\"local-two\"}".to_owned(),
                remote_payload_json: None,
                detected_at: "2026-04-20T19:02:00Z".to_owned(),
                resolved_at: Some("2026-04-20T19:03:00Z".to_owned()),
            },
        ];

        let projection = derive_sync_projection(&checkpoint, &conflicts);

        assert_eq!(projection.run_status, AppSyncRunStatus::Conflicted);
        assert_eq!(projection.checkpoint, checkpoint);
        assert_eq!(projection.conflict_status.unresolved_count, 1);
        assert_eq!(projection.conflict_status.blocking_count, 1);
    }

    #[test]
    fn in_memory_store_construction_and_updates_are_infallible() {
        let mut store = AppStateStore::in_memory(AppShellProjection::for_settings(
            ActiveSurface::Farmer,
            SettingsSection::Account,
        ));

        let changed = store.apply_in_memory(AppStateCommand::SetSettingsPreference {
            preference: SettingsPreference::AllowRelayConnections,
            enabled: false,
        });

        assert!(changed);
        assert!(
            !store
                .projection()
                .shell
                .settings
                .general
                .allow_relay_connections
        );
        assert!(
            store
                .repository()
                .projection()
                .settings
                .general
                .allow_relay_connections
        );
    }

    #[test]
    fn app_projection_defaults_the_new_reminder_contracts() {
        let projection = AppProjection::default();

        assert!(projection.today.reminders.is_empty());
        assert!(projection.orders.reminders.is_empty());
        assert!(projection.orders.recovery_queue.is_empty());
        assert!(projection.reminder_log.is_empty());
        assert!(projection.pack_day.projection.reminders.is_empty());
        assert_eq!(
            projection.orders.reminders,
            ReminderFeedProjection::default()
        );
    }
}
