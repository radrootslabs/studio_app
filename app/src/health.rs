#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppHealthCheckStatus {
    Ok,
    Error,
    Skipped,
}

impl AppHealthCheckStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            AppHealthCheckStatus::Ok => "ok",
            AppHealthCheckStatus::Error => "error",
            AppHealthCheckStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHealthCheckResult {
    pub status: AppHealthCheckStatus,
    pub message: Option<String>,
}

impl AppHealthCheckResult {
    pub fn ok() -> Self {
        Self {
            status: AppHealthCheckStatus::Ok,
            message: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: AppHealthCheckStatus::Error,
            message: Some(message.into()),
        }
    }

    pub fn skipped() -> Self {
        Self {
            status: AppHealthCheckStatus::Skipped,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHealthReport {
    pub key_maps: AppHealthCheckResult,
    pub bootstrap_config: AppHealthCheckResult,
    pub bootstrap_app_data: AppHealthCheckResult,
    pub datastore_roundtrip: AppHealthCheckResult,
    pub keystore: AppHealthCheckResult,
}

impl Default for AppHealthReport {
    fn default() -> Self {
        Self {
            key_maps: AppHealthCheckResult::skipped(),
            bootstrap_config: AppHealthCheckResult::skipped(),
            bootstrap_app_data: AppHealthCheckResult::skipped(),
            datastore_roundtrip: AppHealthCheckResult::skipped(),
            keystore: AppHealthCheckResult::skipped(),
        }
    }
}

impl AppHealthReport {
    pub fn empty() -> Self {
        Self::default()
    }
}

use crate::{app_key_maps_validate, AppKeyMapConfig};

pub fn app_health_check_key_maps(key_maps: &AppKeyMapConfig) -> AppHealthCheckResult {
    match app_key_maps_validate(key_maps) {
        Ok(()) => AppHealthCheckResult::ok(),
        Err(err) => AppHealthCheckResult::error(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_health_check_key_maps,
        AppHealthCheckResult,
        AppHealthCheckStatus,
        AppHealthReport,
    };
    use crate::AppKeyMapConfig;

    #[test]
    fn health_status_as_str() {
        assert_eq!(AppHealthCheckStatus::Ok.as_str(), "ok");
        assert_eq!(AppHealthCheckStatus::Error.as_str(), "error");
        assert_eq!(AppHealthCheckStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn health_result_constructors() {
        let ok = AppHealthCheckResult::ok();
        assert_eq!(ok.status, AppHealthCheckStatus::Ok);
        assert!(ok.message.is_none());

        let err = AppHealthCheckResult::error("boom");
        assert_eq!(err.status, AppHealthCheckStatus::Error);
        assert_eq!(err.message.as_deref(), Some("boom"));
    }

    #[test]
    fn health_report_defaults_skipped() {
        let report = AppHealthReport::default();
        assert_eq!(report.key_maps.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_config.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.bootstrap_app_data.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.datastore_roundtrip.status, AppHealthCheckStatus::Skipped);
        assert_eq!(report.keystore.status, AppHealthCheckStatus::Skipped);
    }

    #[test]
    fn health_check_key_maps_reports_errors() {
        let empty = AppKeyMapConfig::empty();
        let result = app_health_check_key_maps(&empty);
        assert_eq!(result.status, AppHealthCheckStatus::Error);
        assert_eq!(
            result.message.as_deref(),
            Some("error.app.config.key_map_missing")
        );
    }
}
