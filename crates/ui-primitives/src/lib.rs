#![forbid(unsafe_code)]

pub use ui_primitives_core::dialog::DialogModel;
pub use ui_primitives_core::roving_focus::{
    roving_focus_action_from_key as radroots_studio_app_ui_roving_focus_action_from_key,
    roving_focus_next_index as radroots_studio_app_ui_roving_focus_next_index,
    RovingFocusAction as RadrootsAppUiRovingFocusAction,
    RovingFocusOrientation as RadrootsAppUiRovingFocusOrientation,
};
pub use ui_primitives_leptos::builders::{
    dialog_content_attrs,
    dialog_trigger_attrs,
};
pub use ui_primitives_leptos::{
    dismissable_is_escape as radroots_studio_app_ui_dismissable_is_escape,
    dismissable_is_outside as radroots_studio_app_ui_dismissable_is_outside,
    focus_scope_next_index as radroots_studio_app_ui_focus_scope_next_index,
    focus_scope_selector as radroots_studio_app_ui_focus_scope_selector,
    modal_hide_siblings as radroots_studio_app_ui_modal_hide_siblings,
    modal_restore as radroots_studio_app_ui_modal_restore,
    presence_state_next as radroots_studio_app_ui_presence_state_next,
    scroll_lock_acquire as radroots_studio_app_ui_scroll_lock_acquire,
    scroll_lock_release as radroots_studio_app_ui_scroll_lock_release,
    use_primitive,
    DismissableLayer as RadrootsAppUiDismissableLayer,
    DismissableReason as RadrootsAppUiDismissableReason,
    FocusScope as RadrootsAppUiFocusScope,
    ModalError as RadrootsAppUiModalError,
    ModalGuard as RadrootsAppUiModalGuard,
    ModalResult as RadrootsAppUiModalResult,
    ModalTarget as RadrootsAppUiModalTarget,
    Portal as RadrootsAppUiPortal,
    PortalMount as RadrootsAppUiPortalMount,
    Presence as RadrootsAppUiPresence,
    PresenceState as RadrootsAppUiPresenceState,
    PrimitiveAttribute,
    PrimitiveAttributeValue,
    PrimitiveElement,
    PrimitiveError,
    PrimitiveEvent,
    PrimitiveResult,
    ScrollLockError as RadrootsAppUiScrollLockError,
    ScrollLockGuard as RadrootsAppUiScrollLockGuard,
    ScrollLockResult as RadrootsAppUiScrollLockResult,
};
