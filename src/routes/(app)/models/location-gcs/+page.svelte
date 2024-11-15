<script lang="ts">
    import { db } from "$lib/client";
    import { type LocationGcs } from "@radroots/models";
    import {
        app_notify,
        LayoutTrellis,
        LayoutView,
        Nav,
        t,
        Trellis,
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

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const location_gcss = await db.location_gcs_get({
                list: [`all`],
            });
            if (`err` in location_gcss) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            } else if (!location_gcss.results.length) {
                app_notify.set(`${$t(`error.client.page.load`)}`);
                return;
            }

            const data: LoadData = {
                location_gcss: location_gcss.results,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };

    const handle_add_location_gcs = async (): Promise<void> => {
        try {
            console.log(`@todo`);
            //const res = await location_gcs_add_current();
            //if (res) ld = await load_data();
        } catch (e) {
            console.log(`(error) handle_add_location_gcs `, e);
        }
    };
</script>

<LayoutView>
    <LayoutTrellis>
        {#if ld && ld.location_gcss?.length > 0}
            {#each ld.location_gcss as li}
                <Trellis
                    basis={{
                        args: {
                            layer: 1,
                            title: {
                                value: `${$t(`icu.your_*`, { value: `${$t(`common.locations`)}` })}`,
                            },
                            list: [
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Location:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value: li.label || `test`,
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Coordinates:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value: `${li.lat.toFixed(3)} ${li.lng.toFixed(3)}`,
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                            ],
                        },
                    }}
                />
            {/each}
        {:else if ld && ld?.location_gcss?.length === 0}
            <div
                class={`flex flex-col w-full justify-center items-center px-4 gap-3`}
            >
                <p class={`font-sans font-[400] text-layer-2-glyph`}>
                    {`No items to display.`}
                </p>

                <button
                    class={`flex flex-row justify-center items-center`}
                    on:click={async () => {
                        await handle_add_location_gcs();
                    }}
                >
                    <p
                        class={`font-sans font-[400] text-layer-2-glyph-hl text-sm`}
                    >
                        {`Click to add a new location`}
                    </p>
                </button>
            </div>
        {/if}
    </LayoutTrellis>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.locations`)}`,
            },
        },
        option:
            ld && ld?.location_gcss?.length > 0
                ? {
                      label: {
                          value: `${$t(`common.add`)}`,
                          classes: `tap-color`,
                      },
                      callback: async () => {
                          await handle_add_location_gcs();
                      },
                  }
                : undefined,
    }}
/>

<style>
    :global(.map-card) {
        height: 100px;
        width: 160px;
    }
</style>
