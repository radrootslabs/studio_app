<script lang="ts">
    import { db } from "$lib/client";
    import MapPointDisplay from "$lib/components/map_point_display.svelte";
    import type { LocationGcs } from "@radroots/models";
    import {
        ButtonGlyphSimple,
        Fade,
        LayoutView,
        NavToolbar,
        PageHeader,
        TabsFloat,
        app_notify,
        catch_err,
        fmt_geol_latitude,
        fmt_geol_longitude,
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
            await catch_err(e, `load_data`);
        }
    };

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader basis={{ label: `${$ls(`common.farm_land`)}` }}>
        <div slot="option" class={`flex flex-row justify-start items-center`}>
            {#if ld && ld.location_gcss.length}
                <Fade>
                    <ButtonGlyphSimple
                        basis={{
                            label: `${$ls(`common.add`)}`,
                            callback: async () => {
                                await route(`/farm/land/add`);
                            },
                        }}
                    />
                </Fade>
            {/if}
        </div>
    </PageHeader>
    <div class={`flex flex-col w-full px-4 justify-start items-center`}>
        {#if ld && ld.location_gcss.length}
            {#each ld.location_gcss.filter((i) => i.kind === `farm_land`) as li}
                <button
                    class={`group flex flex-row h-[5rem] w-full px-8 gap-8 justify-start items-center bg-layer-1-surface layer-1-active-surface round-36 el-re`}
                    on:click={async () => {
                        await route(`/farm/land/add`);
                    }}
                >
                    <div
                        class={`flex flex-col h-[4rem] w-[4rem] justify-start items-center bg-layer-2-surface round-24`}
                    >
                        <MapPointDisplay
                            basis={{
                                point: {
                                    lat: li.lat,
                                    lng: li.lng,
                                },
                            }}
                        />
                    </div>
                    <div
                        class={`flex flex-col flex-grow h-[3.25rem] justify-between items-start`}
                    >
                        <div
                            class={`flex flex-row w-full justify-start items-center`}
                        >
                            <p
                                class={`font-sans font-[500] text-layer-0-glyph`}
                            >
                                {`${
                                    li.label ||
                                    `${fmt_geol_latitude(
                                        li.lat,
                                        `d`,
                                        4,
                                    )}, ${fmt_geol_longitude(li.lng, `d`, 4)}`
                                }`}
                            </p>
                        </div>
                        <div
                            class={`flex flex-row w-full gap-2 justify-start items-center`}
                        >
                            {#if li.kind === `farm_land`}
                                <div
                                    class={`flex flex-row h-5 px-2 justify-center items-center bg-layer-2-surface rounded-md`}
                                >
                                    <p
                                        class={`font-sans font-[700] text-[0.8rem] text-white`}
                                    >
                                        {`${$ls(`common.farm`)}`}
                                    </p>
                                </div>
                            {/if}
                            <p
                                class={`font-sansd font-[500] text-layer-0-glyph`}
                            >
                                {`${li.gc_name}, ${li.gc_admin1_id}, ${li.gc_country_id}`}
                            </p>
                        </div>
                    </div>
                </button>
            {/each}
        {/if}
    </div>
</LayoutView>
<TabsFloat />

<!--
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
        -->
