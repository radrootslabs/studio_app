#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderUnavailableKind {
    MissingBuildAsset,
    InitializationFailed,
    InternalError,
}

impl RadrootsOfflineGeocoderUnavailableKind {
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
