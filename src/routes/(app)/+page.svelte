<script lang="ts">
    import { db, device, dialog } from "$lib/client";
    import {
        app_nostr_key,
        envelope_visible,
        EnvelopeLower,
        Glyph,
        LayoutView,
        nav_prev,
        route,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            nav_prev.set([]);

            const nostr_profile = await db.nostr_profile_get({
                public_key: $app_nostr_key,
            });
            if (`err` in nostr_profile) {
                await dialog.alert(`@todo Nostr profile configuration failure`);
                return;
            }
        } catch (e) {
        } finally {
        }
    });
</script>

<LayoutView>
    <div class={`flex flex-row h-12 w-full px-6 justify-between items-center`}>
        <div class={`flex flex-row gap-2 justify-start items-center`}>
            <p class={`font-mono font-[600] text-[1.3rem] text-layer-0-glyph`}>
                {`radRoots`}
            </p>
            {#if device.metadata?.version}
                <p
                    class={`font-mono font-[400] text-[1.3rem] text-layer-0-glyph`}
                >
                    {`/${device.metadata.version}`}
                </p>
            {/if}
        </div>
        <button
            class={`flex flex-row justify-center items-center`}
            on:click={async () => {
                await route(`/settings`);
            }}
        >
            <Glyph
                basis={{
                    classes: `text-layer-0-glyph`,
                    dim: `md`,
                    weight: `bold`,
                    key: `gear`,
                }}
            />
        </button>
    </div>
</LayoutView>
<EnvelopeLower
    basis={{
        close: async () => {
            envelope_visible.set(false);
        },
    }}
>
    <div class={`flex flex-col h-full w-full justify-center items-center px-2`}>
        <p class={`font-apercu font-[400] text-layer-2-glyph break-all`}>
            {`Your nostr key is ${$app_nostr_key}`}
        </p>
    </div>
</EnvelopeLower>
