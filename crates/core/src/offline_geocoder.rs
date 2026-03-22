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
    pub code: &'static str,
    pub summary_label: &'static str,
    pub user_message: &'static str,
    pub technical_message: &'static str,
}

impl RadrootsOfflineGeocoderDiagnostic {
    pub fn export_text(&self) -> String {
        format!(
            "offline geocoder diagnostic\ncode: {}\nstatus: {}\nuser: {}\ntechnical: {}",
            self.code, self.summary_label, self.user_message, self.technical_message
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderState {
    Initializing,
    Ready,
    Unavailable {
        kind: RadrootsOfflineGeocoderUnavailableKind,
        debug_message: String,
    },
}

impl RadrootsOfflineGeocoderState {
    pub fn unavailable(
        kind: RadrootsOfflineGeocoderUnavailableKind,
        debug_message: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            kind,
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
            Self::Unavailable { kind, .. } => Some(RadrootsOfflineGeocoderDiagnostic {
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
            "failed to open staged geocoder db: /tmp/geonames.db",
        );
        let diagnostic = state.diagnostic().unwrap();

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
}
