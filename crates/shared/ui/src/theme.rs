#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppUiTheme {
    pub windows: AppWindowTokens,
    pub surfaces: AppSurfaceTokens,
    pub text: AppTextTokens,
    pub typography: AppTypographyTokens,
    pub layout: AppLayoutTokens,
    pub controls: AppControlTokens,
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
    pub startup_title_text_px: f32,
    pub startup_tagline_text_px: f32,
    pub settings_row_text_px: f32,
    pub settings_account_identity_text_px: f32,
    pub settings_account_detail_text_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppLayoutTokens {
    pub divider_thickness_px: f32,
    pub home_window_padding_px: f32,
    pub home_sidebar_width_px: f32,
    pub home_card_max_width_px: f32,
    pub home_card_padding_px: f32,
    pub home_stack_gap_px: f32,
    pub startup_stack_gap_px: f32,
    pub metadata_row_gap_px: f32,
    pub utility_title_row_height_px: f32,
    pub settings_chrome_height_px: f32,
    pub settings_navigation_width_px: f32,
    pub settings_section_gap_px: f32,
    pub settings_navigation_row_padding_px: f32,
    pub settings_navigation_row_gap_px: f32,
    pub settings_content_padding_px: f32,
    pub settings_account_sidebar_width_px: f32,
    pub settings_account_sidebar_padding_px: f32,
    pub settings_account_sidebar_button_height_px: f32,
    pub settings_account_sidebar_button_padding_px: f32,
    pub settings_account_sidebar_button_corner_radius_px: f32,
    pub settings_account_sidebar_button_gap_px: f32,
    pub settings_account_sidebar_avatar_size_px: f32,
    pub settings_account_identity_text_gap_px: f32,
    pub settings_account_sidebar_footer_padding_top_px: f32,
    pub settings_account_sidebar_footer_row_gap_px: f32,
    pub settings_account_sidebar_footer_button_gap_px: f32,
    pub settings_account_main_padding_px: f32,
    pub settings_account_content_max_width_px: f32,
    pub settings_account_main_stack_gap_px: f32,
    pub settings_account_profile_avatar_size_px: f32,
    pub settings_account_detail_row_gap_px: f32,
    pub settings_account_detail_value_gap_px: f32,
    pub settings_checkbox_label_gap_px: f32,
    pub settings_account_status_gap_px: f32,
    pub settings_account_action_row_gap_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppControlTokens {
    pub icon_segment_button: IconSegmentButtonTokens,
    pub action_button: ActionButtonTokens,
    pub text_input: TextInputTokens,
    pub checkbox: CheckboxTokens,
    pub status_indicator: StatusIndicatorTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSegmentButtonTokens {
    pub sizing: IconSegmentButtonSizing,
    pub colors: IconSegmentButtonColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSegmentButtonSizing {
    pub height_px: f32,
    pub corner_radius_px: f32,
    pub inner_padding_px: f32,
    pub icon_size_px: f32,
    pub label_size_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSegmentButtonColors {
    pub active_background: u32,
    pub inactive_background: u32,
    pub active_foreground: u32,
    pub inactive_foreground: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionButtonTokens {
    pub sizing: ActionButtonSizing,
    pub colors: ActionButtonColors,
    pub primary_colors: ActionButtonColors,
    pub disabled_colors: ActionButtonColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionButtonSizing {
    pub height_px: f32,
    pub corner_radius_px: f32,
    pub horizontal_padding_px: f32,
    pub compact_horizontal_padding_px: f32,
    pub label_size_px: f32,
    pub icon_size_px: f32,
    pub square_width_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionButtonColors {
    pub background: u32,
    pub foreground: u32,
    pub hover_changes_background: bool,
    pub hover_background: u32,
    pub active_background: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckboxTokens {
    pub size_px: f32,
    pub corner_radius_px: f32,
    pub icon_size_px: f32,
    pub checked_background: u32,
    pub unchecked_background: u32,
    pub unchecked_border: u32,
    pub check_foreground: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputTokens {
    pub background: u32,
    pub disabled_background: u32,
    pub border: u32,
    pub corner_radius_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusIndicatorTokens {
    pub size_px: f32,
    pub online: u32,
    pub offline: u32,
    pub attention: u32,
}

pub const APP_UI_THEME: AppUiTheme = AppUiTheme {
    windows: AppWindowTokens {
        home_min_width_px: 1284.0,
        home_min_height_px: 795.0,
        settings_width_px: 600.0,
        settings_height_px: 540.0,
    },
    surfaces: AppSurfaceTokens {
        window_background: 0xFFFFFF,
        chrome_background: 0xF5F5F7,
        panel_background: 0xFFFFFF,
        card_background: 0xF2F2F7,
        divider: 0xD2D2D7,
    },
    text: AppTextTokens {
        primary: 0x1D1D1F,
        secondary: 0x6E6E73,
        accent: 0x0A84FF,
    },
    typography: AppTypographyTokens {
        utility_title_text_px: 12.0,
        body_text_px: 14.0,
        brand_text_px: 14.0,
        startup_title_text_px: 18.0,
        startup_tagline_text_px: 16.0,
        settings_row_text_px: 13.0,
        settings_account_identity_text_px: 14.0,
        settings_account_detail_text_px: 14.0,
    },
    layout: AppLayoutTokens {
        divider_thickness_px: 1.0,
        home_window_padding_px: 24.0,
        home_sidebar_width_px: 240.0,
        home_card_max_width_px: 1080.0,
        home_card_padding_px: 24.0,
        home_stack_gap_px: 12.0,
        startup_stack_gap_px: 6.0,
        metadata_row_gap_px: 12.0,
        utility_title_row_height_px: 24.0,
        settings_chrome_height_px: 88.0,
        settings_navigation_width_px: 216.0,
        settings_section_gap_px: 8.0,
        settings_navigation_row_padding_px: 8.0,
        settings_navigation_row_gap_px: 8.0,
        settings_content_padding_px: 24.0,
        settings_account_sidebar_width_px: 200.0,
        settings_account_sidebar_padding_px: 8.0,
        settings_account_sidebar_button_height_px: 42.0,
        settings_account_sidebar_button_padding_px: 4.0,
        settings_account_sidebar_button_corner_radius_px: 8.0,
        settings_account_sidebar_button_gap_px: 8.0,
        settings_account_sidebar_avatar_size_px: 32.0,
        settings_account_identity_text_gap_px: 0.0,
        settings_account_sidebar_footer_padding_top_px: 8.0,
        settings_account_sidebar_footer_row_gap_px: 8.0,
        settings_account_sidebar_footer_button_gap_px: 8.0,
        settings_account_main_padding_px: 24.0,
        settings_account_content_max_width_px: 260.0,
        settings_account_main_stack_gap_px: 16.0,
        settings_account_profile_avatar_size_px: 64.0,
        settings_account_detail_row_gap_px: 16.0,
        settings_account_detail_value_gap_px: 12.0,
        settings_checkbox_label_gap_px: 8.0,
        settings_account_status_gap_px: 4.0,
        settings_account_action_row_gap_px: 8.0,
    },
    controls: AppControlTokens {
        icon_segment_button: IconSegmentButtonTokens {
            sizing: IconSegmentButtonSizing {
                height_px: 44.0,
                corner_radius_px: 8.0,
                inner_padding_px: 2.0,
                icon_size_px: 16.0,
                label_size_px: 12.0,
            },
            colors: IconSegmentButtonColors {
                active_background: 0xFFFFFF,
                inactive_background: 0xF5F5F7,
                active_foreground: 0x0A84FF,
                inactive_foreground: 0x1D1D1F,
            },
        },
        action_button: ActionButtonTokens {
            sizing: ActionButtonSizing {
                height_px: 24.0,
                corner_radius_px: 8.0,
                horizontal_padding_px: 12.0,
                compact_horizontal_padding_px: 4.0,
                label_size_px: 13.0,
                icon_size_px: 14.0,
                square_width_px: 24.0,
            },
            colors: ActionButtonColors {
                background: 0xE5E5EA,
                foreground: 0x1D1D1F,
                hover_changes_background: false,
                hover_background: 0xDCDCE1,
                active_background: 0xD1D1D6,
            },
            primary_colors: ActionButtonColors {
                background: 0x0A84FF,
                foreground: 0xFFFFFF,
                hover_changes_background: true,
                hover_background: 0x007AFF,
                active_background: 0x0062CC,
            },
            disabled_colors: ActionButtonColors {
                background: 0xA7C8F8,
                foreground: 0xFFFFFF,
                hover_changes_background: false,
                hover_background: 0xA7C8F8,
                active_background: 0xA7C8F8,
            },
        },
        text_input: TextInputTokens {
            background: 0xFFFFFF,
            disabled_background: 0xF2F2F7,
            border: 0xD1D1D6,
            corner_radius_px: 10.0,
        },
        checkbox: CheckboxTokens {
            size_px: 16.0,
            corner_radius_px: 5.0,
            icon_size_px: 13.0,
            checked_background: 0x0A84FF,
            unchecked_background: 0xF2F2F7,
            unchecked_border: 0xD1D1D6,
            check_foreground: 0xFFFFFF,
        },
        status_indicator: StatusIndicatorTokens {
            size_px: 12.0,
            online: 0x34C759,
            offline: 0xFFD60A,
            attention: 0xFF3B30,
        },
    },
};

#[cfg(test)]
mod tests {
    use super::APP_UI_THEME;

    #[test]
    fn paperwhite_shell_layers_are_distinct() {
        assert_eq!(APP_UI_THEME.surfaces.window_background, 0xFFFFFF);
        assert_eq!(APP_UI_THEME.surfaces.chrome_background, 0xF5F5F7);
        assert_eq!(APP_UI_THEME.surfaces.card_background, 0xF2F2F7);
        assert_eq!(APP_UI_THEME.surfaces.divider, 0xD2D2D7);
        assert_ne!(
            APP_UI_THEME.surfaces.window_background,
            APP_UI_THEME.surfaces.card_background
        );
    }

    #[test]
    fn home_window_minimums_match_the_upgraded_shell_budget() {
        assert_eq!(APP_UI_THEME.windows.home_min_width_px, 1284.0);
        assert_eq!(APP_UI_THEME.windows.home_min_height_px, 795.0);
        assert_eq!(APP_UI_THEME.layout.home_sidebar_width_px, 240.0);
        assert_eq!(APP_UI_THEME.layout.home_window_padding_px, 24.0);
    }

    #[test]
    fn settings_shell_layout_contract_is_explicit() {
        assert_eq!(APP_UI_THEME.windows.settings_width_px, 600.0);
        assert_eq!(APP_UI_THEME.windows.settings_height_px, 540.0);
        assert_eq!(APP_UI_THEME.layout.settings_chrome_height_px, 88.0);
        assert_eq!(APP_UI_THEME.layout.settings_content_padding_px, 24.0);
        assert_eq!(APP_UI_THEME.layout.settings_account_sidebar_width_px, 200.0);
        assert_eq!(
            APP_UI_THEME
                .layout
                .settings_account_sidebar_button_height_px,
            42.0
        );
    }

    #[test]
    fn control_tokens_match_the_frozen_budget() {
        let segmented = APP_UI_THEME.controls.icon_segment_button.sizing;
        let action = APP_UI_THEME.controls.action_button.sizing;
        let text_input = APP_UI_THEME.controls.text_input;
        let checkbox = APP_UI_THEME.controls.checkbox;
        let status = APP_UI_THEME.controls.status_indicator;

        assert_eq!(segmented.height_px, 44.0);
        assert_eq!(segmented.corner_radius_px, 8.0);
        assert_eq!(segmented.inner_padding_px, 2.0);
        assert_eq!(action.height_px, 24.0);
        assert_eq!(action.corner_radius_px, 8.0);
        assert_eq!(action.square_width_px, 24.0);
        assert_eq!(text_input.corner_radius_px, 10.0);
        assert_eq!(text_input.background, 0xFFFFFF);
        assert_eq!(text_input.disabled_background, 0xF2F2F7);
        assert_eq!(checkbox.size_px, 16.0);
        assert_eq!(checkbox.corner_radius_px, 5.0);
        assert_eq!(status.size_px, 12.0);
    }

    #[test]
    fn accent_and_status_colors_match_the_current_shell_contract() {
        assert_eq!(APP_UI_THEME.text.accent, 0x0A84FF);
        assert_eq!(
            APP_UI_THEME
                .controls
                .action_button
                .primary_colors
                .background,
            0x0A84FF
        );
        assert_eq!(
            APP_UI_THEME
                .controls
                .action_button
                .primary_colors
                .foreground,
            0xFFFFFF
        );
        assert_eq!(APP_UI_THEME.controls.checkbox.checked_background, 0x0A84FF);
        assert_eq!(APP_UI_THEME.controls.status_indicator.online, 0x34C759);
        assert_eq!(APP_UI_THEME.controls.status_indicator.offline, 0xFFD60A);
        assert_eq!(APP_UI_THEME.controls.status_indicator.attention, 0xFF3B30);
    }
}
