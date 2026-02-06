#![forbid(unsafe_code)]

use crate::{
    RadrootsAppConfigBusiness,
    RadrootsAppConfigData,
    RadrootsAppConfigFarmer,
    RadrootsAppConfigIndividual,
    RadrootsAppConfigPreferences,
    RadrootsAppConfigProfile,
    RadrootsAppRole,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppConfigStep {
    Profile,
    Role,
    Preferences,
}

pub const fn app_config_step_default() -> RadrootsAppConfigStep {
    RadrootsAppConfigStep::Profile
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppConfigFlowDraft {
    pub step: RadrootsAppConfigStep,
    pub profile_name: String,
    pub profile_location: String,
    pub role: Option<RadrootsAppRole>,
    pub farmer_farm_name: String,
    pub farmer_location: String,
    pub farmer_products: Vec<String>,
    pub individual_name: String,
    pub individual_location: String,
    pub individual_products: Vec<String>,
    pub business_name: String,
    pub business_location: String,
    pub business_operations: String,
    pub notifications_orders: bool,
    pub notifications_messages: bool,
}

impl Default for RadrootsAppConfigFlowDraft {
    fn default() -> Self {
        Self {
            step: app_config_step_default(),
            profile_name: String::new(),
            profile_location: String::new(),
            role: None,
            farmer_farm_name: String::new(),
            farmer_location: String::new(),
            farmer_products: Vec::new(),
            individual_name: String::new(),
            individual_location: String::new(),
            individual_products: Vec::new(),
            business_name: String::new(),
            business_location: String::new(),
            business_operations: String::new(),
            notifications_orders: true,
            notifications_messages: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadrootsAppConfigFlowValidation {
    pub can_continue: bool,
    pub can_back: bool,
    pub next_step: RadrootsAppConfigStep,
    pub prev_step: RadrootsAppConfigStep,
}

pub fn app_config_flow_next_step(draft: &RadrootsAppConfigFlowDraft) -> RadrootsAppConfigStep {
    match draft.step {
        RadrootsAppConfigStep::Profile => RadrootsAppConfigStep::Role,
        RadrootsAppConfigStep::Role => RadrootsAppConfigStep::Preferences,
        RadrootsAppConfigStep::Preferences => RadrootsAppConfigStep::Preferences,
    }
}

pub fn app_config_flow_prev_step(draft: &RadrootsAppConfigFlowDraft) -> RadrootsAppConfigStep {
    match draft.step {
        RadrootsAppConfigStep::Profile => RadrootsAppConfigStep::Profile,
        RadrootsAppConfigStep::Role => RadrootsAppConfigStep::Profile,
        RadrootsAppConfigStep::Preferences => RadrootsAppConfigStep::Role,
    }
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_items(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

fn normalize_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_items(values: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if out.iter().any(|item| item.eq_ignore_ascii_case(trimmed)) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

fn role_step_valid(draft: &RadrootsAppConfigFlowDraft) -> bool {
    match draft.role {
        Some(RadrootsAppRole::Farm) => {
            has_text(&draft.farmer_farm_name)
                && has_text(&draft.farmer_location)
                && has_items(&draft.farmer_products)
        }
        Some(RadrootsAppRole::Individual) => {
            has_items(&draft.individual_products)
        }
        Some(RadrootsAppRole::Business) => {
            has_text(&draft.business_name)
                && has_text(&draft.business_location)
                && has_text(&draft.business_operations)
        }
        None => false,
    }
}

pub fn app_config_flow_validate(draft: &RadrootsAppConfigFlowDraft) -> RadrootsAppConfigFlowValidation {
    let can_continue = match draft.step {
        RadrootsAppConfigStep::Profile => {
            has_text(&draft.profile_name) && has_text(&draft.profile_location)
        }
        RadrootsAppConfigStep::Role => role_step_valid(draft),
        RadrootsAppConfigStep::Preferences => true,
    };
    let can_back = !matches!(draft.step, RadrootsAppConfigStep::Profile);
    RadrootsAppConfigFlowValidation {
        can_continue,
        can_back,
        next_step: app_config_flow_next_step(draft),
        prev_step: app_config_flow_prev_step(draft),
    }
}

pub fn app_config_flow_build_config(
    draft: &RadrootsAppConfigFlowDraft,
) -> Option<RadrootsAppConfigData> {
    let profile_name = normalize_text(&draft.profile_name)?;
    let profile_location = normalize_text(&draft.profile_location)?;
    let role = draft.role?;
    let profile = RadrootsAppConfigProfile {
        name: profile_name,
        location: profile_location,
    };
    let preferences = RadrootsAppConfigPreferences {
        notifications_orders: draft.notifications_orders,
        notifications_messages: draft.notifications_messages,
        payment_method: None,
    };
    match role {
        RadrootsAppRole::Farm => {
            let farm_name = normalize_text(&draft.farmer_farm_name)?;
            let farm_location = normalize_text(&draft.farmer_location)?;
            let products = normalize_items(&draft.farmer_products);
            if products.is_empty() {
                return None;
            }
            Some(RadrootsAppConfigData {
                profile: profile.clone(),
                role,
                farmer: Some(RadrootsAppConfigFarmer {
                    farm_name,
                    farm_location,
                    products_growing: products,
                }),
                business: None,
                individual: None,
                preferences,
            })
        }
        RadrootsAppRole::Individual => {
            let products = normalize_items(&draft.individual_products);
            if products.is_empty() {
                return None;
            }
            Some(RadrootsAppConfigData {
                profile: profile.clone(),
                role,
                farmer: None,
                business: None,
                individual: Some(RadrootsAppConfigIndividual {
                    name: profile.name.clone(),
                    location: profile.location.clone(),
                    products_interested: products,
                }),
                preferences,
            })
        }
        RadrootsAppRole::Business => {
            let name = normalize_text(&draft.business_name)?;
            let location = normalize_text(&draft.business_location)?;
            let operations = normalize_text(&draft.business_operations)?;
            Some(RadrootsAppConfigData {
                profile: profile.clone(),
                role,
                farmer: None,
                business: Some(RadrootsAppConfigBusiness {
                    name,
                    location,
                    operations,
                }),
                individual: None,
                preferences,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_config_flow_build_config,
        app_config_flow_next_step,
        app_config_flow_prev_step,
        app_config_flow_validate,
        RadrootsAppConfigFlowDraft,
        RadrootsAppConfigStep,
    };
    use crate::RadrootsAppRole;

    #[test]
    fn flow_defaults_to_profile() {
        let draft = RadrootsAppConfigFlowDraft::default();
        assert_eq!(draft.step, RadrootsAppConfigStep::Profile);
        assert!(draft.notifications_orders);
        assert!(draft.notifications_messages);
    }

    #[test]
    fn flow_step_transitions() {
        let mut draft = RadrootsAppConfigFlowDraft::default();
        assert_eq!(app_config_flow_next_step(&draft), RadrootsAppConfigStep::Role);
        draft.step = RadrootsAppConfigStep::Role;
        assert_eq!(app_config_flow_next_step(&draft), RadrootsAppConfigStep::Preferences);
        draft.step = RadrootsAppConfigStep::Preferences;
        assert_eq!(app_config_flow_next_step(&draft), RadrootsAppConfigStep::Preferences);
        assert_eq!(app_config_flow_prev_step(&draft), RadrootsAppConfigStep::Role);
    }

    #[test]
    fn flow_validation_requires_profile_fields() {
        let draft = RadrootsAppConfigFlowDraft::default();
        let validation = app_config_flow_validate(&draft);
        assert!(!validation.can_continue);
    }

    #[test]
    fn flow_validation_requires_role_fields() {
        let mut draft = RadrootsAppConfigFlowDraft::default();
        draft.step = RadrootsAppConfigStep::Role;
        draft.role = Some(RadrootsAppRole::Farm);
        draft.farmer_farm_name = String::from("Radroots Farm");
        draft.farmer_location = String::from("Valley");
        let validation = app_config_flow_validate(&draft);
        assert!(!validation.can_continue);
        draft.farmer_products = vec![String::from("tomatoes")];
        let validation = app_config_flow_validate(&draft);
        assert!(validation.can_continue);
    }

    #[test]
    fn flow_build_config_requires_role() {
        let draft = RadrootsAppConfigFlowDraft::default();
        assert!(app_config_flow_build_config(&draft).is_none());
    }

    #[test]
    fn flow_build_config_maps_farm_values() {
        let mut draft = RadrootsAppConfigFlowDraft::default();
        draft.profile_name = String::from("Radroots");
        draft.profile_location = String::from("Valley");
        draft.role = Some(RadrootsAppRole::Farm);
        draft.farmer_farm_name = String::from("Willow Farm");
        draft.farmer_location = String::from("Valley");
        draft.farmer_products = vec![
            String::from("tomatoes"),
            String::from(" Tomatoes "),
            String::from(" "),
        ];
        let config = app_config_flow_build_config(&draft).expect("config");
        let farmer = config.farmer.expect("farmer");
        assert_eq!(config.profile.name, "Radroots");
        assert_eq!(config.profile.location, "Valley");
        assert_eq!(farmer.products_growing, vec![String::from("tomatoes")]);
    }
}
