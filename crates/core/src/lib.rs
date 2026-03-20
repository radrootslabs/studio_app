#![forbid(unsafe_code)]

use eframe::egui;

pub const APP_NAME: &str = "Rad Roots";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityGateState {
    Missing,
    Ready { account_id: String, npub: String },
    Unsupported { reason: String },
}

pub trait RadrootsAppBackend {
    fn load_identity_state(&self) -> Result<IdentityGateState, String>;
    fn setup_action_state(&self) -> SetupActionState;
    fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String>;
    fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppScreen {
    Setup,
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
            screen: AppScreen::Setup,
            status_message: None,
        };
        match app.backend.load_identity_state() {
            Ok(state) => app.apply_identity_state(state),
            Err(err) => {
                app.screen = AppScreen::Setup;
                app.status_message = Some(err);
            }
        }
        app
    }

    fn apply_identity_state(&mut self, state: IdentityGateState) {
        match state {
            IdentityGateState::Missing => {
                self.screen = AppScreen::Setup;
                self.status_message = None;
            }
            IdentityGateState::Ready { account_id, npub } => {
                self.screen = AppScreen::Home { account_id, npub };
                self.status_message = None;
            }
            IdentityGateState::Unsupported { reason } => {
                self.screen = AppScreen::Setup;
                self.status_message = Some(reason);
            }
        }
    }

    fn request_setup_action(&mut self) {
        self.status_message = None;
        match self.backend.request_setup_action() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn sync_backend(&mut self) {
        match self.backend.poll_identity_state() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }
}

impl eframe::App for RadrootsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.sync_backend();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading(APP_NAME);
                ui.add_space(12.0);

                match &self.screen {
                    AppScreen::Setup => {
                        let action = self.backend.setup_action_state();
                        if action.pending {
                            ctx.request_repaint();
                        }

                        ui.label("setup");
                        ui.add_space(8.0);
                        ui.label("A signing identity is required before the app can continue.");
                        ui.add_space(16.0);
                        let clicked = ui
                            .add_enabled(action.enabled, egui::Button::new(action.label))
                            .clicked();
                        if clicked {
                            self.request_setup_action();
                        }
                    }
                    AppScreen::Home { account_id, npub } => {
                        ui.label("home");
                        ui.add_space(8.0);
                        ui.label("A signing identity is configured.");
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
        action_state: Rc<RefCell<SetupActionState>>,
        request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        poll: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
    }

    impl MockBackend {
        fn new(
            load: Result<IdentityGateState, String>,
            request: Vec<Result<Option<IdentityGateState>, String>>,
            poll: Vec<Result<Option<IdentityGateState>, String>>,
            action_state: SetupActionState,
        ) -> Self {
            Self {
                load,
                action_state: Rc::new(RefCell::new(action_state)),
                request: Rc::new(RefCell::new(request.into())),
                poll: Rc::new(RefCell::new(poll.into())),
            }
        }
    }

    impl RadrootsAppBackend for MockBackend {
        fn load_identity_state(&self) -> Result<IdentityGateState, String> {
            self.load.clone()
        }

        fn setup_action_state(&self) -> SetupActionState {
            self.action_state.borrow().clone()
        }

        fn request_setup_action(&self) -> Result<Option<IdentityGateState>, String> {
            self.request
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err("missing request response".into()))
        }

        fn poll_identity_state(&self) -> Result<Option<IdentityGateState>, String> {
            self.poll.borrow_mut().pop_front().unwrap_or(Ok(None))
        }
    }

    #[test]
    fn startup_missing_key_enters_setup() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));
        assert_eq!(app.screen, AppScreen::Setup);
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
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
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
    fn startup_unsupported_shows_reason() {
        let app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Unsupported {
                reason: "unsupported".into(),
            }),
            vec![],
            vec![],
            SetupActionState {
                label: "Connect Browser Signer".into(),
                enabled: false,
                pending: false,
            },
        )));
        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message.as_deref(), Some("unsupported"));
    }

    #[test]
    fn deferred_setup_action_transitions_to_home_after_poll() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![Ok(None)],
            vec![Ok(Some(IdentityGateState::Ready {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }))],
            SetupActionState {
                label: "Connect Browser Signer".into(),
                enabled: true,
                pending: false,
            },
        )));

        app.request_setup_action();
        assert_eq!(app.screen, AppScreen::Setup);

        app.sync_backend();

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }
        );
    }

    #[test]
    fn immediate_setup_action_transitions_to_home() {
        let mut app = RadrootsApp::new(Box::new(MockBackend::new(
            Ok(IdentityGateState::Missing),
            vec![Ok(Some(IdentityGateState::Ready {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }))],
            vec![],
            SetupActionState {
                label: "Generate New Key".into(),
                enabled: true,
                pending: false,
            },
        )));

        app.request_setup_action();

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }
        );
    }
}
