#![forbid(unsafe_code)]

use icondata::{Icon, LuChevronRight, LuChevronsUpDown, LuPlus};
use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RadrootsAppUiIconKey {
    CaretRight,
    CaretUpDown,
    Plus,
}

impl RadrootsAppUiIconKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsAppUiIconKey::CaretRight => "caret-right",
            RadrootsAppUiIconKey::CaretUpDown => "caret-up-down",
            RadrootsAppUiIconKey::Plus => "plus",
        }
    }
}

pub fn radroots_studio_app_ui_icon_key_from_name(name: &str) -> Option<RadrootsAppUiIconKey> {
    match name {
        "caret-right" | "chevron-right" => Some(RadrootsAppUiIconKey::CaretRight),
        "caret-up-down" | "chevrons-up-down" => Some(RadrootsAppUiIconKey::CaretUpDown),
        "plus" => Some(RadrootsAppUiIconKey::Plus),
        _ => None,
    }
}

pub fn radroots_studio_app_ui_icon_data(key: RadrootsAppUiIconKey) -> Icon {
    match key {
        RadrootsAppUiIconKey::CaretRight => LuChevronRight,
        RadrootsAppUiIconKey::CaretUpDown => LuChevronsUpDown,
        RadrootsAppUiIconKey::Plus => LuPlus,
    }
}

#[component]
pub fn RadrootsAppUiIcon(
    key: RadrootsAppUiIconKey,
    #[prop(optional)] class: Option<String>,
    #[prop(optional)] size: Option<u32>,
) -> impl IntoView {
    let icon = radroots_studio_app_ui_icon_data(key);
    let class_value = class.unwrap_or_default();
    let size_value = size.unwrap_or(20).to_string();
    let view_box = icon.view_box.unwrap_or("0 0 24 24");
    let stroke = icon.stroke.unwrap_or("currentColor");
    let fill = icon.fill.unwrap_or("none");
    let stroke_width = icon.stroke_width.unwrap_or("2");
    let stroke_linecap = icon.stroke_linecap.unwrap_or("round");
    let stroke_linejoin = icon.stroke_linejoin.unwrap_or("round");
    view! {
        <svg
            class=class_value
            width=size_value.clone()
            height=size_value
            viewBox=view_box
            fill=fill
            stroke=stroke
            stroke-width=stroke_width
            stroke-linecap=stroke_linecap
            stroke-linejoin=stroke_linejoin
            xmlns="http://www.w3.org/2000/svg"
            focusable="false"
            aria-hidden="true"
            attr:style=icon.style
            attr:x=icon.x
            attr:y=icon.y
            inner_html=icon.data
        />
    }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_icon_key_from_name,
        radroots_studio_app_ui_icon_data,
        RadrootsAppUiIconKey,
    };

    #[test]
    fn icon_key_parses_names() {
        assert_eq!(
            radroots_studio_app_ui_icon_key_from_name("caret-right"),
            Some(RadrootsAppUiIconKey::CaretRight)
        );
        assert_eq!(
            radroots_studio_app_ui_icon_key_from_name("chevron-right"),
            Some(RadrootsAppUiIconKey::CaretRight)
        );
        assert_eq!(
            radroots_studio_app_ui_icon_key_from_name("caret-up-down"),
            Some(RadrootsAppUiIconKey::CaretUpDown)
        );
        assert_eq!(
            radroots_studio_app_ui_icon_key_from_name("chevrons-up-down"),
            Some(RadrootsAppUiIconKey::CaretUpDown)
        );
        assert_eq!(
            radroots_studio_app_ui_icon_key_from_name("plus"),
            Some(RadrootsAppUiIconKey::Plus)
        );
        assert_eq!(radroots_studio_app_ui_icon_key_from_name("unknown"), None);
    }

    #[test]
    fn icon_data_resolves() {
        let icon = radroots_studio_app_ui_icon_data(RadrootsAppUiIconKey::Plus);
        assert!(!icon.data.is_empty());
    }
}
