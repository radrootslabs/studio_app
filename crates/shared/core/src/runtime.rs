use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const APP_ID: &str = "org.radroots.app";
pub const APP_NAME: &str = "Radroots";
pub const APP_PLATFORM_RUNTIME: &str = "app-macos-native";
pub const APP_PROJECTION_SOURCE: &str = "gpui-native";
pub const APP_RUNTIME_ORIGIN: &str = "gpui://localhost";
pub const APP_HOST_PLATFORM: &str = "desktop";
pub const APP_RUNTIME_CONFIG_ENV: &str = "RADROOTS_APP_RUNTIME_CONFIG_JSON";
pub const APP_RUNTIME_CONFIG_SCHEMA: &str = "radroots.app.runtime-config.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppRuntimeMode {
    LocalhostDev,
    Development,
    Production,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppRuntimeConfig {
    pub runtime_mode: AppRuntimeMode,
    pub run_id: String,
    pub default_nostr_relay_url: String,
    pub bundle_identifier: String,
    pub bundle_name: String,
    pub marketing_version: String,
    pub build_number: String,
    pub platform_name: String,
    pub operating_system_version: String,
    pub host_locale: String,
    pub runtime_origin: String,
    pub local_log_root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppBuildIdentity {
    pub package_name: String,
    pub package_version: String,
    pub build_profile: String,
    pub target_triple: String,
    pub projection_source: String,
    pub git_commit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppCoreRuntimeMetadata {
    pub package_name: String,
    pub package_version: String,
    pub package_authors: String,
    pub rust_edition: String,
    pub rust_toolchain: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Debug, Error)]
pub enum AppRuntimeConfigError {
    #[error("missing required runtime config env: {APP_RUNTIME_CONFIG_ENV}")]
    MissingEnv,
    #[error("invalid app runtime config json")]
    InvalidJson(#[source] serde_json::Error),
    #[error("unsupported app runtime config schema: {0}")]
    UnsupportedSchema(String),
    #[error("unsupported runtime mode: {0}")]
    UnsupportedRuntimeMode(String),
    #[error("missing required runtime config field: {0}")]
    MissingField(&'static str),
}

#[derive(Deserialize)]
struct RawRuntimeConfig {
    schema_version: String,
    runtime_mode: String,
    run_id: String,
    default_nostr_relay_url: String,
    bundle_identifier: String,
    bundle_name: String,
    marketing_version: String,
    build_number: String,
    platform_name: String,
    operating_system_version: String,
    host_locale: String,
    runtime_origin: String,
    local_log_root: String,
}

impl AppRuntimeConfig {
    pub fn from_env() -> Result<Self, AppRuntimeConfigError> {
        let raw =
            std::env::var(APP_RUNTIME_CONFIG_ENV).map_err(|_| AppRuntimeConfigError::MissingEnv)?;
        Self::from_json_str(&raw)
    }

    pub fn from_json_str(raw: &str) -> Result<Self, AppRuntimeConfigError> {
        let config: RawRuntimeConfig =
            serde_json::from_str(raw).map_err(AppRuntimeConfigError::InvalidJson)?;

        if config.schema_version != APP_RUNTIME_CONFIG_SCHEMA {
            return Err(AppRuntimeConfigError::UnsupportedSchema(
                config.schema_version,
            ));
        }

        Ok(Self {
            runtime_mode: parse_config_runtime_mode(&config.runtime_mode)?,
            run_id: require_value("run_id", config.run_id)?,
            default_nostr_relay_url: require_value(
                "default_nostr_relay_url",
                config.default_nostr_relay_url,
            )?,
            bundle_identifier: require_value("bundle_identifier", config.bundle_identifier)?,
            bundle_name: require_value("bundle_name", config.bundle_name)?,
            marketing_version: require_value("marketing_version", config.marketing_version)?,
            build_number: require_value("build_number", config.build_number)?,
            platform_name: require_value("platform_name", config.platform_name)?,
            operating_system_version: require_value(
                "operating_system_version",
                config.operating_system_version,
            )?,
            host_locale: require_value("host_locale", config.host_locale)?,
            runtime_origin: require_value("runtime_origin", config.runtime_origin)?,
            local_log_root: require_path_value("local_log_root", config.local_log_root)?,
        })
    }
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
        let mode = parse_build_runtime_mode(&build.build_profile);
        Self::from_capture(build, mode, AppRuntimeCapture::current(&mode))
    }

    pub fn from_config(build: AppBuildIdentity, config: &AppRuntimeConfig) -> Self {
        Self {
            title: APP_NAME.to_owned(),
            runtime_mode: config.runtime_mode,
            run_id: config.run_id.clone(),
            core: AppCoreRuntimeMetadata {
                package_name: env!("CARGO_PKG_NAME").to_owned(),
                package_version: env!("CARGO_PKG_VERSION").to_owned(),
                package_authors: env!("CARGO_PKG_AUTHORS").to_owned(),
                rust_edition: "2024".to_owned(),
                rust_toolchain: env!("CARGO_PKG_RUST_VERSION").to_owned(),
            },
            build,
            host: AppHostRuntimeMetadata {
                app_identifier: config.bundle_identifier.clone(),
                app_name: config.bundle_name.clone(),
                app_version: config.marketing_version.clone(),
                app_build: config.build_number.clone(),
                platform_name: config.platform_name.clone(),
                operating_system: config.operating_system_version.clone(),
                host_locale: config.host_locale.clone(),
                runtime_origin: config.runtime_origin.clone(),
            },
        }
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
        AppRuntimeMode::LocalhostDev => "localhost-dev",
        AppRuntimeMode::Development => "development",
        AppRuntimeMode::Production => "production",
    }
}

fn parse_build_runtime_mode(build_profile: &str) -> AppRuntimeMode {
    match build_profile.trim() {
        "release" => AppRuntimeMode::Production,
        _ => AppRuntimeMode::Development,
    }
}

fn parse_config_runtime_mode(value: &str) -> Result<AppRuntimeMode, AppRuntimeConfigError> {
    match value.trim() {
        "localhost-dev" => Ok(AppRuntimeMode::LocalhostDev),
        "development" => Ok(AppRuntimeMode::Development),
        "production" => Ok(AppRuntimeMode::Production),
        other => Err(AppRuntimeConfigError::UnsupportedRuntimeMode(
            other.to_owned(),
        )),
    }
}

fn require_path_value(
    field: &'static str,
    value: String,
) -> Result<PathBuf, AppRuntimeConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppRuntimeConfigError::MissingField(field));
    }

    Ok(PathBuf::from(trimmed))
}

fn require_value(field: &'static str, value: String) -> Result<String, AppRuntimeConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppRuntimeConfigError::MissingField(field));
    }

    Ok(trimmed.to_owned())
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
    use std::path::PathBuf;

    use super::{
        APP_HOST_PLATFORM, APP_ID, APP_NAME, APP_PROJECTION_SOURCE, APP_RUNTIME_CONFIG_SCHEMA,
        APP_RUNTIME_ORIGIN, AppBuildIdentity, AppRuntimeCapture, AppRuntimeConfig,
        AppRuntimeConfigError, AppRuntimeMode, AppRuntimeSnapshot, runtime_mode_label,
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

    fn test_runtime_config_json() -> String {
        format!(
            r#"{{
                "schema_version":"{APP_RUNTIME_CONFIG_SCHEMA}",
                "runtime_mode":"localhost-dev",
                "run_id":"run-localhost-dev-20260417T000000Z-deadbeefcafefeed",
                "default_nostr_relay_url":"ws://127.0.0.1:8080",
                "bundle_identifier":"org.radroots.app.macos",
                "bundle_name":"Radroots",
                "marketing_version":"0.1.0",
                "build_number":"dev",
                "platform_name":"macos",
                "operating_system_version":"macos-15.5",
                "host_locale":"en_US.UTF-8",
                "runtime_origin":"gpui://localhost",
                "local_log_root":"/tmp/radroots/logs"
            }}"#
        )
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
    fn runtime_config_requires_supported_schema() {
        let error = AppRuntimeConfig::from_json_str(
            r#"{"schema_version":"unsupported","runtime_mode":"localhost-dev","run_id":"x","default_nostr_relay_url":"ws://127.0.0.1:8080","bundle_identifier":"y","bundle_name":"z","marketing_version":"0.1.0","build_number":"1","platform_name":"macos","operating_system_version":"macos-15.5","host_locale":"en","runtime_origin":"gpui://localhost","local_log_root":"/tmp/logs"}"#,
        )
        .expect_err("schema mismatch should fail");

        assert!(
            error
                .to_string()
                .contains("unsupported app runtime config schema")
        );
    }

    #[test]
    fn runtime_config_surfaces_explicit_local_log_root() {
        let config =
            AppRuntimeConfig::from_json_str(&test_runtime_config_json()).expect("valid config");

        assert_eq!(config.runtime_mode, AppRuntimeMode::LocalhostDev);
        assert_eq!(config.default_nostr_relay_url, "ws://127.0.0.1:8080");
        assert_eq!(config.bundle_identifier, "org.radroots.app.macos");
        assert_eq!(config.local_log_root, PathBuf::from("/tmp/radroots/logs"));
    }

    #[test]
    fn runtime_snapshot_uses_explicit_runtime_config_host_identity() {
        let snapshot = AppRuntimeSnapshot::from_config(
            test_build_identity(),
            &AppRuntimeConfig::from_json_str(&test_runtime_config_json()).expect("valid config"),
        );

        assert_eq!(
            snapshot.run_id,
            "run-localhost-dev-20260417T000000Z-deadbeefcafefeed"
        );
        assert_eq!(snapshot.host.app_identifier, "org.radroots.app.macos");
        assert_eq!(snapshot.host.platform_name, "macos");
        assert_eq!(snapshot.host.operating_system, "macos-15.5");
        assert_eq!(runtime_mode_label(&snapshot.runtime_mode), "localhost-dev");
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

    #[test]
    fn runtime_snapshot_capture_matches_canonical_runtime_config_metadata() {
        let build = test_build_identity();
        let git_commit = build
            .git_commit
            .clone()
            .expect("test build identity should include git commit");
        let snapshot_from_capture = AppRuntimeSnapshot::from_capture(
            build.clone(),
            AppRuntimeMode::Development,
            AppRuntimeCapture {
                host_locale: "en_US.UTF-8".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "run-development-123-pid456".to_owned(),
            },
        );
        let snapshot_from_config = AppRuntimeSnapshot::from_config(
            build.clone(),
            &AppRuntimeConfig::from_json_str(&format!(
                r#"{{
                    "schema_version":"{APP_RUNTIME_CONFIG_SCHEMA}",
                    "runtime_mode":"development",
                    "run_id":"run-development-123-pid456",
                    "default_nostr_relay_url":"ws://127.0.0.1:8080",
                    "bundle_identifier":"{APP_ID}",
                    "bundle_name":"{APP_NAME}",
                    "marketing_version":"{}",
                    "build_number":"{}",
                    "platform_name":"{APP_HOST_PLATFORM}",
                    "operating_system_version":"macos",
                    "host_locale":"en_US.UTF-8",
                    "runtime_origin":"{APP_RUNTIME_ORIGIN}",
                    "local_log_root":"/tmp/radroots/logs"
                }}"#,
                build.package_version, git_commit
            ))
            .expect("canonical config should parse"),
        );

        assert_eq!(snapshot_from_config, snapshot_from_capture);
    }

    #[test]
    fn runtime_config_rejects_empty_required_fields() {
        let error = AppRuntimeConfig::from_json_str(&format!(
            r#"{{
                "schema_version":"{APP_RUNTIME_CONFIG_SCHEMA}",
                "runtime_mode":"localhost-dev",
                "run_id":"",
                "default_nostr_relay_url":"ws://127.0.0.1:8080",
                "bundle_identifier":"org.radroots.app.macos",
                "bundle_name":"Radroots",
                "marketing_version":"0.1.0",
                "build_number":"dev",
                "platform_name":"macos",
                "operating_system_version":"macos-15.5",
                "host_locale":"en_US.UTF-8",
                "runtime_origin":"gpui://localhost",
                "local_log_root":"/tmp/radroots/logs"
            }}"#
        ))
        .expect_err("missing run id should fail");

        assert!(
            matches!(error, AppRuntimeConfigError::MissingField("run_id")),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn runtime_config_rejects_missing_default_nostr_relay_url() {
        let error = AppRuntimeConfig::from_json_str(&format!(
            r#"{{
                "schema_version":"{APP_RUNTIME_CONFIG_SCHEMA}",
                "runtime_mode":"localhost-dev",
                "run_id":"run-localhost-dev-20260417T000000Z-deadbeefcafefeed",
                "default_nostr_relay_url":"",
                "bundle_identifier":"org.radroots.app.macos",
                "bundle_name":"Radroots",
                "marketing_version":"0.1.0",
                "build_number":"dev",
                "platform_name":"macos",
                "operating_system_version":"macos-15.5",
                "host_locale":"en_US.UTF-8",
                "runtime_origin":"gpui://localhost",
                "local_log_root":"/tmp/radroots/logs"
            }}"#
        ))
        .expect_err("missing default relay url should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::MissingField("default_nostr_relay_url")
            ),
            "unexpected error: {error}"
        );
    }
}
