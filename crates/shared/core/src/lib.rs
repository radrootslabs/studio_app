#![forbid(unsafe_code)]

pub const APP_ID: &str = "org.radroots.app";
pub const APP_NAME: &str = "radroots";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppWindowMetrics {
    pub min_width_px: f32,
    pub min_height_px: f32,
}

pub const HOME_WINDOW_METRICS: AppWindowMetrics = AppWindowMetrics {
    min_width_px: 640.0,
    min_height_px: 480.0,
};
