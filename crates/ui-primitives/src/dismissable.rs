use leptos::ev::{FocusEvent, KeyboardEvent, PointerEvent};
use leptos::html;
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiDismissableReason {
    Escape,
    PointerDownOutside,
    FocusOutside,
}

pub fn radroots_studio_app_ui_dismissable_is_escape(key: &str) -> bool {
    key == "Escape"
}

pub fn radroots_studio_app_ui_dismissable_is_outside(is_inside: bool) -> bool {
    !is_inside
}

#[component]
pub fn RadrootsAppUiDismissableLayer(
    #[prop(optional)] on_dismiss: Option<Callback<RadrootsAppUiDismissableReason>>,
    #[prop(optional)] on_escape_key_down: Option<Callback<KeyboardEvent>>,
    #[prop(optional)] on_pointer_down_outside: Option<Callback<PointerEvent>>,
    #[prop(optional)] on_focus_outside: Option<Callback<FocusEvent>>,
    #[prop(optional)] disable_outside_pointer_events: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let node_ref = NodeRef::<html::Div>::new();

    let on_keydown = move |event: KeyboardEvent| {
        if !radroots_studio_app_ui_dismissable_is_escape(&event.key()) {
            return;
        }
        if let Some(callback) = on_escape_key_down.as_ref() {
            callback.run(event.clone());
        }
        if let Some(callback) = on_dismiss.as_ref() {
            callback.run(RadrootsAppUiDismissableReason::Escape);
        }
    };

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let on_dismiss = on_dismiss.clone();
        let on_pointer_down_outside = on_pointer_down_outside.clone();
        let on_focus_outside = on_focus_outside.clone();
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

            if !disable_outside_pointer_events {
                let root_pointer = root.clone();
                let on_dismiss = on_dismiss.clone();
                let on_pointer_down_outside = on_pointer_down_outside.clone();
                let handler = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
                    let target = event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::Node>().ok());
                    let is_inside = target
                        .as_ref()
                        .map(|node| root_pointer.contains(Some(node)))
                        .unwrap_or(false);
                    if radroots_studio_app_ui_dismissable_is_outside(is_inside) {
                        if let Some(callback) = on_pointer_down_outside.as_ref() {
                            callback.run(event.clone());
                        }
                        if let Some(callback) = on_dismiss.as_ref() {
                            callback.run(RadrootsAppUiDismissableReason::PointerDownOutside);
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                let _ = document.add_event_listener_with_callback(
                    "pointerdown",
                    handler.as_ref().unchecked_ref(),
                );
                handler.forget();
            }

            let root_focus = root.clone();
            let on_dismiss = on_dismiss.clone();
            let on_focus_outside = on_focus_outside.clone();
            let focus_handler = Closure::wrap(Box::new(move |event: web_sys::FocusEvent| {
                let target = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Node>().ok());
                let is_inside = target
                    .as_ref()
                    .map(|node| root_focus.contains(Some(node)))
                    .unwrap_or(false);
                if radroots_studio_app_ui_dismissable_is_outside(is_inside) {
                    if let Some(callback) = on_focus_outside.as_ref() {
                        callback.run(event.clone());
                    }
                    if let Some(callback) = on_dismiss.as_ref() {
                        callback.run(RadrootsAppUiDismissableReason::FocusOutside);
                    }
                }
            }) as Box<dyn FnMut(_)>);
            let _ = document.add_event_listener_with_callback(
                "focusin",
                focus_handler.as_ref().unchecked_ref(),
            );
            focus_handler.forget();
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = on_pointer_down_outside;
        let _ = on_focus_outside;
        let _ = disable_outside_pointer_events;
    }

    view! {
        <div node_ref=node_ref on:keydown=on_keydown>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_dismissable_is_escape,
        radroots_studio_app_ui_dismissable_is_outside,
    };

    #[test]
    fn dismissable_escape_match() {
        assert!(radroots_studio_app_ui_dismissable_is_escape("Escape"));
        assert!(!radroots_studio_app_ui_dismissable_is_escape("Enter"));
    }

    #[test]
    fn dismissable_outside_check() {
        assert!(radroots_studio_app_ui_dismissable_is_outside(false));
        assert!(!radroots_studio_app_ui_dismissable_is_outside(true));
    }
}
