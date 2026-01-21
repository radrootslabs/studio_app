use leptos::prelude::*;

#[component]
pub fn RadrootsAppUiButton(
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] style: Option<String>,
    children: Children,
) -> impl IntoView {
    let data_disabled = if disabled { Some("".to_string()) } else { None };
    view! {
        <button
            type="button"
            id=id
            class=class
            style=style
            disabled=disabled
            data-ui="button"
            data-disabled=data_disabled
        >
            {children()}
        </button>
    }
}
