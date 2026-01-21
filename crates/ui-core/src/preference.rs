use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiPreferenceError {
    WindowMissing,
    MatchMediaFailed,
}

impl fmt::Display for RadrootsAppUiPreferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RadrootsAppUiPreferenceError::WindowMissing => {
                write!(f, "preference_window_missing")
            }
            RadrootsAppUiPreferenceError::MatchMediaFailed => {
                write!(f, "preference_match_media_failed")
            }
        }
    }
}

pub type RadrootsAppUiPreferenceResult<T> = Result<T, RadrootsAppUiPreferenceError>;

#[cfg(target_arch = "wasm32")]
fn radroots_studio_app_ui_prefers(query: &str) -> RadrootsAppUiPreferenceResult<bool> {
    let window = web_sys::window().ok_or(RadrootsAppUiPreferenceError::WindowMissing)?;
    let media = window
        .match_media(query)
        .map_err(|_| RadrootsAppUiPreferenceError::MatchMediaFailed)?;
    Ok(media.map(|list| list.matches()).unwrap_or(false))
}

#[cfg(not(target_arch = "wasm32"))]
fn radroots_studio_app_ui_prefers(_query: &str) -> RadrootsAppUiPreferenceResult<bool> {
    Ok(false)
}

pub fn radroots_studio_app_ui_prefers_reduced_motion() -> RadrootsAppUiPreferenceResult<bool> {
    radroots_studio_app_ui_prefers("(prefers-reduced-motion: reduce)")
}

pub fn radroots_studio_app_ui_prefers_contrast_more() -> RadrootsAppUiPreferenceResult<bool> {
    radroots_studio_app_ui_prefers("(prefers-contrast: more)")
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_prefers_contrast_more,
        radroots_studio_app_ui_prefers_reduced_motion,
    };

    #[test]
    fn prefers_helpers_return_result() {
        let _ = radroots_studio_app_ui_prefers_reduced_motion().expect("reduced motion");
        let _ = radroots_studio_app_ui_prefers_contrast_more().expect("contrast more");
    }
}
