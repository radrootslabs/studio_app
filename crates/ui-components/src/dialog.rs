use leptos::ev::MouseEvent;
use leptos::html;
use leptos::prelude::*;
use std::sync::{Arc, Mutex};

use radroots_studio_app_ui_core::RadrootsAppUiId;
use radroots_studio_app_ui_primitives::{
    dialog_content_attrs,
    dialog_trigger_attrs,
    use_primitive,
    DialogModel,
    RadrootsAppUiDismissableReason,
    RadrootsAppUiDismissableLayer,
    RadrootsAppUiFocusScope,
    RadrootsAppUiModalGuard,
    RadrootsAppUiPresence,
    RadrootsAppUiPortal,
    RadrootsAppUiScrollLockGuard,
};

#[cfg(target_arch = "wasm32")]
use radroots_studio_app_ui_primitives::{
    radroots_studio_app_ui_modal_hide_siblings,
    radroots_studio_app_ui_scroll_lock_acquire,
};

#[derive(Clone)]
struct RadrootsAppUiDialogContext {
    open: ReadSignal<bool>,
    set_open: Callback<bool>,
    dismiss: Callback<RadrootsAppUiDismissableReason>,
    modal: bool,
    content_id: String,
    title_id: RwSignal<Option<String>>,
    description_id: RwSignal<Option<String>>,
}

pub fn radroots_studio_app_ui_dialog_state_value(open: bool) -> &'static str {
    if open {
        "open"
    } else {
        "closed"
    }
}

#[component]
pub fn RadrootsAppUiDialogRoot(
    open: Option<ReadSignal<bool>>,
    #[prop(optional)] default_open: bool,
    modal: Option<bool>,
    on_open_change: Option<Callback<bool>>,
    children: ChildrenFn,
) -> impl IntoView {
    let open_state = RwSignal::new(default_open);
    let open_prop = open;
    let is_controlled = open_prop.is_some();
    let open_signal = match open_prop {
        Some(open) => open,
        None => open_state.read_only(),
    };
    let on_open_change = on_open_change.clone();
    let set_open = Callback::new(move |value| {
        if !is_controlled {
            open_state.set(value);
        }
        if let Some(callback) = on_open_change.as_ref() {
            callback.run(value);
        }
    });
    let dismiss = {
        let set_open = set_open.clone();
        Callback::new(move |_reason: RadrootsAppUiDismissableReason| {
            set_open.run(false);
        })
    };
    let content_id = RadrootsAppUiId::next().prefixed("dialog-content");
    let modal = modal.unwrap_or(true);
    let title_id = RwSignal::new(None::<String>);
    let description_id = RwSignal::new(None::<String>);
    provide_context(RadrootsAppUiDialogContext {
        open: open_signal,
        set_open,
        dismiss,
        modal,
        content_id,
        title_id,
        description_id,
    });
    view! { <>{children()}</> }
}

#[component]
pub fn RadrootsAppUiDialogTrigger(
    #[prop(optional)] disabled: bool,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let open = context.open;
    let content_id = context.content_id.clone();
    let attrs = Signal::derive(move || {
        let model = DialogModel::new(open.get());
        dialog_trigger_attrs(&model, Some(content_id.as_str()))
    });
    let trigger = use_primitive::<html::Button>(attrs, Vec::new());
    let on_click = move |_event: MouseEvent| {
        if disabled {
            return;
        }
        context.set_open.run(true);
    };
    view! {
        <button
            node_ref=trigger.node_ref()
            type="button"
            id=id
            class=class
            style=style
            disabled=disabled
            data-ui="dialog-trigger"
            on:click=on_click
        >
            {children()}
        </button>
    }
}

#[component]
pub fn RadrootsAppUiDialogPortal(children: ChildrenFn) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let present = Signal::derive(move || context.open.get());
    let children = StoredValue::new(children);
    view! {
        <RadrootsAppUiPortal>
            <RadrootsAppUiPresence present=present>
                {(children.get_value())()}
            </RadrootsAppUiPresence>
        </RadrootsAppUiPortal>
    }
}

#[component]
pub fn RadrootsAppUiDialogOverlay(
    close_on_click: Option<bool>,
    data_ui: Option<String>,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let close_on_click = close_on_click.unwrap_or(true);
    let data_ui = StoredValue::new(data_ui.unwrap_or_else(|| "dialog-overlay".to_string()));
    let on_click = move |_event: MouseEvent| {
        if close_on_click {
            context.set_open.run(false);
        }
    };
    view! {
        <div
            id=id
            class=class
            style=style
            data-ui=move || data_ui.get_value()
            data-state=move || radroots_studio_app_ui_dialog_state_value(context.open.get())
            on:click=on_click
        ></div>
    }
}

#[component]
pub fn RadrootsAppUiDialogContent(
    #[prop(optional)] disable_outside_pointer_events: bool,
    data_ui: Option<String>,
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: ChildrenFn,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let content_id = context.content_id.clone();
    let open = context.open;
    let title_id = context.title_id;
    let description_id = context.description_id;
    let scroll_guard = Arc::new(Mutex::new(None::<RadrootsAppUiScrollLockGuard>));
    let modal_guard = Arc::new(Mutex::new(None::<RadrootsAppUiModalGuard>));
    let modal = context.modal;
    let attrs = Signal::derive(move || {
        let mut model = DialogModel::new(open.get());
        model.set_modal(modal);
        dialog_content_attrs(
            &model,
            title_id.get().as_deref(),
            description_id.get().as_deref(),
        )
    });
    let primitive = use_primitive::<html::Div>(attrs, Vec::new());
    let node_ref = primitive.node_ref();

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        use leptos::web_sys;

        let node_ref = node_ref;
        let scroll_guard = Arc::clone(&scroll_guard);
        let modal_guard = Arc::clone(&modal_guard);
        node_ref.on_load(move |root| {
            if modal {
                if let Ok(guard) = radroots_studio_app_ui_scroll_lock_acquire() {
                    let mut state = scroll_guard
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *state = Some(guard);
                }
                let element: web_sys::Element = root.unchecked_into();
                if let Ok(guard) = radroots_studio_app_ui_modal_hide_siblings(&element) {
                    let mut state = modal_guard
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *state = Some(guard);
                }
            }
        });
    }

    let scroll_guard_cleanup = Arc::clone(&scroll_guard);
    let modal_guard_cleanup = Arc::clone(&modal_guard);
    on_cleanup(move || {
        let _ = scroll_guard_cleanup
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let _ = modal_guard_cleanup
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
    });

    let on_dismiss = context.dismiss.clone();

    let data_ui = StoredValue::new(data_ui.unwrap_or_else(|| "dialog".to_string()));
    let id_value = StoredValue::new(id.unwrap_or_else(|| content_id.clone()));
    let class_value = StoredValue::new(class);
    let style_value = StoredValue::new(style);
    let children = StoredValue::new(children);

    view! {
        <RadrootsAppUiDismissableLayer
            on_dismiss=on_dismiss
            disable_pointer_down_outside_dismiss=disable_outside_pointer_events
        >
            <RadrootsAppUiFocusScope trapped=modal auto_focus=true return_focus=true>
                <div
                    node_ref=node_ref
                    id=move || id_value.get_value()
                    class=move || class_value.get_value()
                    style=move || style_value.get_value()
                    data-ui=move || data_ui.get_value()
                >
                    {(children.get_value())()}
                </div>
            </RadrootsAppUiFocusScope>
        </RadrootsAppUiDismissableLayer>
    }
}

#[component]
pub fn RadrootsAppUiDialogTitle(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let title_id = id.unwrap_or_else(|| RadrootsAppUiId::next().prefixed("dialog-title"));
    context.title_id.set(Some(title_id.clone()));
    let title_id_cleanup = title_id.clone();
    let title_signal = context.title_id;
    on_cleanup(move || {
        if title_signal.get_untracked().as_deref() == Some(&title_id_cleanup) {
            title_signal.set(None);
        }
    });
    view! {
        <h2
            id=title_id
            class=class
            style=style
            data-ui="dialog-title"
        >
            {children()}
        </h2>
    }
}

#[component]
pub fn RadrootsAppUiDialogDescription(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let description_id = id.unwrap_or_else(|| RadrootsAppUiId::next().prefixed("dialog-desc"));
    context.description_id.set(Some(description_id.clone()));
    let desc_id_cleanup = description_id.clone();
    let desc_signal = context.description_id;
    on_cleanup(move || {
        if desc_signal.get_untracked().as_deref() == Some(&desc_id_cleanup) {
            desc_signal.set(None);
        }
    });
    view! {
        <p
            id=description_id
            class=class
            style=style
            data-ui="dialog-description"
        >
            {children()}
        </p>
    }
}

#[component]
pub fn RadrootsAppUiDialogClose(
    class: Option<String>,
    id: Option<String>,
    style: Option<String>,
    children: Children,
) -> impl IntoView {
    let context = use_context::<RadrootsAppUiDialogContext>()
        .expect("dialog context");
    let on_click = move |_event: MouseEvent| {
        context.set_open.run(false);
    };
    view! {
        <button
            type="button"
            id=id
            class=class
            style=style
            data-ui="dialog-close"
            on:click=on_click
        >
            {children()}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::radroots_studio_app_ui_dialog_state_value;

    #[test]
    fn dialog_state_value_matches_open() {
        assert_eq!(radroots_studio_app_ui_dialog_state_value(true), "open");
        assert_eq!(radroots_studio_app_ui_dialog_state_value(false), "closed");
    }
}
