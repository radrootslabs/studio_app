<script lang="ts">
    import { geoc, notification } from "$lib/client";
    import {
        app_cfg_type,
        app_geoc,
        app_splash,
        sleep,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            const geoc_connected = await geoc.connect();
            app_geoc.set(!!geoc_connected);
        } catch (e) {
            console.log(`e (app) onMount`, e);
        } finally {
            app_splash.set(false);
        }
    });

    app_cfg_type.subscribe(async (_app_cfg_type) => {
        //@todo
    });

    app_splash.subscribe(async (_app_splash) => {
        if (_app_splash) return;
        await sleep(4000);
        await notification.init();
    });
</script>

<slot />
