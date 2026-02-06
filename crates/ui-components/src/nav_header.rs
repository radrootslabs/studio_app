#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiNavHeaderBgMode {
    Transparent,
    Opaque,
    Blur,
    AutoOpaque,
    AutoBlur,
}

impl RadrootsAppUiNavHeaderBgMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsAppUiNavHeaderBgMode::Transparent => "transparent",
            RadrootsAppUiNavHeaderBgMode::Opaque => "opaque",
            RadrootsAppUiNavHeaderBgMode::Blur => "blur",
            RadrootsAppUiNavHeaderBgMode::AutoOpaque => "auto-opaque",
            RadrootsAppUiNavHeaderBgMode::AutoBlur => "auto-blur",
        }
    }

    pub const fn is_auto(self) -> bool {
        matches!(
            self,
            RadrootsAppUiNavHeaderBgMode::AutoOpaque | RadrootsAppUiNavHeaderBgMode::AutoBlur
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiNavHeaderCollapseMode {
    None,
    Scroll,
}

#[component]
pub fn RadrootsAppUiNavHeader(
    label: String,
    on_label_click: Option<Callback<MouseEvent>>,
    bg_mode: Option<RadrootsAppUiNavHeaderBgMode>,
    collapse_mode: Option<RadrootsAppUiNavHeaderCollapseMode>,
    right: Option<ChildrenFn>,
    id: Option<String>,
    class: Option<String>,
) -> impl IntoView {
    let bg_mode = bg_mode.unwrap_or(RadrootsAppUiNavHeaderBgMode::AutoBlur);
    let collapse_mode = collapse_mode.unwrap_or(RadrootsAppUiNavHeaderCollapseMode::Scroll);
    let class_value = match class {
        Some(value) => format!("nav-header {value}"),
        None => "nav-header".to_string(),
    };
    let label_large = label.clone();
    let label_compact = label.clone();
    let title_large = nav_header_title_view(
        label_large,
        "nav-header__title-text nav-header__title-large",
        on_label_click.clone(),
    );
    let title_compact = nav_header_title_view(
        label_compact,
        "nav-header__title-text nav-header__title-compact",
        on_label_click,
    );
    let scroll_context = use_context::<crate::RadrootsAppUiScrollContext>();
    let collapse_progress = Signal::derive(move || match collapse_mode {
        RadrootsAppUiNavHeaderCollapseMode::None => 0.0,
        RadrootsAppUiNavHeaderCollapseMode::Scroll => scroll_context
            .as_ref()
            .map(|context| context.collapse_progress.get())
            .unwrap_or(0.0),
    });
    let bg_active = Signal::derive(move || {
        let scrolled = collapse_progress.get() > 0.02;
        match bg_mode {
            RadrootsAppUiNavHeaderBgMode::Transparent => false,
            RadrootsAppUiNavHeaderBgMode::Opaque | RadrootsAppUiNavHeaderBgMode::Blur => true,
            RadrootsAppUiNavHeaderBgMode::AutoOpaque | RadrootsAppUiNavHeaderBgMode::AutoBlur => {
                scrolled
            }
        }
    });
    let show_actions = Signal::derive(move || !bg_active.get());
    let right_slot = right;
    view! {
        <header
            id=id
            class=class_value
            attr:data-bg=bg_mode.as_str()
            attr:data-bg-state=move || if bg_active.get() { "active" } else { "idle" }
            style=move || format!("--collapse: {:.3};", collapse_progress.get())
        >
            <div class="nav-header__background" aria-hidden="true"></div>
            <div class="nav-header__content">
                <div class="nav-header__bar">
                    <div class="nav-header__compact">
                        {title_compact}
                    </div>
                    {move || {
                        if show_actions.get() {
                            right_slot
                                .as_ref()
                                .map(|slot| slot())
                                .map(|view| view! { <div class="nav-header__actions">{view}</div> }.into_any())
                                .unwrap_or_else(|| view! { <></> }.into_any())
                        } else {
                            view! { <></> }.into_any()
                        }
                    }}
                </div>
                <div class="nav-header__large">
                    {title_large}
                </div>
            </div>
        </header>
    }
}

fn nav_header_title_view(
    label: String,
    class: &str,
    on_label_click: Option<Callback<MouseEvent>>,
) -> AnyView {
    if let Some(callback) = on_label_click {
        let on_click = move |ev: MouseEvent| {
            callback.run(ev);
        };
        view! {
            <button class="nav-header__title-button" on:click=on_click>
                <span class=class>{label}</span>
            </button>
        }
        .into_any()
    } else {
        view! { <span class=class>{label}</span> }.into_any()
    }
}
