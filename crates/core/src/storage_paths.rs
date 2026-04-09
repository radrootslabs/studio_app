use std::path::{Path, PathBuf};

use radroots_runtime_paths::{
    RadrootsHostEnvironment, RadrootsPathOverrides, RadrootsPathProfile, RadrootsPathResolver,
    RadrootsPaths, RadrootsPlatform, RadrootsRuntimeNamespace,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppStorageLayout {
    pub runtime_root: PathBuf,
    pub app_paths: RadrootsPaths,
}

fn app_namespace() -> Result<RadrootsRuntimeNamespace, String> {
    RadrootsRuntimeNamespace::app("app")
        .map_err(|source| format!("failed to resolve app runtime namespace: {source}"))
}

fn runtime_root_from_paths(roots: &RadrootsPaths) -> Result<PathBuf, String> {
    roots
        .config
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "resolved app config root had no parent".to_owned())
}

pub fn interactive_user_app_storage_layout_with_resolver(
    resolver: &RadrootsPathResolver,
) -> Result<RadrootsAppStorageLayout, String> {
    let roots = resolver
        .resolve(
            RadrootsPathProfile::InteractiveUser,
            &RadrootsPathOverrides::default(),
        )
        .map_err(|source| format!("failed to resolve app interactive-user roots: {source}"))?;
    let namespace = app_namespace()?;
    Ok(RadrootsAppStorageLayout {
        runtime_root: runtime_root_from_paths(&roots)?,
        app_paths: roots.namespaced(&namespace),
    })
}

pub fn mobile_native_app_storage_layout(
    platform: RadrootsPlatform,
    base_root: &Path,
) -> Result<RadrootsAppStorageLayout, String> {
    let resolver = RadrootsPathResolver::new(platform, RadrootsHostEnvironment::default());
    let roots = resolver
        .resolve(
            RadrootsPathProfile::MobileNative,
            &RadrootsPathOverrides::mobile(RadrootsPaths::from_base_root(base_root)),
        )
        .map_err(|source| format!("failed to resolve app mobile-native roots: {source}"))?;
    let namespace = app_namespace()?;
    Ok(RadrootsAppStorageLayout {
        runtime_root: runtime_root_from_paths(&roots)?,
        app_paths: roots.namespaced(&namespace),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        interactive_user_app_storage_layout_with_resolver, mobile_native_app_storage_layout,
    };
    use radroots_runtime_paths::{RadrootsHostEnvironment, RadrootsPathResolver, RadrootsPlatform};
    use std::path::PathBuf;

    #[test]
    fn interactive_user_layout_keeps_runtime_root_and_namespaced_paths() {
        let resolver = RadrootsPathResolver::new(
            RadrootsPlatform::Linux,
            RadrootsHostEnvironment {
                home_dir: Some(PathBuf::from("/home/treesap")),
                ..RadrootsHostEnvironment::default()
            },
        );

        let layout =
            interactive_user_app_storage_layout_with_resolver(&resolver).expect("app layout");

        assert_eq!(
            layout.runtime_root,
            PathBuf::from("/home/treesap/.radroots")
        );
        assert_eq!(
            layout.app_paths.data,
            PathBuf::from("/home/treesap/.radroots/data/apps/app")
        );
        assert_eq!(
            layout.app_paths.logs,
            PathBuf::from("/home/treesap/.radroots/logs/apps/app")
        );
    }

    #[test]
    fn mobile_native_layout_keeps_explicit_runtime_root_and_namespaced_paths() {
        let base_root = PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots");

        let layout =
            mobile_native_app_storage_layout(RadrootsPlatform::Android, base_root.as_path())
                .expect("mobile layout");

        assert_eq!(layout.runtime_root, base_root);
        assert_eq!(
            layout.app_paths.config,
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/config/apps/app"
            )
        );
        assert_eq!(
            layout.app_paths.data,
            PathBuf::from("/data/user/0/org.radroots.app.android/no_backup/RadRoots/data/apps/app")
        );
        assert_eq!(
            layout.app_paths.secrets,
            PathBuf::from(
                "/data/user/0/org.radroots.app.android/no_backup/RadRoots/secrets/apps/app"
            )
        );
    }
}
