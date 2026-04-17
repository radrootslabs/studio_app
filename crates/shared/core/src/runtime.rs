use std::time::{SystemTime, UNIX_EPOCH};

pub const APP_ID: &str = "org.radroots.app";
pub const APP_NAME: &str = "radroots";
pub const APP_PLATFORM_RUNTIME: &str = "app-desktop-gpui";
pub const APP_PROJECTION_SOURCE: &str = "gpui-native";
pub const APP_RUNTIME_ORIGIN: &str = "gpui://localhost";
pub const APP_HOST_PLATFORM: &str = "desktop";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppRuntimeMode {
    Development,
    Production,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppBuildIdentity {
    pub package_name: String,
    pub package_version: String,
    pub build_profile: String,
    pub target_triple: String,
    pub projection_source: String,
    pub git_commit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppCoreRuntimeMetadata {
    pub package_name: String,
    pub package_version: String,
    pub package_authors: String,
    pub rust_edition: String,
    pub rust_toolchain: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppHostRuntimeMetadata {
    pub app_identifier: String,
    pub app_name: String,
    pub app_version: String,
    pub app_build: String,
    pub platform_name: String,
    pub operating_system: String,
    pub host_locale: String,
    pub runtime_origin: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppRuntimeCapture {
    pub host_locale: String,
    pub operating_system: String,
    pub run_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppRuntimeSnapshot {
    pub title: String,
    pub runtime_mode: AppRuntimeMode,
    pub run_id: String,
    pub core: AppCoreRuntimeMetadata,
    pub build: AppBuildIdentity,
    pub host: AppHostRuntimeMetadata,
}

impl AppRuntimeCapture {
    pub fn current(mode: &AppRuntimeMode) -> Self {
        Self {
            host_locale: detect_host_locale(),
            operating_system: std::env::consts::OS.to_owned(),
            run_id: build_run_id(mode),
        }
    }
}

impl AppRuntimeSnapshot {
    pub fn capture(build: AppBuildIdentity) -> Self {
        let mode = parse_runtime_mode(&build.build_profile);
        Self::from_capture(build, mode, AppRuntimeCapture::current(&mode))
    }

    pub fn from_capture(
        build: AppBuildIdentity,
        runtime_mode: AppRuntimeMode,
        capture: AppRuntimeCapture,
    ) -> Self {
        let app_version = build.package_version.clone();
        let app_build = build
            .git_commit
            .clone()
            .unwrap_or_else(|| build.build_profile.clone());

        Self {
            title: APP_NAME.to_owned(),
            runtime_mode,
            run_id: capture.run_id,
            core: AppCoreRuntimeMetadata {
                package_name: env!("CARGO_PKG_NAME").to_owned(),
                package_version: env!("CARGO_PKG_VERSION").to_owned(),
                package_authors: env!("CARGO_PKG_AUTHORS").to_owned(),
                rust_edition: "2024".to_owned(),
                rust_toolchain: env!("CARGO_PKG_RUST_VERSION").to_owned(),
            },
            build,
            host: AppHostRuntimeMetadata {
                app_identifier: APP_ID.to_owned(),
                app_name: APP_NAME.to_owned(),
                app_version,
                app_build,
                platform_name: APP_HOST_PLATFORM.to_owned(),
                operating_system: capture.operating_system,
                host_locale: capture.host_locale,
                runtime_origin: APP_RUNTIME_ORIGIN.to_owned(),
            },
        }
    }
}

pub fn runtime_mode_label(mode: &AppRuntimeMode) -> &'static str {
    match mode {
        AppRuntimeMode::Development => "development",
        AppRuntimeMode::Production => "production",
    }
}

fn parse_runtime_mode(build_profile: &str) -> AppRuntimeMode {
    match build_profile.trim() {
        "release" => AppRuntimeMode::Production,
        _ => AppRuntimeMode::Development,
    }
}

fn detect_host_locale() -> String {
    [
        std::env::var("LC_ALL").ok(),
        std::env::var("LC_MESSAGES").ok(),
        std::env::var("LANGUAGE").ok(),
        std::env::var("LANG").ok(),
    ]
    .into_iter()
    .flatten()
    .find_map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
    .unwrap_or_else(|| "en".to_owned())
}

fn build_run_id(mode: &AppRuntimeMode) -> String {
    let started_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!(
        "run-{}-{started_at_ms}-pid{}",
        runtime_mode_label(mode),
        std::process::id()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        APP_HOST_PLATFORM, APP_ID, APP_NAME, APP_PROJECTION_SOURCE, APP_RUNTIME_ORIGIN,
        AppBuildIdentity, AppRuntimeCapture, AppRuntimeMode, AppRuntimeSnapshot,
        runtime_mode_label,
    };

    fn test_build_identity() -> AppBuildIdentity {
        AppBuildIdentity {
            package_name: "radroots_studio_app".to_owned(),
            package_version: "0.1.0".to_owned(),
            build_profile: "debug".to_owned(),
            target_triple: "aarch64-apple-darwin".to_owned(),
            projection_source: APP_PROJECTION_SOURCE.to_owned(),
            git_commit: Some("deadbeefcafefeed".to_owned()),
        }
    }

    #[test]
    fn runtime_snapshot_surfaces_core_and_host_metadata() {
        let snapshot = AppRuntimeSnapshot::from_capture(
            test_build_identity(),
            AppRuntimeMode::Development,
            AppRuntimeCapture {
                host_locale: "en_US.UTF-8".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "run-development-123-pid456".to_owned(),
            },
        );

        assert_eq!(snapshot.title, APP_NAME);
        assert_eq!(snapshot.run_id, "run-development-123-pid456");
        assert_eq!(snapshot.core.package_name, "radroots_studio_app_core");
        assert_eq!(snapshot.core.package_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(snapshot.core.package_authors, env!("CARGO_PKG_AUTHORS"));
        assert_eq!(snapshot.core.rust_edition, "2024");
        assert_eq!(snapshot.core.rust_toolchain, env!("CARGO_PKG_RUST_VERSION"));
        assert_eq!(snapshot.build.package_name, "radroots_studio_app");
        assert_eq!(snapshot.build.target_triple, "aarch64-apple-darwin");
        assert_eq!(snapshot.host.app_identifier, APP_ID);
        assert_eq!(snapshot.host.app_name, APP_NAME);
        assert_eq!(snapshot.host.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(snapshot.host.app_build, "deadbeefcafefeed");
        assert_eq!(snapshot.host.platform_name, APP_HOST_PLATFORM);
        assert_eq!(snapshot.host.operating_system, "macos");
        assert_eq!(snapshot.host.host_locale, "en_US.UTF-8");
        assert_eq!(snapshot.host.runtime_origin, APP_RUNTIME_ORIGIN);
    }

    #[test]
    fn runtime_snapshot_falls_back_to_build_profile_when_git_commit_is_missing() {
        let mut build = test_build_identity();
        build.git_commit = None;
        build.build_profile = "release".to_owned();

        let snapshot = AppRuntimeSnapshot::from_capture(
            build,
            AppRuntimeMode::Production,
            AppRuntimeCapture {
                host_locale: "en".to_owned(),
                operating_system: "linux".to_owned(),
                run_id: "run-production-123-pid456".to_owned(),
            },
        );

        assert_eq!(snapshot.host.app_build, "release");
        assert_eq!(runtime_mode_label(&snapshot.runtime_mode), "production");
    }
}
