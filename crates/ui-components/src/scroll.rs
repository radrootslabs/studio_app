#![forbid(unsafe_code)]

use leptos::ev::Event;
use leptos::prelude::*;
use web_sys::HtmlElement;

const DEFAULT_COLLAPSE_RANGE: f64 = 120.0;

#[derive(Clone, Copy, Debug)]
struct RadrootsAppUiScrollSample {
    scroll_top: f64,
    time_ms: f64,
}

impl Default for RadrootsAppUiScrollSample {
    fn default() -> Self {
        Self {
            scroll_top: 0.0,
            time_ms: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RadrootsAppUiScrollContext {
    pub scroll_top: RwSignal<f64>,
    pub scroll_velocity: RwSignal<f64>,
    pub collapse_progress: RwSignal<f64>,
}

impl RadrootsAppUiScrollContext {
    pub fn new() -> Self {
        Self {
            scroll_top: RwSignal::new(0.0),
            scroll_velocity: RwSignal::new(0.0),
            collapse_progress: RwSignal::new(0.0),
        }
    }
}

pub fn radroots_studio_app_ui_collapse_progress(scroll_top: f64, collapse_range: f64) -> f64 {
    if collapse_range <= 0.0 {
        return 0.0;
    }
    (scroll_top / collapse_range).clamp(0.0, 1.0)
}

pub fn radroots_studio_app_ui_scroll_velocity(prev_top: f64, next_top: f64, dt_ms: f64) -> f64 {
    if dt_ms <= 0.0 {
        return 0.0;
    }
    (next_top - prev_top) / dt_ms * 1000.0
}

#[component]
pub fn RadrootsAppUiScrollContainer(
    id: Option<String>,
    classes: Option<String>,
    collapse_range: Option<f64>,
    context: Option<RadrootsAppUiScrollContext>,
    children: Children,
) -> impl IntoView {
    let context = context.unwrap_or_else(RadrootsAppUiScrollContext::new);
    provide_context(context.clone());
    let last_sample = RwSignal::new_local(RadrootsAppUiScrollSample::default());
    let collapse_range_value = collapse_range.unwrap_or(DEFAULT_COLLAPSE_RANGE);
    let class_value =
        classes.unwrap_or_else(|| "app-page app-page-scroll app-page-chrome".to_string());
    let on_scroll = move |ev: Event| {
        let target = event_target::<HtmlElement>(&ev);
        let scroll_top = target.scroll_top() as f64;
        let time_ms = ev.time_stamp();
        let prev = last_sample.get_untracked();
        let velocity = radroots_studio_app_ui_scroll_velocity(prev.scroll_top, scroll_top, time_ms - prev.time_ms);
        last_sample.set(RadrootsAppUiScrollSample { scroll_top, time_ms });
        context.scroll_top.set(scroll_top);
        context.scroll_velocity.set(velocity);
        context
            .collapse_progress
            .set(radroots_studio_app_ui_collapse_progress(scroll_top, collapse_range_value));
    };
    view! {
        <div id=id class=class_value on:scroll=on_scroll>
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{radroots_studio_app_ui_collapse_progress, radroots_studio_app_ui_scroll_velocity};

    #[test]
    fn collapse_progress_clamps_range() {
        assert_eq!(radroots_studio_app_ui_collapse_progress(0.0, 120.0), 0.0);
        assert_eq!(radroots_studio_app_ui_collapse_progress(60.0, 120.0), 0.5);
        assert_eq!(radroots_studio_app_ui_collapse_progress(180.0, 120.0), 1.0);
        assert_eq!(radroots_studio_app_ui_collapse_progress(-10.0, 120.0), 0.0);
    }

    #[test]
    fn collapse_progress_handles_zero_range() {
        assert_eq!(radroots_studio_app_ui_collapse_progress(10.0, 0.0), 0.0);
    }

    #[test]
    fn scroll_velocity_uses_delta_per_second() {
        assert_eq!(radroots_studio_app_ui_scroll_velocity(0.0, 100.0, 1000.0), 100.0);
        assert_eq!(radroots_studio_app_ui_scroll_velocity(100.0, 0.0, 500.0), -200.0);
    }

    #[test]
    fn scroll_velocity_handles_zero_time() {
        assert_eq!(radroots_studio_app_ui_scroll_velocity(0.0, 100.0, 0.0), 0.0);
    }
}
