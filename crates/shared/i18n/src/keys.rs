macro_rules! define_app_text_keys {
    ($($variant:ident => $id:literal,)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum AppTextKey {
            $($variant,)+
        }

        impl AppTextKey {
            pub const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            pub const fn id(self) -> &'static str {
                match self {
                    $(Self::$variant => $id,)+
                }
            }
        }
    };
}

define_app_text_keys! {
    AppName => "app.name",
    HomeBrand => "home.brand",
    MenuQuit => "menu.quit",
    MenuAbout => "menu.about",
    MenuServices => "menu.services",
    SettingsTitle => "settings.title",
    SettingsNavAccounts => "settings.nav.accounts",
    SettingsNavSettings => "settings.nav.settings",
    SettingsNavAbout => "settings.nav.about",
    SettingsAccountProfileLabel => "settings.account.profile.label",
    SettingsAccountStatusLabel => "settings.account.status.label",
    SettingsAccountStatusLoggedIn => "settings.account.status.logged_in",
    SettingsAccountStatusLoggedOut => "settings.account.status.logged_out",
    SettingsAccountPlaceholderName => "settings.account.placeholder_name",
    SettingsAccountPlaceholderHandle => "settings.account.placeholder_handle",
    SettingsAccountAddAction => "settings.account.action.add_account",
    SettingsAccountLogOutAction => "settings.account.action.log_out",
    SettingsAccountAdminConsoleAction => "settings.account.action.admin_console",
    SettingsViewAccount => "settings.view.account",
    SettingsViewSettings => "settings.view.settings",
    SettingsViewAbout => "settings.view.about",
    SettingsGeneralSectionLabel => "settings.general.section.label",
    SettingsGeneralAllowRelayConnections => "settings.general.allow_relay_connections",
    SettingsGeneralUseMediaServers => "settings.general.use_media_servers",
    SettingsGeneralUseNip05 => "settings.general.use_nip05",
    SettingsGeneralLaunchAtLogin => "settings.general.launch_at_login",
    SettingsGeneralManageAction => "settings.general.action.manage",
    SettingsGeneralUseNip05Note => "settings.general.use_nip05.note",
    SettingsAboutPlaceholderTopPrimary => "settings.about.placeholder.top_primary",
    SettingsAboutPlaceholderTopSecondary => "settings.about.placeholder.top_secondary",
    SettingsAboutPlaceholderTopTertiary => "settings.about.placeholder.top_tertiary",
    SettingsAboutPlaceholderMiddle => "settings.about.placeholder.middle",
    SettingsAboutPlaceholderBottom => "settings.about.placeholder.bottom",
    MetadataCorePackage => "metadata.core_package",
    MetadataCoreVersion => "metadata.core_version",
    MetadataCoreAuthors => "metadata.core_authors",
    MetadataRustEdition => "metadata.rust_edition",
    MetadataRustToolchain => "metadata.rust_toolchain",
    MetadataTargetTriple => "metadata.target_triple",
    MetadataBuildProfile => "metadata.build_profile",
    MetadataProjection => "metadata.projection",
    MetadataGitCommit => "metadata.git_commit",
    MetadataAppName => "metadata.app_name",
    MetadataAppId => "metadata.app_id",
    MetadataAppVersion => "metadata.app_version",
    MetadataAppBuild => "metadata.app_build",
    MetadataPlatform => "metadata.platform",
    MetadataOperatingSystem => "metadata.operating_system",
    MetadataHostLocale => "metadata.host_locale",
    MetadataRuntimeOrigin => "metadata.runtime_origin",
    MetadataRuntimeMode => "metadata.runtime_mode",
    MetadataRunId => "metadata.run_id",
    MetadataDataRoot => "metadata.data_root",
    MetadataLogsRoot => "metadata.logs_root",
    MetadataDatabasePath => "metadata.database_path",
    MetadataDatabaseSchemaVersion => "metadata.database_schema_version",
    MetadataShellSection => "metadata.shell_section",
    MetadataSyncRunStatus => "metadata.sync_run_status",
    MetadataSyncCheckpointState => "metadata.sync_checkpoint_state",
    MetadataSyncConflictCount => "metadata.sync_conflict_count",
    MetadataStartupIssue => "metadata.startup_issue",
    ValueNone => "value.none",
    ValueEnabled => "value.enabled",
    ValueDisabled => "value.disabled",
    ValueRuntimeModeDevelopment => "value.runtime_mode.development",
    ValueRuntimeModeProduction => "value.runtime_mode.production",
    ValueSyncRunStatusIdle => "value.sync_run_status.idle",
    ValueSyncRunStatusSyncing => "value.sync_run_status.syncing",
    ValueSyncRunStatusSucceeded => "value.sync_run_status.succeeded",
    ValueSyncRunStatusConflicted => "value.sync_run_status.conflicted",
    ValueSyncRunStatusFailed => "value.sync_run_status.failed",
    ValueSyncCheckpointNeverSynced => "value.sync_checkpoint_state.never_synced",
    ValueSyncCheckpointSyncing => "value.sync_checkpoint_state.syncing",
    ValueSyncCheckpointCurrent => "value.sync_checkpoint_state.current",
    ValueSyncCheckpointFailed => "value.sync_checkpoint_state.failed",
}
