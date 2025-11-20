<script lang="ts">
    import { dev, version as kit_version } from "$app/environment";
    import { db, geoc } from "$lib/utils/app";
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
    import { cfg_app } from "@radroots/apps-lib-pwa/utils/app";
    import { parse_theme_key, parse_theme_mode } from "@radroots/themes";
    import "css-paint-polyfill";
    import { onMount } from "svelte";
    import "../app.css";
    import type { LayoutProps } from "./$types";

    type MetaTag = {
        name: string;
        content: string;
    };

    const head_meta_tags: MetaTag[] = [
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

    set_context("lib", {
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

    theme_mode.subscribe((_theme_mode) =>
        theme_set(parse_theme_key($theme_key), parse_theme_mode(_theme_mode)),
    );

    theme_key.subscribe((_theme_key) =>
        theme_set(parse_theme_key(_theme_key), parse_theme_mode($theme_mode)),
    );

    win_h.subscribe((_win_h) => {
        if (_win_h > cfg_app.layout.ios1.h) {
            app_lo.set("ios1");
        } else {
            app_lo.set("ios0");
        }
    });

    onMount(async () => {
        await db.init();
        await geoc.connect();
    });
</script>

<svelte:head>
    {#each head_meta_tags as meta_tag (meta_tag.name)}
        <meta name={meta_tag.name} content={meta_tag.content} />
    {/each}
</svelte:head>

<LayoutWindow>
    {@render children()}
</LayoutWindow>
<Css />
