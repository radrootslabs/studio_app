<script lang="ts">
    import { geoc } from "$lib/client";
    import { app_cfg_type, app_geoc, app_splash } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            const geoc_connected = await geoc.connect();
            app_geoc.set(!!geoc_connected);

            /*
            await sleep(4000);
            await notification.init();
            */
        } catch (e) {
            console.log(`e (app) onMount`, e);
        } finally {
            app_splash.set(false);
        }
    });

    app_cfg_type.subscribe(async (_app_cfg_type) => {
        console.log(`_app_cfg_type `, _app_cfg_type);
    });
</script>

<slot />
