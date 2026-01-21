use core::fmt;
use core::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiInputModality {
    Keyboard,
    Pointer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiInputModalityError {
    WindowMissing,
    DocumentMissing,
    RootMissing,
    ListenerFailed,
}

impl fmt::Display for RadrootsAppUiInputModalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RadrootsAppUiInputModalityError::WindowMissing => {
                write!(f, "input_modality_window_missing")
            }
            RadrootsAppUiInputModalityError::DocumentMissing => {
                write!(f, "input_modality_document_missing")
            }
            RadrootsAppUiInputModalityError::RootMissing => {
                write!(f, "input_modality_root_missing")
            }
            RadrootsAppUiInputModalityError::ListenerFailed => {
                write!(f, "input_modality_listener_failed")
            }
        }
    }
}

static RADROOTS_APP_UI_INPUT_MODE: AtomicU8 = AtomicU8::new(0);

pub fn radroots_studio_app_ui_input_modality_get() -> Option<RadrootsAppUiInputModality> {
    match RADROOTS_APP_UI_INPUT_MODE.load(Ordering::Relaxed) {
        1 => Some(RadrootsAppUiInputModality::Keyboard),
        2 => Some(RadrootsAppUiInputModality::Pointer),
        _ => None,
    }
}

pub fn radroots_studio_app_ui_input_modality_set(modality: RadrootsAppUiInputModality) {
    let value = match modality {
        RadrootsAppUiInputModality::Keyboard => 1,
        RadrootsAppUiInputModality::Pointer => 2,
    };
    RADROOTS_APP_UI_INPUT_MODE.store(value, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
pub fn radroots_studio_app_ui_input_modality_attach() -> Result<(), RadrootsAppUiInputModalityError> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or(RadrootsAppUiInputModalityError::WindowMissing)?;
    let document = window
        .document()
        .ok_or(RadrootsAppUiInputModalityError::DocumentMissing)?;
    let root = document
        .document_element()
        .ok_or(RadrootsAppUiInputModalityError::RootMissing)?;

    let root_keyboard = root.clone();
    let keydown = Closure::wrap(Box::new(move |_event: web_sys::KeyboardEvent| {
        radroots_studio_app_ui_input_modality_set(RadrootsAppUiInputModality::Keyboard);
        let _ = root_keyboard.set_attribute("data-input", "keyboard");
    }) as Box<dyn FnMut(_)>);
    document
        .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .map_err(|_| RadrootsAppUiInputModalityError::ListenerFailed)?;
    keydown.forget();

    let root_pointer = root.clone();
    let pointerdown = Closure::wrap(Box::new(move |_event: web_sys::PointerEvent| {
        radroots_studio_app_ui_input_modality_set(RadrootsAppUiInputModality::Pointer);
        let _ = root_pointer.set_attribute("data-input", "pointer");
    }) as Box<dyn FnMut(_)>);
    document
        .add_event_listener_with_callback("pointerdown", pointerdown.as_ref().unchecked_ref())
        .map_err(|_| RadrootsAppUiInputModalityError::ListenerFailed)?;
    pointerdown.forget();

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn radroots_studio_app_ui_input_modality_attach() -> Result<(), RadrootsAppUiInputModalityError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_input_modality_get,
        radroots_studio_app_ui_input_modality_set,
        RadrootsAppUiInputModality,
    };

    #[test]
    fn input_modality_defaults_to_none() {
        let current = radroots_studio_app_ui_input_modality_get();
        assert!(current.is_none());
    }

    #[test]
    fn input_modality_set_roundtrips() {
        radroots_studio_app_ui_input_modality_set(RadrootsAppUiInputModality::Keyboard);
        assert_eq!(
            radroots_studio_app_ui_input_modality_get(),
            Some(RadrootsAppUiInputModality::Keyboard)
        );
    }
}
