use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
pub type RadrootsAppUiPortalMount = web_sys::Element;

#[cfg(not(target_arch = "wasm32"))]
pub type RadrootsAppUiPortalMount = ();

#[component]
pub fn RadrootsAppUiPortal(
    #[prop(into, optional)] mount: Option<RadrootsAppUiPortalMount>,
    children: ChildrenFn,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        view! { <Portal mount=mount>{children()}</Portal> }
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
