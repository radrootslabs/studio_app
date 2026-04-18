use gpui::SharedString;
use radroots_studio_app_core::{AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label};
use radroots_studio_app_i18n::{AppTextKey, app_text};

use crate::LabelValueRow;

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

pub fn settings_preferences_general_rows() -> Vec<LabelValueRow> {
    vec![
        text_row(
            AppTextKey::SettingsGeneralAllowRelayConnections,
            AppTextKey::ValueEnabled,
        ),
        text_row(
            AppTextKey::SettingsGeneralUseMediaServers,
            AppTextKey::ValueEnabled,
        ),
        text_row(
            AppTextKey::SettingsGeneralUseNip05,
            AppTextKey::ValueEnabled,
        ),
        text_row(
            AppTextKey::SettingsGeneralLaunchAtLogin,
            AppTextKey::ValueDisabled,
        ),
    ]
}

pub fn settings_about_status_rows() -> Vec<LabelValueRow> {
    vec![
        text_row(
            AppTextKey::SettingsViewAbout,
            AppTextKey::SettingsAboutPlaceholderTopPrimary,
        ),
        text_row(
            AppTextKey::SettingsGeneralSectionLabel,
            AppTextKey::SettingsAboutPlaceholderMiddle,
        ),
        text_row(
            AppTextKey::SettingsAccountProfileLabel,
            AppTextKey::SettingsAboutPlaceholderBottom,
        ),
    ]
}

fn metadata_row(label: AppTextKey, value: impl Into<String>) -> LabelValueRow {
    LabelValueRow::new(app_shared_text(label), value.into())
}

fn text_row(label: AppTextKey, value: AppTextKey) -> LabelValueRow {
    metadata_row(label, app_text(value))
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
        runtime_metadata_rows, settings_about_status_rows, settings_preferences_general_rows,
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
    fn settings_placeholder_rows_use_localized_copy() {
        let general_rows = settings_preferences_general_rows();
        let about_rows = settings_about_status_rows();

        let allow_relay_label = app_text(AppTextKey::SettingsGeneralAllowRelayConnections);
        let enabled_value = app_text(AppTextKey::ValueEnabled);
        let about_label = app_text(AppTextKey::SettingsViewAbout);
        let about_primary = app_text(AppTextKey::SettingsAboutPlaceholderTopPrimary);

        assert!(
            general_rows
                .iter()
                .any(|row| row.label == allow_relay_label && row.value == enabled_value)
        );
        assert!(
            about_rows
                .iter()
                .any(|row| row.label == about_label && row.value == about_primary)
        );
    }
}
