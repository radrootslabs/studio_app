<script lang="ts">
    import { app_init } from "$lib/utils/app";
    import { handle_err } from "@radroots/apps-lib";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();
    let app_ready = $state(false);

    onMount(async () => {
        try {
            await app_init();
            app_ready = true;
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });
</script>

{#if !app_ready}
    <div class={`flex min-h-screen w-full items-center justify-center`}>
        <p class={`text-sm`}>Loading...</p>
    </div>
{:else}
    {@render children()}
{/if}
