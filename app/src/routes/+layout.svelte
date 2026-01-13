<script lang="ts">
    import { dev, version as kit_version } from "$app/environment";
    import { resolve } from "$app/paths";
    import { page } from "$app/state";
    import { app_init } from "$lib/utils/app";
    import { app_cfg } from "$lib/utils/app/config";
    import {
        lc_color_mode,
        lc_geocode,
        lc_geop_current,
        lc_gui_alert,
        lc_gui_confirm,
        lc_img_bin,
        lc_photos_add,
        lc_photos_upload,
    } from "$lib/utils/app/handlers";
    import { locale, ls } from "$lib/utils/i18n";
    import {
        set_context,
        theme_key,
        theme_mode,
        theme_set,
        win_h,
    } from "@radroots/apps-lib";
    import { Css, LayoutWindow } from "@radroots/apps-lib-pwa";
    import { app_lo } from "@radroots/apps-lib-pwa/stores/app";
    import type { LibContext } from "@radroots/apps-lib-pwa/types/context";
    import { CFG_APP } from "@radroots/apps-lib-pwa/utils/app";
    import { parse_theme_key, parse_theme_mode } from "@radroots/themes";
    import { RADROOTS_ASSET_CACHE_NAME, str_cap_words } from "@radroots/utils";
    import "css-paint-polyfill";
    import { onMount } from "svelte";
    import "../app.css";
    import type { LayoutProps } from "./$types";

    type MetaTag = {
        name: string;
        content: string;
    };


    const HEAD_META_TAGS: MetaTag[] = [
        {
            name: "app_version",
            content: app_cfg.version,
        },
        {
            name: "app_backup_version",
            content: app_cfg.backup.version,
        },
        {
            name: "app_build_id",
            content: kit_version,
        },
        {
            name: "app_build_mode",
            content: dev ? "development" : "production",
        },
    ];

    let { children }: LayoutProps = $props();

    const LIB_CONTEXT: LibContext = {
        ls,
        locale,
        lc_color_mode,
        lc_gui_alert,
        lc_gui_confirm,
        lc_geocode,
        lc_geop_current,
        lc_img_bin,
        lc_photos_add,
        lc_photos_upload,
    };

    set_context("lib", LIB_CONTEXT);

    theme_mode.subscribe((_theme_mode) =>
        theme_set(parse_theme_key($theme_key), parse_theme_mode(_theme_mode)),
    );

    theme_key.subscribe((_theme_key) =>
        theme_set(parse_theme_key(_theme_key), parse_theme_mode($theme_mode)),
    );

    win_h.subscribe((_win_h) => {
        if (_win_h > CFG_APP.layout.ios1.h) {
            app_lo.set("ios1");
        } else {
            app_lo.set("ios0");
        }
    });

    const register_service_worker = async (): Promise<void> => {
        if (dev) return;
        if (!("serviceWorker" in navigator)) return;
        const service_worker_root = resolve("/");
        const service_worker_path = service_worker_root.endsWith("/")
            ? `${service_worker_root}service-worker.js`
            : `${service_worker_root}/service-worker.js`;
        try {
            await navigator.serviceWorker.register(service_worker_path);
            await navigator.serviceWorker.ready;
        } catch {
            return;
        }
    };

    const unregister_service_workers = async (): Promise<void> => {
        if (!("serviceWorker" in navigator)) return;
        const registrations = await navigator.serviceWorker.getRegistrations();
        await Promise.all(registrations.map((registration) => registration.unregister()));
        if (!("caches" in globalThis)) return;
        const cache_names = await caches.keys();
        const stale_cache_names = cache_names.filter((name) => name !== RADROOTS_ASSET_CACHE_NAME);
        await Promise.all(stale_cache_names.map((name) => caches.delete(name)));
    };

    onMount(async () => {
        if (dev) await unregister_service_workers();
        await app_init();
        await register_service_worker();
    });

    const format_title = (title: string): string => {
        return str_cap_words(title.replaceAll(`/`, ` `));
    };

    const head_title = $derived(format_title(page.url.pathname));
</script>

<svelte:head>
    <title>{`${head_title || "Home"} | Rad Roots`}</title>
    {#each HEAD_META_TAGS as meta_tag (meta_tag.name)}
        <meta name={meta_tag.name} content={meta_tag.content} />
    {/each}
</svelte:head>

<LayoutWindow>
    {@render children()}
</LayoutWindow>
<Css />
