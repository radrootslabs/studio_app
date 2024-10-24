<script lang="ts">
    import { geoc } from "$lib/client";
    import { app_geoc, app_splash } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            const geoc_connected = await geoc.connect();
            app_geoc.set(!!geoc_connected);

            /*
            const nostr_publickey = await keystore.get(ks.keys.nostr_publickey);
            if (`result` in nostr_publickey) {
                await sleep(4000);
                await notification.init();
            }
            */
        } catch (e) {
            console.log(`e (app) onMount`, e);
        } finally {
            app_splash.set(false);
        }
    });
</script>

<slot />
