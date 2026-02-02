#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppSetupStatus {
    Unknown,
    Required,
    Configured,
    Corrupt,
    Locked,
}

impl Default for RadrootsAppSetupStatus {
    fn default() -> Self {
        RadrootsAppSetupStatus::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadrootsAppSetupGate {
    pub show_app: bool,
    pub show_setup: bool,
    pub show_setup_nav: bool,
    pub show_recovery: bool,
}

impl RadrootsAppSetupGate {
    pub const fn splash() -> Self {
        Self {
            show_app: false,
            show_setup: false,
            show_setup_nav: false,
            show_recovery: false,
        }
    }
}

pub const fn app_setup_gate_from_status(status: RadrootsAppSetupStatus) -> RadrootsAppSetupGate {
    match status {
        RadrootsAppSetupStatus::Unknown => RadrootsAppSetupGate::splash(),
        RadrootsAppSetupStatus::Required => RadrootsAppSetupGate {
            show_app: false,
            show_setup: true,
            show_setup_nav: false,
            show_recovery: false,
        },
        RadrootsAppSetupStatus::Configured => RadrootsAppSetupGate {
            show_app: true,
            show_setup: false,
            show_setup_nav: false,
            show_recovery: false,
        },
        RadrootsAppSetupStatus::Corrupt => RadrootsAppSetupGate {
            show_app: false,
            show_setup: false,
            show_setup_nav: false,
            show_recovery: true,
        },
        RadrootsAppSetupStatus::Locked => RadrootsAppSetupGate {
            show_app: false,
            show_setup: true,
            show_setup_nav: false,
            show_recovery: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{app_setup_gate_from_status, RadrootsAppSetupGate, RadrootsAppSetupStatus};

    #[test]
    fn unknown_status_routes_to_splash() {
        assert_eq!(
            app_setup_gate_from_status(RadrootsAppSetupStatus::Unknown),
            RadrootsAppSetupGate::splash()
        );
    }

    #[test]
    fn required_status_shows_setup() {
        let gate = app_setup_gate_from_status(RadrootsAppSetupStatus::Required);
        assert!(gate.show_setup);
        assert!(!gate.show_app);
        assert!(!gate.show_setup_nav);
        assert!(!gate.show_recovery);
    }

    #[test]
    fn configured_status_shows_app() {
        let gate = app_setup_gate_from_status(RadrootsAppSetupStatus::Configured);
        assert!(gate.show_app);
        assert!(!gate.show_setup);
        assert!(!gate.show_setup_nav);
        assert!(!gate.show_recovery);
    }

    #[test]
    fn corrupt_status_shows_recovery() {
        let gate = app_setup_gate_from_status(RadrootsAppSetupStatus::Corrupt);
        assert!(gate.show_recovery);
        assert!(!gate.show_app);
        assert!(!gate.show_setup);
        assert!(!gate.show_setup_nav);
    }

    #[test]
    fn locked_status_shows_setup() {
        let gate = app_setup_gate_from_status(RadrootsAppSetupStatus::Locked);
        assert!(gate.show_setup);
        assert!(!gate.show_app);
        assert!(!gate.show_setup_nav);
        assert!(!gate.show_recovery);
    }
}
