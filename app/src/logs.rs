#![forbid(unsafe_code)]

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::rc::Rc;

use radroots_studio_app_core::datastore::RadrootsClientWebDatastore;

use crate::{
    app_context,
    app_log_entries_dump,
    app_log_entries_load,
    AppLogEntry,
    AppLogLevel,
};

fn log_level_color(level: AppLogLevel) -> &'static str {
    match level {
        AppLogLevel::Debug => "#6b7280",
        AppLogLevel::Info => "#0f172a",
        AppLogLevel::Warn => "#b45309",
        AppLogLevel::Error => "#b91c1c",
    }
}

#[component]
pub fn LogsPage() -> impl IntoView {
    let entries = RwSignal::new_local(Vec::<AppLogEntry>::new());
    let dump = RwSignal::new_local(String::new());
    let loading = RwSignal::new_local(false);
    let did_load = RwSignal::new_local(false);
    let context = app_context();
    let refresh = Rc::new(move || {
        let Some(context) = context.clone() else {
            entries.set(Vec::new());
            dump.set(String::new());
            return;
        };
        let config = context
            .backends
            .with_untracked(|value| value.as_ref().map(|backends| backends.config.clone()));
        let Some(config) = config else {
            entries.set(Vec::new());
            dump.set(String::new());
            return;
        };
        loading.set(true);
        let entries_signal = entries;
        let dump_signal = dump;
        let loading_signal = loading;
        spawn_local(async move {
            let datastore = RadrootsClientWebDatastore::new(Some(config.datastore.idb_config));
            let result = app_log_entries_load(&datastore, &config.datastore.key_maps).await;
            match result {
                Ok(mut items) => {
                    items.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
                    dump_signal.set(app_log_entries_dump(&items));
                    entries_signal.set(items);
                }
                Err(err) => {
                    dump_signal.set(format!("error: {err}"));
                    entries_signal.set(Vec::new());
                }
            }
            loading_signal.set(false);
        });
    });
    let refresh_effect = Rc::clone(&refresh);
    Effect::new(move || {
        if did_load.get() {
            return;
        }
        did_load.set(true);
        refresh_effect();
    });
    let status_label = move || if loading.get() { "loading" } else { "idle" };
    view! {
        <main>
            <div style="display:flex;align-items:center;gap:12px;">
                <div style="font-size:18px;font-weight:600;">"logs"</div>
                <button on:click=move |_| refresh()>"refresh"</button>
                <div style="font-size:12px;color:#6b7280;">{status_label}</div>
            </div>
            <div style="margin-top:12px;display:flex;flex-wrap:wrap;gap:16px;">
                <section style="flex:1 1 520px;min-width:280px;">
                    <div style="font-weight:600;font-size:14px;">"entries"</div>
                    <div style="margin-top:8px;border:1px solid #e5e7eb;border-radius:8px;height:60vh;overflow:auto;padding:10px;display:flex;flex-direction:column;gap:10px;">
                        <For
                            each=move || entries.get()
                            key=|entry| entry.id.clone()
                            children=move |entry| {
                                let level = entry.level;
                                let timestamp_ms = entry.timestamp_ms;
                                let code = entry.code;
                                let message = entry.message;
                                let context = entry.context;
                                view! {
                                    <div style="display:flex;flex-direction:column;gap:4px;">
                                        <div style="display:flex;align-items:baseline;gap:8px;">
                                            <span style="font-size:11px;color:#6b7280;">
                                                {timestamp_ms}
                                            </span>
                                            <span
                                                style=move || format!(
                                                    "font-size:11px;font-weight:600;color:{};",
                                                    log_level_color(level)
                                                )
                                            >
                                                {level.as_str()}
                                            </span>
                                            <span style="font-size:12px;font-weight:600;color:#111827;">
                                                {code}
                                            </span>
                                        </div>
                                        <div style="font-size:13px;color:#111827;">
                                            {message}
                                        </div>
                                        {context.map(|context| {
                                            view! {
                                                <div style="font-size:12px;color:#6b7280;">
                                                    {context}
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            }
                        />
                    </div>
                </section>
                <section style="flex:1 1 320px;min-width:260px;">
                    <div style="font-weight:600;font-size:14px;">"dump (jsonl)"</div>
                    <textarea
                        readonly
                        prop:value=move || dump.get()
                        style="margin-top:8px;width:100%;height:60vh;border:1px solid #e5e7eb;border-radius:8px;padding:8px;font-size:12px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace;"
                    ></textarea>
                </section>
            </div>
        </main>
    }
}
