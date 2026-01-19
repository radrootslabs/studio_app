#![forbid(unsafe_code)]

use leptos::prelude::{use_context, LocalStorage, RwSignal};

use crate::{AppBackends, AppInitError, AppInitState};

#[derive(Clone)]
pub struct AppContext {
    pub backends: RwSignal<Option<AppBackends>, LocalStorage>,
    pub init_error: RwSignal<Option<AppInitError>, LocalStorage>,
    pub init_state: RwSignal<AppInitState, LocalStorage>,
}

pub fn app_context() -> Option<AppContext> {
    Some(AppContext {
        backends: use_context::<RwSignal<Option<AppBackends>, LocalStorage>>()?,
        init_error: use_context::<RwSignal<Option<AppInitError>, LocalStorage>>()?,
        init_state: use_context::<RwSignal<AppInitState, LocalStorage>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::app_context;
    use crate::{app_init_state_default, AppBackends, AppInitError, AppInitStage};
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
        let backends = RwSignal::new_local(None::<AppBackends>);
        let init_error = RwSignal::new_local(None::<AppInitError>);
        let init_state = RwSignal::new_local(app_init_state_default());
        provide_context(backends);
        provide_context(init_error);
        provide_context(init_state);
        let context = app_context().expect("context");
        assert!(context.backends.with_untracked(|value| value.is_none()));
        assert!(context.init_error.with_untracked(|value| value.is_none()));
        assert_eq!(
            context.init_state.with_untracked(|state| state.stage),
            AppInitStage::Idle
        );
    }
}
