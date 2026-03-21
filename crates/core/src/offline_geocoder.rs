#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsOfflineGeocoderState {
    Initializing,
    Ready,
    Unavailable {
        user_message: String,
        debug_message: String,
    },
}

impl RadrootsOfflineGeocoderState {
    pub fn summary_label(&self) -> &'static str {
        match self {
            Self::Initializing => "Offline geocoder: initializing",
            Self::Ready => "Offline geocoder: ready",
            Self::Unavailable { .. } => "Offline geocoder unavailable",
        }
    }
}
