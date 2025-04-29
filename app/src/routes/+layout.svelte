<script lang="ts">
    import { locale, ls } from "$lib/locale/i18n";
    import {
        lc_color_mode,
        lc_geocode,
        lc_geop_current,
        lc_gui_alert,
        lc_gui_confirm,
        lc_img_bin,
        lc_photos_add,
        lc_photos_upload,
    } from "$lib/util/callback";
    import {
        app_lo,
        app_th,
        app_thc,
        LayoutWindow,
        set_context,
        theme_set,
        win_h,
    } from "@radroots/lib-app";
    import { parse_color_mode, parse_theme_key } from "@radroots/theme";
    import { cfg_app } from "@radroots/util";
    import "css-paint-polyfill";
    import "../app.css";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();

    set_context(`lib`, {
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
    });

    app_thc.subscribe((_app_thc) =>
        theme_set(parse_theme_key($app_th), parse_color_mode(_app_thc)),
    );

    app_th.subscribe((_app_th) =>
        theme_set(parse_theme_key(_app_th), parse_color_mode($app_thc)),
    );

    win_h.subscribe((_win_h) => {
        if (_win_h > cfg_app.layout.ios1.h) app_lo.set(`ios1`);
        else app_lo.set(`ios0`);
    });
</script>

<LayoutWindow>
    {@render children()}
</LayoutWindow>
