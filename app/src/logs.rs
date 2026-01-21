#![forbid(unsafe_code)]

use futures::future::{AbortHandle, Abortable};
use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use leptos::prelude::*;
use serde::Serialize;
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
    app_log_metadata,
    RadrootsAppLogEntry,
    RadrootsAppLogLevel,
};

#[cfg(target_arch = "wasm32")]
use js_sys::Array;

const LOGS_AUTO_REFRESH_MS: u32 = 5000;
const LOGS_MAX_VISIBLE: usize = 500;
const LOGS_PAGE_SIZE: usize = 100;

fn logs_auto_refresh_ms() -> u32 {
    LOGS_AUTO_REFRESH_MS
}

fn logs_max_visible() -> usize {
    LOGS_MAX_VISIBLE
}

fn logs_page_size_default() -> usize {
    LOGS_PAGE_SIZE
}

fn log_level_color(level: RadrootsAppLogLevel) -> &'static str {
    match level {
        RadrootsAppLogLevel::Debug => "#6b7280",
        RadrootsAppLogLevel::Info => "#0f172a",
        RadrootsAppLogLevel::Warn => "#b45309",
        RadrootsAppLogLevel::Error => "#b91c1c",
    }
}

fn log_level_matches(level: RadrootsAppLogLevel, filter: &str) -> bool {
    if filter.is_empty() || filter == "all" {
        return true;
    }
    level.as_str() == filter
}

fn log_query_matches(entry: &RadrootsAppLogEntry, query: &str) -> bool {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }
    let needle = trimmed.to_lowercase();
    if entry.code.to_lowercase().contains(&needle) {
        return true;
    }
    if entry.message.to_lowercase().contains(&needle) {
        return true;
    }
    if let Some(context) = entry.context.as_ref() {
        return context.to_lowercase().contains(&needle);
    }
    false
}

fn parse_log_timestamp(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

fn log_timestamp_matches(timestamp_ms: i64, from_ms: Option<i64>, to_ms: Option<i64>) -> bool {
    if let Some(from_ms) = from_ms {
        if timestamp_ms < from_ms {
            return false;
        }
    }
    if let Some(to_ms) = to_ms {
        if timestamp_ms > to_ms {
            return false;
        }
    }
    true
}

fn log_entry_matches(
    entry: &RadrootsAppLogEntry,
    level_filter: &str,
    query: &str,
    from_ms: Option<i64>,
    to_ms: Option<i64>,
) -> bool {
    log_level_matches(entry.level, level_filter)
        && log_query_matches(entry, query)
        && log_timestamp_matches(entry.timestamp_ms, from_ms, to_ms)
}

fn log_page_count(total: usize, page_size: usize) -> usize {
    if page_size == 0 {
        return 0;
    }
    (total + page_size - 1) / page_size
}

fn log_entries_page(
    entries: &[RadrootsAppLogEntry],
    page_index: usize,
    page_size: usize,
) -> Vec<RadrootsAppLogEntry> {
    if page_size == 0 {
        return Vec::new();
    }
    let start = page_index.saturating_mul(page_size);
    if start >= entries.len() {
        return Vec::new();
    }
    let end = (start + page_size).min(entries.len());
    entries[start..end].to_vec()
}

fn log_page_index_clamp(page_index: usize, total_pages: usize) -> usize {
    if total_pages == 0 {
        return 0;
    }
    if page_index >= total_pages {
        return total_pages - 1;
    }
    page_index
}

fn log_dump_config_from_app(config: &crate::RadrootsAppConfig) -> RadrootsAppLogDumpConfig {
    RadrootsAppLogDumpConfig {
        datastore_database: config.datastore.idb_config.database.to_string(),
        datastore_store: config.datastore.idb_config.store.to_string(),
        keystore_nostr_database: config.keystore.nostr_store.database.to_string(),
        keystore_nostr_store: config.keystore.nostr_store.store.to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
struct RadrootsAppLogDumpConfig {
    datastore_database: String,
    datastore_store: String,
    keystore_nostr_database: String,
    keystore_nostr_store: String,
}

#[derive(Debug, Clone, Serialize)]
struct RadrootsAppLogDumpFilters {
    level: String,
    query: String,
    from_ms: Option<i64>,
    to_ms: Option<i64>,
    page_size: usize,
    page_index: usize,
    limit: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RadrootsAppLogDumpStats {
    total_entries: usize,
    filtered_entries: usize,
    visible_entries: usize,
}

#[derive(Debug, Clone, Serialize)]
struct RadrootsAppLogDumpContext {
    kind: String,
    generated_at_ms: i64,
    metadata: crate::RadrootsAppLogMetadata,
    config: Option<RadrootsAppLogDumpConfig>,
    filters: RadrootsAppLogDumpFilters,
    stats: RadrootsAppLogDumpStats,
}

fn log_dump_header_with_context(context: RadrootsAppLogDumpContext) -> String {
    serde_json::to_string(&context)
        .unwrap_or_else(|_| String::from("{\"error\":\"log_dump_header_failed\"}"))
}

fn log_dump_with_context(entries: &[RadrootsAppLogEntry], header: String) -> String {
    if entries.is_empty() {
        return String::new();
    }
    format!("{header}\n{}", app_log_entries_dump(entries))
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
        let clipboard = window.navigator().clipboard();
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
    let dump_error = RwSignal::new_local(None::<String>);
    let loading = RwSignal::new_local(false);
    let dump_status = RwSignal::new_local(None::<String>);
    let dump_action_running = RwSignal::new_local(false);
    let did_load = RwSignal::new_local(false);
    let interval_started = RwSignal::new_local(false);
    let filter_query = RwSignal::new_local(String::new());
    let filter_level = RwSignal::new_local(String::from("all"));
    let filter_from = RwSignal::new_local(String::new());
    let filter_to = RwSignal::new_local(String::new());
    let filter_limit = RwSignal::new_local(logs_max_visible());
    let page_size = RwSignal::new_local(logs_page_size_default());
    let page_index = RwSignal::new_local(0usize);
    let dump_config = RwSignal::new_local(None::<RadrootsAppLogDumpConfig>);
    let context = Rc::new(app_context());
    let filtered_entries = Memo::new(move |_| {
        let level_filter = filter_level.get();
        let query = filter_query.get();
        let from_ms = parse_log_timestamp(&filter_from.get());
        let to_ms = parse_log_timestamp(&filter_to.get());
        let limit = filter_limit.get();
        entries.with(|items| {
            items
                .iter()
                .filter(|entry| log_entry_matches(entry, &level_filter, &query, from_ms, to_ms))
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let paged_entries = Memo::new(move |_| {
        let items = filtered_entries.get();
        log_entries_page(&items, page_index.get(), page_size.get())
    });
    let page_total = Memo::new(move |_| {
        log_page_count(filtered_entries.get().len(), page_size.get())
    });
    Effect::new(move || {
        let _ = filter_query.get();
        let _ = filter_level.get();
        let _ = filter_from.get();
        let _ = filter_to.get();
        page_index.set(0);
    });
    Effect::new(move || {
        let total_pages = page_total.get();
        let next = log_page_index_clamp(page_index.get(), total_pages);
        if next != page_index.get() {
            page_index.set(next);
        }
    });
    let dump_text = Memo::new(move |_| {
        if let Some(err) = dump_error.get() {
            return err;
        }
        let items = filtered_entries.get();
        let total_entries = entries.get().len();
        let filtered_len = filtered_entries.get().len();
        let visible_len = paged_entries.get().len();
        let filters = RadrootsAppLogDumpFilters {
            level: filter_level.get(),
            query: filter_query.get(),
            from_ms: parse_log_timestamp(&filter_from.get()),
            to_ms: parse_log_timestamp(&filter_to.get()),
            page_size: page_size.get(),
            page_index: page_index.get(),
            limit: filter_limit.get(),
        };
        let stats = RadrootsAppLogDumpStats {
            total_entries,
            filtered_entries: filtered_len,
            visible_entries: visible_len,
        };
        let context = RadrootsAppLogDumpContext {
            kind: String::from("radroots_log_dump"),
            generated_at_ms: crate::app_log_timestamp_ms(),
            metadata: app_log_metadata().clone(),
            config: dump_config.get(),
            filters,
            stats,
        };
        let header = log_dump_header_with_context(context);
        log_dump_with_context(&items, header)
    });
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
                dump_error.set(None);
                return;
            };
            loading.set(true);
            let entries_signal = entries;
            let loading_signal = loading;
            spawn_local(async move {
                let _ = app_log_buffer_flush_no_prune(datastore.as_ref(), &key_maps).await;
                let result = app_log_entries_load(datastore.as_ref(), &key_maps).await;
                match result {
                    Ok(mut items) => {
                        items.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
                        dump_error.set(None);
                        entries_signal.set(items);
                    }
                    Err(err) => {
                        dump_error.set(Some(format!("error: {err}")));
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
                dump_error.set(None);
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
        let dump_text = dump_text.clone();
        Rc::new(move || {
            let text = dump_text.get();
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
        let dump_text = dump_text.clone();
        Rc::new(move || {
            let text = dump_text.get();
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
    let config_effect = Rc::clone(&context);
    Effect::new(move || {
        let Some(context) = config_effect.as_ref() else {
            return;
        };
        let config = context
            .backends
            .with(|value| value.as_ref().map(|backends| backends.config.clone()));
        let Some(config) = config else {
            return;
        };
        dump_config.set(Some(log_dump_config_from_app(&config)));
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
    let prev_disabled = move || page_index.get() == 0;
    let next_disabled = move || {
        let total = page_total.get();
        total == 0 || page_index.get() + 1 >= total
    };
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
            <div style="margin-top:12px;display:flex;flex-wrap:wrap;gap:12px;align-items:center;">
                <input
                    type="text"
                    placeholder="search code/message/context"
                    prop:value=move || filter_query.get()
                    on:input=move |ev| {
                        filter_query.set(event_target_value(&ev));
                    }
                    style="flex:1 1 260px;border:1px solid #e5e7eb;border-radius:8px;padding:6px 8px;font-size:12px;"
                />
                <select
                    prop:value=move || filter_level.get()
                    on:change=move |ev| {
                        filter_level.set(event_target_value(&ev));
                    }
                    style="border:1px solid #e5e7eb;border-radius:8px;padding:6px 8px;font-size:12px;"
                >
                    <option value="all">"all"</option>
                    <option value="debug">"debug"</option>
                    <option value="info">"info"</option>
                    <option value="warn">"warn"</option>
                    <option value="error">"error"</option>
                </select>
                <input
                    type="number"
                    placeholder="from ms"
                    prop:value=move || filter_from.get()
                    on:input=move |ev| {
                        filter_from.set(event_target_value(&ev));
                    }
                    style="width:130px;border:1px solid #e5e7eb;border-radius:8px;padding:6px 8px;font-size:12px;"
                />
                <input
                    type="number"
                    placeholder="to ms"
                    prop:value=move || filter_to.get()
                    on:input=move |ev| {
                        filter_to.set(event_target_value(&ev));
                    }
                    style="width:130px;border:1px solid #e5e7eb;border-radius:8px;padding:6px 8px;font-size:12px;"
                />
                <select
                    prop:value=move || page_size.get().to_string()
                    on:change=move |ev| {
                        if let Ok(size) = event_target_value(&ev).parse::<usize>() {
                            page_size.set(size);
                            page_index.set(0);
                        }
                    }
                    style="border:1px solid #e5e7eb;border-radius:8px;padding:6px 8px;font-size:12px;"
                >
                    <option value="25">"25"</option>
                    <option value="50">"50"</option>
                    <option value="100">"100"</option>
                    <option value="250">"250"</option>
                </select>
                <div style="font-size:12px;color:#6b7280;">
                    {move || {
                        let total = entries.get().len();
                        let visible = filtered_entries.get().len();
                        let limit = filter_limit.get();
                        let pages = page_total.get();
                        let page = if pages == 0 { 0 } else { page_index.get() + 1 };
                        format!("showing {visible} of {total} (limit {limit}) page {page}/{pages}")
                    }}
                </div>
            </div>
            <div style="margin-top:8px;display:flex;align-items:center;gap:8px;">
                <button
                    on:click=move |_| {
                        let next = page_index.get().saturating_sub(1);
                        page_index.set(next);
                    }
                    disabled=prev_disabled
                >
                    "prev"
                </button>
                <button
                    on:click=move |_| {
                        let next = page_index.get() + 1;
                        page_index.set(next);
                    }
                    disabled=next_disabled
                >
                    "next"
                </button>
            </div>
            <div style="margin-top:12px;display:flex;flex-wrap:wrap;gap:16px;">
                <section style="flex:1 1 520px;min-width:280px;">
                    <div style="font-weight:600;font-size:14px;">"entries"</div>
                    <div style="margin-top:8px;border:1px solid #e5e7eb;border-radius:8px;height:60vh;overflow:auto;padding:10px;display:flex;flex-direction:column;gap:10px;">
                        <For
                            each=move || paged_entries.get()
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
                        prop:value=move || dump_text.get()
                        style="margin-top:8px;width:100%;height:60vh;border:1px solid #e5e7eb;border-radius:8px;padding:8px;font-size:12px;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace;"
                    ></textarea>
                </section>
            </div>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        log_dump_filename_from_ms,
        log_dump_header_with_context,
        log_dump_with_context,
        log_entry_matches,
        log_entries_page,
        RadrootsAppLogDumpContext,
        RadrootsAppLogDumpFilters,
        RadrootsAppLogDumpStats,
        log_page_index_clamp,
        log_page_count,
        log_timestamp_matches,
        parse_log_timestamp,
        logs_auto_refresh_ms,
        logs_max_visible,
        logs_page_size_default,
    };
    use crate::{RadrootsAppLogEntry, RadrootsAppLogLevel, RadrootsAppLogMetadata};

    #[test]
    fn logs_auto_refresh_is_positive() {
        assert!(logs_auto_refresh_ms() > 0);
    }

    #[test]
    fn log_dump_filename_uses_timestamp() {
        let name = log_dump_filename_from_ms(123);
        assert_eq!(name, "radroots-logs-123.jsonl");
    }

    #[test]
    fn logs_max_visible_is_positive() {
        assert!(logs_max_visible() > 0);
    }

    #[test]
    fn logs_page_size_default_is_positive() {
        assert!(logs_page_size_default() > 0);
    }

    #[test]
    fn log_entry_matches_filters_level_and_query() {
        let entry = RadrootsAppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: RadrootsAppLogLevel::Info,
            code: String::from("log.code.test"),
            message: String::from("Hello World"),
            context: Some(String::from("context")),
            metadata: RadrootsAppLogMetadata::default(),
        };
        assert!(log_entry_matches(&entry, "info", "hello", None, None));
        assert!(!log_entry_matches(&entry, "error", "hello", None, None));
        assert!(!log_entry_matches(&entry, "info", "missing", None, None));
    }

    #[test]
    fn log_dump_with_context_prefixes_dump() {
        let entry = RadrootsAppLogEntry {
            id: String::from("a"),
            timestamp_ms: 1,
            level: RadrootsAppLogLevel::Info,
            code: String::from("log.code.test"),
            message: String::from("Hello"),
            context: None,
            metadata: RadrootsAppLogMetadata::default(),
        };
        let context = RadrootsAppLogDumpContext {
            kind: String::from("radroots_log_dump"),
            generated_at_ms: 1,
            metadata: RadrootsAppLogMetadata::default(),
            config: None,
            filters: RadrootsAppLogDumpFilters {
                level: String::from("all"),
                query: String::new(),
                from_ms: None,
                to_ms: None,
                page_size: 100,
                page_index: 0,
                limit: 500,
            },
            stats: RadrootsAppLogDumpStats {
                total_entries: 1,
                filtered_entries: 1,
                visible_entries: 1,
            },
        };
        let header = log_dump_header_with_context(context);
        let dump = log_dump_with_context(&[entry], header);
        let mut lines = dump.lines();
        let header = lines.next().expect("header");
        assert!(header.contains("radroots_log_dump"));
        let entry_line = lines.next().expect("entry");
        assert!(entry_line.contains("\"log.code.test\""));
    }

    #[test]
    fn parse_log_timestamp_accepts_integers() {
        assert_eq!(parse_log_timestamp("123"), Some(123));
        assert_eq!(parse_log_timestamp(""), None);
        assert_eq!(parse_log_timestamp("abc"), None);
    }

    #[test]
    fn log_timestamp_matches_respects_bounds() {
        assert!(log_timestamp_matches(100, None, None));
        assert!(log_timestamp_matches(100, Some(50), None));
        assert!(log_timestamp_matches(100, None, Some(150)));
        assert!(!log_timestamp_matches(100, Some(120), None));
        assert!(!log_timestamp_matches(100, None, Some(80)));
    }

    #[test]
    fn log_page_count_rounds_up() {
        assert_eq!(log_page_count(0, 10), 0);
        assert_eq!(log_page_count(1, 10), 1);
        assert_eq!(log_page_count(11, 10), 2);
    }

    #[test]
    fn log_entries_page_slices() {
        let entries = (0..5)
            .map(|idx| RadrootsAppLogEntry {
                id: format!("id-{idx}"),
                timestamp_ms: idx,
                level: RadrootsAppLogLevel::Info,
                code: String::from("log.code.test"),
                message: String::from("Hello"),
                context: None,
                metadata: RadrootsAppLogMetadata::default(),
            })
            .collect::<Vec<_>>();
        let page = log_entries_page(&entries, 1, 2);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].id, "id-2");
    }

    #[test]
    fn log_page_index_clamp_bounds() {
        assert_eq!(log_page_index_clamp(0, 0), 0);
        assert_eq!(log_page_index_clamp(3, 2), 1);
        assert_eq!(log_page_index_clamp(1, 3), 1);
    }
}
