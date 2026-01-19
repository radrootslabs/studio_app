use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{app_init_backends, AppBackends, AppInitError};

#[component]
pub fn App() -> impl IntoView {
    let backends = RwSignal::new_local(None::<AppBackends>);
    let init_error = RwSignal::new_local(None::<AppInitError>);
    provide_context(backends);
    provide_context(init_error);
    Effect::new(move || {
        spawn_local(async move {
            match app_init_backends().await {
                Ok(value) => backends.set(Some(value)),
                Err(err) => init_error.set(Some(err)),
            }
        })
    });
    view! {
        <main>"app"</main>
    }
}
