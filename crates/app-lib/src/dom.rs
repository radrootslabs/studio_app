#![forbid(unsafe_code)]

use std::fmt;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomError {
    WindowUnavailable,
    DocumentUnavailable,
    QueryFailure,
    ClassListFailure,
}

impl fmt::Display for DomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomError::WindowUnavailable => f.write_str("error.app.dom.window_unavailable"),
            DomError::DocumentUnavailable => f.write_str("error.app.dom.document_unavailable"),
            DomError::QueryFailure => f.write_str("error.app.dom.query_failure"),
            DomError::ClassListFailure => f.write_str("error.app.dom.class_list_failure"),
        }
    }
}

impl std::error::Error for DomError {}

pub fn view_effect(view: &str) -> Result<(), DomError> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().ok_or(DomError::WindowUnavailable)?;
        let document = window.document().ok_or(DomError::DocumentUnavailable)?;
        let nodes = document
            .query_selector_all("[data-view]")
            .map_err(|_| DomError::QueryFailure)?;
        for idx in 0..nodes.length() {
            let Some(node) = nodes.get(idx) else {
                continue;
            };
            let element: web_sys::Element = node.unchecked_into();
            let attr = element.get_attribute("data-view").unwrap_or_default();
            let class_list = element.class_list();
            if attr != view {
                class_list
                    .add_1("hidden")
                    .map_err(|_| DomError::ClassListFailure)?;
            } else {
                class_list
                    .remove_1("hidden")
                    .map_err(|_| DomError::ClassListFailure)?;
            }
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = view;
        Err(DomError::WindowUnavailable)
    }
}

pub fn el_id(id: &str) -> Option<web_sys::Element> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let document = window.document()?;
        document.get_element_by_id(id)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{el_id, view_effect, DomError};

    #[test]
    fn view_effect_errors_on_non_wasm() {
        let err = view_effect("home").expect_err("non-wasm");
        assert_eq!(err, DomError::WindowUnavailable);
    }

    #[test]
    fn el_id_returns_none_on_non_wasm() {
        assert!(el_id("missing").is_none());
    }
}
