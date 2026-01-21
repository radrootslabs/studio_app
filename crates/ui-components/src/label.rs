use leptos::prelude::*;

#[component]
pub fn RadrootsAppUiLabel(
    #[prop(optional)] for_id: Option<String>,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] style: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <label
            id=id
            class=class
            style=style
            for=for_id
            data-ui="label"
        >
            {children()}
        </label>
    }
}
