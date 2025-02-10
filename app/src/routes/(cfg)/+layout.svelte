<script lang="ts">
    import { app_splash, handle_err } from "@radroots/lib-app";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();

    onMount(async () => {
        try {
        } catch (e) {
            handle_err(e, `on_mount`);
        } finally {
            app_splash.set(false);
        }
    });
</script>

{@render children()}
