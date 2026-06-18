use std::{
    fmt, io,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use radroots_sdk::{
    RadrootsSdk, RadrootsSdkError, RadrootsSdkStoragePaths,
    SdkRelayUrlPolicy as SdkRuntimeRelayUrlPolicy,
};
use serde_json::{Value, json};
use thiserror::Error;
use tokio::runtime::Builder as TokioRuntimeBuilder;

use crate::AppDesktopRuntimePaths;

pub const APP_SDK_STORAGE_DIR_NAME: &str = "sdk";
pub const APP_SDK_DEFAULT_COMMAND_QUEUE_CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkRelayUrlPolicy {
    Public,
    Localhost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppSdkLifecycleState {
    Starting,
    Ready,
    Degraded,
    Pausing,
    Paused,
    Restoring,
    RebuildingProjections,
    ShuttingDown,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkConfig {
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub command_queue_capacity: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSdkStoragePaths {
    pub event_store_path: PathBuf,
    pub outbox_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRuntimeIssue {
    pub code: String,
    pub class: String,
    pub retryable: bool,
    pub message: String,
    pub recovery_actions: Vec<String>,
    pub detail_json: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSdkRuntimeStatus {
    pub state: AppSdkLifecycleState,
    pub storage_root: PathBuf,
    pub relay_urls: Vec<String>,
    pub relay_url_policy: AppSdkRelayUrlPolicy,
    pub storage_paths: Option<AppSdkStoragePaths>,
    pub last_issue: Option<AppSdkRuntimeIssue>,
}

#[derive(Debug, Error)]
pub enum AppSdkRuntimeError {
    #[error("app sdk command queue capacity must be greater than zero")]
    CommandQueueCapacityZero,
    #[error("failed to start app sdk worker: {0}")]
    WorkerSpawn(#[from] io::Error),
    #[error("app sdk command queue is full")]
    CommandQueueFull,
    #[error("app sdk command queue is closed")]
    CommandQueueClosed,
    #[error("app sdk shutdown acknowledgement failed")]
    ShutdownAck,
    #[error("app sdk worker failed to join")]
    WorkerJoin,
}

#[derive(Debug)]
pub struct AppSdkRuntime {
    command_sender: SyncSender<AppSdkWorkerCommand>,
    shared: Arc<AppSdkRuntimeShared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug)]
struct AppSdkRuntimeShared {
    status: Mutex<AppSdkRuntimeStatus>,
    status_changed: Condvar,
}

enum AppSdkWorkerCommand {
    Shutdown(mpsc::Sender<()>),
}

impl fmt::Debug for AppSdkWorkerCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shutdown(_) => formatter.write_str("Shutdown"),
        }
    }
}

impl AppSdkConfig {
    pub fn from_desktop_paths(paths: &AppDesktopRuntimePaths, relay_urls: Vec<String>) -> Self {
        Self::from_app_data_root(paths.app.data.as_path(), relay_urls)
    }

    pub fn from_app_data_root(data_root: &Path, relay_urls: Vec<String>) -> Self {
        Self {
            storage_root: app_sdk_storage_root_from_data_root(data_root),
            relay_url_policy: app_sdk_relay_url_policy(relay_urls.as_slice()),
            relay_urls,
            command_queue_capacity: APP_SDK_DEFAULT_COMMAND_QUEUE_CAPACITY,
        }
    }

    pub fn with_command_queue_capacity(mut self, capacity: usize) -> Self {
        self.command_queue_capacity = capacity;
        self
    }
}

impl AppSdkRuntime {
    pub fn start(config: AppSdkConfig) -> Result<Self, AppSdkRuntimeError> {
        if config.command_queue_capacity == 0 {
            return Err(AppSdkRuntimeError::CommandQueueCapacityZero);
        }

        let initial_status =
            AppSdkRuntimeStatus::from_config(&config, AppSdkLifecycleState::Starting, None, None);
        let shared = Arc::new(AppSdkRuntimeShared {
            status: Mutex::new(initial_status),
            status_changed: Condvar::new(),
        });
        let (command_sender, command_receiver) = mpsc::sync_channel(config.command_queue_capacity);
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("radroots-app-sdk-runtime".to_owned())
            .spawn(move || run_app_sdk_worker(config, worker_shared, command_receiver))?;

        Ok(Self {
            command_sender,
            shared,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn status(&self) -> AppSdkRuntimeStatus {
        lock_status(&self.shared).clone()
    }

    pub fn wait_for_startup(&self, timeout: Duration) -> AppSdkRuntimeStatus {
        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(Instant::now);
        let mut status = lock_status(&self.shared);
        loop {
            if !matches!(status.state, AppSdkLifecycleState::Starting) {
                return status.clone();
            }
            let now = Instant::now();
            if now >= deadline {
                return status.clone();
            }
            let remaining = deadline.saturating_duration_since(now);
            let wait_result = self.shared.status_changed.wait_timeout(status, remaining);
            let (next_status, timeout_result) = wait_result.unwrap_or_else(|poisoned| {
                let (guard, timeout_result) = poisoned.into_inner();
                (guard, timeout_result)
            });
            status = next_status;
            if timeout_result.timed_out() {
                return status.clone();
            }
        }
    }

    pub fn shutdown(&self) -> Result<(), AppSdkRuntimeError> {
        if matches!(self.status().state, AppSdkLifecycleState::Stopped) {
            return self.join_worker();
        }

        let (ack_sender, ack_receiver) = mpsc::channel();
        match self
            .command_sender
            .try_send(AppSdkWorkerCommand::Shutdown(ack_sender))
        {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => return Err(AppSdkRuntimeError::CommandQueueFull),
            Err(TrySendError::Disconnected(_)) => {
                transition_status_state(&self.shared, AppSdkLifecycleState::Stopped);
                return self.join_worker();
            }
        }
        ack_receiver
            .recv()
            .map_err(|_| AppSdkRuntimeError::ShutdownAck)?;
        self.join_worker()
    }

    fn join_worker(&self) -> Result<(), AppSdkRuntimeError> {
        let mut worker = self
            .worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(worker) = worker.take() else {
            return Ok(());
        };
        worker.join().map_err(|_| AppSdkRuntimeError::WorkerJoin)
    }
}

impl Drop for AppSdkRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl From<AppSdkRelayUrlPolicy> for SdkRuntimeRelayUrlPolicy {
    fn from(policy: AppSdkRelayUrlPolicy) -> Self {
        match policy {
            AppSdkRelayUrlPolicy::Public => Self::Public,
            AppSdkRelayUrlPolicy::Localhost => Self::Localhost,
        }
    }
}

impl From<&RadrootsSdkStoragePaths> for AppSdkStoragePaths {
    fn from(paths: &RadrootsSdkStoragePaths) -> Self {
        Self {
            event_store_path: paths.event_store_path.clone(),
            outbox_path: paths.outbox_path.clone(),
        }
    }
}

impl AppSdkRuntimeIssue {
    fn from_sdk_error(error: &RadrootsSdkError) -> Self {
        Self {
            code: error.code().to_owned(),
            class: sdk_error_class_label(error),
            retryable: error.retryable(),
            message: error.to_string(),
            recovery_actions: error
                .recovery_actions()
                .into_iter()
                .filter_map(|action| serde_json::to_value(action).ok())
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect(),
            detail_json: error.detail_json(),
        }
    }

    fn runtime_error(code: &'static str, message: String) -> Self {
        Self {
            code: code.to_owned(),
            class: "runtime".to_owned(),
            retryable: true,
            message: message.clone(),
            recovery_actions: vec!["retry_startup".to_owned()],
            detail_json: json!({
                "code": code,
                "class": "runtime",
                "retryable": true,
                "message": message,
                "recovery_actions": ["retry_startup"],
                "detail": {}
            }),
        }
    }
}

impl AppSdkRuntimeStatus {
    fn from_config(
        config: &AppSdkConfig,
        state: AppSdkLifecycleState,
        storage_paths: Option<AppSdkStoragePaths>,
        last_issue: Option<AppSdkRuntimeIssue>,
    ) -> Self {
        Self {
            state,
            storage_root: config.storage_root.clone(),
            relay_urls: config.relay_urls.clone(),
            relay_url_policy: config.relay_url_policy,
            storage_paths,
            last_issue,
        }
    }
}

pub fn app_sdk_storage_root_from_data_root(data_root: &Path) -> PathBuf {
    data_root.join(APP_SDK_STORAGE_DIR_NAME)
}

fn app_sdk_relay_url_policy(relay_urls: &[String]) -> AppSdkRelayUrlPolicy {
    if relay_urls
        .iter()
        .any(|relay_url| relay_url.trim().to_ascii_lowercase().starts_with("ws://"))
    {
        AppSdkRelayUrlPolicy::Localhost
    } else {
        AppSdkRelayUrlPolicy::Public
    }
}

fn run_app_sdk_worker(
    config: AppSdkConfig,
    shared: Arc<AppSdkRuntimeShared>,
    command_receiver: Receiver<AppSdkWorkerCommand>,
) {
    let runtime = match TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Degraded,
                    None,
                    Some(AppSdkRuntimeIssue::runtime_error(
                        "tokio_runtime_init",
                        error.to_string(),
                    )),
                ),
            );
            run_degraded_worker(config, shared, command_receiver);
            return;
        }
    };

    let mut sdk = match runtime.block_on(build_sdk_runtime(&config)) {
        Ok(sdk) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Ready,
                    sdk.storage_paths().map(AppSdkStoragePaths::from),
                    None,
                ),
            );
            Some(sdk)
        }
        Err(error) => {
            replace_status(
                &shared,
                AppSdkRuntimeStatus::from_config(
                    &config,
                    AppSdkLifecycleState::Degraded,
                    None,
                    Some(AppSdkRuntimeIssue::from_sdk_error(&error)),
                ),
            );
            None
        }
    };

    while let Ok(command) = command_receiver.recv() {
        match command {
            AppSdkWorkerCommand::Shutdown(ack_sender) => {
                transition_status_state(&shared, AppSdkLifecycleState::ShuttingDown);
                drop(sdk.take());
                transition_status_state(&shared, AppSdkLifecycleState::Stopped);
                let _ = ack_sender.send(());
                return;
            }
        }
    }

    drop(sdk.take());
    transition_status_state(&shared, AppSdkLifecycleState::Stopped);
}

fn run_degraded_worker(
    config: AppSdkConfig,
    shared: Arc<AppSdkRuntimeShared>,
    command_receiver: Receiver<AppSdkWorkerCommand>,
) {
    while let Ok(command) = command_receiver.recv() {
        match command {
            AppSdkWorkerCommand::Shutdown(ack_sender) => {
                transition_status_state(&shared, AppSdkLifecycleState::ShuttingDown);
                let last_issue = lock_status(&shared).last_issue.clone();
                replace_status(
                    &shared,
                    AppSdkRuntimeStatus::from_config(
                        &config,
                        AppSdkLifecycleState::Stopped,
                        None,
                        last_issue,
                    ),
                );
                let _ = ack_sender.send(());
                return;
            }
        }
    }

    let last_issue = lock_status(&shared).last_issue.clone();
    replace_status(
        &shared,
        AppSdkRuntimeStatus::from_config(&config, AppSdkLifecycleState::Stopped, None, last_issue),
    );
}

async fn build_sdk_runtime(config: &AppSdkConfig) -> Result<RadrootsSdk, RadrootsSdkError> {
    let mut builder = RadrootsSdk::builder()
        .directory_storage(config.storage_root.clone())
        .relay_url_policy(config.relay_url_policy.into());
    for relay_url in &config.relay_urls {
        builder = builder.relay_url(relay_url.clone());
    }
    builder.build().await
}

fn replace_status(shared: &AppSdkRuntimeShared, status: AppSdkRuntimeStatus) {
    *lock_status(shared) = status;
    shared.status_changed.notify_all();
}

fn transition_status_state(shared: &AppSdkRuntimeShared, state: AppSdkLifecycleState) {
    lock_status(shared).state = state;
    shared.status_changed.notify_all();
}

fn lock_status(shared: &AppSdkRuntimeShared) -> MutexGuard<'_, AppSdkRuntimeStatus> {
    shared
        .status
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sdk_error_class_label(error: &RadrootsSdkError) -> String {
    serde_json::to_value(error.class())
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{:?}", error.class()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use crate::{
        APP_RUNTIME_NAMESPACE, AppDesktopRuntimePaths, AppRuntimeHostEnvironment,
        AppRuntimePlatform,
    };

    use super::{
        APP_SDK_STORAGE_DIR_NAME, AppSdkConfig, AppSdkLifecycleState, AppSdkRelayUrlPolicy,
        AppSdkRuntime, app_sdk_storage_root_from_data_root,
    };

    #[test]
    fn sdk_config_uses_app_data_sdk_storage_root() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some("/Users/treesap".into()),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("desktop paths should resolve");
        let config =
            AppSdkConfig::from_desktop_paths(&paths, vec!["wss://relay.example".to_owned()]);

        assert_eq!(
            config.storage_root,
            paths.app.data.join(APP_SDK_STORAGE_DIR_NAME)
        );
        assert_eq!(
            config.storage_root,
            app_sdk_storage_root_from_data_root(paths.app.data.as_path())
        );
        assert_eq!(config.storage_root.parent(), Some(paths.app.data.as_path()));
        assert!(paths.app.data.ends_with(APP_RUNTIME_NAMESPACE));
        assert_eq!(config.relay_url_policy, AppSdkRelayUrlPolicy::Public);
    }

    #[test]
    fn sdk_config_uses_localhost_policy_for_ws_relay_urls() {
        let config = AppSdkConfig::from_app_data_root(
            "/tmp/radroots-app-data".as_ref(),
            vec![
                "wss://relay.example".to_owned(),
                "ws://127.0.0.1:8080".to_owned(),
            ],
        );

        assert_eq!(config.relay_url_policy, AppSdkRelayUrlPolicy::Localhost);
    }

    #[test]
    fn sdk_runtime_reaches_ready_with_directory_storage() {
        let storage_root = temp_storage_root("ready");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://127.0.0.1:8080".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");

        let status = runtime.wait_for_startup(Duration::from_secs(5));

        assert_eq!(status.state, AppSdkLifecycleState::Ready);
        assert_eq!(status.storage_root, storage_root);
        assert_eq!(status.relay_url_policy, AppSdkRelayUrlPolicy::Localhost);
        let storage_paths = status
            .storage_paths
            .expect("storage paths should be present");
        assert_eq!(
            storage_paths.event_store_path,
            storage_root.join("event_store.sqlite")
        );
        assert_eq!(
            storage_paths.outbox_path,
            storage_root.join("outbox.sqlite")
        );
        runtime.shutdown().expect("sdk runtime should shut down");
        assert_eq!(runtime.status().state, AppSdkLifecycleState::Stopped);
        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn sdk_runtime_degrades_with_structured_sdk_error() {
        let storage_root = temp_storage_root("invalid_relay");
        let config = AppSdkConfig::from_app_data_root(
            storage_root
                .parent()
                .expect("storage root should have parent"),
            vec!["ws://relay.example".to_owned()],
        );
        let runtime = AppSdkRuntime::start(config).expect("sdk runtime should start");

        let status = runtime.wait_for_startup(Duration::from_secs(5));

        assert_eq!(status.state, AppSdkLifecycleState::Degraded);
        let issue = status
            .last_issue
            .expect("degraded status should include issue");
        assert_eq!(issue.code, "invalid_relay_url");
        assert_eq!(issue.class, "configuration");
        assert!(!issue.retryable);
        assert!(
            issue
                .recovery_actions
                .contains(&"configure_relay_targets".to_owned())
        );
        assert_eq!(issue.detail_json["code"], "invalid_relay_url");
        runtime.shutdown().expect("sdk runtime should shut down");
        let _ = fs::remove_dir_all(storage_root);
    }

    fn temp_storage_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("radroots_studio_app_sdk_runtime_{label}_{nanos}"))
            .join(APP_SDK_STORAGE_DIR_NAME)
    }
}
