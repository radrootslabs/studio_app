<script lang="ts">
    import { handle_err } from "@radroots/apps-lib";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();

    onMount(async () => {
        try {
        } catch (e) {
            handle_err(e, `on_mount`);
        }
    });
</script>

{@render children()}
