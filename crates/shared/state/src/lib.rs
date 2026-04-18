#![forbid(unsafe_code)]

use radroots_studio_app_models::{
    AppMode, SettingsPreference, SettingsSection, ShellSection, TodayAgendaProjection,
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
pub struct AppShellProjection {
    pub app_mode: AppMode,
    pub selected_section: ShellSection,
    pub settings: SettingsShellProjection,
}

impl Default for AppShellProjection {
    fn default() -> Self {
        Self::new(ShellSection::default())
    }
}

impl AppShellProjection {
    pub fn new(selected_section: ShellSection) -> Self {
        let settings = match selected_section {
            ShellSection::Settings(section) => SettingsShellProjection::new(section),
            _ => SettingsShellProjection::default(),
        };

        Self {
            app_mode: selected_section.mode(),
            selected_section,
            settings,
        }
    }

    pub fn for_settings(selected_section: SettingsSection) -> Self {
        Self::new(ShellSection::Settings(selected_section))
    }

    fn select_section(&mut self, selected_section: ShellSection) {
        self.app_mode = selected_section.mode();
        self.selected_section = selected_section;

        if let ShellSection::Settings(settings_section) = selected_section {
            self.settings.selected_section = settings_section;
        }
    }

    fn select_settings_section(&mut self, selected_section: SettingsSection) {
        self.settings.selected_section = selected_section;

        if matches!(self.selected_section, ShellSection::Settings(_)) {
            self.selected_section = ShellSection::Settings(selected_section);
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppProjection {
    pub shell: AppShellProjection,
    pub today: TodayAgendaProjection,
}

impl AppProjection {
    pub fn new(shell: AppShellProjection, today: TodayAgendaProjection) -> Self {
        Self { shell, today }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppStateCommand {
    SelectSection(ShellSection),
    SelectSettingsSection(SettingsSection),
    SetSettingsPreference {
        preference: SettingsPreference,
        enabled: bool,
    },
    ReplaceTodayAgenda(TodayAgendaProjection),
}

impl AppStateCommand {
    pub const fn select_settings_section(section: SettingsSection) -> Self {
        Self::SelectSettingsSection(section)
    }

    pub fn replace_today_agenda(projection: TodayAgendaProjection) -> Self {
        Self::ReplaceTodayAgenda(projection)
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
            AppStateMutation::TodayChanged => {
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
            projection: AppProjection::new(projection, TodayAgendaProjection::default()),
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
            AppStateMutation::TodayChanged => {
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
    TodayChanged,
}

fn apply_command(projection: &mut AppProjection, command: AppStateCommand) -> AppStateMutation {
    let before = projection.clone();

    match command {
        AppStateCommand::SelectSection(selected_section) => {
            projection.shell.select_section(selected_section);
        }
        AppStateCommand::SelectSettingsSection(selected_section) => {
            projection.shell.select_settings_section(selected_section);
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
    }

    if *projection == before {
        AppStateMutation::NoChange
    } else if projection.shell != before.shell {
        AppStateMutation::ShellChanged
    } else {
        AppStateMutation::TodayChanged
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppProjection, AppShellProjection, AppStateCommand, AppStateRepository,
        AppStateRepositoryError, AppStateStore, AppStateStoreError, InMemoryAppStateRepository,
        SettingsPreference,
    };
    use radroots_studio_app_models::{
        AppMode, FarmerSection, SettingsSection, ShellSection, TodayAgendaProjection,
        TodaySetupTask, TodaySetupTaskKind,
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

    #[test]
    fn default_projection_starts_on_farmer_home() {
        let projection = AppProjection::default();

        assert_eq!(projection.shell.app_mode, AppMode::Farmer);
        assert_eq!(projection.shell.selected_section, ShellSection::Home);
        assert_eq!(
            projection.shell.settings.selected_section,
            SettingsSection::Account
        );
        assert!(projection.shell.settings.general.allow_relay_connections);
        assert!(projection.shell.settings.general.use_media_servers);
        assert!(projection.shell.settings.general.use_nip05);
        assert!(!projection.shell.settings.general.launch_at_login);
        assert_eq!(projection.today, TodayAgendaProjection::default());
    }

    #[test]
    fn load_uses_repository_projection() {
        let repository = InMemoryAppStateRepository::new(AppShellProjection::for_settings(
            SettingsSection::About,
        ));
        let store = AppStateStore::load(repository).expect("in-memory repository should load");

        assert_eq!(store.projection().shell.app_mode, AppMode::Farmer);
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Settings(SettingsSection::About)
        );
        assert_eq!(
            store.projection().shell.settings.selected_section,
            SettingsSection::About
        );
        assert_eq!(store.projection().today, TodayAgendaProjection::default());
    }

    #[test]
    fn select_settings_section_updates_shared_settings_without_clobbering_home() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::select_settings_section(
            SettingsSection::Settings,
        ));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.projection().shell.app_mode, AppMode::Farmer);
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
    fn select_section_still_updates_the_root_shell() {
        let mut store = AppStateStore::load(InMemoryAppStateRepository::default())
            .expect("in-memory repository should load");

        let changed = store.apply(AppStateCommand::SelectSection(ShellSection::Farmer(
            FarmerSection::Products,
        )));

        assert_eq!(changed, Ok(true));
        assert_eq!(
            store.projection().shell.selected_section,
            ShellSection::Farmer(FarmerSection::Products)
        );
        assert_eq!(
            store.repository().projection().selected_section,
            ShellSection::Farmer(FarmerSection::Products)
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
        let today = TodayAgendaProjection {
            setup_checklist: vec![TodaySetupTask {
                kind: TodaySetupTaskKind::AddFulfillmentWindow,
                is_complete: false,
            }],
            ..TodayAgendaProjection::default()
        };

        let changed = store.apply(AppStateCommand::replace_today_agenda(today.clone()));

        assert_eq!(changed, Ok(true));
        assert_eq!(store.projection().today, today);
    }

    #[test]
    fn in_memory_store_construction_and_updates_are_infallible() {
        let mut store =
            AppStateStore::in_memory(AppShellProjection::for_settings(SettingsSection::Account));

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
