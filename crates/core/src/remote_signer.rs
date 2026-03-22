#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsRemoteSignerPreview {
    pub source_label: String,
    pub signer_npub: String,
    pub relays: Vec<String>,
    pub requested_permissions: Vec<String>,
}

impl RadrootsRemoteSignerPreview {
    pub fn pending_summary(&self) -> RadrootsPendingRemoteSignerConnection {
        RadrootsPendingRemoteSignerConnection {
            signer_npub: self.signer_npub.clone(),
            relays: self.relays.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsPendingRemoteSignerConnection {
    pub signer_npub: String,
    pub relays: Vec<String>,
}
