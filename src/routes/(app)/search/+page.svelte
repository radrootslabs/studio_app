<script lang="ts">
    import { db } from "$lib/client";
    import SearchResultDisplay from "$lib/components/search_result_display.svelte";
    import { cfg } from "$lib/conf";
    import type {
        LocationGcs,
        NostrProfile,
        NostrRelay,
        TradeProduct,
    } from "@radroots/models";
    import {
        catch_err,
        debounce_input,
        fmt_id,
        Glyph,
        InputElement,
        LayoutView,
        ls,
        NavToolbar,
        PageHeader,
        TabsFloat,
    } from "@radroots/svelte-lib";
    import { SearchService, type SearchServiceResult } from "@radroots/utils";
    import { onMount } from "svelte";

    type LoadData = {
        location_gcs: LocationGcs[];
        nostr_profile: NostrProfile[];
        nostr_relay: NostrRelay[];
        trade_product: TradeProduct[];
    };
    let ld: LoadData | undefined = undefined;

    let client_search: SearchService | undefined = undefined;
    let search_val = ``;
    let search_results: SearchServiceResult[] = [];

    onMount(async () => {
        try {
            await init_page();
        } catch (e) {
        } finally {
        }
    });

    const init_page = async (): Promise<void> => {
        try {
            ld = await load_data();
            if (ld) client_search = new SearchService(ld);
        } catch (e) {
            await catch_err(e, `init_page`);
        }
    };

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const location_gcs = await db.location_gcs_get({ list: [`all`] });
            const nostr_profile = await db.nostr_profile_get({
                list: [`all`],
            });
            const nostr_relay = await db.nostr_relay_get({ list: [`all`] });
            const trade_product = await db.trade_product_get({
                list: [`all`],
            });
            return {
                location_gcs:
                    `results` in location_gcs ? location_gcs.results : [],
                nostr_profile:
                    `results` in nostr_profile ? nostr_profile.results : [],
                nostr_relay:
                    `results` in nostr_relay ? nostr_relay.results : [],
                trade_product:
                    `results` in trade_product ? trade_product.results : [],
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_page`);
        }
    };

    const handle_search_input = debounce_input((val: string) => {
        if (client_search) search_results = client_search.search(val);
    }, cfg.debounce.search);
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader basis={{ label: `${$ls(`common.search`)}` }}></PageHeader>
    <div class={`flex flex-col w-full px-4 gap-6 justify-center items-center`}>
        <div
            class={`relative flex flex-row w-full justify-center items-center bg-layer-1-surface rounded-2xl`}
        >
            <Glyph
                basis={{
                    classes: `absolute left-4 text-layer-0-glyph-shade`,
                    dim: `sm`,
                    weight: `bold`,
                    key: `magnifying-glass`,
                }}
            />
            <InputElement
                bind:value={search_val}
                basis={{
                    id: fmt_id(`search`),
                    sync: true,
                    classes: `pl-12 text-layer-0-glyph`,
                    placeholder: `Enter search query`,
                    callback: async ({ value }) => handle_search_input(value),
                }}
            />
        </div>
        <div class={`flex flex-col w-full gap-4 justify-center items-center`}>
            {#each search_results as li (li.id)}
                <SearchResultDisplay basis={li} />
            {/each}
        </div>
    </div>
</LayoutView>
<TabsFloat />
