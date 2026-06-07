#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppUiTheme {
    pub foundation: AppFoundationTokens,
    pub components: AppComponentTokens,
    pub shells: AppShellTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppFoundationTokens {
    pub surfaces: AppSurfaceTokens,
    pub text: AppTextTokens,
    pub typography: AppTypographyTokens,
    pub spacing: AppSpacingTokens,
    pub radii: AppRadiusTokens,
    pub borders: AppBorderTokens,
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
pub struct AppSpacingTokens {
    pub micro_px: f32,
    pub tight_px: f32,
    pub small_px: f32,
    pub medium_px: f32,
    pub large_px: f32,
    pub xlarge_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppRadiusTokens {
    pub small_px: f32,
    pub medium_px: f32,
    pub large_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppBorderTokens {
    pub divider_thickness_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppComponentTokens {
    pub app_segment_button_icon: AppSegmentButtonIconTokens,
    pub app_button: AppButtonTokens,
    pub app_input_text: AppInputTextTokens,
    pub app_checkbox_field: AppCheckboxFieldTokens,
    pub app_status_indicator: AppStatusIndicatorTokens,
    pub app_account_selector_row: AppAccountSelectorRowTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppAccountSelectorRowTokens {
    pub inactive_background: u32,
    pub active_background: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSegmentButtonIconTokens {
    pub sizing: AppSegmentButtonIconSizing,
    pub colors: AppSegmentButtonIconColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSegmentButtonIconSizing {
    pub height_px: f32,
    pub corner_radius_px: f32,
    pub inner_padding_px: f32,
    pub icon_size_px: f32,
    pub label_size_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppSegmentButtonIconColors {
    pub active_background: u32,
    pub inactive_background: u32,
    pub active_foreground: u32,
    pub inactive_foreground: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppButtonTokens {
    pub sizing: AppButtonSizing,
    pub secondary_colors: AppButtonColors,
    pub primary_colors: AppButtonColors,
    pub primary_disabled_colors: AppButtonColors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppButtonSizing {
    pub height_px: f32,
    pub corner_radius_px: f32,
    pub horizontal_padding_px: f32,
    pub compact_horizontal_padding_px: f32,
    pub label_size_px: f32,
    pub icon_size_px: f32,
    pub square_width_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppButtonColors {
    pub background: u32,
    pub foreground: u32,
    pub hover_changes_background: bool,
    pub hover_background: u32,
    pub active_background: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppInputTextTokens {
    pub background: u32,
    pub disabled_background: u32,
    pub border: u32,
    pub corner_radius_px: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppCheckboxFieldTokens {
    pub size_px: f32,
    pub corner_radius_px: f32,
    pub icon_size_px: f32,
    pub checked_background: u32,
    pub unchecked_background: u32,
    pub unchecked_border: u32,
    pub check_foreground: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppStatusIndicatorTokens {
    pub size_px: f32,
    pub online: u32,
    pub offline: u32,
    pub attention: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppShellTokens {
    pub home_min_width_px: f32,
    pub home_min_height_px: f32,
    pub settings_width_px: f32,
    pub settings_height_px: f32,
    pub home_window_padding_px: f32,
    pub home_sidebar_width_px: f32,
    pub home_card_max_width_px: f32,
    pub focused_task_max_width_px: f32,
    pub focused_detail_max_width_px: f32,
    pub settings_panel_content_max_width_px: f32,
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

const APP_SURFACE_WINDOW_BACKGROUND: u32 = 0xFFFFFF;
const APP_SURFACE_CHROME_BACKGROUND: u32 = 0xF5F5F7;
const APP_SURFACE_PANEL_BACKGROUND: u32 = 0xFFFFFF;
const APP_SURFACE_CARD_BACKGROUND: u32 = 0xF2F2F7;
const APP_SURFACE_DIVIDER: u32 = 0xD2D2D7;
const APP_SURFACE_ACCOUNT_SELECTOR_ACTIVE_BACKGROUND: u32 = 0xE5E5EA;
const APP_TEXT_PRIMARY: u32 = 0x1D1D1F;
const APP_TEXT_SECONDARY: u32 = 0x6E6E73;
const APP_TEXT_ACCENT: u32 = 0x0A84FF;
const APP_STATUS_ONLINE: u32 = 0x34C759;
const APP_STATUS_OFFLINE: u32 = 0xFFD60A;
const APP_STATUS_ATTENTION: u32 = 0xFF3B30;
const APP_SPACING_MICRO_PX: f32 = 4.0;
const APP_SPACING_TIGHT_PX: f32 = 6.0;
const APP_SPACING_SMALL_PX: f32 = 8.0;
const APP_SPACING_MEDIUM_PX: f32 = 12.0;
const APP_SPACING_LARGE_PX: f32 = 16.0;
const APP_SPACING_XLARGE_PX: f32 = 24.0;
const APP_RADIUS_SMALL_PX: f32 = 5.0;
const APP_RADIUS_MEDIUM_PX: f32 = 8.0;
const APP_RADIUS_LARGE_PX: f32 = 10.0;

pub const APP_UI_THEME: AppUiTheme = AppUiTheme {
    foundation: AppFoundationTokens {
        surfaces: AppSurfaceTokens {
            window_background: APP_SURFACE_WINDOW_BACKGROUND,
            chrome_background: APP_SURFACE_CHROME_BACKGROUND,
            panel_background: APP_SURFACE_PANEL_BACKGROUND,
            card_background: APP_SURFACE_CARD_BACKGROUND,
            divider: APP_SURFACE_DIVIDER,
        },
        text: AppTextTokens {
            primary: APP_TEXT_PRIMARY,
            secondary: APP_TEXT_SECONDARY,
            accent: APP_TEXT_ACCENT,
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
        spacing: AppSpacingTokens {
            micro_px: APP_SPACING_MICRO_PX,
            tight_px: APP_SPACING_TIGHT_PX,
            small_px: APP_SPACING_SMALL_PX,
            medium_px: APP_SPACING_MEDIUM_PX,
            large_px: APP_SPACING_LARGE_PX,
            xlarge_px: APP_SPACING_XLARGE_PX,
        },
        radii: AppRadiusTokens {
            small_px: APP_RADIUS_SMALL_PX,
            medium_px: APP_RADIUS_MEDIUM_PX,
            large_px: APP_RADIUS_LARGE_PX,
        },
        borders: AppBorderTokens {
            divider_thickness_px: 1.0,
        },
    },
    components: AppComponentTokens {
        app_segment_button_icon: AppSegmentButtonIconTokens {
            sizing: AppSegmentButtonIconSizing {
                height_px: 44.0,
                corner_radius_px: APP_RADIUS_MEDIUM_PX,
                inner_padding_px: 2.0,
                icon_size_px: 16.0,
                label_size_px: 12.0,
            },
            colors: AppSegmentButtonIconColors {
                active_background: APP_SURFACE_WINDOW_BACKGROUND,
                inactive_background: APP_SURFACE_CHROME_BACKGROUND,
                active_foreground: APP_TEXT_ACCENT,
                inactive_foreground: APP_TEXT_PRIMARY,
            },
        },
        app_button: AppButtonTokens {
            sizing: AppButtonSizing {
                height_px: 24.0,
                corner_radius_px: APP_RADIUS_MEDIUM_PX,
                horizontal_padding_px: APP_SPACING_MEDIUM_PX,
                compact_horizontal_padding_px: APP_SPACING_MICRO_PX,
                label_size_px: 13.0,
                icon_size_px: 14.0,
                square_width_px: 24.0,
            },
            secondary_colors: AppButtonColors {
                background: 0xE5E5EA,
                foreground: APP_TEXT_PRIMARY,
                hover_changes_background: false,
                hover_background: 0xDCDCE1,
                active_background: 0xD1D1D6,
            },
            primary_colors: AppButtonColors {
                background: APP_TEXT_ACCENT,
                foreground: APP_SURFACE_WINDOW_BACKGROUND,
                hover_changes_background: true,
                hover_background: 0x007AFF,
                active_background: 0x0062CC,
            },
            primary_disabled_colors: AppButtonColors {
                background: 0xA7C8F8,
                foreground: APP_SURFACE_WINDOW_BACKGROUND,
                hover_changes_background: false,
                hover_background: 0xA7C8F8,
                active_background: 0xA7C8F8,
            },
        },
        app_input_text: AppInputTextTokens {
            background: APP_SURFACE_WINDOW_BACKGROUND,
            disabled_background: APP_SURFACE_CARD_BACKGROUND,
            border: 0xD1D1D6,
            corner_radius_px: APP_RADIUS_LARGE_PX,
        },
        app_checkbox_field: AppCheckboxFieldTokens {
            size_px: 16.0,
            corner_radius_px: APP_RADIUS_SMALL_PX,
            icon_size_px: 13.0,
            checked_background: APP_TEXT_ACCENT,
            unchecked_background: APP_SURFACE_CARD_BACKGROUND,
            unchecked_border: 0xD1D1D6,
            check_foreground: APP_SURFACE_WINDOW_BACKGROUND,
        },
        app_status_indicator: AppStatusIndicatorTokens {
            size_px: 12.0,
            online: APP_STATUS_ONLINE,
            offline: APP_STATUS_OFFLINE,
            attention: APP_STATUS_ATTENTION,
        },
        app_account_selector_row: AppAccountSelectorRowTokens {
            inactive_background: APP_SURFACE_CARD_BACKGROUND,
            active_background: APP_SURFACE_ACCOUNT_SELECTOR_ACTIVE_BACKGROUND,
        },
    },
    shells: AppShellTokens {
        home_min_width_px: 1284.0,
        home_min_height_px: 795.0,
        settings_width_px: 600.0,
        settings_height_px: 540.0,
        home_window_padding_px: APP_SPACING_XLARGE_PX,
        home_sidebar_width_px: 240.0,
        home_card_max_width_px: 1080.0,
        focused_task_max_width_px: 720.0,
        focused_detail_max_width_px: 840.0,
        settings_panel_content_max_width_px: 560.0,
        home_card_padding_px: APP_SPACING_XLARGE_PX,
        home_stack_gap_px: APP_SPACING_MEDIUM_PX,
        startup_stack_gap_px: APP_SPACING_TIGHT_PX,
        metadata_row_gap_px: APP_SPACING_MEDIUM_PX,
        utility_title_row_height_px: 24.0,
        settings_chrome_height_px: 88.0,
        settings_navigation_width_px: 216.0,
        settings_section_gap_px: APP_SPACING_SMALL_PX,
        settings_navigation_row_padding_px: APP_SPACING_SMALL_PX,
        settings_navigation_row_gap_px: APP_SPACING_SMALL_PX,
        settings_content_padding_px: APP_SPACING_XLARGE_PX,
        settings_account_sidebar_width_px: 200.0,
        settings_account_sidebar_padding_px: APP_SPACING_SMALL_PX,
        settings_account_sidebar_button_height_px: 42.0,
        settings_account_sidebar_button_padding_px: APP_SPACING_MICRO_PX,
        settings_account_sidebar_button_corner_radius_px: APP_RADIUS_MEDIUM_PX,
        settings_account_sidebar_button_gap_px: APP_SPACING_SMALL_PX,
        settings_account_sidebar_avatar_size_px: 32.0,
        settings_account_identity_text_gap_px: 0.0,
        settings_account_sidebar_footer_padding_top_px: APP_SPACING_SMALL_PX,
        settings_account_sidebar_footer_row_gap_px: APP_SPACING_SMALL_PX,
        settings_account_sidebar_footer_button_gap_px: APP_SPACING_SMALL_PX,
        settings_account_main_padding_px: APP_SPACING_XLARGE_PX,
        settings_account_content_max_width_px: 260.0,
        settings_account_main_stack_gap_px: APP_SPACING_LARGE_PX,
        settings_account_profile_avatar_size_px: 64.0,
        settings_account_detail_row_gap_px: APP_SPACING_LARGE_PX,
        settings_account_detail_value_gap_px: APP_SPACING_MEDIUM_PX,
        settings_checkbox_label_gap_px: APP_SPACING_SMALL_PX,
        settings_account_status_gap_px: APP_SPACING_MICRO_PX,
        settings_account_action_row_gap_px: APP_SPACING_SMALL_PX,
    },
};

#[cfg(test)]
mod tests {
    use super::APP_UI_THEME;

    #[test]
    fn paperwhite_shell_layers_are_distinct() {
        assert_eq!(APP_UI_THEME.foundation.surfaces.window_background, 0xFFFFFF);
        assert_eq!(APP_UI_THEME.foundation.surfaces.chrome_background, 0xF5F5F7);
        assert_eq!(APP_UI_THEME.foundation.surfaces.card_background, 0xF2F2F7);
        assert_eq!(APP_UI_THEME.foundation.surfaces.divider, 0xD2D2D7);
        assert_ne!(
            APP_UI_THEME.foundation.surfaces.window_background,
            APP_UI_THEME.foundation.surfaces.card_background
        );
    }

    #[test]
    fn foundation_scales_are_explicit() {
        assert_eq!(APP_UI_THEME.foundation.spacing.micro_px, 4.0);
        assert_eq!(APP_UI_THEME.foundation.spacing.tight_px, 6.0);
        assert_eq!(APP_UI_THEME.foundation.spacing.xlarge_px, 24.0);
        assert_eq!(APP_UI_THEME.foundation.radii.small_px, 5.0);
        assert_eq!(APP_UI_THEME.foundation.radii.medium_px, 8.0);
        assert_eq!(APP_UI_THEME.foundation.radii.large_px, 10.0);
        assert_eq!(APP_UI_THEME.foundation.borders.divider_thickness_px, 1.0);
    }

    #[test]
    fn home_window_minimums_match_the_upgraded_shell_budget() {
        assert_eq!(APP_UI_THEME.shells.home_min_width_px, 1284.0);
        assert_eq!(APP_UI_THEME.shells.home_min_height_px, 795.0);
        assert_eq!(APP_UI_THEME.shells.home_sidebar_width_px, 240.0);
        assert_eq!(APP_UI_THEME.shells.home_window_padding_px, 24.0);
        assert_eq!(APP_UI_THEME.shells.focused_task_max_width_px, 720.0);
        assert_eq!(APP_UI_THEME.shells.focused_detail_max_width_px, 840.0);
    }

    #[test]
    fn settings_shell_layout_contract_is_explicit() {
        assert_eq!(APP_UI_THEME.shells.settings_width_px, 600.0);
        assert_eq!(APP_UI_THEME.shells.settings_height_px, 540.0);
        assert_eq!(APP_UI_THEME.shells.settings_chrome_height_px, 88.0);
        assert_eq!(APP_UI_THEME.shells.settings_content_padding_px, 24.0);
        assert_eq!(
            APP_UI_THEME.shells.settings_panel_content_max_width_px,
            560.0
        );
        assert_eq!(APP_UI_THEME.shells.settings_account_sidebar_width_px, 200.0);
        assert_eq!(
            APP_UI_THEME
                .shells
                .settings_account_sidebar_button_height_px,
            42.0
        );
    }

    #[test]
    fn control_tokens_match_the_frozen_budget() {
        let segmented = APP_UI_THEME.components.app_segment_button_icon.sizing;
        let action = APP_UI_THEME.components.app_button.sizing;
        let text_input = APP_UI_THEME.components.app_input_text;
        let checkbox = APP_UI_THEME.components.app_checkbox_field;
        let status = APP_UI_THEME.components.app_status_indicator;
        let account_selector = APP_UI_THEME.components.app_account_selector_row;

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
        assert_eq!(account_selector.inactive_background, 0xF2F2F7);
        assert_eq!(account_selector.active_background, 0xE5E5EA);
    }

    #[test]
    fn accent_and_status_colors_match_the_current_shell_contract() {
        assert_eq!(APP_UI_THEME.foundation.text.accent, 0x0A84FF);
        assert_eq!(
            APP_UI_THEME.components.app_button.primary_colors.background,
            0x0A84FF
        );
        assert_eq!(
            APP_UI_THEME.components.app_button.primary_colors.foreground,
            0xFFFFFF
        );
        assert_eq!(
            APP_UI_THEME
                .components
                .app_checkbox_field
                .checked_background,
            0x0A84FF
        );
        assert_eq!(
            APP_UI_THEME.components.app_status_indicator.online,
            0x34C759
        );
        assert_eq!(
            APP_UI_THEME.components.app_status_indicator.offline,
            0xFFD60A
        );
        assert_eq!(
            APP_UI_THEME.components.app_status_indicator.attention,
            0xFF3B30
        );
    }
}
