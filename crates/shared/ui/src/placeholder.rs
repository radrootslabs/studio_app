use gpui::{Context, FontWeight, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};
use radroots_studio_app_i18n::{AppTextKey, app_text};

use crate::{APP_UI_THEME, app_center_stage, app_window_shell};

pub struct PlaceholderView;

impl Render for PlaceholderView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        app_window_shell(
            APP_UI_THEME.surfaces.window_background,
            app_center_stage(
                div()
                    .text_size(px(APP_UI_THEME.typography.brand_text_px))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(APP_UI_THEME.text.primary))
                    .child(app_text(AppTextKey::Brand)),
            ),
        )
    }
}
