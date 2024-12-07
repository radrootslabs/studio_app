<script lang="ts">
    import { db, dialog } from "$lib/client";
    import {
        app_nostr_key,
        envelope_visible,
        EnvelopeLower,
        Glyph,
        LayoutView,
        LogoCircleSm,
        nav_prev,
        route,
        t,
        TabsFloat,
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
            <LogoCircleSm />
            <p
                class={`font-sansd italic font-[700] text-[1.7rem] text-layer-0-glyph`}
            >
                {`radroots`}
            </p>
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
                    dim: `lg`,
                    weight: `bold`,
                    key: `gear`,
                }}
            />
        </button>
    </div>
    <div
        class={`flex flex-col w-full pt-4 px-6 gap-4 justify-center items-center`}
    >
        <div class={`flex flex-row w-full justify-start items-center`}>
            <p class={`font-sansd font-[600] text-2xl text-layer-0-glyph`}>
                {`${$t(`common.general`)}`}
            </p>
        </div>
        <div class={`flex flex-col w-full gap-5 justify-center items-center`}>
            <button
                class={`group flex flex-row h-[3.5rem] w-full justify-center items-center rounded-touch bg-layer-1-surface layer-1-active-surface layer-1-active-ring`}
                on:click={async () => {
                    await route(`/settings/profile`);
                }}
            >
                <p
                    class={`font-sans font-[700] text-xl text-layer-0-glyph capitalize tracking-wider opacity-active`}
                >
                    {`${$t(`common.profile`)}`}
                </p>
            </button>
        </div>
    </div>
</LayoutView>
<TabsFloat />
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
