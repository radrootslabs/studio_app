#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAccountCustody {
    LocalManaged,
    BrowserSigner,
    RemoteSigner,
}

impl RadrootsAccountCustody {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalManaged => "local managed",
            Self::BrowserSigner => "browser signer",
            Self::RemoteSigner => "remote signer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAccountSummary {
    pub account_id: String,
    pub npub: String,
    pub label: Option<String>,
    pub custody: RadrootsAccountCustody,
}

impl RadrootsAccountSummary {
    pub fn display_label(&self) -> String {
        match self.label.as_deref() {
            Some(label) if !label.trim().is_empty() => label.to_owned(),
            _ => match self.custody {
                RadrootsAccountCustody::LocalManaged => "local account".to_owned(),
                RadrootsAccountCustody::BrowserSigner => "browser signer".to_owned(),
                RadrootsAccountCustody::RemoteSigner => "remote signer".to_owned(),
            },
        }
    }
}
