<script lang="ts">
    import { gui, route } from "$lib/util";
    import { cfg_delay } from "$lib/util/conf";
    import {
        app_lo,
        app_loading,
        app_notify,
        app_th,
        app_thc,
        app_win,
        LayoutWindow,
        theme_set,
    } from "@radroots/lib-app";
    import { parse_color_mode, parse_theme_key } from "@radroots/theme";
    import { cfg_app, sleep } from "@radroots/util";
    import "css-paint-polyfill";
    import "../app.css";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();

    app_thc.subscribe((_app_thc) =>
        theme_set(parse_theme_key($app_th), parse_color_mode(_app_thc)),
    );

    app_th.subscribe((_app_th) =>
        theme_set(parse_theme_key(_app_th), parse_color_mode($app_thc)),
    );

    app_win.subscribe((_app_win) => {
        // @todo android layout
        if (_app_win.h > cfg_app.layout.ios1.h) app_lo.set(`ios1`);
        else app_lo.set(`ios0`);
    });

    app_notify.subscribe(async (_app_notify) => {
        if (!_app_notify) return;
        app_loading.set(true);
        await sleep(cfg_delay.notify);
        route(`/`);
        app_loading.set(false);
        await gui.alert(_app_notify);
        app_notify.set(``);
    });
</script>

<LayoutWindow>
    {@render children()}
</LayoutWindow>
