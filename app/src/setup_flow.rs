#![forbid(unsafe_code)]

use crate::{app_setup_step_default, RadrootsAppRole, RadrootsAppSetupStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppSetupKeyChoice {
    Generate,
    AddExisting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppSetupFarmerChoice {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppSetupBusinessChoice {
    Yes,
    No,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppSetupFlowDraft {
    pub step: RadrootsAppSetupStep,
    pub key_choice: Option<RadrootsAppSetupKeyChoice>,
    pub farmer_choice: Option<RadrootsAppSetupFarmerChoice>,
    pub business_choice: Option<RadrootsAppSetupBusinessChoice>,
    pub profile_name: String,
    pub profile_nip05: bool,
}

impl Default for RadrootsAppSetupFlowDraft {
    fn default() -> Self {
        Self {
            step: app_setup_step_default(),
            key_choice: None,
            farmer_choice: None,
            business_choice: None,
            profile_name: String::new(),
            profile_nip05: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppSetupFlowValidation {
    pub can_continue: bool,
    pub can_back: bool,
    pub next_step: RadrootsAppSetupStep,
    pub prev_step: RadrootsAppSetupStep,
}

pub fn app_setup_flow_role_from_choices(
    farmer_choice: Option<RadrootsAppSetupFarmerChoice>,
    business_choice: Option<RadrootsAppSetupBusinessChoice>,
) -> Option<RadrootsAppRole> {
    match farmer_choice? {
        RadrootsAppSetupFarmerChoice::Yes => Some(RadrootsAppRole::Farm),
        RadrootsAppSetupFarmerChoice::No => match business_choice? {
            RadrootsAppSetupBusinessChoice::Yes => Some(RadrootsAppRole::Business),
            RadrootsAppSetupBusinessChoice::No => Some(RadrootsAppRole::Individual),
        },
    }
}

pub fn app_setup_flow_next_step(draft: &RadrootsAppSetupFlowDraft) -> RadrootsAppSetupStep {
    match draft.step {
        RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::KeyChoice,
        RadrootsAppSetupStep::KeyChoice => match draft.key_choice {
            Some(RadrootsAppSetupKeyChoice::Generate) => RadrootsAppSetupStep::Profile,
            Some(RadrootsAppSetupKeyChoice::AddExisting) => RadrootsAppSetupStep::KeyAddExisting,
            None => RadrootsAppSetupStep::KeyChoice,
        },
        RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::Profile,
        RadrootsAppSetupStep::Profile => RadrootsAppSetupStep::FarmerSetup,
        RadrootsAppSetupStep::FarmerSetup => match draft.farmer_choice {
            Some(RadrootsAppSetupFarmerChoice::Yes) => RadrootsAppSetupStep::Eula,
            Some(RadrootsAppSetupFarmerChoice::No) => RadrootsAppSetupStep::BusinessSetup,
            None => RadrootsAppSetupStep::FarmerSetup,
        },
        RadrootsAppSetupStep::BusinessSetup => RadrootsAppSetupStep::Eula,
        RadrootsAppSetupStep::Eula => RadrootsAppSetupStep::Eula,
    }
}

pub fn app_setup_flow_prev_step(draft: &RadrootsAppSetupFlowDraft) -> RadrootsAppSetupStep {
    match draft.step {
        RadrootsAppSetupStep::Intro => RadrootsAppSetupStep::Intro,
        RadrootsAppSetupStep::KeyChoice => RadrootsAppSetupStep::Intro,
        RadrootsAppSetupStep::KeyAddExisting => RadrootsAppSetupStep::KeyChoice,
        RadrootsAppSetupStep::Profile => match draft.key_choice {
            Some(RadrootsAppSetupKeyChoice::AddExisting) => RadrootsAppSetupStep::KeyAddExisting,
            _ => RadrootsAppSetupStep::KeyChoice,
        },
        RadrootsAppSetupStep::FarmerSetup => RadrootsAppSetupStep::Profile,
        RadrootsAppSetupStep::BusinessSetup => RadrootsAppSetupStep::FarmerSetup,
        RadrootsAppSetupStep::Eula => match draft.farmer_choice {
            Some(RadrootsAppSetupFarmerChoice::No) => RadrootsAppSetupStep::BusinessSetup,
            _ => RadrootsAppSetupStep::FarmerSetup,
        },
    }
}

pub fn app_setup_flow_validate(draft: &RadrootsAppSetupFlowDraft) -> RadrootsAppSetupFlowValidation {
    let can_continue = match draft.step {
        RadrootsAppSetupStep::KeyChoice => draft.key_choice.is_some(),
        RadrootsAppSetupStep::FarmerSetup => draft.farmer_choice.is_some(),
        RadrootsAppSetupStep::BusinessSetup => draft.business_choice.is_some(),
        RadrootsAppSetupStep::Profile => {
            !(draft.profile_nip05 && draft.profile_name.trim().is_empty())
        }
        _ => true,
    };
    let can_back = !matches!(draft.step, RadrootsAppSetupStep::Intro);
    RadrootsAppSetupFlowValidation {
        can_continue,
        can_back,
        next_step: app_setup_flow_next_step(draft),
        prev_step: app_setup_flow_prev_step(draft),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_setup_flow_next_step,
        app_setup_flow_prev_step,
        app_setup_flow_role_from_choices,
        app_setup_flow_validate,
        RadrootsAppSetupBusinessChoice,
        RadrootsAppSetupFarmerChoice,
        RadrootsAppSetupFlowDraft,
        RadrootsAppSetupKeyChoice,
    };
    use crate::{RadrootsAppRole, RadrootsAppSetupStep};

    #[test]
    fn flow_defaults_to_intro() {
        let draft = RadrootsAppSetupFlowDraft::default();
        assert_eq!(draft.step, RadrootsAppSetupStep::Intro);
        assert!(draft.profile_nip05);
    }

    #[test]
    fn flow_role_from_choices_maps_roles() {
        assert_eq!(
            app_setup_flow_role_from_choices(
                Some(RadrootsAppSetupFarmerChoice::Yes),
                None,
            ),
            Some(RadrootsAppRole::Farm)
        );
        assert_eq!(
            app_setup_flow_role_from_choices(
                Some(RadrootsAppSetupFarmerChoice::No),
                Some(RadrootsAppSetupBusinessChoice::Yes),
            ),
            Some(RadrootsAppRole::Business)
        );
        assert_eq!(
            app_setup_flow_role_from_choices(
                Some(RadrootsAppSetupFarmerChoice::No),
                Some(RadrootsAppSetupBusinessChoice::No),
            ),
            Some(RadrootsAppRole::Individual)
        );
        assert_eq!(
            app_setup_flow_role_from_choices(
                Some(RadrootsAppSetupFarmerChoice::No),
                None,
            ),
            None
        );
    }

    #[test]
    fn flow_next_step_respects_choices() {
        let mut draft = RadrootsAppSetupFlowDraft::default();
        draft.step = RadrootsAppSetupStep::KeyChoice;
        draft.key_choice = Some(RadrootsAppSetupKeyChoice::Generate);
        assert_eq!(app_setup_flow_next_step(&draft), RadrootsAppSetupStep::Profile);
        draft.key_choice = Some(RadrootsAppSetupKeyChoice::AddExisting);
        assert_eq!(
            app_setup_flow_next_step(&draft),
            RadrootsAppSetupStep::KeyAddExisting
        );
        draft.step = RadrootsAppSetupStep::FarmerSetup;
        draft.farmer_choice = Some(RadrootsAppSetupFarmerChoice::No);
        assert_eq!(
            app_setup_flow_next_step(&draft),
            RadrootsAppSetupStep::BusinessSetup
        );
    }

    #[test]
    fn flow_prev_step_respects_choices() {
        let mut draft = RadrootsAppSetupFlowDraft::default();
        draft.step = RadrootsAppSetupStep::Profile;
        draft.key_choice = Some(RadrootsAppSetupKeyChoice::AddExisting);
        assert_eq!(
            app_setup_flow_prev_step(&draft),
            RadrootsAppSetupStep::KeyAddExisting
        );
        draft.step = RadrootsAppSetupStep::Eula;
        draft.farmer_choice = Some(RadrootsAppSetupFarmerChoice::No);
        assert_eq!(
            app_setup_flow_prev_step(&draft),
            RadrootsAppSetupStep::BusinessSetup
        );
    }

    #[test]
    fn flow_validation_disables_continue_for_missing_name() {
        let mut draft = RadrootsAppSetupFlowDraft::default();
        draft.step = RadrootsAppSetupStep::Profile;
        draft.profile_name = String::new();
        draft.profile_nip05 = true;
        let validation = app_setup_flow_validate(&draft);
        assert!(!validation.can_continue);
        draft.profile_nip05 = false;
        let validation = app_setup_flow_validate(&draft);
        assert!(validation.can_continue);
    }
}
