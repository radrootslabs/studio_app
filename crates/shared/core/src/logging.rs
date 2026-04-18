use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber, info};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    EnvFilter, fmt as tracing_fmt,
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    prelude::*,
    registry::LookupSpan,
};

use crate::{
    APP_PLATFORM_RUNTIME, AppBuildIdentity, AppCoreRuntimeMetadata, AppHostRuntimeMetadata,
};
use crate::{AppRuntimeSnapshot, runtime_mode_label};

pub const APP_LOG_SCHEMA_VERSION: &str = "radroots.app.log.v1";
pub const APP_LOG_PRODUCT: &str = "app";

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static TRACING_INSTALLED: OnceLock<()> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppLoggingOptions {
    pub log_dir: PathBuf,
    pub snapshot: AppRuntimeSnapshot,
    pub stdout: bool,
    pub default_level: String,
}

#[derive(Debug, Error)]
pub enum AppLoggingError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    AppenderInit(#[from] tracing_appender::rolling::InitError),
    #[error(transparent)]
    TracingInit(#[from] tracing_subscriber::util::TryInitError),
}

#[derive(Clone, Debug)]
struct StructuredLogFormatter {
    snapshot: AppRuntimeSnapshot,
}

#[derive(Debug, Default)]
struct StructuredFieldVisitor {
    event_name: Option<String>,
    message: Option<String>,
    metadata: Map<String, Value>,
}

#[derive(Serialize)]
struct AppLogRecord<'a> {
    timestamp: String,
    schema_version: &'static str,
    product: &'static str,
    category: String,
    event: String,
    level: &'static str,
    message: String,
    runtime_mode: &'static str,
    run_id: &'a str,
    platform_runtime: &'static str,
    core: &'a AppCoreRuntimeMetadata,
    build: &'a AppBuildIdentity,
    host: &'a AppHostRuntimeMetadata,
    metadata: Map<String, Value>,
}

impl AppLoggingOptions {
    pub fn localhost_dev(snapshot: AppRuntimeSnapshot, local_log_root: &Path) -> Self {
        Self {
            log_dir: app_runtime_log_dir(local_log_root),
            snapshot,
            stdout: true,
            default_level: "info".to_owned(),
        }
    }
}

impl StructuredFieldVisitor {
    fn record_value(&mut self, field: &Field, value: Value) {
        match field.name() {
            "message" => {
                self.message = match value {
                    Value::String(message) => Some(message),
                    other => Some(other.to_string()),
                };
            }
            "event" => {
                self.event_name = match value {
                    Value::String(event_name) => Some(event_name),
                    other => Some(other.to_string()),
                };
            }
            _ => {
                self.metadata.insert(field.name().to_owned(), value);
            }
        }
    }
}

impl Visit for StructuredFieldVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, Value::from(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record_value(field, Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, Value::String(value.to_owned()));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record_value(field, Value::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, Value::String(format!("{value:?}")));
    }
}

impl<S, N> FormatEvent<S, N> for StructuredLogFormatter
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut visitor = StructuredFieldVisitor::default();
        event.record(&mut visitor);

        let record = AppLogRecord {
            timestamp: structured_timestamp(),
            schema_version: APP_LOG_SCHEMA_VERSION,
            product: APP_LOG_PRODUCT,
            category: target_category(event.metadata().target()),
            event: visitor
                .event_name
                .unwrap_or_else(|| format!("{}.log", target_category(event.metadata().target()))),
            level: level_label(event.metadata().level()),
            message: visitor.message.unwrap_or_default(),
            runtime_mode: runtime_mode_label(&self.snapshot.runtime_mode),
            run_id: &self.snapshot.run_id,
            platform_runtime: APP_PLATFORM_RUNTIME,
            core: &self.snapshot.core,
            build: &self.snapshot.build,
            host: &self.snapshot.host,
            metadata: visitor.metadata,
        };
        let payload = serde_json::to_string(&record).map_err(|_| fmt::Error)?;

        writeln!(writer, "{payload}")
    }
}

pub fn app_runtime_log_dir(local_log_root: &Path) -> PathBuf {
    local_log_root
        .join("apps")
        .join("local")
        .join(APP_LOG_PRODUCT)
        .join(APP_PLATFORM_RUNTIME)
}

pub fn bootstrap_logging(
    snapshot: &AppRuntimeSnapshot,
    local_log_root: &Path,
) -> Result<PathBuf, AppLoggingError> {
    let options = AppLoggingOptions::localhost_dev(snapshot.clone(), local_log_root);
    let log_dir = options.log_dir.clone();
    init_logging(options)?;
    Ok(log_dir)
}

pub fn init_logging(options: AppLoggingOptions) -> Result<(), AppLoggingError> {
    if TRACING_INSTALLED.get().is_some() {
        return Ok(());
    }

    fs::create_dir_all(&options.log_dir)?;
    prepare_latest_alias(&options.log_dir)?;

    let file_appender = build_file_appender(&options.log_dir)?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let formatter = StructuredLogFormatter {
        snapshot: options.snapshot.clone(),
    };
    let file_layer = tracing_fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .event_format(formatter.clone());
    let stdout_layer = options.stdout.then(|| {
        tracing_fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(false)
            .event_format(formatter)
    });

    tracing_subscriber::registry()
        .with(resolve_env_filter(options.default_level.as_str()))
        .with(file_layer)
        .with(stdout_layer)
        .try_init()?;
    let _ = TRACING_INSTALLED.set(());

    info!(
        target: "runtime",
        event = "logging.initialized",
        file = %options.log_dir.join(format!("{}.jsonl", current_utc_day())).display(),
        stdout = options.stdout,
        "logging initialized"
    );

    Ok(())
}

pub fn install_panic_hook() {
    if PANIC_HOOK_INSTALLED.set(()).is_err() {
        return;
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(
            target: "panic",
            event = "runtime.panic",
            panic = %render_panic_payload(panic_info),
            location = %render_panic_location(panic_info),
            "panic captured"
        );
        default_hook(panic_info);
    }));
}

fn build_file_appender(log_dir: &Path) -> Result<RollingFileAppender, AppLoggingError> {
    Ok(RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix("jsonl")
        .build(log_dir)?)
}

fn current_utc_day() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn level_label(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "error",
        Level::WARN => "warning",
        Level::INFO => "info",
        Level::DEBUG => "debug",
        Level::TRACE => "debug",
    }
}

fn prepare_latest_alias(log_dir: &Path) -> Result<(), AppLoggingError> {
    let latest_path = log_dir.join("latest.jsonl");
    match fs::symlink_metadata(&latest_path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || metadata.is_file() {
                fs::remove_file(&latest_path)?;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(format!("{}.jsonl", current_utc_day()), &latest_path)?;

    #[cfg(not(unix))]
    fs::write(&latest_path, [])?;

    Ok(())
}

fn render_panic_location(panic_info: &std::panic::PanicHookInfo<'_>) -> String {
    panic_info
        .location()
        .map(|location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        })
        .unwrap_or_else(|| "<unknown>".to_owned())
}

fn render_panic_payload(panic_info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
        (*payload).to_owned()
    } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
        payload.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn resolve_env_filter(default_level: &str) -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level))
}

fn structured_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn target_category(target: &str) -> String {
    if target.is_empty() {
        return "runtime".to_owned();
    }

    target
        .rsplit("::")
        .next()
        .unwrap_or(target)
        .trim()
        .trim_matches(':')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{SecondsFormat, Utc};
    use serde_json::json;

    use crate::{
        APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeCapture, AppRuntimeMode,
        AppRuntimeSnapshot,
    };

    use super::{
        APP_LOG_PRODUCT, APP_LOG_SCHEMA_VERSION, app_runtime_log_dir, prepare_latest_alias,
    };

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("radroots-app-log-{name}-{nanos}"));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn test_snapshot() -> AppRuntimeSnapshot {
        AppRuntimeSnapshot::from_capture(
            AppBuildIdentity {
                package_name: "radroots_studio_app".to_owned(),
                package_version: "0.1.0".to_owned(),
                build_profile: "debug".to_owned(),
                target_triple: "aarch64-apple-darwin".to_owned(),
                projection_source: APP_PROJECTION_SOURCE.to_owned(),
                git_commit: None,
            },
            AppRuntimeMode::LocalhostDev,
            AppRuntimeCapture {
                host_locale: "en_US.UTF-8".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "run-localhost-dev-123-pid456".to_owned(),
            },
        )
    }

    #[test]
    fn app_runtime_log_dir_uses_canonical_local_layout() {
        let dir = app_runtime_log_dir(Path::new("/tmp/repo/logs"));

        assert_eq!(
            dir,
            PathBuf::from("/tmp/repo/logs")
                .join("apps")
                .join("local")
                .join(APP_LOG_PRODUCT)
                .join("app-macos-native")
        );
    }

    #[cfg(unix)]
    #[test]
    fn prepare_latest_alias_tracks_current_day_log_file() {
        let dir = temp_dir("latest-alias");
        fs::create_dir_all(&dir).expect("create dir");

        prepare_latest_alias(&dir).expect("prepare latest alias");

        let target = fs::read_link(dir.join("latest.jsonl")).expect("read latest symlink");
        assert_eq!(
            target,
            PathBuf::from(format!("{}.jsonl", Utc::now().format("%Y-%m-%d")))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn structured_record_shape_remains_stable() {
        let snapshot = test_snapshot();
        let payload = serde_json::to_value(json!({
            "timestamp": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "schema_version": APP_LOG_SCHEMA_VERSION,
            "product": APP_LOG_PRODUCT,
            "category": "bootstrap",
            "event": "runtime.launch",
            "level": "info",
            "message": "app launch",
            "runtime_mode": "localhost-dev",
            "run_id": snapshot.run_id,
            "platform_runtime": "app-macos-native",
            "core": snapshot.core,
            "build": snapshot.build,
            "host": snapshot.host,
            "metadata": {
                "home_screen": "Radroots"
            }
        }))
        .expect("serialize");

        assert_eq!(payload["schema_version"], "radroots.app.log.v1");
        assert_eq!(payload["event"], "runtime.launch");
        assert_eq!(payload["platform_runtime"], "app-macos-native");
        assert_eq!(payload["metadata"]["home_screen"], "Radroots");
    }
}
