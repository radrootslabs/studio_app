#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderPlatform {
    Desktop,
    Ios,
    Android,
    Web,
}

impl RadrootsOfflineGeocoderPlatform {
    pub fn code(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Web => "web",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderUnavailableKind {
    MissingBuildAsset,
    InitializationFailed,
    InternalError,
}

impl RadrootsOfflineGeocoderUnavailableKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::MissingBuildAsset => "missing_build_asset",
            Self::InitializationFailed => "initialization_failed",
            Self::InternalError => "internal_error",
        }
    }

    pub fn technical_message(self) -> &'static str {
        match self {
            Self::MissingBuildAsset => {
                "The offline geocoder data file is missing from this app build."
            }
            Self::InitializationFailed => {
                "The offline geocoder data file could not be prepared on this device."
            }
            Self::InternalError => {
                "The app could not complete offline geocoder setup because of an internal error."
            }
        }
    }

    pub fn user_message(self) -> &'static str {
        match self {
            Self::MissingBuildAsset => "Offline geocoder is not available in this build.",
            Self::InitializationFailed | Self::InternalError => {
                "Offline geocoder could not be initialized on this device."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsOfflineGeocoderDiagnostic {
    pub platform_code: &'static str,
    pub asset_revision: Option<String>,
    pub code: &'static str,
    pub summary_label: &'static str,
    pub user_message: &'static str,
    pub technical_message: &'static str,
}

impl RadrootsOfflineGeocoderDiagnostic {
    pub fn export_text(&self) -> String {
        format!(
            "offline geocoder diagnostic\nplatform: {}\nasset_revision: {}\ncode: {}\nstatus: {}\nuser: {}\ntechnical: {}",
            self.platform_code,
            self.asset_revision.as_deref().unwrap_or("unknown"),
            self.code,
            self.summary_label,
            self.user_message,
            self.technical_message
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderState {
    Initializing,
    Ready,
    Unavailable {
        kind: RadrootsOfflineGeocoderUnavailableKind,
        platform: RadrootsOfflineGeocoderPlatform,
        asset_revision: Option<String>,
        debug_message: String,
    },
}

impl RadrootsOfflineGeocoderState {
    pub fn unavailable(
        kind: RadrootsOfflineGeocoderUnavailableKind,
        platform: RadrootsOfflineGeocoderPlatform,
        debug_message: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            kind,
            platform,
            asset_revision: None,
            debug_message: debug_message.into(),
        }
    }

    pub fn unavailable_with_revision(
        kind: RadrootsOfflineGeocoderUnavailableKind,
        platform: RadrootsOfflineGeocoderPlatform,
        asset_revision: impl Into<String>,
        debug_message: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            kind,
            platform,
            asset_revision: Some(asset_revision.into()),
            debug_message: debug_message.into(),
        }
    }

    pub fn debug_message(&self) -> Option<&str> {
        match self {
            Self::Unavailable { debug_message, .. } => Some(debug_message.as_str()),
            Self::Initializing | Self::Ready => None,
        }
    }

    pub fn diagnostic(&self) -> Option<RadrootsOfflineGeocoderDiagnostic> {
        match self {
            Self::Unavailable {
                kind,
                platform,
                asset_revision,
                ..
            } => Some(RadrootsOfflineGeocoderDiagnostic {
                platform_code: platform.code(),
                asset_revision: asset_revision.clone(),
                code: kind.code(),
                summary_label: self.summary_label(),
                user_message: kind.user_message(),
                technical_message: kind.technical_message(),
            }),
            Self::Initializing | Self::Ready => None,
        }
    }

    pub fn summary_label(&self) -> &'static str {
        match self {
            Self::Initializing => "Offline geocoder: initializing",
            Self::Ready => "Offline geocoder: ready",
            Self::Unavailable { .. } => "Offline geocoder unavailable",
        }
    }

    pub fn platform(&self) -> Option<RadrootsOfflineGeocoderPlatform> {
        match self {
            Self::Unavailable { platform, .. } => Some(*platform),
            Self::Initializing | Self::Ready => None,
        }
    }

    pub fn asset_revision(&self) -> Option<&str> {
        match self {
            Self::Unavailable { asset_revision, .. } => asset_revision.as_deref(),
            Self::Initializing | Self::Ready => None,
        }
    }

    pub fn technical_message(&self) -> Option<&'static str> {
        match self {
            Self::Unavailable { kind, .. } => Some(kind.technical_message()),
            Self::Initializing | Self::Ready => None,
        }
    }

    pub fn user_message(&self) -> Option<&'static str> {
        match self {
            Self::Unavailable { kind, .. } => Some(kind.user_message()),
            Self::Initializing | Self::Ready => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_state_exposes_release_safe_diagnostic() {
        let state = RadrootsOfflineGeocoderState::unavailable(
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            RadrootsOfflineGeocoderPlatform::Desktop,
            "failed to open staged geocoder db: /tmp/geonames.db",
        );
        let diagnostic = state.diagnostic().unwrap();

        assert_eq!(diagnostic.platform_code, "desktop");
        assert_eq!(diagnostic.asset_revision, None);
        assert_eq!(diagnostic.code, "initialization_failed");
        assert_eq!(diagnostic.summary_label, "Offline geocoder unavailable");
        assert_eq!(
            diagnostic.user_message,
            "Offline geocoder could not be initialized on this device."
        );
        assert_eq!(
            diagnostic.technical_message,
            "The offline geocoder data file could not be prepared on this device."
        );
        assert!(!diagnostic.export_text().contains("/tmp/geonames.db"));
    }

    #[test]
    fn unavailable_state_with_revision_exports_release_safe_platform_context() {
        let state = RadrootsOfflineGeocoderState::unavailable_with_revision(
            RadrootsOfflineGeocoderUnavailableKind::InitializationFailed,
            RadrootsOfflineGeocoderPlatform::Android,
            "6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c",
            "failed to open staged android geocoder db: /data/user/0/org.radroots.app.android/files/geocoder.db",
        );
        let diagnostic = state.diagnostic().unwrap();
        let export_text = diagnostic.export_text();

        assert_eq!(diagnostic.platform_code, "android");
        assert_eq!(
            diagnostic.asset_revision.as_deref(),
            Some("6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c")
        );
        assert!(export_text.contains("platform: android"));
        assert!(export_text.contains(
            "asset_revision: 6ca5f1a324de02922d40b1ff33eedf3a5a133c978de921eee5130a0c7876079c"
        ));
        assert!(!export_text.contains("/data/user/0/org.radroots.app.android/files/geocoder.db"));
    }
}
