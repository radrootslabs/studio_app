#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn RadrootsAppUiNavHeader(
    label: String,
    #[prop(optional)] on_label_click: Option<Callback<MouseEvent>>,
    #[prop(optional)] right: Option<AnyView>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let class_value = match class {
        Some(value) => format!("nav-header {value}"),
        None => "nav-header".to_string(),
    };
    let title_view: AnyView = if let Some(callback) = on_label_click {
        let on_click = move |ev: MouseEvent| {
            callback.run(ev);
        };
        view! {
            <button class="nav-header__title-button" on:click=on_click>
                <span class="nav-header__title-text">{label}</span>
            </button>
        }
        .into_any()
    } else {
        view! { <div class="nav-header__title-text">{label}</div> }.into_any()
    };
    view! {
        <header id=id class=class_value>
            <div class="nav-header__bar">
                <div class="nav-header__title">
                    {title_view}
                </div>
                {right
                    .map(|view| view! { <div class="nav-header__actions">{view}</div> }.into_any())
                    .unwrap_or_else(|| view! { <></> }.into_any())
                }
            </div>
        </header>
    }
}
