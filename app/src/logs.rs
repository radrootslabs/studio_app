#![forbid(unsafe_code)]

use futures::future::{AbortHandle, Abortable};
use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use leptos::prelude::*;
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use std::rc::Rc;

use crate::{
    app_context,
    app_log_buffer_flush_no_prune,
    app_log_entries_clear,
    app_log_entries_dump,
    app_log_entries_load,
    RadrootsAppLogEntry,
    RadrootsAppLogLevel,
};

#[cfg(target_arch = "wasm32")]
use js_sys::Array;

const LOGS_AUTO_REFRESH_MS: u32 = 5000;

fn logs_auto_refresh_ms() -> u32 {
    LOGS_AUTO_REFRESH_MS
}

fn log_level_color(level: RadrootsAppLogLevel) -> &'static str {
    match level {
        RadrootsAppLogLevel::Debug => "#6b7280",
        RadrootsAppLogLevel::Info => "#0f172a",
        RadrootsAppLogLevel::Warn => "#b45309",
        RadrootsAppLogLevel::Error => "#b91c1c",
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn log_dump_filename_from_ms(timestamp_ms: i64) -> String {
    format!("radroots-logs-{timestamp_ms}.jsonl")
}

#[cfg(target_arch = "wasm32")]
fn log_dump_filename() -> String {
    log_dump_filename_from_ms(crate::app_log_timestamp_ms())
}

async fn log_dump_copy(text: String) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        return Err(String::from("copy_unavailable"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return Err(String::from("window_unavailable"));
        };
        let Some(clipboard) = window.navigator().clipboard() else {
            return Err(String::from("clipboard_unavailable"));
        };
        let promise = clipboard.write_text(&text);
        JsFuture::from(promise)
            .await
            .map_err(|_| String::from("copy_failed"))?;
        Ok(())
    }
}

async fn log_dump_download(text: String) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        return Err(String::from("download_unavailable"));
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return Err(String::from("window_unavailable"));
        };
        let Some(document) = window.document() else {
            return Err(String::from("document_unavailable"));
        };
        let parts = Array::new();
        parts.push(&JsValue::from_str(&text));
        let blob = web_sys::Blob::new_with_str_sequence(&parts)
            .map_err(|_| String::from("blob_failed"))?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| String::from("url_failed"))?;
        let anchor = document
            .create_element("a")
            .map_err(|_| String::from("anchor_failed"))?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(|_| String::from("anchor_cast_failed"))?;
        anchor.set_href(&url);
        anchor.set_download(&log_dump_filename());
        anchor.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }
}

#[component]
pub fn RadrootsAppLogsPage() -> impl IntoView {
    let entries = RwSignal::new_local(Vec::<RadrootsAppLogEntry>::new());
    let dump = RwSignal::new_local(String::new());
    let loading = RwSignal::new_local(false);
    let dump_status = RwSignal::new_local(None::<String>);
    let dump_action_running = RwSignal::new_local(false);
    let did_load = RwSignal::new_local(false);
    let interval_started = RwSignal::new_local(false);
    let context = Rc::new(app_context());
    let resolve_backends = {
        let context = Rc::clone(&context);
        Rc::new(move || {
            context.as_ref().as_ref().and_then(|context| {
                context
                    .backends
                    .with_untracked(|value| {
                        value.as_ref().map(|backends| {
                            (backends.datastore.clone(), backends.config.datastore.key_maps.clone())
                        })
                    })
            })
        })
    };
    let refresh = {
        let resolve_backends = Rc::clone(&resolve_backends);
        Rc::new(move || {
            let Some((datastore, key_maps)) = resolve_backends() else {
                entries.set(Vec::new());
                dump.set(String::new());
                return;
            };
            loading.set(true);
            let entries_signal = entries;
            let dump_signal = dump;
            let loading_signal = loading;
            spawn_local(async move {
                let _ = app_log_buffer_flush_no_prune(datastore.as_ref(), &key_maps).await;
                let result = app_log_entries_load(datastore.as_ref(), &key_maps).await;
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
        })
    };
    let clear = {
        let resolve_backends = Rc::clone(&resolve_backends);
        let refresh = Rc::clone(&refresh);
        Rc::new(move || {
            let Some((datastore, key_maps)) = resolve_backends() else {
                entries.set(Vec::new());
                dump.set(String::new());
                return;
            };
            loading.set(true);
            let refresh = Rc::clone(&refresh);
            spawn_local(async move {
                let _ = app_log_entries_clear(datastore.as_ref(), &key_maps).await;
                refresh();
            });
        })
    };
    let copy_dump = {
        let dump_signal = dump;
        Rc::new(move || {
            let text = dump_signal.get();
            if text.is_empty() {
                dump_status.set(Some(String::from("dump_empty")));
                return;
            }
            dump_action_running.set(true);
            spawn_local(async move {
                let status = match log_dump_copy(text).await {
                    Ok(()) => String::from("copy_ok"),
                    Err(err) => err,
                };
                dump_status.set(Some(status));
                dump_action_running.set(false);
            });
        })
    };
    let download_dump = {
        let dump_signal = dump;
        Rc::new(move || {
            let text = dump_signal.get();
            if text.is_empty() {
                dump_status.set(Some(String::from("dump_empty")));
                return;
            }
            dump_action_running.set(true);
            spawn_local(async move {
                let status = match log_dump_download(text).await {
                    Ok(()) => String::from("download_ok"),
                    Err(err) => err,
                };
                dump_status.set(Some(status));
                dump_action_running.set(false);
            });
        })
    };
    let refresh_effect = Rc::clone(&refresh);
    let context_effect = Rc::clone(&context);
    Effect::new(move || {
        let Some(context) = context_effect.as_ref() else {
            return;
        };
        if did_load.get() {
            return;
        }
        let has_backends = context.backends.with(|value| value.is_some());
        if !has_backends {
            return;
        }
        did_load.set(true);
        refresh_effect();
    });
    let interval_effect = Rc::clone(&refresh);
    Effect::new(move || {
        if interval_started.get() {
            return;
        }
        interval_started.set(true);
        let refresh = Rc::clone(&interval_effect);
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let abort_handle_cleanup = abort_handle.clone();
        spawn_local(async move {
            let mut ticks = IntervalStream::new(logs_auto_refresh_ms());
            let task = async move {
                while ticks.next().await.is_some() {
                    refresh();
                }
            };
            let _ = Abortable::new(task, abort_reg).await;
        });
        on_cleanup(move || abort_handle_cleanup.abort());
    });
    let status_label = move || if loading.get() { "loading" } else { "idle" };
    let dump_action_label =
        move || dump_status.get().unwrap_or_else(|| "idle".to_string());
    let dump_action_disabled = move || dump_action_running.get();
    view! {
        <main>
            <div style="display:flex;align-items:center;gap:12px;">
                <div style="font-size:18px;font-weight:600;">"logs"</div>
                <button on:click=move |_| refresh()>"refresh"</button>
                <button on:click=move |_| clear()>"clear"</button>
                <button on:click=move |_| copy_dump() disabled=dump_action_disabled>"copy dump"</button>
                <button on:click=move |_| download_dump() disabled=dump_action_disabled>"download dump"</button>
                <div style="font-size:12px;color:#6b7280;">{status_label}</div>
                <div style="font-size:12px;color:#6b7280;">{dump_action_label}</div>
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

#[cfg(test)]
mod tests {
    use super::{log_dump_filename_from_ms, logs_auto_refresh_ms};

    #[test]
    fn logs_auto_refresh_is_positive() {
        assert!(logs_auto_refresh_ms() > 0);
    }

    #[test]
    fn log_dump_filename_uses_timestamp() {
        let name = log_dump_filename_from_ms(123);
        assert_eq!(name, "radroots-logs-123.jsonl");
    }
}
