#![forbid(unsafe_code)]

use leptos::prelude::*;

const NAV_TABS_HIDE_VELOCITY: f64 = 220.0;
const NAV_TABS_SHOW_VELOCITY: f64 = 120.0;
const NAV_TABS_HIDE_SCROLL: f64 = 40.0;

#[component]
pub fn RadrootsAppUiNavTabs(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] auto_hide: Option<bool>,
    children: Children,
) -> impl IntoView {
    let auto_hide = auto_hide.unwrap_or(true);
    let scroll_context = use_context::<crate::RadrootsAppUiScrollContext>();
    let hidden = RwSignal::new(false);
    Effect::new(move || {
        if !auto_hide {
            hidden.set(false);
            return;
        }
        let Some(context) = scroll_context.as_ref() else {
            hidden.set(false);
            return;
        };
        let scroll_top = context.scroll_top.get();
        if scroll_top <= NAV_TABS_HIDE_SCROLL {
            hidden.set(false);
            return;
        }
        let velocity = context.scroll_velocity.get();
        if velocity > NAV_TABS_HIDE_VELOCITY {
            hidden.set(true);
        } else if velocity < -NAV_TABS_SHOW_VELOCITY {
            hidden.set(false);
        }
    });
    let class_value = match class {
        Some(value) => format!("nav-tabs {value}"),
        None => "nav-tabs".to_string(),
    };
    view! {
        <nav
            id=id
            class=class_value
            attr:data-hidden=move || if hidden.get() { "true" } else { "false" }
        >
            <div class="nav-tabs__tray">
                {children()}
            </div>
        </nav>
    }
}
