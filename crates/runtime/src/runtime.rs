use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use radroots_sdk::{NostrRelayUrlPolicy, TargetSet};
use serde::Serialize;
use thiserror::Error;

use crate::{AppRuntimePathsError, AppRuntimeRoots};

pub const APP_ID: &str = "org.radroots.app";
pub const APP_NAME: &str = "Radroots";
pub const APP_PLATFORM_RUNTIME: &str = "app-macos-native";
pub const APP_PROJECTION_SOURCE: &str = "gpui-native";
pub const APP_RUNTIME_ORIGIN: &str = "gpui://localhost";
pub const APP_HOST_PLATFORM: &str = "desktop";
pub const APP_RUNTIME_MODE_ENV: &str = "RADROOTS_APP_RUNTIME_MODE";
pub const APP_NOSTR_RELAY_URLS_ENV: &str = "RADROOTS_APP_NOSTR_RELAY_URLS";
pub const APP_LOCAL_LOG_ROOT_ENV: &str = "RADROOTS_APP_LOCAL_LOG_ROOT";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppRuntimeMode {
    LocalhostDev,
    Development,
    Production,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppRuntimeConfig {
    pub runtime_mode: AppRuntimeMode,
    pub nostr_relay_urls: Vec<String>,
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
    #[error(transparent)]
    RuntimePaths(#[from] AppRuntimePathsError),
    #[error("missing required runtime env: {0}")]
    MissingEnv(&'static str),
    #[error("unsupported runtime mode: {0}")]
    UnsupportedRuntimeMode(String),
    #[error("missing required runtime config field: {0}")]
    MissingField(&'static str),
    #[error("invalid runtime relay url in {field}: {value}")]
    InvalidRelayUrl { field: &'static str, value: String },
}

impl AppRuntimeConfig {
    pub fn from_env() -> Result<Self, AppRuntimeConfigError> {
        Self::from_env_with(|name| std::env::var(name).ok(), None)
    }

    fn from_env_with<F>(
        mut read_env: F,
        default_log_root: Option<PathBuf>,
    ) -> Result<Self, AppRuntimeConfigError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let runtime_mode =
            parse_config_runtime_mode(&require_env_value(&mut read_env, APP_RUNTIME_MODE_ENV)?)?;
        let nostr_relay_urls = parse_relay_url_set(
            APP_NOSTR_RELAY_URLS_ENV,
            require_env_value(&mut read_env, APP_NOSTR_RELAY_URLS_ENV)?,
        )?;
        let local_log_root = read_env(APP_LOCAL_LOG_ROOT_ENV)
            .map(|value| require_path_value(APP_LOCAL_LOG_ROOT_ENV, value))
            .transpose()?;
        let local_log_root = match local_log_root {
            Some(local_log_root) => local_log_root,
            None => match default_log_root {
                Some(default_log_root) => default_log_root,
                None => AppRuntimeRoots::current_desktop()?.logs,
            },
        };

        Ok(Self {
            runtime_mode,
            nostr_relay_urls,
            local_log_root,
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
        Self::capture_for_mode(build, mode)
    }

    pub fn capture_for_mode(build: AppBuildIdentity, runtime_mode: AppRuntimeMode) -> Self {
        Self::from_capture(
            build,
            runtime_mode,
            AppRuntimeCapture::current(&runtime_mode),
        )
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

fn parse_relay_url_set(
    field: &'static str,
    value: String,
) -> Result<Vec<String>, AppRuntimeConfigError> {
    let mut relays = Vec::new();
    for relay in value.split(',') {
        let relay = relay.trim();
        if relay.is_empty() {
            return Err(AppRuntimeConfigError::InvalidRelayUrl {
                field,
                value: relay.to_owned(),
            });
        }
        let normalized = normalize_app_relay_url(field, relay).map_err(|_| {
            AppRuntimeConfigError::InvalidRelayUrl {
                field,
                value: relay.to_owned(),
            }
        })?;
        if !relays.iter().any(|existing| existing == &normalized) {
            relays.push(normalized);
        }
    }

    if relays.is_empty() {
        return Err(AppRuntimeConfigError::MissingField(field));
    }

    Ok(relays)
}

fn normalize_app_relay_url(
    field: &'static str,
    relay: &str,
) -> Result<String, AppRuntimeConfigError> {
    TargetSet::nostr_relays([relay], NostrRelayUrlPolicy::Localhost)
        .map(|targets| {
            targets
                .nostr_relay_urls()
                .into_iter()
                .next()
                .expect("single relay target set must contain one relay")
        })
        .map_err(|_| AppRuntimeConfigError::InvalidRelayUrl {
            field,
            value: relay.to_owned(),
        })
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

fn require_env_value<F>(
    read_env: &mut F,
    field: &'static str,
) -> Result<String, AppRuntimeConfigError>
where
    F: FnMut(&str) -> Option<String>,
{
    let value = read_env(field).ok_or(AppRuntimeConfigError::MissingEnv(field))?;
    require_value(field, value)
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
    use std::{collections::BTreeMap, path::PathBuf};

    use super::{
        APP_HOST_PLATFORM, APP_ID, APP_LOCAL_LOG_ROOT_ENV, APP_NAME, APP_NOSTR_RELAY_URLS_ENV,
        APP_PROJECTION_SOURCE, APP_RUNTIME_MODE_ENV, APP_RUNTIME_ORIGIN, AppBuildIdentity,
        AppRuntimeCapture, AppRuntimeConfig, AppRuntimeConfigError, AppRuntimeMode,
        AppRuntimeSnapshot, runtime_mode_label,
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

    fn test_runtime_env() -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (
                APP_NOSTR_RELAY_URLS_ENV,
                " ws://127.0.0.1:8080 , ws://127.0.0.1:8081 , ws://127.0.0.1:8080 ".to_owned(),
            ),
        ])
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
    fn runtime_config_requires_explicit_runtime_mode_env() {
        let env = BTreeMap::from([(APP_NOSTR_RELAY_URLS_ENV, "ws://127.0.0.1:8080".to_owned())]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("missing runtime mode env should fail");

        assert!(matches!(
            error,
            AppRuntimeConfigError::MissingEnv(APP_RUNTIME_MODE_ENV)
        ));
    }

    #[test]
    fn runtime_config_surfaces_explicit_local_log_root() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "ws://127.0.0.1:8080".to_owned()),
            (APP_LOCAL_LOG_ROOT_ENV, "/tmp/radroots/logs".to_owned()),
        ]);
        let config = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect("valid env config");

        assert_eq!(config.runtime_mode, AppRuntimeMode::LocalhostDev);
        assert_eq!(config.nostr_relay_urls, vec!["ws://127.0.0.1:8080"]);
        assert_eq!(config.local_log_root, PathBuf::from("/tmp/radroots/logs"));
    }

    #[test]
    fn runtime_config_normalizes_configured_nostr_relay_urls() {
        let env = test_runtime_env();
        let config = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect("valid env config");

        assert_eq!(
            config.nostr_relay_urls,
            vec!["ws://127.0.0.1:8080", "ws://127.0.0.1:8081"]
        );
    }

    #[test]
    fn runtime_config_rejects_malformed_nostr_relay_urls() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "not-a-url".to_owned()),
        ]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("malformed relay url should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::InvalidRelayUrl {
                    field: APP_NOSTR_RELAY_URLS_ENV,
                    ref value
                } if value == "not-a-url"
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn runtime_config_rejects_non_websocket_nostr_relay_urls() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "https://relay.example".to_owned()),
        ]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("non-websocket relay url should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::InvalidRelayUrl {
                    field: APP_NOSTR_RELAY_URLS_ENV,
                    ref value
                } if value == "https://relay.example"
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn runtime_config_rejects_hostless_nostr_relay_urls() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "wss://".to_owned()),
        ]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("hostless relay url should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::InvalidRelayUrl {
                    field: APP_NOSTR_RELAY_URLS_ENV,
                    ref value
                } if value == "wss://"
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn runtime_config_rejects_malformed_nostr_relay_authority() {
        for relay_url in [
            "wss://user@relay.example",
            "wss://relay.example:abc",
            "wss://2001:db8::1",
            "wss://relay.example,,wss://relay-two.example",
        ] {
            let env = BTreeMap::from([
                (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
                (APP_NOSTR_RELAY_URLS_ENV, relay_url.to_owned()),
            ]);
            let error = AppRuntimeConfig::from_env_with(
                |name| env.get(name).cloned(),
                Some(PathBuf::from("/tmp/default-logs")),
            )
            .expect_err("malformed relay authority should fail");

            assert!(
                matches!(
                    error,
                    AppRuntimeConfigError::InvalidRelayUrl {
                        field: APP_NOSTR_RELAY_URLS_ENV,
                        ..
                    }
                ),
                "unexpected error for {relay_url}: {error}"
            );
        }
    }

    #[test]
    fn runtime_config_accepts_bracketed_ipv6_nostr_relay_urls() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (
                APP_NOSTR_RELAY_URLS_ENV,
                " wss://[2001:db8::1]:443/relay ".to_owned(),
            ),
        ]);
        let config = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect("ipv6 relay url should resolve");

        assert_eq!(config.nostr_relay_urls, vec!["wss://[2001:db8::1]/relay"]);
    }

    #[test]
    fn runtime_config_defaults_local_log_root_from_runtime_paths() {
        let env = test_runtime_env();
        let config = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect("default log root should apply");

        assert_eq!(config.local_log_root, PathBuf::from("/tmp/default-logs"));
    }

    #[test]
    fn runtime_config_accepts_explicit_log_root_without_default_runtime_paths() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "ws://127.0.0.1:8080".to_owned()),
            (APP_LOCAL_LOG_ROOT_ENV, "/tmp/explicit-logs".to_owned()),
        ]);
        let config = AppRuntimeConfig::from_env_with(|name| env.get(name).cloned(), None)
            .expect("explicit local log root should bypass runtime root discovery");

        assert_eq!(config.local_log_root, PathBuf::from("/tmp/explicit-logs"));
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
    fn runtime_snapshot_capture_for_mode_uses_rust_owned_host_identity() {
        let snapshot = AppRuntimeSnapshot::capture_for_mode(
            test_build_identity(),
            AppRuntimeMode::LocalhostDev,
        );

        assert_eq!(snapshot.title, APP_NAME);
        assert!(snapshot.run_id.starts_with("run-localhost-dev-"));
        assert!(snapshot.run_id.contains("-pid"));
        assert_eq!(snapshot.host.app_identifier, APP_ID);
        assert_eq!(snapshot.host.app_name, APP_NAME);
        assert_eq!(snapshot.host.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(snapshot.host.platform_name, APP_HOST_PLATFORM);
        assert_eq!(snapshot.host.operating_system, std::env::consts::OS);
        assert_eq!(snapshot.host.runtime_origin, APP_RUNTIME_ORIGIN);
        assert!(!snapshot.host.host_locale.trim().is_empty());
    }

    #[test]
    fn runtime_config_rejects_empty_required_fields() {
        let env = BTreeMap::from([
            (APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned()),
            (APP_NOSTR_RELAY_URLS_ENV, "".to_owned()),
        ]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("missing relay env should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::MissingField(APP_NOSTR_RELAY_URLS_ENV)
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn runtime_config_rejects_missing_nostr_relay_urls() {
        let env = BTreeMap::from([(APP_RUNTIME_MODE_ENV, "localhost-dev".to_owned())]);
        let error = AppRuntimeConfig::from_env_with(
            |name| env.get(name).cloned(),
            Some(PathBuf::from("/tmp/default-logs")),
        )
        .expect_err("missing relay urls should fail");

        assert!(
            matches!(
                error,
                AppRuntimeConfigError::MissingEnv(APP_NOSTR_RELAY_URLS_ENV)
            ),
            "unexpected error: {error}"
        );
    }
}
