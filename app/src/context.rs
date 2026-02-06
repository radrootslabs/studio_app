#![forbid(unsafe_code)]

use leptos::prelude::{use_context, LocalStorage, RwSignal};

use crate::{
    RadrootsAppBackends,
    RadrootsAppConfigStatus,
    RadrootsAppInitError,
    RadrootsAppInitState,
    RadrootsAppSetupStatus,
};

#[derive(Clone)]
pub struct RadrootsAppContext {
    pub backends: RwSignal<Option<RadrootsAppBackends>, LocalStorage>,
    pub init_error: RwSignal<Option<RadrootsAppInitError>, LocalStorage>,
    pub init_state: RwSignal<RadrootsAppInitState, LocalStorage>,
    pub setup_status: RwSignal<RadrootsAppSetupStatus, LocalStorage>,
    pub config_status: RwSignal<RadrootsAppConfigStatus, LocalStorage>,
}

pub fn app_context() -> Option<RadrootsAppContext> {
    Some(RadrootsAppContext {
        backends: use_context::<RwSignal<Option<RadrootsAppBackends>, LocalStorage>>()?,
        init_error: use_context::<RwSignal<Option<RadrootsAppInitError>, LocalStorage>>()?,
        init_state: use_context::<RwSignal<RadrootsAppInitState, LocalStorage>>()?,
        setup_status: use_context::<RwSignal<RadrootsAppSetupStatus, LocalStorage>>()?,
        config_status: use_context::<RwSignal<RadrootsAppConfigStatus, LocalStorage>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::app_context;
    use crate::{
        app_init_state_default,
        RadrootsAppBackends,
        RadrootsAppInitError,
        RadrootsAppInitStage,
        RadrootsAppConfigStatus,
        RadrootsAppSetupStatus,
    };
    use leptos::prelude::{provide_context, Owner, RwSignal, WithUntracked};

    #[test]
    fn app_context_is_none_without_providers() {
        let owner = Owner::new();
        owner.set();
        assert!(app_context().is_none());
    }

    #[test]
    fn app_context_reads_provided_signals() {
        let owner = Owner::new();
        owner.set();
        let backends = RwSignal::new_local(None::<RadrootsAppBackends>);
        let init_error = RwSignal::new_local(None::<RadrootsAppInitError>);
        let init_state = RwSignal::new_local(app_init_state_default());
        let setup_status = RwSignal::new_local(RadrootsAppSetupStatus::Unknown);
        let config_status = RwSignal::new_local(RadrootsAppConfigStatus::Unknown);
        provide_context(backends);
        provide_context(init_error);
        provide_context(init_state);
        provide_context(setup_status);
        provide_context(config_status);
        let context = app_context().expect("context");
        assert!(context.backends.with_untracked(|value| value.is_none()));
        assert!(context.init_error.with_untracked(|value| value.is_none()));
        assert_eq!(
            context.init_state.with_untracked(|state| state.stage),
            RadrootsAppInitStage::Idle
        );
        assert_eq!(
            context
                .setup_status
                .with_untracked(|value| *value),
            RadrootsAppSetupStatus::Unknown
        );
        assert_eq!(
            context
                .config_status
                .with_untracked(|value| *value),
            RadrootsAppConfigStatus::Unknown
        );
    }
}
