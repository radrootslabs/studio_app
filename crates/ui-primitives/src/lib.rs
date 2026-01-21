#![forbid(unsafe_code)]

mod portal;
mod presence;
mod dismissable;
mod focus;
mod scroll_lock;
mod roving_focus;

pub use portal::{RadrootsAppUiPortal, RadrootsAppUiPortalMount};
pub use presence::{
    radroots_studio_app_ui_presence_state_next,
    RadrootsAppUiPresence,
    RadrootsAppUiPresenceState,
};
pub use dismissable::{
    radroots_studio_app_ui_dismissable_is_escape,
    radroots_studio_app_ui_dismissable_is_outside,
    RadrootsAppUiDismissableLayer,
    RadrootsAppUiDismissableReason,
};
pub use focus::{
    radroots_studio_app_ui_focus_scope_next_index,
    radroots_studio_app_ui_focus_scope_selector,
    RadrootsAppUiFocusScope,
};
pub use scroll_lock::{
    radroots_studio_app_ui_scroll_lock_acquire,
    radroots_studio_app_ui_scroll_lock_release,
    RadrootsAppUiScrollLockError,
    RadrootsAppUiScrollLockGuard,
    RadrootsAppUiScrollLockResult,
};
pub use roving_focus::{
    radroots_studio_app_ui_roving_focus_action_from_key,
    radroots_studio_app_ui_roving_focus_next_index,
    RadrootsAppUiRovingFocusAction,
    RadrootsAppUiRovingFocusOrientation,
};
