use crate::runtime::{AppRuntimeSnapshot, runtime_mode_label};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppStartupEvent {
    pub category: &'static str,
    pub name: &'static str,
    pub message: &'static str,
    pub metadata: AppStartupEventMetadata,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppStartupEventMetadata {
    pub home_screen: String,
    pub core_package: String,
    pub host_surface: String,
    pub runtime_mode: String,
}

pub fn launch_startup_event(snapshot: &AppRuntimeSnapshot) -> AppStartupEvent {
    AppStartupEvent {
        category: "bootstrap",
        name: "runtime.launch",
        message: "app launch",
        metadata: AppStartupEventMetadata {
            home_screen: snapshot.title.clone(),
            core_package: snapshot.core.package_name.clone(),
            host_surface: snapshot.host.platform_name.clone(),
            runtime_mode: runtime_mode_label(&snapshot.runtime_mode).to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        APP_PROJECTION_SOURCE, AppBuildIdentity, AppRuntimeCapture, AppRuntimeMode,
        AppRuntimeSnapshot,
    };

    use super::launch_startup_event;

    #[test]
    fn launch_startup_event_uses_runtime_snapshot_fields() {
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
                host_locale: "en".to_owned(),
                operating_system: "macos".to_owned(),
                run_id: "run-development-123-pid456".to_owned(),
            },
        );

        let event = launch_startup_event(&snapshot);

        assert_eq!(event.category, "bootstrap");
        assert_eq!(event.name, "runtime.launch");
        assert_eq!(event.message, "app launch");
        assert_eq!(event.metadata.home_screen, "radroots");
        assert_eq!(event.metadata.core_package, "radroots_studio_app_core");
        assert_eq!(event.metadata.host_surface, "desktop");
        assert_eq!(event.metadata.runtime_mode, "development");
    }
}
