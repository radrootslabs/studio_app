#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeLayer {
    Layer0,
    Layer1,
    Layer2,
}

impl ThemeLayer {
    pub const fn as_u8(self) -> u8 {
        match self {
            ThemeLayer::Layer0 => 0,
            ThemeLayer::Layer1 => 1,
            ThemeLayer::Layer2 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeError {
    WindowUnavailable,
    DocumentUnavailable,
    ElementUnavailable,
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeError::WindowUnavailable => f.write_str("error.app.theme.window_unavailable"),
            ThemeError::DocumentUnavailable => f.write_str("error.app.theme.document_unavailable"),
            ThemeError::ElementUnavailable => f.write_str("error.app.theme.element_unavailable"),
        }
    }
}

impl std::error::Error for ThemeError {}

pub fn parse_layer(layer: Option<i32>, fallback: Option<ThemeLayer>) -> ThemeLayer {
    match layer {
        Some(0) => ThemeLayer::Layer0,
        Some(1) => ThemeLayer::Layer1,
        Some(2) => ThemeLayer::Layer2,
        _ => fallback.unwrap_or(ThemeLayer::Layer0),
    }
}

pub fn get_system_theme(fallback: ThemeMode) -> ThemeMode {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(query)) = window.match_media("(prefers-color-scheme: dark)") {
                if query.matches() {
                    return ThemeMode::Dark;
                }
                return ThemeMode::Light;
            }
        }
        fallback
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        fallback
    }
}

pub fn theme_set(theme_key: &str, color_mode: ThemeMode) -> Result<(), ThemeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().ok_or(ThemeError::WindowUnavailable)?;
        let document = window.document().ok_or(ThemeError::DocumentUnavailable)?;
        let element = document.document_element().ok_or(ThemeError::ElementUnavailable)?;
        let value = format!("{theme_key}_{}", color_mode.as_str());
        element
            .set_attribute("data-theme", &value)
            .map_err(|_| ThemeError::ElementUnavailable)?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = theme_key;
        let _ = color_mode;
        Err(ThemeError::WindowUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::{get_system_theme, parse_layer, theme_set, ThemeError, ThemeLayer, ThemeMode};

    #[test]
    fn parse_layer_handles_fallback() {
        assert_eq!(parse_layer(Some(2), None).as_u8(), 2);
        assert_eq!(
            parse_layer(Some(4), Some(ThemeLayer::Layer1)),
            ThemeLayer::Layer1
        );
    }

    #[test]
    fn get_system_theme_uses_fallback() {
        assert_eq!(get_system_theme(ThemeMode::Dark), ThemeMode::Dark);
    }

    #[test]
    fn theme_set_errors_on_non_wasm() {
        let err = theme_set("radroots", ThemeMode::Light).expect_err("non-wasm error");
        assert_eq!(err, ThemeError::WindowUnavailable);
    }
}
