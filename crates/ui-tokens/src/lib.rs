#![forbid(unsafe_code)]

pub const RADROOTS_APP_UI_TOKENS_CSS: &str = include_str!("../assets/tokens.css");

pub struct RadrootsAppUiTokens;

impl RadrootsAppUiTokens {
    pub const fn css() -> &'static str {
        RADROOTS_APP_UI_TOKENS_CSS
    }
}

#[cfg(test)]
mod tests {
    use super::RADROOTS_APP_UI_TOKENS_CSS;

    #[test]
    fn tokens_css_is_not_empty() {
        assert!(!RADROOTS_APP_UI_TOKENS_CSS.trim().is_empty());
    }
}
