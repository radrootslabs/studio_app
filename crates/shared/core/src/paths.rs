use std::{
    env,
    error::Error,
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

pub const APP_RUNTIME_NAMESPACE_KIND: &str = "apps";
pub const APP_RUNTIME_NAMESPACE_VALUE: &str = "app";
pub const APP_RUNTIME_NAMESPACE: &str = "apps/app";
pub const SHARED_ACCOUNTS_NAMESPACE_KIND: &str = "shared";
pub const SHARED_ACCOUNTS_NAMESPACE_VALUE: &str = "accounts";
pub const SHARED_ACCOUNTS_NAMESPACE: &str = "shared/accounts";
pub const SHARED_ACCOUNTS_STORE_FILE_NAME: &str = "store.json";
pub const SHARED_IDENTITIES_NAMESPACE_KIND: &str = "shared";
pub const SHARED_IDENTITIES_NAMESPACE_VALUE: &str = "identities";
pub const SHARED_IDENTITIES_NAMESPACE: &str = "shared/identities";
pub const SHARED_IDENTITY_FILE_NAME: &str = "default.json";
pub const APP_PATHS_PROFILE_ENV: &str = "RADROOTS_APP_PATHS_PROFILE";
pub const APP_PATHS_REPO_LOCAL_ROOT_ENV: &str = "RADROOTS_APP_PATHS_REPO_LOCAL_ROOT";

const APP_INTERACTIVE_USER_PROFILE: &str = "interactive_user";
const APP_REPO_LOCAL_PROFILE: &str = "repo_local";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppRuntimePlatform {
    Linux,
    Macos,
    Windows,
    Other(&'static str),
}

impl AppRuntimePlatform {
    pub fn current() -> Self {
        match env::consts::OS {
            "linux" => Self::Linux,
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            other => Self::Other(other),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Other(other) => other,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppRuntimeHostEnvironment {
    pub home_dir: Option<PathBuf>,
    pub appdata_dir: Option<PathBuf>,
    pub localappdata_dir: Option<PathBuf>,
    pub paths_profile: Option<String>,
    pub repo_local_root: Option<PathBuf>,
}

impl AppRuntimeHostEnvironment {
    pub fn from_current_process() -> Self {
        Self::from_env_reader(|name| env::var_os(name))
    }

    pub fn from_env_reader<F>(mut read_env: F) -> Self
    where
        F: FnMut(&str) -> Option<OsString>,
    {
        Self {
            home_dir: read_env("HOME").map(PathBuf::from),
            appdata_dir: read_env("APPDATA").map(PathBuf::from),
            localappdata_dir: read_env("LOCALAPPDATA").map(PathBuf::from),
            paths_profile: read_env(APP_PATHS_PROFILE_ENV)
                .and_then(|value| value.into_string().ok()),
            repo_local_root: read_env(APP_PATHS_REPO_LOCAL_ROOT_ENV).map(PathBuf::from),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppRuntimeRoots {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub run: PathBuf,
    pub secrets: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSharedAccountsPaths {
    pub data_root: PathBuf,
    pub secrets_root: PathBuf,
    pub store_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSharedIdentityPaths {
    pub default_identity_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppDesktopRuntimePaths {
    pub app: AppRuntimeRoots,
    pub shared_accounts: AppSharedAccountsPaths,
    pub shared_identity: AppSharedIdentityPaths,
}

impl AppRuntimeRoots {
    pub fn current_desktop() -> Result<Self, AppRuntimePathsError> {
        AppDesktopRuntimePaths::current_desktop().map(|paths| paths.app)
    }

    pub fn for_desktop(
        platform: AppRuntimePlatform,
        host_environment: AppRuntimeHostEnvironment,
    ) -> Result<Self, AppRuntimePathsError> {
        Ok(resolve_desktop_base_roots(platform, host_environment)?.namespaced_app())
    }

    pub fn from_base_root(base_root: impl AsRef<Path>) -> Self {
        let base_root = base_root.as_ref();
        Self {
            config: base_root.join("config"),
            data: base_root.join("data"),
            cache: base_root.join("cache"),
            logs: base_root.join("logs"),
            run: base_root.join("run"),
            secrets: base_root.join("secrets"),
        }
    }

    pub fn namespaced_app(&self) -> Self {
        self.namespaced(APP_RUNTIME_NAMESPACE_KIND, APP_RUNTIME_NAMESPACE_VALUE)
    }

    fn namespaced_shared(&self, value: &str) -> Self {
        self.namespaced(SHARED_ACCOUNTS_NAMESPACE_KIND, value)
    }

    fn namespaced(&self, kind: &str, value: &str) -> Self {
        let namespace = PathBuf::from(kind).join(value);
        Self {
            config: self.config.join(&namespace),
            data: self.data.join(&namespace),
            cache: self.cache.join(&namespace),
            logs: self.logs.join(&namespace),
            run: self.run.join(&namespace),
            secrets: self.secrets.join(namespace),
        }
    }
}

impl AppDesktopRuntimePaths {
    pub fn current_desktop() -> Result<Self, AppRuntimePathsError> {
        Self::for_desktop(
            AppRuntimePlatform::current(),
            AppRuntimeHostEnvironment::from_current_process(),
        )
    }

    pub fn for_desktop(
        platform: AppRuntimePlatform,
        host_environment: AppRuntimeHostEnvironment,
    ) -> Result<Self, AppRuntimePathsError> {
        let base_roots = resolve_desktop_base_roots(platform, host_environment)?;
        let shared_accounts = base_roots.namespaced_shared(SHARED_ACCOUNTS_NAMESPACE_VALUE);
        let shared_identity = base_roots.namespaced_shared(SHARED_IDENTITIES_NAMESPACE_VALUE);

        Ok(Self {
            app: base_roots.namespaced_app(),
            shared_accounts: AppSharedAccountsPaths {
                data_root: shared_accounts.data.clone(),
                secrets_root: shared_accounts.secrets.clone(),
                store_path: shared_accounts.data.join(SHARED_ACCOUNTS_STORE_FILE_NAME),
            },
            shared_identity: AppSharedIdentityPaths {
                default_identity_path: shared_identity.secrets.join(SHARED_IDENTITY_FILE_NAME),
            },
        })
    }
}

fn resolve_desktop_base_roots(
    platform: AppRuntimePlatform,
    host_environment: AppRuntimeHostEnvironment,
) -> Result<AppRuntimeRoots, AppRuntimePathsError> {
    let roots = match resolve_desktop_profile(host_environment.paths_profile.as_deref())? {
        AppDesktopPathProfile::InteractiveUser => resolve_interactive_user_roots(
            platform,
            host_environment.home_dir,
            host_environment.appdata_dir,
            host_environment.localappdata_dir,
        )?,
        AppDesktopPathProfile::RepoLocal => {
            let repo_local_root = host_environment
                .repo_local_root
                .ok_or(AppRuntimePathsError::MissingRepoLocalRoot)?;
            if repo_local_root.as_os_str().is_empty() {
                return Err(AppRuntimePathsError::EmptyRepoLocalRoot);
            }
            AppRuntimeRoots::from_base_root(repo_local_root)
        }
    };

    Ok(roots)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppDesktopPathProfile {
    InteractiveUser,
    RepoLocal,
}

fn resolve_desktop_profile(
    profile: Option<&str>,
) -> Result<AppDesktopPathProfile, AppRuntimePathsError> {
    match profile {
        None => Ok(AppDesktopPathProfile::InteractiveUser),
        Some(value) => match value.trim().to_ascii_lowercase().as_str() {
            APP_INTERACTIVE_USER_PROFILE => Ok(AppDesktopPathProfile::InteractiveUser),
            APP_REPO_LOCAL_PROFILE => Ok(AppDesktopPathProfile::RepoLocal),
            _ => Err(AppRuntimePathsError::UnsupportedPathProfile {
                value: value.to_owned(),
            }),
        },
    }
}

fn resolve_interactive_user_roots(
    platform: AppRuntimePlatform,
    home_dir: Option<PathBuf>,
    appdata_dir: Option<PathBuf>,
    localappdata_dir: Option<PathBuf>,
) -> Result<AppRuntimeRoots, AppRuntimePathsError> {
    match platform {
        AppRuntimePlatform::Linux | AppRuntimePlatform::Macos => {
            let home_dir = home_dir.ok_or(AppRuntimePathsError::MissingHomeDir { platform })?;
            Ok(AppRuntimeRoots::from_base_root(home_dir.join(".radroots")))
        }
        AppRuntimePlatform::Windows => {
            let appdata_dir = appdata_dir.ok_or(AppRuntimePathsError::MissingWindowsUserDirs)?;
            let localappdata_dir =
                localappdata_dir.ok_or(AppRuntimePathsError::MissingWindowsUserDirs)?;
            let config_root = appdata_dir.join("Radroots");
            let local_root = localappdata_dir.join("Radroots");
            Ok(AppRuntimeRoots {
                config: config_root.join("config"),
                data: local_root.join("data"),
                cache: local_root.join("cache"),
                logs: local_root.join("logs"),
                run: local_root.join("run"),
                secrets: config_root.join("secrets"),
            })
        }
        AppRuntimePlatform::Other(_) => Err(AppRuntimePathsError::UnsupportedPlatform { platform }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppRuntimePathsError {
    MissingHomeDir { platform: AppRuntimePlatform },
    MissingWindowsUserDirs,
    MissingRepoLocalRoot,
    EmptyRepoLocalRoot,
    UnsupportedPathProfile { value: String },
    UnsupportedPlatform { platform: AppRuntimePlatform },
}

impl fmt::Display for AppRuntimePathsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHomeDir { platform } => {
                write!(
                    formatter,
                    "desktop runtime roots require HOME for {}",
                    platform.label()
                )
            }
            Self::MissingWindowsUserDirs => formatter
                .write_str("desktop runtime roots require APPDATA and LOCALAPPDATA on windows"),
            Self::MissingRepoLocalRoot => write!(
                formatter,
                "desktop runtime roots require {APP_PATHS_REPO_LOCAL_ROOT_ENV} when {APP_PATHS_PROFILE_ENV}=repo_local"
            ),
            Self::EmptyRepoLocalRoot => write!(
                formatter,
                "{APP_PATHS_REPO_LOCAL_ROOT_ENV} must not be empty when {APP_PATHS_PROFILE_ENV}=repo_local"
            ),
            Self::UnsupportedPathProfile { value } => write!(
                formatter,
                "{APP_PATHS_PROFILE_ENV} must be `interactive_user` or `repo_local`, got `{value}`"
            ),
            Self::UnsupportedPlatform { platform } => write!(
                formatter,
                "desktop runtime roots are unsupported on {}",
                platform.label()
            ),
        }
    }
}

impl Error for AppRuntimePathsError {}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

    use super::{
        APP_PATHS_PROFILE_ENV, APP_PATHS_REPO_LOCAL_ROOT_ENV, APP_RUNTIME_NAMESPACE,
        AppDesktopRuntimePaths, AppRuntimeHostEnvironment, AppRuntimePathsError,
        AppRuntimePlatform, AppRuntimeRoots, SHARED_ACCOUNTS_NAMESPACE,
        SHARED_ACCOUNTS_STORE_FILE_NAME, SHARED_IDENTITIES_NAMESPACE, SHARED_IDENTITY_FILE_NAME,
    };

    #[test]
    fn desktop_runtime_roots_use_canonical_macos_namespace() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                home_dir: Some(PathBuf::from("/Users/treesap")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("macos roots should resolve");

        assert_eq!(
            paths.app.data,
            PathBuf::from("/Users/treesap/.radroots/data").join(APP_RUNTIME_NAMESPACE)
        );
        assert_eq!(
            paths.app.logs,
            PathBuf::from("/Users/treesap/.radroots/logs").join(APP_RUNTIME_NAMESPACE)
        );
        assert_eq!(
            paths.shared_accounts.data_root,
            PathBuf::from("/Users/treesap/.radroots/data").join(SHARED_ACCOUNTS_NAMESPACE)
        );
        assert_eq!(
            paths.shared_accounts.secrets_root,
            PathBuf::from("/Users/treesap/.radroots/secrets").join(SHARED_ACCOUNTS_NAMESPACE)
        );
        assert_eq!(
            paths.shared_accounts.store_path,
            PathBuf::from("/Users/treesap/.radroots/data")
                .join(SHARED_ACCOUNTS_NAMESPACE)
                .join(SHARED_ACCOUNTS_STORE_FILE_NAME)
        );
        assert_eq!(
            paths.shared_identity.default_identity_path,
            PathBuf::from("/Users/treesap/.radroots/secrets")
                .join(SHARED_IDENTITIES_NAMESPACE)
                .join(SHARED_IDENTITY_FILE_NAME)
        );
    }

    #[test]
    fn desktop_runtime_roots_use_canonical_linux_namespace() {
        let roots = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Linux,
            AppRuntimeHostEnvironment {
                home_dir: Some(PathBuf::from("/home/treesap")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("linux roots should resolve");

        assert_eq!(
            roots.data,
            PathBuf::from("/home/treesap/.radroots/data").join(APP_RUNTIME_NAMESPACE)
        );
        assert_eq!(
            roots.logs,
            PathBuf::from("/home/treesap/.radroots/logs").join(APP_RUNTIME_NAMESPACE)
        );
    }

    #[test]
    fn desktop_runtime_roots_use_native_windows_roots() {
        let roots = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Windows,
            AppRuntimeHostEnvironment {
                appdata_dir: Some(PathBuf::from(r"C:\Users\treesap\AppData\Roaming")),
                localappdata_dir: Some(PathBuf::from(r"C:\Users\treesap\AppData\Local")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("windows roots should resolve");

        assert_eq!(
            roots.config,
            PathBuf::from(r"C:\Users\treesap\AppData\Roaming")
                .join("Radroots")
                .join("config")
                .join(APP_RUNTIME_NAMESPACE)
        );
        assert_eq!(
            roots.data,
            PathBuf::from(r"C:\Users\treesap\AppData\Local")
                .join("Radroots")
                .join("data")
                .join(APP_RUNTIME_NAMESPACE)
        );
    }

    #[test]
    fn desktop_runtime_roots_use_explicit_repo_local_root() {
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                paths_profile: Some("repo_local".to_owned()),
                repo_local_root: Some(PathBuf::from("/repo/infra/local/runtime/radroots")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect("repo-local roots should resolve");

        assert_eq!(
            paths.app.data,
            PathBuf::from("/repo/infra/local/runtime/radroots/data/apps/app")
        );
        assert_eq!(
            paths.app.logs,
            PathBuf::from("/repo/infra/local/runtime/radroots/logs/apps/app")
        );
        assert_eq!(
            paths.shared_accounts.data_root,
            PathBuf::from("/repo/infra/local/runtime/radroots/data/shared/accounts")
        );
        assert_eq!(
            paths.shared_identity.default_identity_path,
            PathBuf::from("/repo/infra/local/runtime/radroots/secrets/shared/identities")
                .join(SHARED_IDENTITY_FILE_NAME)
        );
    }

    #[test]
    fn host_environment_can_resolve_from_env_reader() {
        let env = BTreeMap::from([
            (APP_PATHS_PROFILE_ENV, OsString::from("repo_local")),
            (
                APP_PATHS_REPO_LOCAL_ROOT_ENV,
                OsString::from("/repo/infra/local/runtime/radroots"),
            ),
        ]);
        let paths = AppDesktopRuntimePaths::for_desktop(
            AppRuntimePlatform::Linux,
            AppRuntimeHostEnvironment::from_env_reader(|name| env.get(name).cloned()),
        )
        .expect("repo-local env-backed roots should resolve");

        assert_eq!(
            paths.app.data,
            PathBuf::from("/repo/infra/local/runtime/radroots/data/apps/app")
        );
    }

    #[test]
    fn repo_local_profile_requires_explicit_root() {
        let err = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                paths_profile: Some("repo_local".to_owned()),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect_err("repo-local root should be required");

        assert_eq!(err, AppRuntimePathsError::MissingRepoLocalRoot);
    }

    #[test]
    fn unsupported_path_profile_is_rejected() {
        let err = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment {
                paths_profile: Some("dev".to_owned()),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect_err("unsupported profile should fail");

        assert_eq!(
            err,
            AppRuntimePathsError::UnsupportedPathProfile {
                value: "dev".to_owned(),
            }
        );
    }

    #[test]
    fn desktop_runtime_roots_require_home_dir_on_unix() {
        let err = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Macos,
            AppRuntimeHostEnvironment::default(),
        )
        .expect_err("missing home dir should fail");

        assert_eq!(
            err,
            AppRuntimePathsError::MissingHomeDir {
                platform: AppRuntimePlatform::Macos,
            }
        );
    }

    #[test]
    fn desktop_runtime_roots_require_windows_user_dirs() {
        let err = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Windows,
            AppRuntimeHostEnvironment {
                appdata_dir: Some(PathBuf::from(r"C:\Users\treesap\AppData\Roaming")),
                ..AppRuntimeHostEnvironment::default()
            },
        )
        .expect_err("missing local appdata should fail");

        assert_eq!(err, AppRuntimePathsError::MissingWindowsUserDirs);
    }

    #[test]
    fn desktop_runtime_roots_reject_unsupported_platforms() {
        let err = AppRuntimeRoots::for_desktop(
            AppRuntimePlatform::Other("freebsd"),
            AppRuntimeHostEnvironment::default(),
        )
        .expect_err("unsupported platform should fail");

        assert_eq!(
            err,
            AppRuntimePathsError::UnsupportedPlatform {
                platform: AppRuntimePlatform::Other("freebsd"),
            }
        );
    }
}
