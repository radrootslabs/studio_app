use gpui::SharedString;
use radroots_studio_app_core::{AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label};
use radroots_studio_app_i18n::{AppTextKey, app_text};

use crate::LabelValueRow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettingsPreferencesGeneralRowState {
    pub allow_relay_connections: bool,
    pub use_media_servers: bool,
    pub use_nip05: bool,
    pub launch_at_login: bool,
}

pub fn app_shared_text(key: AppTextKey) -> SharedString {
    app_text(key).into()
}

pub fn app_shared_label_text(key: AppTextKey) -> SharedString {
    format!("{}:", app_text(key)).into()
}

pub fn runtime_metadata_rows(snapshot: &AppRuntimeSnapshot) -> Vec<LabelValueRow> {
    vec![
        metadata_row(
            AppTextKey::MetadataCorePackage,
            snapshot.core.package_name.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataCoreVersion,
            snapshot.core.package_version.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataCoreAuthors,
            snapshot.core.package_authors.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataRustEdition,
            snapshot.core.rust_edition.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataRustToolchain,
            snapshot.core.rust_toolchain.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataTargetTriple,
            snapshot.build.target_triple.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataBuildProfile,
            snapshot.build.build_profile.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataProjection,
            snapshot.build.projection_source.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataGitCommit,
            snapshot
                .build
                .git_commit
                .clone()
                .unwrap_or_else(|| app_text(AppTextKey::ValueNone)),
        ),
        metadata_row(AppTextKey::MetadataAppName, snapshot.host.app_name.clone()),
        metadata_row(
            AppTextKey::MetadataAppId,
            snapshot.host.app_identifier.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataAppVersion,
            snapshot.host.app_version.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataAppBuild,
            snapshot.host.app_build.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataPlatform,
            snapshot.host.platform_name.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataOperatingSystem,
            snapshot.host.operating_system.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataHostLocale,
            snapshot.host.host_locale.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataRuntimeOrigin,
            snapshot.host.runtime_origin.clone(),
        ),
        metadata_row(
            AppTextKey::MetadataRuntimeMode,
            runtime_mode_text(&snapshot.runtime_mode),
        ),
        metadata_row(AppTextKey::MetadataRunId, snapshot.run_id.clone()),
    ]
}

pub fn settings_preferences_general_rows(
    state: SettingsPreferencesGeneralRowState,
) -> Vec<LabelValueRow> {
    vec![
        text_row(
            AppTextKey::SettingsGeneralAllowRelayConnections,
            enabled_value_key(state.allow_relay_connections),
        ),
        text_row(
            AppTextKey::SettingsGeneralUseMediaServers,
            enabled_value_key(state.use_media_servers),
        ),
        text_row(
            AppTextKey::SettingsGeneralUseNip05,
            enabled_value_key(state.use_nip05),
        ),
        text_row(
            AppTextKey::SettingsGeneralLaunchAtLogin,
            enabled_value_key(state.launch_at_login),
        ),
    ]
}

fn metadata_row(label: AppTextKey, value: impl Into<String>) -> LabelValueRow {
    LabelValueRow::new(app_shared_text(label), value.into())
}

fn text_row(label: AppTextKey, value: AppTextKey) -> LabelValueRow {
    metadata_row(label, app_text(value))
}

fn enabled_value_key(enabled: bool) -> AppTextKey {
    if enabled {
        AppTextKey::ValueEnabled
    } else {
        AppTextKey::ValueDisabled
    }
}

fn runtime_mode_text(mode: &AppRuntimeMode) -> String {
    match mode {
        AppRuntimeMode::Production => app_text(AppTextKey::ValueRuntimeModeProduction),
        _ => runtime_mode_label(mode).to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use radroots_studio_app_core::{
        APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeCapture, AppRuntimeMode,
        AppRuntimeSnapshot,
    };
    use radroots_studio_app_i18n::{AppTextKey, app_text};

    use super::{
        SettingsPreferencesGeneralRowState, runtime_metadata_rows,
        settings_preferences_general_rows,
    };

    #[test]
    fn runtime_metadata_rows_use_localized_labels() {
        let snapshot = AppRuntimeSnapshot::from_capture(
            AppBuildIdentity {
                package_name: "radroots_studio_app".to_owned(),
                package_version: "0.1.0".to_owned(),
                build_profile: "debug".to_owned(),
                target_triple: "aarch64-apple-darwin".to_owned(),
                projection_source: APP_PROJECTION_SOURCE.to_owned(),
                git_commit: None,
            },
            AppRuntimeMode::Development,
            AppRuntimeCapture {
                host_locale: "en_US.UTF-8".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "run-development-123-pid456".to_owned(),
            },
        );

        let rows = runtime_metadata_rows(&snapshot);

        assert!(
            rows.iter()
                .any(|row| row.label == "runtime mode" && row.value == "development")
        );
        assert!(
            rows.iter()
                .any(|row| row.label == "app id" && row.value == "org.radroots.app")
        );
    }

    #[test]
    fn settings_preferences_rows_use_localized_copy() {
        let general_rows = settings_preferences_general_rows(SettingsPreferencesGeneralRowState {
            allow_relay_connections: false,
            use_media_servers: true,
            use_nip05: false,
            launch_at_login: true,
        });

        let allow_relay_label = app_text(AppTextKey::SettingsGeneralAllowRelayConnections);
        let enabled_value = app_text(AppTextKey::ValueEnabled);
        let disabled_value = app_text(AppTextKey::ValueDisabled);

        assert!(
            general_rows
                .iter()
                .any(|row| row.label == allow_relay_label && row.value == disabled_value)
        );
        assert!(general_rows.iter().any(|row| {
            row.label == app_text(AppTextKey::SettingsGeneralLaunchAtLogin)
                && row.value == enabled_value
        }));
    }
}
