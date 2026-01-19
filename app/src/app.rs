use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{
    app_init_backends,
    app_init_state_default,
    app_init_mark_completed,
    app_init_reset,
    app_config_default,
    AppBackends,
    AppInitError,
    AppInitStage,
};

#[component]
pub fn App() -> impl IntoView {
    let backends = RwSignal::new_local(None::<AppBackends>);
    let init_error = RwSignal::new_local(None::<AppInitError>);
    let init_state = RwSignal::new_local(app_init_state_default());
    let reset_status = RwSignal::new_local(None::<String>);
    provide_context(backends);
    provide_context(init_error);
    provide_context(init_state);
    Effect::new(move || {
        spawn_local(async move {
            init_state.update(|state| state.stage = AppInitStage::Storage);
            match app_init_backends(app_config_default()).await {
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
    let reset_disabled = move || backends.with_untracked(|value| value.is_none());
    let reset_label = move || {
        reset_status
            .get()
            .unwrap_or_else(|| "reset_idle".to_string())
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
            <div style="margin-top: 12px; display: flex; align-items: center; gap: 8px;">
                <button
                    on:click=move |_| {
                        let config = backends.with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
                        reset_status.set(Some("resetting".to_string()));
                        spawn_local(async move {
                            let Some(config) = config else {
                                reset_status.set(Some("reset_missing_backends".to_string()));
                                return;
                            };
                            let datastore = radroots_studio_app_core::datastore::RadrootsClientWebDatastore::new(
                                Some(config.datastore.idb_config),
                            );
                            match app_init_reset(Some(&datastore), Some(&config.datastore.key_maps)).await {
                                Ok(()) => reset_status.set(Some("reset_done".to_string())),
                                Err(err) => reset_status.set(Some(err.to_string())),
                            }
                        });
                    }
                    disabled=reset_disabled
                >
                    "reset"
                </button>
                <span>{reset_label}</span>
            </div>
        </main>
    }
}
