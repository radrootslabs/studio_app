<script lang="ts">
    import { goto } from "$app/navigation";
    import { dialog, keystore, notification } from "$lib/client";
    import { cfg, ks } from "$lib/conf";
    import {
        app_config,
        app_db,
        app_geoc,
        app_loading,
        app_nostr_key,
        app_notify,
        app_th,
        app_thc,
        AppControls,
        CssStatic,
        CssStyles,
        LayoutWindow,
        LoadingView,
        route,
        sleep,
        theme_set,
        type NavigationRoute,
    } from "@radroots/svelte-lib";
    import { parse_color_mode, parse_theme_key } from "@radroots/theme";
    import { onMount } from "svelte";
    import "../app.css";

    let route_render: NavigationRoute | undefined = undefined;

    onMount(async () => {
        try {
            app_loading.set(true);
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

    app_config.subscribe(async (_app_config) => {
        try {
            if (!_app_config) {
                console.log(`(app_config) done`);
                return;
            }
            console.log(`(app_config) start`);

            await keystore.init();
            await notification.init();

            const ks_public_key = await keystore.get(ks.nostr.nostr_key_active);
            if (`result` in ks_public_key) {
                const ks_secret_key = await keystore.get(
                    ks.nostr.nostr_key(ks_public_key.result),
                );
                if (`result` in ks_secret_key) {
                    app_nostr_key.set(ks_public_key.result);
                }
            } else {
                route_render = "/conf/init";
            }
            await goto(`/`);
            if (route_render) {
                await sleep(cfg.delay.load);
                await route(route_render);
            }
        } catch (e) {
            console.log(`(app_config) error `, e);
        } finally {
            app_loading.set(false);
            //await win.splash_hide();
        }
    });

    app_notify.subscribe(async (_app_notify) => {
        if (!_app_notify) {
            app_loading.set(false);
            return;
        }
        app_loading.set(true);
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
    {#if !$app_loading}
        <slot />
    {:else}
        <LoadingView />
    {/if}
</LayoutWindow>
<AppControls />
<CssStatic />
<CssStyles />
