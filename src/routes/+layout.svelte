<script lang="ts">
    import { device, dialog, http, logger, os } from "$lib/client";
    import { cfg } from "$lib/conf";
    import { kv_init_app } from "$lib/util/kv";
    import type {
        IClientDeviceMetadata,
        IClientUnlisten,
    } from "@radroots/client";
    import {
        app_db,
        app_geoc,
        app_loading,
        app_notify,
        app_splash,
        app_th,
        app_thc,
        catch_err,
        Controls,
        CssStatic,
        CssStyles,
        LayoutWindow,
        LoadingView,
        locale,
        route,
        sleep,
        SplashScreen,
        theme_set,
        type NavigationRoute,
    } from "@radroots/svelte-lib";
    import { parse_color_mode, parse_theme_key } from "@radroots/theme";
    import "css-paint-polyfill";
    import { onDestroy, onMount } from "svelte";
    import "../app.css";

    let route_render: NavigationRoute | undefined = undefined;
    let log_unlisten: IClientUnlisten | undefined = undefined;

    onMount(async () => {
        try {
            await init_app();
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            route_render = undefined;
            if (log_unlisten) log_unlisten();
        } catch (e) {
        } finally {
        }
    });

    app_thc.subscribe((_app_thc) => {
        const color_mode = parse_color_mode(_app_thc);
        theme_set(parse_theme_key($app_th), color_mode);
    });

    app_th.subscribe((_app_th) => {
        const color_mode = parse_color_mode($app_thc);
        theme_set(parse_theme_key(_app_th), color_mode);
    });

    app_db.subscribe((_app_db) => {
        if (!_app_db) return;
        console.log(`(app_db) success`);
    });

    app_geoc.subscribe((_app_geoc) => {
        if (!_app_geoc) return;
        console.log(`(app_geoc) success`);
    });

    app_notify.subscribe(async (_app_notify) => {
        if (!_app_notify) {
            return;
        }
        route(`/`);
        await sleep(cfg.delay.notify);
        dialog.alert(_app_notify);
        app_notify.set(``);
    });

    const init_app = async (): Promise<void> => {
        try {
            if (`paintWorklet` in CSS)
                (CSS as any).paintWorklet.addModule(`/squircle.min.js`);
            const metadata: IClientDeviceMetadata = {
                version: cfg.app.version,
                platform: os.platform(),
                locale: $locale,
            };
            await device.init(metadata);
            await http.init(metadata);
            log_unlisten = await logger.init();
            await kv_init_app();
        } catch (e) {
            await catch_err(e, `init_app`);
        }
    };
</script>

<svelte:head>
    <meta name="description" content={cfg.app.description} />
    <meta property="og:title" content={cfg.app.title} />
    <meta property="og:description" content={cfg.app.description} />
</svelte:head>

<LayoutWindow>
    {#if $app_splash}
        <SplashScreen />
    {:else if $app_loading}
        <LoadingView />
    {/if}
    <slot />
</LayoutWindow>
<Controls />
<CssStatic />
<CssStyles />
<div
    class="hidden h-entry_guide h-entry_line h-[calc(100vh-12%)] h-trellis_centered_mobile_base h-trellis_centered_mobile_y bg-white/40 backdrop-blur-lg backdrop-blur-sm"
/>
