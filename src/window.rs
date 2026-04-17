use gpui::{Context, FontWeight, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

pub struct PlaceholderView;

impl Render for PlaceholderView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0xF5F1E8))
            .child(
                div()
                    .text_size(px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0x1F2C23))
                    .child("radroots"),
            )
    }
}
