#![forbid(unsafe_code)]

use leptos::ev::MouseEvent;
use leptos::prelude::Callback;

use crate::{radroots_studio_app_ui_icon_key_from_name, RadrootsAppUiIconKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadrootsAppUiListStylesResolved {
    pub hide_border_top: bool,
    pub hide_border_bottom: bool,
    pub hide_rounded: bool,
    pub set_title_background: bool,
    pub set_default_background: bool,
}

impl Default for RadrootsAppUiListStylesResolved {
    fn default() -> Self {
        Self {
            hide_border_top: true,
            hide_border_bottom: true,
            hide_rounded: false,
            set_title_background: false,
            set_default_background: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListStyles {
    pub hide_border_top: Option<bool>,
    pub hide_border_bottom: Option<bool>,
    pub hide_rounded: Option<bool>,
    pub set_title_background: Option<bool>,
    pub set_default_background: Option<bool>,
}

pub fn radroots_studio_app_ui_list_styles_resolve(
    styles: Option<&RadrootsAppUiListStyles>,
) -> RadrootsAppUiListStylesResolved {
    let defaults = RadrootsAppUiListStylesResolved::default();
    match styles {
        Some(styles) => RadrootsAppUiListStylesResolved {
            hide_border_top: styles.hide_border_top.unwrap_or(defaults.hide_border_top),
            hide_border_bottom: styles
                .hide_border_bottom
                .unwrap_or(defaults.hide_border_bottom),
            hide_rounded: styles.hide_rounded.unwrap_or(defaults.hide_rounded),
            set_title_background: styles
                .set_title_background
                .unwrap_or(defaults.set_title_background),
            set_default_background: styles
                .set_default_background
                .unwrap_or(defaults.set_default_background),
        },
        None => defaults,
    }
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListIcon {
    pub key: String,
    pub class: Option<String>,
}

pub fn radroots_studio_app_ui_list_icon_key(
    icon: &RadrootsAppUiListIcon,
) -> Option<RadrootsAppUiIconKey> {
    radroots_studio_app_ui_icon_key_from_name(icon.key.as_str())
}

#[derive(Debug, Clone)]
pub enum RadrootsAppUiListTitleValue {
    Text(String),
    Spacer,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListTitleLink {
    pub label: Option<RadrootsAppUiListLabelValue>,
    pub icon: Option<RadrootsAppUiListIcon>,
    pub classes: Option<String>,
    pub on_click: Option<Callback<()>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListTitle {
    pub value: RadrootsAppUiListTitleValue,
    pub classes: Option<String>,
    pub mod_value: Option<RadrootsAppUiListOffsetMod>,
    pub link: Option<RadrootsAppUiListTitleLink>,
    pub on_click: Option<Callback<()>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListDefaultLabel {
    pub label: String,
    pub classes: Option<String>,
    pub on_click: Option<Callback<()>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListDefault {
    pub labels: Option<Vec<RadrootsAppUiListDefaultLabel>>,
    pub show_title: bool,
    pub classes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListLabelText {
    pub value: String,
    pub classes: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RadrootsAppUiListLabelValueKind {
    Text(RadrootsAppUiListLabelText),
    Icon(RadrootsAppUiListIcon),
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListLabelValue {
    pub classes_wrap: Option<String>,
    pub hide_truncate: bool,
    pub value: RadrootsAppUiListLabelValueKind,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListLabel {
    pub left: Vec<RadrootsAppUiListLabelValue>,
    pub right: Vec<RadrootsAppUiListLabelValue>,
}

#[derive(Debug, Clone)]
pub enum RadrootsAppUiListDisplayValue {
    Icon(RadrootsAppUiListIcon),
    Label(RadrootsAppUiListLabelText),
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListDisplay {
    pub value: RadrootsAppUiListDisplayValue,
    pub loading: bool,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListTouchEnd {
    pub icon: RadrootsAppUiListIcon,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListTouch {
    pub label: RadrootsAppUiListLabel,
    pub display: Option<RadrootsAppUiListDisplay>,
    pub end: Option<RadrootsAppUiListTouchEnd>,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListInputAction {
    pub visible: bool,
    pub loading: bool,
    pub icon: Option<RadrootsAppUiListIcon>,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListInputLineLabel {
    pub value: String,
    pub classes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListInputField {
    pub value: String,
    pub placeholder: Option<String>,
    pub disabled: bool,
    pub classes: Option<String>,
    pub id: Option<String>,
    pub on_input: Option<Callback<String>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListInput {
    pub field: RadrootsAppUiListInputField,
    pub line_label: Option<RadrootsAppUiListInputLineLabel>,
    pub action: Option<RadrootsAppUiListInputAction>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListSelectOption {
    pub label: String,
    pub value: String,
    pub classes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListSelectField {
    pub value: String,
    pub options: Vec<RadrootsAppUiListSelectOption>,
    pub disabled: bool,
    pub classes: Option<String>,
    pub id: Option<String>,
    pub on_change: Option<Callback<String>>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListSelect {
    pub field: RadrootsAppUiListSelectField,
    pub label: RadrootsAppUiListLabel,
    pub display: Option<RadrootsAppUiListDisplay>,
    pub end: Option<RadrootsAppUiListTouchEnd>,
    pub loading: bool,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub enum RadrootsAppUiListOffsetMod {
    Small,
    Glyph,
    Icon {
        icon: RadrootsAppUiListIcon,
        loading: bool,
    },
    IconCircle {
        icon: RadrootsAppUiListIcon,
        loading: bool,
    },
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListOffset {
    pub mod_value: Option<RadrootsAppUiListOffsetMod>,
    pub classes: Option<String>,
    pub hide_space: bool,
    pub hide_offset: bool,
    pub on_click: Option<Callback<MouseEvent>>,
}

#[derive(Debug, Clone)]
pub enum RadrootsAppUiListItemKind {
    Touch(RadrootsAppUiListTouch),
    Input(RadrootsAppUiListInput),
    Select(RadrootsAppUiListSelect),
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiListItem {
    pub kind: RadrootsAppUiListItemKind,
    pub loading: bool,
    pub hide_active: bool,
    pub hide_field: bool,
    pub full_rounded: bool,
    pub offset: Option<RadrootsAppUiListOffset>,
}

#[derive(Debug, Clone)]
pub struct RadrootsAppUiList {
    pub id: Option<String>,
    pub view: Option<String>,
    pub classes: Option<String>,
    pub title: Option<RadrootsAppUiListTitle>,
    pub default_state: Option<RadrootsAppUiListDefault>,
    pub list: Option<Vec<Option<RadrootsAppUiListItem>>>,
    pub hide_offset: bool,
    pub styles: Option<RadrootsAppUiListStyles>,
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_list_styles_resolve,
        RadrootsAppUiListStyles,
        RadrootsAppUiListStylesResolved,
    };

    #[test]
    fn list_style_defaults_match_spec() {
        let resolved = radroots_studio_app_ui_list_styles_resolve(None);
        assert_eq!(resolved, RadrootsAppUiListStylesResolved::default());
    }

    #[test]
    fn list_style_overrides_apply() {
        let styles = RadrootsAppUiListStyles {
            hide_border_top: Some(false),
            hide_border_bottom: Some(true),
            hide_rounded: Some(true),
            set_title_background: Some(true),
            set_default_background: Some(true),
        };
        let resolved = radroots_studio_app_ui_list_styles_resolve(Some(&styles));
        assert_eq!(
            resolved,
            RadrootsAppUiListStylesResolved {
                hide_border_top: false,
                hide_border_bottom: true,
                hide_rounded: true,
                set_title_background: true,
                set_default_background: true,
            }
        );
    }
}
