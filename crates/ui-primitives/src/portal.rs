use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use leptos::portal::Portal;

#[cfg(target_arch = "wasm32")]
pub type RadrootsAppUiPortalMount = web_sys::Element;

#[cfg(not(target_arch = "wasm32"))]
pub type RadrootsAppUiPortalMount = ();

#[component]
pub fn RadrootsAppUiPortal(
    #[prop(optional)] mount: Option<RadrootsAppUiPortalMount>,
    children: ChildrenFn,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        match mount {
            Some(mount) => view! { <Portal mount=mount>{children()}</Portal> },
            None => view! { <Portal>{children()}</Portal> },
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = mount;
        children()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn portal_availability_matches_target() {
        assert!(!cfg!(target_arch = "wasm32"));
    }
}
