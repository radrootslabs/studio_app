#![forbid(unsafe_code)]

mod controller;
mod custody;
mod error;
mod input;
mod protocol;
mod session;

pub use controller::{RadrootsAppRemoteSignerController, RadrootsAppRemoteSignerControllerHooks};
pub use custody::{
    radroots_studio_app_remote_signer_clear_pending_session,
    radroots_studio_app_remote_signer_disconnect_selected, radroots_studio_app_remote_signer_reconcile_startup,
};
pub use error::RadrootsAppRemoteSignerError;
pub use input::{
    RadrootsAppRemoteSignerSource, RadrootsAppRemoteSignerTarget,
    radroots_studio_app_remote_signer_preview,
};
pub use protocol::{
    RadrootsAppRemoteSignerPendingPollOutcome, RadrootsAppRemoteSignerPendingSession,
    radroots_studio_app_remote_signer_connect_pending, radroots_studio_app_remote_signer_poll_pending_session,
};
pub use session::{
    RADROOTS_APP_REMOTE_SIGNER_SESSION_STORE_VERSION, RadrootsAppRemoteSignerSessionRecord,
    RadrootsAppRemoteSignerSessionStatus, RadrootsAppRemoteSignerSessionStoreLoadResult,
    RadrootsAppRemoteSignerSessionStoreState,
};
