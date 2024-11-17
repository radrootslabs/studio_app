<script lang="ts">
    import type { NostrEventPageStore } from "$lib/types";
    import { type NDKFilter, NDKKind } from "@nostr-dev-kit/ndk";
    import {
        app_nostr_key,
        LayoutView,
        Nav,
        ndk,
        t,
    } from "@radroots/svelte-lib";
    import { onDestroy } from "svelte";

    let events_store: NostrEventPageStore;

    $: {
        let authors = [$app_nostr_key];
        let ndk_filter: NDKFilter = {
            kinds: [NDKKind.Classified],
            ...{ authors },
        };

        fetch_events(ndk_filter).then(() => {
            events_store?.startSubscription();
        });
    }

    const fetch_events = async (filter: NDKFilter): Promise<void> => {
        try {
            events_store = $ndk.storeSubscribe(filter, {
                closeOnEose: true,
                groupable: false,
                autoStart: false,
            });
            if (events_store) events_store.onEose(() => {});
        } catch (e) {
            console.log(`(error) fetch_events `, e);
        }
    };

    onDestroy(() => events_store?.unsubscribe());
</script>

<LayoutView>
    <div class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}>
        {#if $events_store?.length}
            {#each $events_store as ev, ev_i (ev.id)}
                <p class={`font-sans font-[400] text-layer-0-glyph break-all`}>
                    {JSON.stringify(ev.content)}
                </p>
                <p class={`font-sans font-[400] text-layer-0-glyph break-all`}>
                    {JSON.stringify(ev.tags)}
                </p>
            {/each}
        {/if}
    </div>
</LayoutView>

<Nav
    basis={{
        prev: {
            label: `${$t("common.home")}`,
            route: `/`,
        },
        title: {
            label: { value: `Image Upload` },
        },
    }}
/>
