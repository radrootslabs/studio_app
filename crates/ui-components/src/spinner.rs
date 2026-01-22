#![forbid(unsafe_code)]

use leptos::prelude::*;

const RADROOTS_APP_UI_SPINNER_BLADE_COUNT: usize = 8;

fn radroots_studio_app_ui_spinner_class_merge(parts: &[Option<&str>]) -> String {
    let mut result = String::new();
    for part in parts {
        if let Some(value) = part {
            if value.is_empty() {
                continue;
            }
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(value);
        }
    }
    result
}

#[component]
pub fn RadrootsAppUiSpinner(
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] style: Option<String>,
    #[prop(optional)] white: bool,
    #[prop(optional)] center: bool,
) -> impl IntoView {
    let class_value = radroots_studio_app_ui_spinner_class_merge(&[
        Some("spinner8"),
        if white { Some("spinner8-white") } else { None },
        if center { Some("center") } else { None },
        class.as_deref(),
    ]);
    let blade_class = radroots_studio_app_ui_spinner_class_merge(&[
        Some("spinner8-blade"),
        if white { Some("spinner8-blade-white") } else { None },
    ]);
    let blades = (0..RADROOTS_APP_UI_SPINNER_BLADE_COUNT)
        .map(|_| view! { <span class=blade_class.clone()></span> })
        .collect_view();
    view! {
        <span id=id class=class_value style=style>
            {blades}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::RADROOTS_APP_UI_SPINNER_BLADE_COUNT;

    #[test]
    fn spinner_blade_count_is_expected() {
        assert_eq!(RADROOTS_APP_UI_SPINNER_BLADE_COUNT, 8);
    }
}
