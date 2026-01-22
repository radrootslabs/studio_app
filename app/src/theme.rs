#![forbid(unsafe_code)]

pub const APP_THEME_STORAGE_KEY: &str = "app:theme:mode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppThemeMode {
    System,
    Light,
    Dark,
}

impl RadrootsAppThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            RadrootsAppThemeMode::System => "system",
            RadrootsAppThemeMode::Light => "light",
            RadrootsAppThemeMode::Dark => "dark",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppThemeError {
    Unavailable,
    Storage,
}

pub type RadrootsAppThemeResult<T> = Result<T, RadrootsAppThemeError>;

impl RadrootsAppThemeError {
    pub const fn message(&self) -> &'static str {
        match self {
            RadrootsAppThemeError::Unavailable => "error.app.theme.unavailable",
            RadrootsAppThemeError::Storage => "error.app.theme.storage",
        }
    }
}

impl std::fmt::Display for RadrootsAppThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for RadrootsAppThemeError {}

pub fn app_theme_mode_from_value(value: &str) -> Option<RadrootsAppThemeMode> {
    match value {
        "system" => Some(RadrootsAppThemeMode::System),
        "light" => Some(RadrootsAppThemeMode::Light),
        "dark" => Some(RadrootsAppThemeMode::Dark),
        _ => None,
    }
}

pub fn app_theme_mode_to_name(mode: RadrootsAppThemeMode, prefers_dark: bool) -> &'static str {
    match mode {
        RadrootsAppThemeMode::System => {
            if prefers_dark { "os_dark" } else { "os_light" }
        }
        RadrootsAppThemeMode::Light => "os_light",
        RadrootsAppThemeMode::Dark => "os_dark",
    }
}

#[cfg(target_arch = "wasm32")]
fn app_theme_prefers_dark() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    match window.match_media("(prefers-color-scheme: dark)") {
        Ok(Some(query)) => query.matches(),
        _ => false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn app_theme_prefers_dark() -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
fn app_theme_apply_name(name: &str) -> RadrootsAppThemeResult<()> {
    use leptos::wasm_bindgen::JsCast;
    let Some(window) = web_sys::window() else {
        return Err(RadrootsAppThemeError::Unavailable);
    };
    let Some(document) = window.document() else {
        return Err(RadrootsAppThemeError::Unavailable);
    };
    let Some(root) = document.document_element() else {
        return Err(RadrootsAppThemeError::Unavailable);
    };
    root.set_attribute("data-theme", name)
        .map_err(|_| RadrootsAppThemeError::Unavailable)?;
    let color_scheme = if name == "os_dark" { "dark" } else { "light" };
    let html = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| RadrootsAppThemeError::Unavailable)?;
    html.style()
        .set_property("color-scheme", color_scheme)
        .map_err(|_| RadrootsAppThemeError::Unavailable)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn app_theme_apply_name(_name: &str) -> RadrootsAppThemeResult<()> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn app_theme_read_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage.get_item(APP_THEME_STORAGE_KEY).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
fn app_theme_read_storage() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn app_theme_write_storage(value: &str) -> RadrootsAppThemeResult<()> {
    let window = web_sys::window().ok_or(RadrootsAppThemeError::Unavailable)?;
    let storage = window
        .local_storage()
        .map_err(|_| RadrootsAppThemeError::Storage)?
        .ok_or(RadrootsAppThemeError::Storage)?;
    storage
        .set_item(APP_THEME_STORAGE_KEY, value)
        .map_err(|_| RadrootsAppThemeError::Storage)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn app_theme_write_storage(_value: &str) -> RadrootsAppThemeResult<()> {
    Ok(())
}

pub fn app_theme_read_mode() -> Option<RadrootsAppThemeMode> {
    app_theme_read_storage()
        .as_deref()
        .and_then(app_theme_mode_from_value)
}

pub fn app_theme_init() -> RadrootsAppThemeResult<&'static str> {
    let prefers_dark = app_theme_prefers_dark();
    let mode = app_theme_read_mode().unwrap_or(RadrootsAppThemeMode::System);
    let theme_name = app_theme_mode_to_name(mode, prefers_dark);
    app_theme_apply_name(theme_name)?;
    Ok(theme_name)
}

pub fn app_theme_apply_mode(mode: RadrootsAppThemeMode) -> RadrootsAppThemeResult<&'static str> {
    let prefers_dark = app_theme_prefers_dark();
    let name = app_theme_mode_to_name(mode, prefers_dark);
    app_theme_apply_name(name)?;
    Ok(name)
}

pub fn app_theme_store_mode(mode: RadrootsAppThemeMode) -> RadrootsAppThemeResult<()> {
    app_theme_write_storage(mode.as_str())
}

#[cfg(test)]
mod tests {
    use super::{
        app_theme_mode_from_value,
        app_theme_mode_to_name,
        RadrootsAppThemeMode,
    };

    #[test]
    fn theme_mode_from_value_parses_known_values() {
        assert_eq!(app_theme_mode_from_value("system"), Some(RadrootsAppThemeMode::System));
        assert_eq!(app_theme_mode_from_value("light"), Some(RadrootsAppThemeMode::Light));
        assert_eq!(app_theme_mode_from_value("dark"), Some(RadrootsAppThemeMode::Dark));
        assert_eq!(app_theme_mode_from_value("other"), None);
    }

    #[test]
    fn theme_mode_to_name_respects_preference() {
        assert_eq!(app_theme_mode_to_name(RadrootsAppThemeMode::System, true), "os_dark");
        assert_eq!(app_theme_mode_to_name(RadrootsAppThemeMode::System, false), "os_light");
        assert_eq!(app_theme_mode_to_name(RadrootsAppThemeMode::Light, true), "os_light");
        assert_eq!(app_theme_mode_to_name(RadrootsAppThemeMode::Dark, false), "os_dark");
    }
}
