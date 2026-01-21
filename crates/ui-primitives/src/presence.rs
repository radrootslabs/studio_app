use leptos::ev::{AnimationEvent, TransitionEvent};
use leptos::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadrootsAppUiPresenceState {
    Mounted,
    Exiting,
    Unmounted,
}

pub fn radroots_studio_app_ui_presence_state_next(
    current: RadrootsAppUiPresenceState,
    present: bool,
    exit_complete: bool,
) -> RadrootsAppUiPresenceState {
    match (current, present, exit_complete) {
        (_, true, _) => RadrootsAppUiPresenceState::Mounted,
        (RadrootsAppUiPresenceState::Mounted, false, true) => {
            RadrootsAppUiPresenceState::Unmounted
        }
        (RadrootsAppUiPresenceState::Mounted, false, false) => {
            RadrootsAppUiPresenceState::Exiting
        }
        (RadrootsAppUiPresenceState::Exiting, false, true) => {
            RadrootsAppUiPresenceState::Unmounted
        }
        (RadrootsAppUiPresenceState::Exiting, false, false) => {
            RadrootsAppUiPresenceState::Exiting
        }
        (RadrootsAppUiPresenceState::Unmounted, false, _) => {
            RadrootsAppUiPresenceState::Unmounted
        }
    }
}

#[component]
pub fn RadrootsAppUiPresence(
    #[prop(into)] present: Signal<bool>,
    #[prop(optional)] on_exit_complete: Option<Callback<()>>,
    children: ChildrenFn,
) -> impl IntoView {
    let state = RwSignal::new(if present.get() {
        RadrootsAppUiPresenceState::Mounted
    } else {
        RadrootsAppUiPresenceState::Unmounted
    });

    Effect::new(move || {
        let next = radroots_studio_app_ui_presence_state_next(state.get(), present.get(), false);
        if next != state.get() {
            state.set(next);
        }
    });

    let on_exit_complete = on_exit_complete.clone();
    let end_handler = Arc::new(move || {
        let next = radroots_studio_app_ui_presence_state_next(state.get(), present.get(), true);
        if next != state.get() {
            state.set(next);
            if next == RadrootsAppUiPresenceState::Unmounted {
                if let Some(callback) = on_exit_complete.as_ref() {
                    callback.run(());
                }
            }
        }
    });

    let render = move || -> AnyView {
        if state.get() == RadrootsAppUiPresenceState::Unmounted {
            ().into_any()
        } else {
            let transition_end = {
                let end_handler = Arc::clone(&end_handler);
                move |_event: TransitionEvent| {
                    end_handler();
                }
            };
            let animation_end = {
                let end_handler = Arc::clone(&end_handler);
                move |_event: AnimationEvent| {
                    end_handler();
                }
            };
            view! {
                <div
                    data-state=move || if present.get() { "open" } else { "closed" }
                    on:transitionend=transition_end
                    on:animationend=animation_end
                >
                    {children()}
                </div>
            }
            .into_any()
        }
    };

    view! { {render} }
}

#[cfg(test)]
mod tests {
    use super::{
        radroots_studio_app_ui_presence_state_next,
        RadrootsAppUiPresenceState,
    };

    #[test]
    fn presence_state_moves_to_exiting_on_close() {
        let next = radroots_studio_app_ui_presence_state_next(
            RadrootsAppUiPresenceState::Mounted,
            false,
            false,
        );
        assert_eq!(next, RadrootsAppUiPresenceState::Exiting);
    }

    #[test]
    fn presence_state_unmounts_after_exit() {
        let next = radroots_studio_app_ui_presence_state_next(
            RadrootsAppUiPresenceState::Exiting,
            false,
            true,
        );
        assert_eq!(next, RadrootsAppUiPresenceState::Unmounted);
    }

    #[test]
    fn presence_state_mounts_when_present() {
        let next = radroots_studio_app_ui_presence_state_next(
            RadrootsAppUiPresenceState::Unmounted,
            true,
            false,
        );
        assert_eq!(next, RadrootsAppUiPresenceState::Mounted);
    }
}
