#![forbid(unsafe_code)]

use radroots_studio_app_models::{
    ActiveSurface, AppIdentityProjection, AppStartupGate, FarmReadiness, FarmReadinessBlocker,
    FarmRulesProjection, FarmSetupBlocker, FarmSetupProjection, FarmSetupReadiness,
    FarmTimingConflict, FulfillmentWindowId, LoggedOutStartupPhase, LoggedOutStartupProjection,
    OrderDetailProjection, OrdersFilter, OrdersListProjection, OrdersScreenQueryState,
    PackDayProjection, PackDayScreenQueryState, ProductEditorDraft, ProductId,
    ProductPublishBlocker, ProductsFilter, ProductsListProjection, ProductsSort,
    SelectedSurfaceProjection, SettingsAccountProjection, SettingsPreference, SettingsSection,
    ShellSection, TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
};
use thiserror::Error;

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

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

impl PackDayScreenProjection {
    fn select_fulfillment_window(&mut self, fulfillment_window_id: Option<FulfillmentWindowId>) {
        self.query.fulfillment_window_id = fulfillment_window_id;
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
                if matches!(self.selected_section, ShellSection::Home) {
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
    pub logged_out_startup: LoggedOutStartupProjection,
    pub today: TodayAgendaProjection,
    pub products: ProductsScreenProjection,
    pub orders: OrdersScreenProjection,
    pub pack_day: PackDayScreenProjection,
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
            logged_out_startup: LoggedOutStartupProjection::default(),
            today,
            products: ProductsScreenProjection::default(),
            orders: OrdersScreenProjection::default(),
            pack_day: PackDayScreenProjection::default(),
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
    ReplaceOrderDetail(Option<OrderDetailProjection>),
    SetPackDayFulfillmentWindow(Option<FulfillmentWindowId>),
    ReplacePackDayProjection(PackDayProjection),
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
    fn load_shell_projection(&self) -> Result<AppShellProjection, AppStateRepositoryError>;

    fn save_shell_projection(
        &mut self,
        projection: &AppShellProjection,
    ) -> Result<(), AppStateRepositoryError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InMemoryAppStateRepository {
    projection: AppShellProjection,
}

impl Default for InMemoryAppStateRepository {
    fn default() -> Self {
        Self::new(AppShellProjection::default())
    }
}

impl InMemoryAppStateRepository {
    pub fn new(projection: AppShellProjection) -> Self {
        Self { projection }
    }

    pub fn projection(&self) -> &AppShellProjection {
        &self.projection
    }

    pub fn overwrite(&mut self, projection: AppShellProjection) {
        self.projection = projection;
    }
}

impl AppStateRepository for InMemoryAppStateRepository {
    fn load_shell_projection(&self) -> Result<AppShellProjection, AppStateRepositoryError> {
        Ok(self.projection.clone())
    }

    fn save_shell_projection(
        &mut self,
        projection: &AppShellProjection,
    ) -> Result<(), AppStateRepositoryError> {
        self.projection = projection.clone();
        Ok(())
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
}

impl<R: AppStateRepository> AppStateStore<R> {
    pub fn load(repository: R) -> Result<Self, AppStateStoreError> {
        let projection = AppProjection::new(
            repository.load_shell_projection()?,
            AppIdentityProjection::default(),
            TodayAgendaProjection::default(),
        );

        Ok(Self {
            repository,
            projection,
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

    pub fn products_projection(&self) -> &ProductsScreenProjection {
        &self.projection.products
    }

    pub fn orders_projection(&self) -> &OrdersScreenProjection {
        &self.projection.orders
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

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn apply(&mut self, command: AppStateCommand) -> Result<bool, AppStateStoreError> {
        let mut next_projection = self.projection.clone();

        match apply_command(&mut next_projection, command) {
            AppStateMutation::NoChange => Ok(false),
            AppStateMutation::ShellChanged => {
                self.repository
                    .save_shell_projection(&next_projection.shell)?;
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::FarmSetupChanged => {
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::StartupChanged => {
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::TodayChanged => {
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::ProductsChanged => {
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::OrdersChanged => {
                self.projection = next_projection;

                Ok(true)
            }
            AppStateMutation::PackDayChanged => {
                self.projection = next_projection;

                Ok(true)
            }
        }
    }
}

impl AppStateStore<InMemoryAppStateRepository> {
    pub fn in_memory(projection: AppShellProjection) -> Self {
        Self {
            repository: InMemoryAppStateRepository::new(projection.clone()),
            projection: AppProjection::new(
                projection,
                AppIdentityProjection::default(),
                TodayAgendaProjection::default(),
            ),
        }
    }

    pub fn apply_in_memory(&mut self, command: AppStateCommand) -> bool {
        let mut next_projection = self.projection.clone();

        match apply_command(&mut next_projection, command) {
            AppStateMutation::NoChange => false,
            AppStateMutation::ShellChanged => {
                self.repository.overwrite(next_projection.shell.clone());
                self.projection = next_projection;

                true
            }
            AppStateMutation::FarmSetupChanged => {
                self.projection = next_projection;

                true
            }
            AppStateMutation::StartupChanged => {
                self.projection = next_projection;

                true
            }
            AppStateMutation::TodayChanged => {
                self.projection = next_projection;

                true
            }
            AppStateMutation::ProductsChanged => {
                self.projection = next_projection;

                true
            }
            AppStateMutation::OrdersChanged => {
                self.projection = next_projection;

                true
            }
            AppStateMutation::PackDayChanged => {
                self.projection = next_projection;

                true
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppStateMutation {
    NoChange,
    ShellChanged,
    FarmSetupChanged,
    StartupChanged,
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
        AppStateCommand::ReplaceOrderDetail(order_detail_projection) => {
            projection.orders.replace_detail(order_detail_projection);
        }
        AppStateCommand::SetPackDayFulfillmentWindow(fulfillment_window_id) => {
            projection
                .pack_day
                .select_fulfillment_window(fulfillment_window_id);
        }
        AppStateCommand::ReplacePackDayProjection(pack_day_projection) => {
            projection.pack_day.projection = pack_day_projection;
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
    } else if projection.products != before.products {
        AppStateMutation::ProductsChanged
    } else if projection.orders != before.orders {
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
    sync_logged_out_startup(&mut projection.logged_out_startup, projection.startup_gate);
    sync_farm_setup_flow_stage(
        &mut projection.farm_setup_flow_stage,
        projection.startup_gate,
        projection.farm_setup.has_saved_farm(),
    );
}

fn sync_shell_to_identity(shell: &mut AppShellProjection, identity: &AppIdentityProjection) {
    match identity.startup_gate() {
        AppStartupGate::Blocked | AppStartupGate::SetupRequired | AppStartupGate::Personal => {
            shell.active_surface = ActiveSurface::Personal;
            if matches!(shell.selected_section, ShellSection::Farmer(_)) {
                shell.selected_section = ShellSection::Home;
            }
        }
        AppStartupGate::Farmer => {
            shell.active_surface = ActiveSurface::Farmer;
            if matches!(shell.selected_section, ShellSection::Home) {
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

#[cfg(test)]
mod tests {
    use super::{
        AppProjection, AppShellProjection, AppStateCommand, AppStateRepository,
        AppStateRepositoryError, AppStateStore, AppStateStoreError, FarmSetupFlowStage, HomeRoute,
        InMemoryAppStateRepository, OrdersScreenProjection, PackDayScreenProjection,
        ProductEditorState, ProductsScreenProjection, ProductsScreenQueryState, SettingsPreference,
    };
    use radroots_studio_app_models::{
        AccountCustody, AccountSummary, ActiveSurface, AppIdentityProjection, AppStartupGate,
        FarmId, FarmOrderMethod, FarmReadiness, FarmSetupDraft, FarmSetupProjection,
        FarmerActivationProjection, FarmerSection, FulfillmentWindowId, LoggedOutStartupPhase,
        LoggedOutStartupProjection, OrderDetailItemRow, OrderDetailProjection, OrderId,
        OrderPrimaryAction, OrderStatus, OrdersFilter, OrdersListProjection, OrdersListRow,
        OrdersListSummary, OrdersScreenQueryState, PackDayPackListRow, PackDayProductTotalRow,
        PackDayProjection, PackDayRosterRow, PackDayScreenQueryState, ProductEditorDraft,
        ProductId, ProductPublishBlocker, ProductsFilter, ProductsListProjection, ProductsSort,
        SelectedAccountProjection, SelectedSurfaceProjection, SettingsSection, ShellSection,
        TodayAgendaProjection, TodaySetupTask, TodaySetupTaskKind,
    };

    struct FailingRepository;

    impl AppStateRepository for FailingRepository {
        fn load_shell_projection(&self) -> Result<AppShellProjection, AppStateRepositoryError> {
            Ok(AppShellProjection::default())
        }

        fn save_shell_projection(
            &mut self,
            _: &AppShellProjection,
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

    #[test]
    fn default_projection_starts_on_personal_setup_gate() {
        let projection = AppProjection::default();

        assert_eq!(projection.shell.active_surface, ActiveSurface::Personal);
        assert_eq!(projection.shell.selected_section, ShellSection::Home);
        assert_eq!(projection.identity, AppIdentityProjection::default());
        assert_eq!(projection.startup_gate, AppStartupGate::SetupRequired);
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
            &AppShellProjection::default()
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
        };
        let pack_day = PackDayProjection {
            fulfillment_window: Some(radroots_studio_app_models::FulfillmentWindowSummary {
                fulfillment_window_id,
                farm_id,
                pickup_location_id: None,
                label: "Friday pickup".to_owned(),
                starts_at: "2026-04-18T16:00:00Z".to_owned(),
                ends_at: "2026-04-18T18:00:00Z".to_owned(),
                order_cutoff_at: "2026-04-17T18:00:00Z".to_owned(),
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
            &AppShellProjection::default()
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
            &AppShellProjection::default()
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
            ShellSection::Home
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
            store
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
            !store
                .repository()
                .projection()
                .settings
                .general
                .allow_relay_connections
        );
    }
}
