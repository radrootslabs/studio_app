<script lang="ts">
    import { db } from "$lib/client";
    import type { LocationGcs } from "@radroots/models";
    import { LayoutView, Nav } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    let list1: LocationGcs[] = [];

    onMount(async () => {
        try {
            const res1 = await db.location_gcs_get({ list: [`all`] });
            if (!(`err` in res1)) list1 = res1.results;
        } catch (e) {
        } finally {
        }
    });
</script>

<LayoutView>
    <div class={`flex flex-col w-full justify-center items-center`}>
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            {`Models`}
        </p>
        <div class={`flex flex-col w-full px-4 justify-center items-center`}>
            {#each list1 as li}
                <div class={`flex flex-col justify-start items-center`}>
                    <p
                        class={`font-sans font-[400] text-layer-0-glyph break-all`}
                    >
                        {JSON.stringify(li, null, 4)}
                    </p>
                    <div
                        class={`flex flex-row w-full justify-center items-center`}
                    >
                        <button
                            class={`flex flex-row justify-center items-center`}
                            on:click={async () => {
                                await db.location_gcs_delete({ id: li.id });
                            }}
                        >
                            {`del`}
                        </button>
                    </div>
                </div>
            {/each}
        </div>
    </div>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `Test`,
            },
        },
    }}
/>
