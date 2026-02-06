#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

use crate::RadrootsAppUiLabel;

#[component]
pub fn RadrootsAppUiFormField(
    label: String,
    #[prop(optional)] hint: Option<String>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <section id=id class=class.unwrap_or_else(|| "form-field".to_string())>
            <RadrootsAppUiLabel class="form-field__label".to_string()>
                {label}
            </RadrootsAppUiLabel>
            <div class="form-field__control">{children()}</div>
            {move || {
                hint.clone()
                    .map(|value| view! { <p class="form-field__hint">{value}</p> }.into_any())
                    .unwrap_or_else(|| view! { <></> }.into_any())
            }}
        </section>
    }
}

#[component]
pub fn RadrootsAppUiChips(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let class_value = match class {
        Some(value) => format!("form-chips {value}"),
        None => "form-chips".to_string(),
    };
    view! {
        <div id=id class=class_value>
            {children()}
        </div>
    }
}

#[component]
pub fn RadrootsAppUiChip(
    label: String,
    active: bool,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] on_click: Option<Callback<MouseEvent>>,
) -> impl IntoView {
    let class_value = match class {
        Some(value) => format!("form-chip {value}"),
        None => "form-chip".to_string(),
    };
    let on_click = move |ev: MouseEvent| {
        if let Some(handler) = on_click {
            handler.run(ev);
        }
    };
    view! {
        <button
            type="button"
            class=class_value
            attr:data-active=move || if active { "true" } else { "false" }
            on:click=on_click
        >
            {label}
        </button>
    }
}
