#![forbid(unsafe_code)]

use eframe::egui;

pub const APP_NAME: &str = "Rad Roots";

#[derive(Default)]
pub struct RadrootsApp;

impl eframe::App for RadrootsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading(APP_NAME);
                ui.add_space(12.0);
                ui.label("radroots app");
            });
        });
    }
}
