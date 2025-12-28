<script lang="ts">
    import {
        app_init_has_completed,
        app_init_state,
        type AppInitStage,
    } from "$lib/utils/app";
    import { LogoCircle } from "@radroots/apps-lib-pwa";
    import { onDestroy, onMount } from "svelte";

    const stage_messages: Record<AppInitStage, string[]> = {
        idle: [`Preparing runtime`, `Starting services`, `Allocating memory`],
        storage: [
            `Opening local storage`,
            `Verifying data stores`,
            `Syncing storage index`,
        ],
        download_sql: [
            `Fetching SQL runtime`,
            `Downloading core engine`,
            `Verifying module`,
        ],
        download_geo: [
            `Fetching map data`,
            `Downloading geocoder DB`,
            `Validating data file`,
        ],
        database: [
            `Preparing database`,
            `Applying migrations`,
            `Indexing records`,
        ],
        geocoder: [
            `Loading geocoder`,
            `Preparing lookup tables`,
            `Calibrating index`,
        ],
        ready: [`Final checks`, `Ready`, `Starting UI`],
        error: [`Init error`, `Retrying setup`, `Check connection`],
    };

    const bytes_to_mb = (bytes: number): string =>
        (bytes / (1024 * 1024)).toFixed(1);

    let show_ui = $state(false);
    let message = $state(``);
    let message_index = $state(0);
    let stage = $state<AppInitStage>(`idle`);
    let message_timer_id = $state<number | null>(null);

    let progress_ratio = $state(0);
    let progress_text = $state(`0.0 MB`);

    const set_message_for_stage = (next_stage: AppInitStage): void => {
        const list = stage_messages[next_stage] ?? stage_messages.idle;
        message_index = 0;
        message = list[0] ?? ``;
    };

    const step_message = (): void => {
        const list = stage_messages[stage] ?? stage_messages.idle;
        if (list.length <= 1) return;
        message_index = (message_index + 1) % list.length;
        message = list[message_index] ?? ``;
    };

    $effect(() => {
        stage = $app_init_state.stage;
        set_message_for_stage(stage);
    });

    $effect(() => {
        const loaded = $app_init_state.loaded_bytes;
        const total = $app_init_state.total_bytes;
        progress_ratio = total ? Math.min(1, loaded / total) : 0;
        const loaded_mb = bytes_to_mb(loaded);
        progress_text = total
            ? `${loaded_mb} / ${bytes_to_mb(total)} MB`
            : `${loaded_mb} MB`;
    });

    onMount(() => {
        show_ui = !app_init_has_completed();
        if (!show_ui) return;
        message_timer_id = window.setInterval(() => {
            step_message();
        }, 4200);
    });

    onDestroy(() => {
        if (message_timer_id !== null) window.clearInterval(message_timer_id);
    });
</script>

{#if show_ui}
    <div
        class={`flex flex-col min-h-screen w-full justify-center items-center`}
    >
        <div class={`flex flex-col justify-center items-center gap-6`}>
            <LogoCircle />
            <div class={`flex flex-col items-center gap-3`}>
                <p class={`text-sm text-ly0-gl text-center`}>{message}</p>
                <div
                    class={`flex flex-col items-center gap-2 w-[72vw] max-w-[22rem]`}
                >
                    <div
                        class={`w-full h-2 bg-ly0-gl/15 rounded-full overflow-hidden`}
                    >
                        <div
                            class={`h-full bg-ly0-gl/55 rounded-full`}
                            style={`width: ${Math.max(progress_ratio * 100, 4)}%`}
                        ></div>
                    </div>
                    <p class={`text-xs text-ly0-gl-label`}>
                        {`Downloaded ${progress_text}`}
                    </p>
                </div>
            </div>
        </div>
    </div>
{/if}
