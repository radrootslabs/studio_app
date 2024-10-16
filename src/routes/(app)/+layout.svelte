<script lang="ts">
    import { geoc } from "$lib/client";
    import { app_geoc } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            const geoc_connected = await geoc.connect();
            app_geoc.set(!!geoc_connected);
        } catch (e) {
        } finally {
        }
    });
</script>

<slot />
