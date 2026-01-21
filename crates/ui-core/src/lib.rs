#![forbid(unsafe_code)]
#![no_std]

extern crate alloc;

mod id;
mod event;
mod input;

pub use event::{
    radroots_studio_app_ui_compose_event_handlers,
    radroots_studio_app_ui_compose_event_handlers_unchecked,
};
pub use id::{RadrootsAppUiId, RadrootsAppUiIdSequence};
pub use input::{
    radroots_studio_app_ui_input_modality_attach,
    radroots_studio_app_ui_input_modality_get,
    radroots_studio_app_ui_input_modality_set,
    RadrootsAppUiInputModality,
    RadrootsAppUiInputModalityError,
};
