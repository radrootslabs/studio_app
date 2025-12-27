import { build, files, prerendered, version } from "$service-worker";
import { _env } from "$lib/_env";
import { DEFAULT_SQL_WASM_PATH } from "@radroots/client/sql/constants";
import { DEFAULT_GEOCODER_DATABASE_PATH } from "@radroots/geocoder/constants";

const APP_SHELL_URL = new URL(self.registration.scope).pathname;
const normalize_env_path = (value) =>
    typeof value === "string" && value.trim().length ? value.trim() : undefined;
const parse_env_path = (route_path) => {
    const path_end = route_path.search(/[?#]/u);
    return path_end >= 0 ? route_path.slice(0, path_end) : route_path;
};
const ensure_env_wasm_path = (value, env_name) => {
    const path = parse_env_path(value);
    const normalized = path.toLowerCase();
    if (!normalized || normalized.endsWith("/"))
        throw new Error(`${env_name} must include a .wasm filename`);
    if (!normalized.endsWith(".wasm"))
        throw new Error(`${env_name} must end with .wasm`);
    return value;
};
const ensure_env_asset_path = (value, env_name) => {
    const path = parse_env_path(value);
    if (!path || path.endsWith("/"))
        throw new Error(`${env_name} must include a file path`);
    return value;
};
const SQL_WASM_ENV = normalize_env_path(_env.SQL_WASM_URL);
const SQL_WASM_URL = SQL_WASM_ENV
    ? ensure_env_wasm_path(
        SQL_WASM_ENV,
        "VITE_PUBLIC_SQL_WASM_URL",
    )
    : DEFAULT_SQL_WASM_PATH;
const GEOCODER_DB_ENV = normalize_env_path(_env.GEOCODER_DB_URL);
const GEOCODER_DB_URL = GEOCODER_DB_ENV
    ? ensure_env_asset_path(GEOCODER_DB_ENV, "VITE_PUBLIC_GEOCODER_DB_URL")
    : DEFAULT_GEOCODER_DATABASE_PATH;
const ASSET_URLS = [...new Set([SQL_WASM_URL, GEOCODER_DB_URL])];
const PRECACHE_URLS = [...new Set([...build, ...files, ...prerendered, APP_SHELL_URL])].filter(
    (url) => !url.includes("/.")
);
const PRECACHE_LIST = PRECACHE_URLS.map((url) => ({
    url,
    revision: version
}));
const APP_CACHE = `cache-app-shell-v${version}`;
const APP_CACHE_PREFIX = "cache-app-shell-v";
const ASSET_CACHE = "cache-app-assets-v1";
const ASSET_CACHE_PREFIX = "cache-app-assets-v";

const normalize_asset_url = (url) => {
    const resolved = new URL(url, self.location.origin);
    resolved.search = "";
    resolved.hash = "";
    return resolved.href;
};
const ASSET_URL_KEYS = new Set(ASSET_URLS.map((url) => normalize_asset_url(url)));
const ASSET_URLS_ABS = ASSET_URLS.map((url) => new URL(url, self.location.origin).href);

const cache_assets = async () => {
    const cache = await caches.open(ASSET_CACHE);
    await Promise.all(
        ASSET_URLS_ABS.map(async (url) => {
            const cached = await cache.match(url);
            if (cached) return;
            try {
                const response = await fetch(url);
                if (response.ok || response.type === "opaque") await cache.put(url, response.clone());
            } catch { }
        })
    );
};

const is_asset_request = (request_url) => ASSET_URL_KEYS.has(normalize_asset_url(request_url));

const precache = async () => {
    const cache = await caches.open(APP_CACHE);
    await cache.addAll(PRECACHE_LIST.map((entry) => entry.url));
    await cache_assets();
};

const cleanup_caches = async () => {
    const keys = await caches.keys();
    for (const key of keys) {
        const is_app_cache = key.startsWith(APP_CACHE_PREFIX) && key !== APP_CACHE;
        const is_asset_cache = key.startsWith(ASSET_CACHE_PREFIX) && key !== ASSET_CACHE;
        if (!is_app_cache && !is_asset_cache) continue;
        await caches.delete(key);
    }
};

const range_response = async (request, response) => {
    const range = request.headers.get("range");
    if (!range || !response || response.type === "opaque") return response;
    const bytes = /bytes=(\d+)-(\d+)?/u.exec(range);
    if (!bytes) return response;
    const start = Number(bytes[1]);
    const end_raw = bytes[2];
    const buffer = await response.arrayBuffer();
    const end = end_raw ? Number(end_raw) : buffer.byteLength - 1;
    if (!Number.isFinite(start) || !Number.isFinite(end) || start > end) return response;
    const sliced = buffer.slice(start, end + 1);
    const headers = new Headers(response.headers);
    headers.set("Content-Range", `bytes ${start}-${end}/${buffer.byteLength}`);
    headers.set("Content-Length", `${sliced.byteLength}`);
    return new Response(sliced, { status: 206, statusText: "Partial Content", headers });
};

const cache_first = async (request, cache_name = APP_CACHE) => {
    const cache = await caches.open(cache_name);
    const cached = await cache.match(request);
    if (cached) return await range_response(request, cached);
    const response = await fetch(request);
    if (response.ok || response.type === "opaque") await cache.put(request, response.clone());
    return response;
};

const network_first = async (request, cache_name = APP_CACHE) => {
    const cache = await caches.open(cache_name);
    try {
        const response = await fetch(request);
        if (response.ok || response.type === "opaque") await cache.put(request, response.clone());
        return response;
    } catch {
        const cached = await cache.match(request);
        if (cached) return await range_response(request, cached);
        const fallback = await cache.match(APP_SHELL_URL);
        if (fallback) return fallback;
        return new Response("offline", { status: 503 });
    }
};

self.addEventListener("install", (event) => {
    event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
    event.waitUntil(cleanup_caches().then(() => self.clients.claim()));
});

self.addEventListener("fetch", (event) => {
    const request = event.request;
    if (request.method !== "GET") return;
    const url = new URL(request.url);
    if (is_asset_request(request.url)) {
        event.respondWith(cache_first(request, ASSET_CACHE));
        return;
    }
    if (request.mode === "navigate") {
        event.respondWith(network_first(request));
        return;
    }
    if (url.origin === self.location.origin) event.respondWith(cache_first(request));
});
