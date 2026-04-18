#![forbid(unsafe_code)]

mod error;
mod input;
mod protocol;
mod session;

pub use error::RadrootsAppRemoteSignerError;
pub use input::{
    RadrootsAppRemoteSignerSource, RadrootsAppRemoteSignerTarget,
    radroots_studio_app_remote_signer_preview, radroots_studio_app_remote_signer_requested_permissions,
};
pub use protocol::{
    RadrootsAppRemoteSignerApprovedSession, RadrootsAppRemoteSignerPendingPollOutcome,
    RadrootsAppRemoteSignerPendingSession, RadrootsAppRemoteSignerProgressUpdate,
    RadrootsAppRemoteSignerSignedEvent, radroots_studio_app_remote_signer_connect_pending,
    radroots_studio_app_remote_signer_poll_pending_session,
    radroots_studio_app_remote_signer_poll_pending_session_with_progress,
    radroots_studio_app_remote_signer_sign_kind1_note,
    radroots_studio_app_remote_signer_sign_kind1_note_with_progress,
    radroots_studio_app_remote_signer_sign_unsigned_event,
    radroots_studio_app_remote_signer_sign_unsigned_event_with_progress,
};
pub use session::{
    RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION, RadrootsAppRemoteSignerSessionRecord,
    RadrootsAppRemoteSignerSessionStatus, RadrootsAppRemoteSignerSessionStoreLoadResult,
    RadrootsAppRemoteSignerSessionStoreState,
};
