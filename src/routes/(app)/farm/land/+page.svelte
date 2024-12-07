<script lang="ts">
    import { db } from "$lib/client";
    import type { LocationGcs } from "@radroots/models";
    import {
        Fill,
        LayoutView,
        NavToolbar,
        PageHeader,
        TabsFloat,
        app_notify,
        ls,
        route,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        location_gcss: LocationGcs[];
    };
    let ld: LoadData | undefined = undefined;

    onMount(async () => {
        try {
            ld = await load_data();
        } catch (e) {
        } finally {
        }
    });

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const location_gcss = await db.location_gcs_get({
                list: [`all`],
            });
            if (`err` in location_gcss) {
                app_notify.set(`${$ls(`error.client.page.load`)}`);
                return;
            }
            return {
                location_gcss: location_gcss.results,
            } satisfies LoadData;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader basis={{ label: `${$ls(`common.farm_land`)}` }} />
    <div class={`flex flex-col w-full px-4 justify-start items-center`}>
        <button
            class={`group flex flex-row h-[7rem] w-full px-8 justify-start items-center bg-layer-1-surface layer-1-active-surface round-36 el-re`}
            on:click={async () => {
                await route(`/farm/land/add`);
            }}
        >
            <div
                class={`flex flex-col h-[5rem] w-[5rem] justify-start items-center bg-layer-2-surface round-24 bg-repeat heropattern-topography-white`}
            >
                <Fill />
            </div>
            <div class={`flex flex-row flex-grow justify-center items-center`}>
                <p
                    class={`font-sans font-[500] text-[1.4rem] text-layer-0-glyph capitalize opacity-active el-re`}
                >
                    {`${$ls(`icu.add_*`, { value: `${$ls(`common.land_plot`)}` })}`}
                </p>
            </div>
        </button>
    </div>
</LayoutView>
<TabsFloat />
