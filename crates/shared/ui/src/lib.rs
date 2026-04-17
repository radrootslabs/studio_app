#![forbid(unsafe_code)]

use gpui::{
    Context, FontWeight, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb,
};
use radroots_studio_app_i18n::{AppTextKey, app_text};

pub const HOME_WINDOW_MIN_WIDTH_PX: f32 = 640.0;
pub const HOME_WINDOW_MIN_HEIGHT_PX: f32 = 480.0;
pub const WINDOW_BACKGROUND: u32 = 0xF5F1E8;
pub const TEXT_PRIMARY: u32 = 0x1F2C23;
pub const BRAND_TEXT_SIZE_PX: f32 = 20.0;

pub struct PlaceholderView;

impl Render for PlaceholderView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(WINDOW_BACKGROUND))
            .child(
                div()
                    .text_size(px(BRAND_TEXT_SIZE_PX))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT_PRIMARY))
                    .child(app_text(AppTextKey::Brand)),
            )
    }
}
