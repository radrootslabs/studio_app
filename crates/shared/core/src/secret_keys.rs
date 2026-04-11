#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadrootsSecretImportMode {
    #[default]
    EncryptedSecretKey,
    RawSecretKey,
}

impl RadrootsSecretImportMode {
    pub fn helper_text(self) -> &'static str {
        match self {
            Self::EncryptedSecretKey => {
                "Import an existing local identity by entering its encrypted secret key and password."
            }
            Self::RawSecretKey => {
                "Advanced: import an existing local identity by entering its raw nsec secret key."
            }
        }
    }

    pub fn hint_text(self) -> &'static str {
        match self {
            Self::EncryptedSecretKey => "ncryptsec1...",
            Self::RawSecretKey => "nsec1...",
        }
    }

    pub fn mode_label(self) -> &'static str {
        match self {
            Self::EncryptedSecretKey => "Encrypted Secret Key",
            Self::RawSecretKey => "Raw Secret Key",
        }
    }

    pub fn switch_label(self) -> &'static str {
        match self {
            Self::EncryptedSecretKey => "Use Raw Secret Key Instead",
            Self::RawSecretKey => "Use Encrypted Secret Key Instead",
        }
    }

    pub fn requires_password(self) -> bool {
        matches!(self, Self::EncryptedSecretKey)
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::EncryptedSecretKey => Self::RawSecretKey,
            Self::RawSecretKey => Self::EncryptedSecretKey,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsSecretImportRequest {
    pub mode: RadrootsSecretImportMode,
    pub secret_text: String,
    pub password: Option<String>,
}
