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
    HomeTitle => "home.title",
    HomeMetadataTitle => "home.metadata_title",
    MenuSettings => "menu.settings",
    MenuQuit => "menu.quit",
    SettingsTitle => "settings.title",
    SettingsNavAccounts => "settings.nav.accounts",
    SettingsNavSettings => "settings.nav.settings",
    SettingsNavAbout => "settings.nav.about",
    SettingsAccountsSummary => "settings.accounts.summary",
    SettingsAccountsProfileTitle => "settings.accounts.profile.title",
    SettingsAccountsIdentityLabel => "settings.accounts.identity.label",
    SettingsAccountsIdentityValue => "settings.accounts.identity.value",
    SettingsAccountsStatusLabel => "settings.accounts.status.label",
    SettingsAccountsStatusValue => "settings.accounts.status.value",
    SettingsAccountsRuntimeTitle => "settings.accounts.runtime.title",
    SettingsAccountsSyncLabel => "settings.accounts.sync.label",
    SettingsAccountsSyncValue => "settings.accounts.sync.value",
    SettingsAccountsRecoveryLabel => "settings.accounts.recovery.label",
    SettingsAccountsRecoveryValue => "settings.accounts.recovery.value",
    SettingsPreferencesSummary => "settings.preferences.summary",
    SettingsPreferencesGeneralTitle => "settings.preferences.general.title",
    SettingsPreferencesLaunchLabel => "settings.preferences.launch.label",
    SettingsPreferencesLaunchValue => "settings.preferences.launch.value",
    SettingsPreferencesNetworkLabel => "settings.preferences.network.label",
    SettingsPreferencesNetworkValue => "settings.preferences.network.value",
    SettingsPreferencesDeviceTitle => "settings.preferences.device.title",
    SettingsPreferencesNotificationsLabel => "settings.preferences.notifications.label",
    SettingsPreferencesNotificationsValue => "settings.preferences.notifications.value",
    SettingsPreferencesDiagnosticsLabel => "settings.preferences.diagnostics.label",
    SettingsPreferencesDiagnosticsValue => "settings.preferences.diagnostics.value",
    SettingsAboutSummary => "settings.about.summary",
    SettingsAboutBuildTitle => "settings.about.build.title",
    SettingsAboutShellLabel => "settings.about.shell.label",
    SettingsAboutShellValue => "settings.about.shell.value",
    SettingsAboutLicenseLabel => "settings.about.license.label",
    SettingsAboutLicenseValue => "settings.about.license.value",
    SettingsAboutStatusTitle => "settings.about.status.title",
    SettingsAboutPostureLabel => "settings.about.posture.label",
    SettingsAboutPostureValue => "settings.about.posture.value",
    SettingsAboutMilestoneLabel => "settings.about.milestone.label",
    SettingsAboutMilestoneValue => "settings.about.milestone.value",
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
    ValueNone => "value.none",
    ValueRuntimeModeDevelopment => "value.runtime_mode.development",
    ValueRuntimeModeProduction => "value.runtime_mode.production",
}
