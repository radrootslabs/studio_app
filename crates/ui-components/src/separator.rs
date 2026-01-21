use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiSeparatorOrientation {
    Horizontal,
    Vertical,
}

impl Default for RadrootsAppUiSeparatorOrientation {
    fn default() -> Self {
        RadrootsAppUiSeparatorOrientation::Horizontal
    }
}

pub fn radroots_studio_app_ui_separator_orientation_value(
    orientation: RadrootsAppUiSeparatorOrientation,
) -> &'static str {
    match orientation {
        RadrootsAppUiSeparatorOrientation::Horizontal => "horizontal",
        RadrootsAppUiSeparatorOrientation::Vertical => "vertical",
    }
}

#[component]
pub fn RadrootsAppUiSeparator(
    #[prop(optional)] orientation: RadrootsAppUiSeparatorOrientation,
    #[prop(optional)] decorative: bool,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] style: Option<String>,
) -> impl IntoView {
    let data_orientation = radroots_studio_app_ui_separator_orientation_value(orientation);
    let data_decorative = if decorative { Some("".to_string()) } else { None };
    let role = if decorative { "presentation" } else { "separator" };
    let aria_orientation = if decorative {
        None
    } else {
        Some(data_orientation.to_string())
    };
    view! {
        <div
            id=id
            class=class
            style=style
            role=role
            data-ui="separator"
            data-orientation=data_orientation
            data-decorative=data_decorative
            aria-orientation=aria_orientation
        ></div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_separator_orientation_value,
        RadrootsAppUiSeparatorOrientation,
    };

    #[test]
    fn separator_orientation_values() {
        assert_eq!(
            radroots_studio_app_ui_separator_orientation_value(RadrootsAppUiSeparatorOrientation::Horizontal),
            "horizontal"
        );
        assert_eq!(
            radroots_studio_app_ui_separator_orientation_value(RadrootsAppUiSeparatorOrientation::Vertical),
            "vertical"
        );
    }
}
