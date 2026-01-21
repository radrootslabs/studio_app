use leptos::ev::KeyboardEvent;
use leptos::html;
use leptos::prelude::*;

const RADROOTS_APP_UI_FOCUSABLE_SELECTOR: &str =
    "a[href],button,textarea,input,select,[tabindex]:not([tabindex='-1'])";

pub fn radroots_studio_app_ui_focus_scope_selector() -> &'static str {
    RADROOTS_APP_UI_FOCUSABLE_SELECTOR
}

pub fn radroots_studio_app_ui_focus_scope_next_index(
    current: usize,
    count: usize,
    shift: bool,
) -> usize {
    if count == 0 {
        return 0;
    }
    if shift {
        if current == 0 {
            count - 1
        } else {
            current - 1
        }
    } else if current + 1 >= count {
        0
    } else {
        current + 1
    }
}

#[component]
pub fn RadrootsAppUiFocusScope(
    #[prop(optional)] trapped: bool,
    #[prop(optional)] auto_focus: bool,
    #[prop(optional)] return_focus: bool,
    #[prop(optional)] on_mount_auto_focus: Option<Callback<()>>,
    #[prop(optional)] on_unmount_auto_focus: Option<Callback<()>>,
    children: ChildrenFn,
) -> impl IntoView {
    let node_ref = NodeRef::<html::Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let on_mount_auto_focus = on_mount_auto_focus.clone();
        let on_unmount_auto_focus = on_unmount_auto_focus.clone();
        let node_ref = node_ref.clone();

        on_mount(move || {
            let document = match window().and_then(|window| window.document()) {
                Some(document) => document,
                None => return,
            };
            let root = match node_ref.get() {
                Some(root) => root,
                None => return,
            };
            let previous_focus = document.active_element();

            if auto_focus {
                let _ = radroots_studio_app_ui_focus_scope_focus_first(&root, &document);
                if let Some(callback) = on_mount_auto_focus.as_ref() {
                    callback.run(());
                }
            }

            if trapped {
                let root_focus = root.clone();
                let document_focus = document.clone();
                let handler = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if event.key() != "Tab" {
                        return;
                    }
                    let _ = radroots_studio_app_ui_focus_scope_cycle(
                        &root_focus,
                        &document_focus,
                        event.shift_key(),
                    );
                }) as Box<dyn FnMut(_)>);
                let _ = root.add_event_listener_with_callback(
                    "keydown",
                    handler.as_ref().unchecked_ref(),
                );
                handler.forget();
            }

            if return_focus {
                let on_unmount_auto_focus = on_unmount_auto_focus.clone();
                let previous_focus = previous_focus.clone();
                on_cleanup(move || {
                    if let Some(element) = previous_focus {
                        let _ = element.dyn_ref::<web_sys::HtmlElement>().map(|el| el.focus());
                    }
                    if let Some(callback) = on_unmount_auto_focus.as_ref() {
                        callback.run(());
                    }
                });
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = trapped;
        let _ = auto_focus;
        let _ = return_focus;
        let _ = on_mount_auto_focus;
        let _ = on_unmount_auto_focus;
    }

    let on_keydown = move |event: KeyboardEvent| {
        if trapped && event.key() == "Escape" {
            event.prevent_default();
        }
    };

    view! {
        <div node_ref=node_ref tabindex="-1" on:keydown=on_keydown>
            {children()}
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn radroots_studio_app_ui_focus_scope_focus_first(
    root: &web_sys::Element,
    document: &web_sys::Document,
) -> Result<(), ()> {
    let list = root
        .query_selector_all(RADROOTS_APP_UI_FOCUSABLE_SELECTOR)
        .map_err(|_| ())?;
    if list.length() == 0 {
        if let Some(element) = root.dyn_ref::<web_sys::HtmlElement>() {
            element.focus();
        }
        return Ok(());
    }
    let first = list
        .item(0)
        .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
        .or_else(|| document.active_element().and_then(|el| el.dyn_into().ok()));
    if let Some(element) = first {
        element.focus();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn radroots_studio_app_ui_focus_scope_cycle(
    root: &web_sys::Element,
    document: &web_sys::Document,
    shift: bool,
) -> Result<(), ()> {
    let list = root
        .query_selector_all(RADROOTS_APP_UI_FOCUSABLE_SELECTOR)
        .map_err(|_| ())?;
    let count = list.length() as usize;
    if count == 0 {
        return Ok(());
    }
    let active = document.active_element();
    let mut current_index = 0usize;
    if let Some(active) = active {
        for index in 0..count {
            let candidate = list
                .item(index as u32)
                .and_then(|node| node.dyn_into::<web_sys::Element>().ok());
            if let Some(candidate) = candidate {
                if active.is_same_node(Some(&candidate)) {
                    current_index = index;
                    break;
                }
            }
        }
    }
    let next_index = radroots_studio_app_ui_focus_scope_next_index(current_index, count, shift);
    if let Some(next) = list
        .item(next_index as u32)
        .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
    {
        next.focus();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_focus_scope_next_index,
        radroots_studio_app_ui_focus_scope_selector,
    };

    #[test]
    fn focus_scope_selector_is_non_empty() {
        assert!(!radroots_studio_app_ui_focus_scope_selector().is_empty());
    }

    #[test]
    fn focus_scope_next_index_wraps() {
        assert_eq!(radroots_studio_app_ui_focus_scope_next_index(0, 3, true), 2);
        assert_eq!(radroots_studio_app_ui_focus_scope_next_index(2, 3, false), 0);
    }
}
