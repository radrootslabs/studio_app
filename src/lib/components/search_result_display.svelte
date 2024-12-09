<script lang="ts">
    import type { ClientSearchResult } from "$lib/util/client-search";
    import { ascii, Glyph, ls, route } from "@radroots/svelte-lib";
    import SearchResultContainer from "./search_result_container.svelte";

    export let basis: ClientSearchResult;
</script>

{#if `location_gcs` in basis && basis.location_gcs.id}
    <SearchResultContainer
        basis={{
            callback: async () => {
                if (basis.location_gcs.kind === `farm_land`)
                    await route(`/farm/land/view`, [
                        [`id`, basis.location_gcs.id],
                    ]);
            },
        }}
    >
        <div class={`flex flex-row gap-4 justify-start items-center`}>
            <div
                class={`flex flex-row h-[1.5rem] w-[1.5rem] justify-center items-center bg-stone-500 round-24`}
            >
                <Glyph
                    basis={{
                        classes: `text-white`,
                        dim: `xs`,
                        weight: `bold`,
                        key: `compass`,
                    }}
                />
            </div>
            <div class={`flex flex-row gap-1 justify-start items-center`}>
                <div
                    class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-stone-600 rounded-md`}
                >
                    <p
                        class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                    >
                        {`${$ls(`common.location`)}`}
                    </p>
                </div>
                <p
                    class={`font-sans font-[700] text-layer-0-glyph/30 -translate-y-[1px]`}
                >
                    {ascii.bullet}
                </p>

                {#if basis.location_gcs.kind === `farm_land`}
                    <div
                        class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-lime-500 rounded-md`}
                    >
                        <p
                            class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                        >
                            {`${$ls(`common.farm_land`)}`}
                        </p>
                    </div>
                {/if}
            </div>
        </div>
        <div
            class={`flex flex-row flex-grow pl-4 pr-2 justify-end items-center overflow-hidden`}
        >
            {#if basis.location_gcs.label}
                <p
                    class={`font-sand font-[500] text-[0.9rem] text-layer-0-glyph`}
                >
                    {`${basis.location_gcs.label}`}
                </p>
            {:else}
                <p
                    class={`font-sand font-[500] text-[0.9rem] text-layer-0-glyph truncate`}
                >
                    {`${basis.location_gcs.gc_name}, ${basis.location_gcs.gc_admin1_id}`}
                </p>
            {/if}
        </div>
    </SearchResultContainer>
{:else if `nostr_profile` in basis && basis.nostr_profile.id}
    <SearchResultContainer
        basis={{
            callback: async () => {
                await route(`/settings/profile`);
            },
        }}
    >
        <div class={`flex flex-row gap-4 justify-start items-center`}>
            <div
                class={`flex flex-row h-[1.5rem] w-[1.5rem] justify-center items-center bg-blue-400 round-24`}
            >
                <Glyph
                    basis={{
                        classes: `text-white`,
                        dim: `xs`,
                        weight: `bold`,
                        key: `user`,
                    }}
                />
            </div>
            <div class={`flex flex-row gap-1 justify-start items-center`}>
                <div
                    class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-stone-600 rounded-md`}
                >
                    <p
                        class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                    >
                        {`${$ls(`common.profile`)}`}
                    </p>
                </div>
                <p
                    class={`font-sans font-[700] text-layer-0-glyph/30 -translate-y-[1px]`}
                >
                    {ascii.bullet}
                </p>
                <div
                    class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-purple-400 rounded-md`}
                >
                    <p
                        class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                    >
                        {#if basis.result_k === `name`}
                            {`${$ls(`common.name`)}`}
                        {:else}
                            {`@todo`}
                        {/if}
                    </p>
                </div>
            </div>
        </div>
        <div
            class={`flex flex-row flex-grow pl-4 pr-2 justify-end items-center overflow-hidden`}
        >
            {#if basis.nostr_profile.name}
                <p
                    class={`font-sand font-[500] text-[0.9rem] text-layer-0-glyph tracking-wide truncate`}
                >
                    {#if basis.result_k === `name`}
                        {`${basis.nostr_profile.name}`}
                    {:else if basis.result_v}
                        {`${basis.result_v}`}
                    {:else}
                        {`@todo`}
                    {/if}
                </p>
            {/if}
        </div>
    </SearchResultContainer>
{:else if `nostr_relay` in basis && basis.nostr_relay.id}
    <SearchResultContainer
        basis={{
            callback: async () => {},
        }}
    >
        <div class={`flex flex-row gap-4 justify-start items-center`}>
            <div
                class={`flex flex-row h-[1.5rem] w-[1.5rem] justify-center items-center bg-blue-400 round-24`}
            >
                <Glyph
                    basis={{
                        classes: `text-white`,
                        dim: `xs`,
                        weight: `bold`,
                        key: `user`,
                    }}
                />
            </div>
            <div class={`flex flex-row gap-1 justify-start items-center`}>
                <div
                    class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-stone-600 rounded-md`}
                >
                    <p
                        class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                    >
                        {`${$ls(`common.relay`)}`}
                    </p>
                </div>
                <p
                    class={`font-sans font-[700] text-layer-0-glyph/30 -translate-y-[1px]`}
                >
                    {ascii.bullet}
                </p>
                <div
                    class={`flex flex-row h-[1.2rem] px-2 justify-center items-center bg-yellow-400 rounded-md`}
                >
                    <p
                        class={`font-sans font-[900] text-[0.7rem] text-white uppercase`}
                    >
                        {`${$ls(`common.url`)}`}
                    </p>
                </div>
            </div>
        </div>
        <div
            class={`flex flex-row flex-grow pr-2 justify-end items-center overflow-hidden`}
        >
            <p
                class={`font-sand font-[500] text-[0.9rem] text-layer-0-glyph tracking-wide truncate`}
            >
                {`${basis.nostr_relay.url}`}
            </p>
        </div>
    </SearchResultContainer>
{/if}
