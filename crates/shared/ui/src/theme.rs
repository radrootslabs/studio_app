#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppUiTheme {
    pub windows: AppWindowTokens,
    pub surfaces: AppSurfaceTokens,
    pub text: AppTextTokens,
    pub typography: AppTypographyTokens,
    pub layout: AppLayoutTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppWindowTokens {
    pub home_min_width_px: f32,
    pub home_min_height_px: f32,
    pub settings_width_px: f32,
    pub settings_height_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSurfaceTokens {
    pub window_background: u32,
    pub chrome_background: u32,
    pub panel_background: u32,
    pub card_background: u32,
    pub divider: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppTextTokens {
    pub primary: u32,
    pub secondary: u32,
    pub accent: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppTypographyTokens {
    pub utility_title_text_px: f32,
    pub body_text_px: f32,
    pub brand_text_px: f32,
    pub settings_row_text_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppLayoutTokens {
    pub divider_thickness_px: f32,
    pub home_window_padding_px: f32,
    pub home_sidebar_width_px: f32,
    pub home_card_max_width_px: f32,
    pub home_card_padding_px: f32,
    pub home_stack_gap_px: f32,
    pub metadata_row_gap_px: f32,
    pub utility_title_row_height_px: f32,
    pub settings_chrome_height_px: f32,
    pub settings_navigation_width_px: f32,
    pub settings_section_gap_px: f32,
    pub settings_navigation_row_padding_px: f32,
    pub settings_navigation_row_gap_px: f32,
    pub settings_content_padding_px: f32,
}

pub const APP_UI_THEME: AppUiTheme = AppUiTheme {
    windows: AppWindowTokens {
        home_min_width_px: 640.0,
        home_min_height_px: 480.0,
        settings_width_px: 600.0,
        settings_height_px: 540.0,
    },
    surfaces: AppSurfaceTokens {
        window_background: 0xF5F1E8,
        chrome_background: 0xEAE5D8,
        panel_background: 0xF8F4EC,
        card_background: 0xEFE8D8,
        divider: 0xD4CCBA,
    },
    text: AppTextTokens {
        primary: 0x1F2C23,
        secondary: 0x5D665B,
        accent: 0x3B6A3E,
    },
    typography: AppTypographyTokens {
        utility_title_text_px: 12.0,
        body_text_px: 14.0,
        brand_text_px: 20.0,
        settings_row_text_px: 13.0,
    },
    layout: AppLayoutTokens {
        divider_thickness_px: 1.0,
        home_window_padding_px: 24.0,
        home_sidebar_width_px: 240.0,
        home_card_max_width_px: 960.0,
        home_card_padding_px: 24.0,
        home_stack_gap_px: 12.0,
        metadata_row_gap_px: 12.0,
        utility_title_row_height_px: 24.0,
        settings_chrome_height_px: 88.0,
        settings_navigation_width_px: 216.0,
        settings_section_gap_px: 8.0,
        settings_navigation_row_padding_px: 8.0,
        settings_navigation_row_gap_px: 8.0,
        settings_content_padding_px: 24.0,
    },
};
