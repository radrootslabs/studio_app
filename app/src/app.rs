use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{
    app_init_backends,
    app_init_state_default,
    app_init_mark_completed,
    AppBackends,
    AppInitError,
    AppInitStage,
};

#[component]
pub fn App() -> impl IntoView {
    let backends = RwSignal::new_local(None::<AppBackends>);
    let init_error = RwSignal::new_local(None::<AppInitError>);
    let init_state = RwSignal::new_local(app_init_state_default());
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    Effect::new(move || {
        spawn_local(async move {
            init_state.update(|state| state.stage = AppInitStage::Storage);
            match app_init_backends().await {
                Ok(value) => {
                    backends.set(Some(value));
                    app_init_mark_completed();
                    init_state.update(|state| state.stage = AppInitStage::Ready);
                }
                Err(err) => {
                    init_error.set(Some(err));
                    init_state.update(|state| state.stage = AppInitStage::Error);
                }
            }
        })
    });
    let status_color = move || match init_state.get().stage {
        AppInitStage::Ready => "green",
        AppInitStage::Error => "red",
        AppInitStage::Storage => "orange",
        AppInitStage::DownloadSql => "orange",
        AppInitStage::DownloadGeo => "orange",
        AppInitStage::Database => "orange",
        AppInitStage::Geocoder => "orange",
        AppInitStage::Idle => "gray",
    };
    view! {
        <main>
            <div>"app"</div>
            <div style="margin-top: 8px; display: flex; align-items: center; gap: 8px;">
                <span
                    style=move || format!(
                        "display:inline-block;width:10px;height:10px;border-radius:50%;background:{};",
                        status_color()
                    )
                ></span>
                <span>{move || init_state.get().stage.as_str()}</span>
            </div>
        </main>
    }
}
