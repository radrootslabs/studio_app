#![forbid(unsafe_code)]

use eframe::egui;

pub const APP_NAME: &str = "Rad Roots";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityGateState {
    Missing,
    Ready { account_id: String, npub: String },
    Unsupported { reason: String },
}

pub trait RadrootsAppBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String>;
    fn generate_new_key(&self) -> Result<IdentityGateState, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppScreen {
    Setup { generate_enabled: bool },
    Home { account_id: String, npub: String },
}

pub struct RadrootsApp {
    backend: Box<dyn RadrootsAppBackend>,
    screen: AppScreen,
    status_message: Option<String>,
}

impl RadrootsApp {
    pub fn new(backend: Box<dyn RadrootsAppBackend>) -> Self {
        let mut app = Self {
            backend,
            screen: AppScreen::Setup {
                generate_enabled: true,
            },
            status_message: None,
        };
        match app.backend.load_identity_state() {
            Ok(state) => app.apply_identity_state(state),
            Err(err) => {
                app.screen = AppScreen::Setup {
                    generate_enabled: false,
                };
                app.status_message = Some(err);
            }
        }
        app
    }

    fn apply_identity_state(&mut self, state: IdentityGateState) {
        match state {
            IdentityGateState::Missing => {
                self.screen = AppScreen::Setup {
                    generate_enabled: true,
                };
                self.status_message = None;
            }
            IdentityGateState::Ready { account_id, npub } => {
                self.screen = AppScreen::Home { account_id, npub };
                self.status_message = None;
            }
            IdentityGateState::Unsupported { reason } => {
                self.screen = AppScreen::Setup {
                    generate_enabled: false,
                };
                self.status_message = Some(reason);
            }
        }
    }
}

impl eframe::App for RadrootsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading(APP_NAME);
                ui.add_space(12.0);

                match &self.screen {
                    AppScreen::Setup { generate_enabled } => {
                        ui.label("setup");
                        ui.add_space(8.0);
                        ui.label("A local Nostr key is required before the app can continue.");
                        ui.add_space(16.0);
                        let clicked = ui
                            .add_enabled(*generate_enabled, egui::Button::new("Generate New Key"))
                            .clicked();
                        if clicked {
                            match self.backend.generate_new_key() {
                                Ok(state) => self.apply_identity_state(state),
                                Err(err) => {
                                    self.status_message = Some(err);
                                }
                            }
                        }
                    }
                    AppScreen::Home { account_id, npub } => {
                        ui.label("home");
                        ui.add_space(8.0);
                        ui.label("A local signing identity is configured.");
                        ui.add_space(12.0);
                        ui.monospace(format!("account id: {account_id}"));
                        ui.monospace(format!("npub: {npub}"));
                    }
                }

                if let Some(message) = &self.status_message {
                    ui.add_space(16.0);
                    ui.label(message);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    #[derive(Clone)]
    struct MockBackend {
        load: Result<IdentityGateState, String>,
        generate: Rc<RefCell<VecDeque<Result<IdentityGateState, String>>>>,
    }

    impl MockBackend {
        fn new(
            load: Result<IdentityGateState, String>,
            generate: Vec<Result<IdentityGateState, String>>,
        ) -> Self {
            Self {
                load,
                generate: Rc::new(RefCell::new(generate.into())),
            }
        }
    }

    impl RadrootsAppBackend for MockBackend {
        fn load_identity_state(&self) -> Result<IdentityGateState, String> {
            self.load.clone()
        }

        fn generate_new_key(&self) -> Result<IdentityGateState, String> {
            self.generate
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err("missing generate response".into()))
        }
    }

    #[test]
    fn startup_missing_key_enters_setup() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![],
        )));
        assert_eq!(
            app.screen,
            AppScreen::Setup {
                generate_enabled: true
            }
        );
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn startup_ready_key_enters_home() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Ready {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }),
            vec![],
        )));
        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }
        );
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn startup_unsupported_disables_generation() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Unsupported {
                reason: "unsupported".into(),
            }),
            vec![],
        )));
        assert_eq!(
            app.screen,
            AppScreen::Setup {
                generate_enabled: false
            }
        );
        assert_eq!(app.status_message.as_deref(), Some("unsupported"));
    }

    #[test]
    fn generate_result_transitions_to_home() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![Ok(IdentityGateState::Ready {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            })],
        )));

        let state = app.backend.generate_new_key().expect("generate");
        app.apply_identity_state(state);

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }
        );
    }
}
