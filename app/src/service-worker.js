import { build, files, prerendered, version } from "$service-worker";

const APP_SHELL_URL = "/index.html";
const PRECACHE_URLS = [...new Set([...build, ...files, ...prerendered, APP_SHELL_URL])].filter(
    (url) => !url.includes("/.")
);
const PRECACHE_LIST = PRECACHE_URLS.map((url) => ({
    url,
    revision: version
}));
const APP_CACHE = `cache-app-shell-v${version}`;
const APP_CACHE_PREFIX = "cache-app-shell-v";

const precache = async () => {
    const cache = await caches.open(APP_CACHE);
    await cache.addAll(PRECACHE_LIST.map((entry) => entry.url));
};

const cleanup_caches = async () => {
    const keys = await caches.keys();
    for (const key of keys) {
        if (!key.startsWith(APP_CACHE_PREFIX)) continue;
        if (key === APP_CACHE) continue;
        await caches.delete(key);
    }
};

const range_response = async (request, response) => {
    const range = request.headers.get("range");
    if (!range || !response) return response;
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

const cache_first = async (request) => {
    const cache = await caches.open(APP_CACHE);
    const cached = await cache.match(request);
    if (cached) return await range_response(request, cached);
    const response = await fetch(request);
    if (response.ok) await cache.put(request, response.clone());
    return response;
};

const network_first = async (request) => {
    const cache = await caches.open(APP_CACHE);
    try {
        const response = await fetch(request);
        if (response.ok) await cache.put(request, response.clone());
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
    if (request.mode === "navigate") {
        event.respondWith(network_first(request));
        return;
    }
    if (url.origin === self.location.origin) event.respondWith(cache_first(request));
});
