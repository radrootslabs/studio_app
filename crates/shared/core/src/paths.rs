use std::{
    env,
    error::Error,
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
}

impl AppRuntimeHostEnvironment {
    pub fn from_current_process() -> Self {
        Self {
            home_dir: env::var_os("HOME").map(PathBuf::from),
            appdata_dir: env::var_os("APPDATA").map(PathBuf::from),
            localappdata_dir: env::var_os("LOCALAPPDATA").map(PathBuf::from),
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
    let roots = match platform {
        AppRuntimePlatform::Linux | AppRuntimePlatform::Macos => {
            let home_dir = host_environment
                .home_dir
                .ok_or(AppRuntimePathsError::MissingHomeDir { platform })?;
            AppRuntimeRoots::from_base_root(home_dir.join(".radroots"))
        }
        AppRuntimePlatform::Windows => {
            let appdata_dir = host_environment
                .appdata_dir
                .ok_or(AppRuntimePathsError::MissingWindowsUserDirs)?;
            let localappdata_dir = host_environment
                .localappdata_dir
                .ok_or(AppRuntimePathsError::MissingWindowsUserDirs)?;
            let config_root = appdata_dir.join("Radroots");
            let local_root = localappdata_dir.join("Radroots");
            AppRuntimeRoots {
                config: config_root.join("config"),
                data: local_root.join("data"),
                cache: local_root.join("cache"),
                logs: local_root.join("logs"),
                run: local_root.join("run"),
                secrets: config_root.join("secrets"),
            }
        }
        AppRuntimePlatform::Other(_) => {
            return Err(AppRuntimePathsError::UnsupportedPlatform { platform });
        }
    };

    Ok(roots)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppRuntimePathsError {
    MissingHomeDir { platform: AppRuntimePlatform },
    MissingWindowsUserDirs,
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
    use std::path::PathBuf;

    use super::{
        APP_RUNTIME_NAMESPACE, AppDesktopRuntimePaths, AppRuntimeHostEnvironment,
        AppRuntimePathsError, AppRuntimePlatform, AppRuntimeRoots, SHARED_ACCOUNTS_NAMESPACE,
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
