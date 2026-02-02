#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::*;

use crate::RadrootsAppUiSpinner;

fn radroots_studio_app_ui_button_class_merge(parts: &[Option<&str>]) -> String {
    let mut result = String::new();
    for part in parts {
        let Some(value) = part else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(value);
    }
    result
}

#[derive(Clone)]
pub struct RadrootsAppUiButtonLayoutAction {
    pub label: String,
    pub disabled: bool,
    pub loading: bool,
    pub on_click: Callback<MouseEvent>,
    pub class: Option<String>,
    pub class_label: Option<String>,
    pub style: Option<String>,
}

#[derive(Clone)]
pub struct RadrootsAppUiButtonLayoutBackAction {
    pub visible: bool,
    pub label: Option<String>,
    pub disabled: bool,
    pub on_click: Callback<MouseEvent>,
    pub compact: bool,
}

#[component]
pub fn RadrootsAppUiButtonLayout(
    label: String,
    on_click: Callback<MouseEvent>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] class_label: Option<String>,
    #[prop(optional)] style: Option<String>,
    #[prop(optional)] hide_active: bool,
) -> impl IntoView {
    let allow_active = !disabled && !hide_active;
    let base_class = if allow_active {
        "button-layout"
    } else {
        "flex flex-row h-touch_guide w-lo_ios0 ios1:w-lo_ios1 justify-center items-center bg-ly1 rounded-touch el-re disabled:opacity-60"
    };
    let button_class = radroots_studio_app_ui_button_class_merge(&[
        if allow_active { Some("group") } else { None },
        Some(base_class),
        class.as_deref(),
    ]);
    let label_class = radroots_studio_app_ui_button_class_merge(&[
        Some("button-layout-label"),
        class_label.as_deref(),
    ]);
    view! {
        <button
            type="button"
            class=button_class
            style=style
            disabled=disabled
            on:click=move |ev| {
                ev.stop_propagation();
                if disabled {
                    return;
                }
                on_click.run(ev);
            }
        >
            {move || {
                if loading {
                    view! { <RadrootsAppUiSpinner class="text-[18px]".to_string() /> }.into_any()
                } else {
                    view! { <span class=label_class.clone()>{label.clone()}</span> }.into_any()
                }
            }}
        </button>
    }
}

#[component]
pub fn RadrootsAppUiButtonLayoutPair(
    continue_action: RadrootsAppUiButtonLayoutAction,
    #[prop(optional)] back: Option<RadrootsAppUiButtonLayoutBackAction>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView {
    let wrapper_class = radroots_studio_app_ui_button_class_merge(&[
        Some("flex flex-col gap-1 justify-center items-center"),
        class.as_deref(),
    ]);
    view! {
        <div class=wrapper_class>
            <RadrootsAppUiButtonLayout
                label=continue_action.label
                disabled=continue_action.disabled
                loading=continue_action.loading
                on_click=continue_action.on_click
                class=continue_action.class.unwrap_or_default()
                class_label=continue_action.class_label.unwrap_or_default()
                style=continue_action.style.unwrap_or_default()
            />
            {back.map(|back_action| {
                view! {
                    <div class="flex flex-col justify-center items-center">
                        {{
                            let back_label = back_action.label.clone().unwrap_or_default();
                            let back_disabled = back_action.disabled;
                            let back_on_click = back_action.on_click.clone();
                            let back_visible = back_action.visible;
                            let back_compact = back_action.compact;
                            let back_text_class = radroots_studio_app_ui_button_class_merge(&[
                                Some("font-sans font-[600] tracking-wide text-ly1-gl-shade"),
                                if back_disabled { None } else { Some("group-active:text-ly1-gl/40") },
                            ]);
                            let back_button_class = if back_compact {
                                radroots_studio_app_ui_button_class_merge(&[
                                    if back_disabled { None } else { Some("group") },
                                    Some("flex flex-row w-fit justify-center items-center py-1 transition-opacity duration-[160ms] ease-[cubic-bezier(.2,.8,.2,1)]"),
                                    if back_visible { Some("opacity-100") } else { Some("opacity-0 pointer-events-none") },
                                ])
                            } else {
                                radroots_studio_app_ui_button_class_merge(&[
                                    if back_disabled { None } else { Some("group") },
                                    Some("flex flex-row h-12 w-lo_ios0 ios1:w-lo_ios1 justify-center items-center -translate-y-[2px] transition-opacity duration-[160ms] ease-[cubic-bezier(.2,.8,.2,1)]"),
                                    if back_visible { Some("opacity-100") } else { Some("opacity-0 pointer-events-none") },
                                ])
                            };
                            view! {
                                <button
                                    type="button"
                                    class=back_button_class
                                    disabled=back_disabled
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        if back_disabled {
                                            return;
                                        }
                                        back_on_click.run(ev);
                                    }
                                >
                                    <span class=back_text_class>{back_label}</span>
                                </button>
                            }.into_any()
                        }}
                    </div>
                }
            })}
        </div>
    }
}

#[component]
pub fn RadrootsAppUiButtonLayoutBottom(
    #[prop(optional)] hidden: bool,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    if hidden {
        view! { <></> }.into_any()
    } else {
        let wrapper_class = radroots_studio_app_ui_button_class_merge(&[
            Some("z-10 absolute bottom-0 h-lo_bottom_button_ios0 ios1:h-lo_bottom_button_ios1 flex flex-col w-full px-4 gap-1 justify-start items-center"),
            class.as_deref(),
        ]);
        view! {
            <div class=wrapper_class>
                {children()}
            </div>
        }.into_any()
    }
}
