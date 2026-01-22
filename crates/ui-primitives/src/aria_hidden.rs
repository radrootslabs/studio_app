#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};

#[cfg(target_arch = "wasm32")]
use web_sys::{window, Element};

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
struct HiddenElement {
    element: Element,
    prev_aria_hidden: Option<String>,
    prev_inert: bool,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
struct HiddenElement;

#[derive(Debug, Clone)]
struct ModalLayer {
    id: u64,
    hidden: Vec<HiddenElement>,
}

#[derive(Debug, Default)]
struct ModalState {
    next_id: u64,
    layers: Vec<ModalLayer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadrootsAppUiModalError {
    WindowUnavailable,
    DocumentUnavailable,
    BodyUnavailable,
    AttributeUnavailable,
}

pub type RadrootsAppUiModalResult<T> = Result<T, RadrootsAppUiModalError>;

#[cfg(target_arch = "wasm32")]
pub type RadrootsAppUiModalTarget = Element;

#[cfg(not(target_arch = "wasm32"))]
pub type RadrootsAppUiModalTarget = ();

#[derive(Debug)]
pub struct RadrootsAppUiModalGuard {
    id: u64,
    active: bool,
}

impl Drop for RadrootsAppUiModalGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = radroots_studio_app_ui_modal_restore(self.id);
            self.active = false;
        }
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static MODAL_STATE: RefCell<ModalState> = RefCell::new(ModalState::default());
}

#[cfg(not(target_arch = "wasm32"))]
static MODAL_STATE: OnceLock<Mutex<ModalState>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
fn modal_state_with<T>(f: impl FnOnce(&mut ModalState) -> T) -> T {
    MODAL_STATE.with(|state| f(&mut state.borrow_mut()))
}

#[cfg(not(target_arch = "wasm32"))]
fn modal_state_with<T>(f: impl FnOnce(&mut ModalState) -> T) -> T {
    let mut state = MODAL_STATE
        .get_or_init(|| Mutex::new(ModalState::default()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut state)
}

pub fn radroots_studio_app_ui_modal_hide_siblings(
    root: &RadrootsAppUiModalTarget,
) -> RadrootsAppUiModalResult<RadrootsAppUiModalGuard> {
    let id = modal_state_with(|state| {
        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        let hidden = modal_collect_hidden(root)?;
        state.layers.push(ModalLayer { id, hidden });
        Ok(id)
    })?;
    Ok(RadrootsAppUiModalGuard { id, active: true })
}

pub fn radroots_studio_app_ui_modal_restore(id: u64) -> RadrootsAppUiModalResult<()> {
    modal_state_with(|state| {
        let index = state.layers.iter().position(|layer| layer.id == id);
        let Some(index) = index else {
            return Ok(());
        };
        let removed = state.layers.remove(index);
        modal_restore_hidden(&state.layers, removed.hidden)?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
fn modal_collect_hidden(root: &Element) -> RadrootsAppUiModalResult<Vec<HiddenElement>> {
    let window = window().ok_or(RadrootsAppUiModalError::WindowUnavailable)?;
    let document = window
        .document()
        .ok_or(RadrootsAppUiModalError::DocumentUnavailable)?;
    let body = document
        .body()
        .ok_or(RadrootsAppUiModalError::BodyUnavailable)?;
    let children = body.children();
    let mut hidden = Vec::new();
    for index in 0..children.length() {
        let Some(child) = children.item(index) else {
            continue;
        };
        if modal_is_root_or_ancestor(root, &child) {
            continue;
        }
        let prev_aria_hidden = child.get_attribute("aria-hidden");
        let prev_inert = child.has_attribute("inert");
        child
            .set_attribute("aria-hidden", "true")
            .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?;
        child
            .set_attribute("inert", "")
            .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?;
        hidden.push(HiddenElement {
            element: child,
            prev_aria_hidden,
            prev_inert,
        });
    }
    Ok(hidden)
}

#[cfg(not(target_arch = "wasm32"))]
fn modal_collect_hidden(_root: &RadrootsAppUiModalTarget) -> RadrootsAppUiModalResult<Vec<HiddenElement>> {
    Ok(Vec::new())
}

#[cfg(target_arch = "wasm32")]
fn modal_is_root_or_ancestor(root: &Element, candidate: &Element) -> bool {
    candidate.is_same_node(Some(root)) || candidate.contains(Some(root))
}

#[cfg(target_arch = "wasm32")]
fn modal_restore_hidden(
    layers: &[ModalLayer],
    hidden: Vec<HiddenElement>,
) -> RadrootsAppUiModalResult<()> {
    for item in hidden {
        if modal_is_hidden_by_layers(layers, &item.element) {
            continue;
        }
        match item.prev_aria_hidden {
            Some(value) => item
                .element
                .set_attribute("aria-hidden", &value)
                .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?,
            None => item
                .element
                .remove_attribute("aria-hidden")
                .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?,
        }
        if item.prev_inert {
            item.element
                .set_attribute("inert", "")
                .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?;
        } else {
            item.element
                .remove_attribute("inert")
                .map_err(|_| RadrootsAppUiModalError::AttributeUnavailable)?;
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn modal_restore_hidden(
    _layers: &[ModalLayer],
    _hidden: Vec<HiddenElement>,
) -> RadrootsAppUiModalResult<()> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn modal_is_hidden_by_layers(layers: &[ModalLayer], element: &Element) -> bool {
    layers.iter().any(|layer| {
        layer.hidden.iter().any(|item| {
            item.element.is_same_node(Some(element))
        })
    })
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[cfg(not(target_arch = "wasm32"))]
fn modal_is_hidden_by_layers(_layers: &[ModalLayer], _element: &RadrootsAppUiModalTarget) -> bool {
    false
}

#[cfg(test)]
fn modal_layer_count_for_test() -> usize {
    modal_state_with(|state| state.layers.len())
}

#[cfg(test)]
mod tests {
    use super::{modal_layer_count_for_test, radroots_studio_app_ui_modal_hide_siblings};

    #[test]
    fn modal_guard_tracks_layers() {
        assert_eq!(modal_layer_count_for_test(), 0);
        let guard = radroots_studio_app_ui_modal_hide_siblings(&()).expect("guard");
        assert_eq!(modal_layer_count_for_test(), 1);
        drop(guard);
        assert_eq!(modal_layer_count_for_test(), 0);
    }
}
