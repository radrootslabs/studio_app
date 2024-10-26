<script lang="ts">
    import { device, dialog, http, os } from "$lib/client";
    import { cfg } from "$lib/conf";
    import type { IClientDeviceMetadata } from "@radroots/client";
    import {
        app_db,
        app_geoc,
        app_loading,
        app_notify,
        app_splash,
        app_th,
        app_thc,
        AppControls,
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

    onMount(async () => {
        try {
            if (`paintWorklet` in CSS)
                (CSS as any).paintWorklet.addModule(`/squircle.min.js`);
            const metadata: IClientDeviceMetadata = {
                version: os.version(),
                platform: os.platform(),
                locale: $locale,
            };
            await device.init(metadata);
            await http.init(metadata);
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            route_render = undefined;
        } catch (e) {
        } finally {
        }
    });

    app_thc.subscribe((app_thc) => {
        const color_mode = parse_color_mode(app_thc);
        theme_set(parse_theme_key($app_th), color_mode);
        //window.status_style(color_mode);
    });

    app_th.subscribe((app_th) => {
        const color_mode = parse_color_mode($app_thc);
        theme_set(parse_theme_key(app_th), color_mode);
        //window.status_style(color_mode);
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
<AppControls />
<CssStatic />
<CssStyles />
