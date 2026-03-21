#![forbid(unsafe_code)]

use eframe::egui;
use zeroize::Zeroizing;

pub const APP_NAME: &str = "Rad Roots";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportActionState {
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeActionState {
    pub kind: HomeActionKind,
    pub label: String,
    pub enabled: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeActionKind {
    BackupSecretKey,
    RemoveLocalKey,
    ResetDevice,
    DisconnectSigner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeActionResult {
    None,
    IdentityState(IdentityGateState),
    RevealSecretKey { nsec: String },
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
    fn import_action_state(&self) -> Option<ImportActionState> {
        None
    }
    fn request_import_action(
        &self,
        _secret_key: &str,
    ) -> Result<Option<IdentityGateState>, String> {
        Ok(None)
    }
    fn home_action_states(&self) -> Vec<HomeActionState> {
        Vec::new()
    }
    fn request_home_action(&self, _action: HomeActionKind) -> Result<HomeActionResult, String> {
        Ok(HomeActionResult::None)
    }
    fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
        Ok(None)
    }
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
    pending_home_confirmation: Option<HomeActionKind>,
    pending_import_entry: bool,
    secret_key_input: Zeroizing<String>,
    revealed_secret_key: Option<Zeroizing<String>>,
}

impl RadrootsApp {
    pub fn new(backend: Box<dyn RadrootsAppBackend>) -> Self {
        let mut app = Self {
            backend,
            screen: AppScreen::Setup,
            status_message: None,
            pending_home_confirmation: None,
            pending_import_entry: false,
            secret_key_input: Zeroizing::new(String::new()),
            revealed_secret_key: None,
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
                self.pending_home_confirmation = None;
                self.pending_import_entry = false;
                self.secret_key_input.clear();
                self.revealed_secret_key = None;
            }
            IdentityGateState::Ready { account_id, npub } => {
                self.screen = AppScreen::Home { account_id, npub };
                self.status_message = None;
                self.pending_home_confirmation = None;
                self.pending_import_entry = false;
                self.secret_key_input.clear();
                self.revealed_secret_key = None;
            }
            IdentityGateState::Unsupported { reason } => {
                self.screen = AppScreen::Setup;
                self.status_message = Some(reason);
                self.pending_home_confirmation = None;
                self.pending_import_entry = false;
                self.secret_key_input.clear();
                self.revealed_secret_key = None;
            }
        }
    }

    fn request_setup_action(&mut self) {
        self.status_message = None;
        self.revealed_secret_key = None;
        match self.backend.request_setup_action() {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_import_action(&mut self) {
        self.status_message = None;
        self.revealed_secret_key = None;
        match self
            .backend
            .request_import_action(self.secret_key_input.trim())
        {
            Ok(Some(state)) => self.apply_identity_state(state),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn request_home_action(&mut self, action: HomeActionKind) {
        self.status_message = None;
        self.revealed_secret_key = None;
        match self.backend.request_home_action(action) {
            Ok(result) => self.apply_home_action_result(result),
            Err(err) => {
                self.status_message = Some(err);
            }
        }
    }

    fn apply_home_action_result(&mut self, result: HomeActionResult) {
        match result {
            HomeActionResult::IdentityState(state) => self.apply_identity_state(state),
            HomeActionResult::RevealSecretKey { nsec } => {
                self.revealed_secret_key = Some(Zeroizing::new(nsec));
                self.pending_home_confirmation = None;
            }
            HomeActionResult::None => {}
        }
    }

    fn home_action_requires_confirmation(action: HomeActionKind) -> bool {
        !matches!(action, HomeActionKind::BackupSecretKey)
    }

    fn home_action_confirmation_message(action: HomeActionKind) -> &'static str {
        match action {
            HomeActionKind::BackupSecretKey => {
                "This reveals the current local secret key for backup. Do not share it."
            }
            HomeActionKind::RemoveLocalKey => {
                "This removes the current key from this device and returns the app to setup."
            }
            HomeActionKind::ResetDevice => {
                "This removes all app-managed local identity state from this device and returns the app to setup."
            }
            HomeActionKind::DisconnectSigner => {
                "This disconnects the current browser signer from the app. It does not delete the signer key."
            }
        }
    }

    fn sync_backend(&mut self) {
        match self.backend.poll_home_action_result() {
            Ok(Some(result)) => self.apply_home_action_result(result),
            Ok(None) => {}
            Err(err) => {
                self.status_message = Some(err);
            }
        }
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
                        let import_action = self.backend.import_action_state();
                        if let Some(import_action) = &import_action {
                            if import_action.pending {
                                ctx.request_repaint();
                            }
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

                        if let Some(import_action) = import_action {
                            ui.add_space(12.0);
                            if self.pending_import_entry {
                                ui.vertical_centered(|ui| {
                                    ui.set_max_width(ui.available_width().min(560.0));
                                    ui.label(
                                        "Import an existing local identity by entering its nsec secret key.",
                                    );
                                    ui.add_space(8.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut *self.secret_key_input)
                                            .hint_text("nsec1...")
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.add_space(8.0);
                                    ui.horizontal_centered(|ui| {
                                        let confirm_clicked = ui
                                            .add_enabled(
                                                import_action.enabled,
                                                egui::Button::new(import_action.label.clone()),
                                            )
                                            .clicked();
                                        if confirm_clicked {
                                            self.request_import_action();
                                        }

                                        if ui.button("Cancel").clicked() {
                                            self.pending_import_entry = false;
                                            self.secret_key_input.clear();
                                            self.status_message = None;
                                        }
                                    });
                                });
                            } else if ui.button(import_action.label).clicked() {
                                self.pending_import_entry = true;
                                self.status_message = None;
                            }
                        }
                    }
                    AppScreen::Home { account_id, npub } => {
                        ui.label("home");
                        ui.add_space(8.0);
                        ui.label("A signing identity is configured.");
                        ui.add_space(12.0);
                        ui.monospace(format!("account id: {account_id}"));
                        ui.monospace(format!("npub: {npub}"));

                        let actions = self.backend.home_action_states();
                        for (index, action) in actions.into_iter().enumerate() {
                            ui.add_space(if index == 0 { 20.0 } else { 12.0 });
                            if action.pending {
                                ctx.request_repaint();
                            }

                            if Self::home_action_requires_confirmation(action.kind)
                                && self.pending_home_confirmation == Some(action.kind)
                            {
                                ui.vertical_centered(|ui| {
                                    ui.set_max_width(ui.available_width().min(560.0));
                                    ui.label(Self::home_action_confirmation_message(action.kind));
                                    ui.add_space(8.0);
                                    ui.horizontal_centered(|ui| {
                                        let confirm_clicked = ui
                                            .add_enabled(
                                                action.enabled,
                                                egui::Button::new(action.label.clone()),
                                            )
                                            .clicked();
                                        if confirm_clicked {
                                            self.request_home_action(action.kind);
                                        }

                                        if ui.button("Cancel").clicked() {
                                            self.pending_home_confirmation = None;
                                            self.status_message = None;
                                        }
                                    });
                                });
                            } else if Self::home_action_requires_confirmation(action.kind)
                                && self.pending_home_confirmation.is_none()
                                && ui.button(action.label.clone()).clicked()
                            {
                                self.pending_home_confirmation = Some(action.kind);
                            } else if !Self::home_action_requires_confirmation(action.kind)
                                && ui
                                    .add_enabled(
                                        action.enabled,
                                        egui::Button::new(action.label.clone()),
                                    )
                                    .clicked()
                            {
                                self.request_home_action(action.kind);
                            }
                        }

                        if let Some(nsec) = &self.revealed_secret_key {
                            ui.add_space(20.0);
                            ui.label("Secret key");
                            ui.add_space(8.0);
                            ui.monospace(nsec.as_str());
                            ui.add_space(8.0);
                            if ui.button("Dismiss Secret Key").clicked() {
                                self.revealed_secret_key = None;
                            }
                        }
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
        import_action_state: Rc<RefCell<Option<ImportActionState>>>,
        home_action_states: Rc<RefCell<Vec<HomeActionState>>>,
        request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        import_request: Rc<RefCell<VecDeque<Result<Option<IdentityGateState>, String>>>>,
        home_request: Rc<RefCell<VecDeque<(HomeActionKind, Result<HomeActionResult, String>)>>>,
        home_poll: Rc<RefCell<VecDeque<Result<Option<HomeActionResult>, String>>>>,
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
                import_action_state: Rc::new(RefCell::new(None)),
                home_action_states: Rc::new(RefCell::new(Vec::new())),
                request: Rc::new(RefCell::new(request.into())),
                import_request: Rc::new(RefCell::new(VecDeque::new())),
                home_request: Rc::new(RefCell::new(VecDeque::new())),
                home_poll: Rc::new(RefCell::new(VecDeque::new())),
                poll: Rc::new(RefCell::new(poll.into())),
            }
        }

        fn with_import_action(
            self,
            action_state: ImportActionState,
            request: Vec<Result<Option<IdentityGateState>, String>>,
        ) -> Self {
            *self.import_action_state.borrow_mut() = Some(action_state);
            self.import_request.borrow_mut().extend(request);
            self
        }

        fn with_home_action(
            self,
            action_state: HomeActionState,
            request: Vec<Result<HomeActionResult, String>>,
        ) -> Self {
            self.home_action_states
                .borrow_mut()
                .push(action_state.clone());
            self.home_request.borrow_mut().extend(
                request
                    .into_iter()
                    .map(|result| (action_state.kind, result)),
            );
            self
        }

        fn with_home_action_poll(
            self,
            poll: Vec<Result<Option<HomeActionResult>, String>>,
        ) -> Self {
            self.home_poll.borrow_mut().extend(poll);
            self
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

        fn import_action_state(&self) -> Option<ImportActionState> {
            self.import_action_state.borrow().clone()
        }

        fn request_import_action(
            &self,
            _secret_key: &str,
        ) -> Result<Option<IdentityGateState>, String> {
            self.import_request
                .borrow_mut()
                .pop_front()
                .unwrap_or(Ok(None))
        }

        fn home_action_states(&self) -> Vec<HomeActionState> {
            self.home_action_states.borrow().clone()
        }

        fn request_home_action(&self, action: HomeActionKind) -> Result<HomeActionResult, String> {
            let Some((expected_action, response)) = self.home_request.borrow_mut().pop_front()
            else {
                return Err("missing home action response".into());
            };
            if expected_action != action {
                return Err(format!(
                    "unexpected home action request: expected {:?}, got {:?}",
                    expected_action, action
                ));
            }
            response
        }

        fn poll_home_action_result(&self) -> Result<Option<HomeActionResult>, String> {
            self.home_poll.borrow_mut().pop_front().unwrap_or(Ok(None))
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

    #[test]
    fn home_remove_action_transitions_to_setup() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::RemoveLocalKey,
                    label: "Remove Key From This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::IdentityState(
                    IdentityGateState::Missing,
                ))],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::RemoveLocalKey);
        app.request_home_action(HomeActionKind::RemoveLocalKey);

        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message, None);
        assert_eq!(app.pending_home_confirmation, None);
    }

    #[test]
    fn failed_home_remove_action_keeps_home_screen_and_message() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::RemoveLocalKey,
                    label: "Remove Key From This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Err("remove failed".into())],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::RemoveLocalKey);
        app.request_home_action(HomeActionKind::RemoveLocalKey);

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.status_message.as_deref(), Some("remove failed"));
        assert_eq!(
            app.pending_home_confirmation,
            Some(HomeActionKind::RemoveLocalKey)
        );
    }

    #[test]
    fn home_reset_action_transitions_to_setup() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::ResetDevice,
                    label: "Reset This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::IdentityState(
                    IdentityGateState::Missing,
                ))],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::ResetDevice);
        app.request_home_action(HomeActionKind::ResetDevice);

        assert_eq!(app.screen, AppScreen::Setup);
        assert_eq!(app.status_message, None);
        assert_eq!(app.pending_home_confirmation, None);
    }

    #[test]
    fn failed_home_reset_action_keeps_home_screen_and_message() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::ResetDevice,
                    label: "Reset This Device".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Err("reset failed".into())],
            ),
        ));

        app.pending_home_confirmation = Some(HomeActionKind::ResetDevice);
        app.request_home_action(HomeActionKind::ResetDevice);

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.status_message.as_deref(), Some("reset failed"));
        assert_eq!(
            app.pending_home_confirmation,
            Some(HomeActionKind::ResetDevice)
        );
    }

    #[test]
    fn import_action_transitions_to_home() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
                Ok(IdentityGateState::Missing),
                vec![],
                vec![],
                SetupActionState {
                    label: "Generate New Key".into(),
                    enabled: true,
                    pending: false,
                },
            )
            .with_import_action(
                ImportActionState {
                    label: "Import Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(Some(IdentityGateState::Ready {
                    account_id: "abc".into(),
                    npub: "npub1abc".into(),
                }))],
            ),
        ));

        app.pending_import_entry = true;
        app.secret_key_input = Zeroizing::new("nsec1example".into());
        app.request_import_action();

        assert_eq!(
            app.screen,
            AppScreen::Home {
                account_id: "abc".into(),
                npub: "npub1abc".into(),
            }
        );
        assert_eq!(app.pending_import_entry, false);
        assert_eq!(app.secret_key_input.as_str(), "");
    }

    #[test]
    fn backup_home_action_reveals_secret_key_without_leaving_home() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::BackupSecretKey,
                    label: "Back Up Secret Key".into(),
                    enabled: true,
                    pending: false,
                },
                vec![Ok(HomeActionResult::RevealSecretKey {
                    nsec: "nsec1example".into(),
                })],
            ),
        ));

        app.request_home_action(HomeActionKind::BackupSecretKey);

        assert!(matches!(app.screen, AppScreen::Home { .. }));
        assert_eq!(app.pending_home_confirmation, None);
        assert_eq!(
            app.revealed_secret_key.as_ref().map(|value| value.as_str()),
            Some("nsec1example")
        );
    }

    #[test]
    fn deferred_backup_home_action_reveals_secret_key_after_poll() {
        let mut app = RadrootsApp::new(Box::new(
            MockBackend::new(
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
            )
            .with_home_action(
                HomeActionState {
                    kind: HomeActionKind::BackupSecretKey,
                    label: "Back Up Secret Key".into(),
                    enabled: true,
                    pending: true,
                },
                vec![Ok(HomeActionResult::None)],
            )
            .with_home_action_poll(vec![Ok(Some(HomeActionResult::RevealSecretKey {
                nsec: "nsec1example".into(),
            }))]),
        ));

        app.request_home_action(HomeActionKind::BackupSecretKey);
        assert_eq!(app.revealed_secret_key, None);

        app.sync_backend();

        assert_eq!(
            app.revealed_secret_key.as_ref().map(|value| value.as_str()),
            Some("nsec1example")
        );
    }
}
