<script lang="ts">
    import { db } from "$lib/client";
    import MapPointDisplay from "$lib/components/map_point_display.svelte";
    import type { LocationGcs } from "@radroots/models";
    import {
        app_notify,
        ButtonGlyphSimple,
        catch_err,
        LayoutView,
        ls,
        NavToolbar,
        PageHeader,
        qp_id,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        location_gcs: LocationGcs;
    };
    let ld: LoadData | undefined = undefined;

    onMount(async () => {
        try {
            if (!$qp_id) app_notify.set(`${$ls(`error.client.page.load`)}`);
            ld = await load_data();
        } catch (e) {
        } finally {
        }
    });

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const location_gcs = await db.location_gcs_get_one({ id: $qp_id });
            if (`err` in location_gcs)
                return void app_notify.set(`${$ls(`error.client.page.load`)}`);
            return {
                location_gcs: location_gcs.result,
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_data`);
        }
    };
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader
        basis={{
            label: [
                `${$ls(`common.farm_land`)}`,
                {
                    route: `/farm/land`,
                },
            ],
        }}
    >
        <div slot="option" class={`flex flex-row justify-start items-center`}>
            <ButtonGlyphSimple
                basis={{
                    label: `${$ls(`common.edit`)}`,
                    callback: async () => {
                        alert(`@todo!`);
                    },
                }}
            />
        </div>
    </PageHeader>
    {#if ld?.location_gcs}
        <div class={`flex flex-col w-full px-4 justify-center items-center`}>
            <div
                class={`flex flex-row h-[20rem] w-full justify-center items-center bg-layer-2-surface round-44 overflow-hidden`}
            >
                <MapPointDisplay
                    basis={{
                        zoom: 12,
                        point: {
                            lat: ld.location_gcs.lat,
                            lng: ld.location_gcs.lng,
                        },
                    }}
                />
            </div>
            <div class={`flex flex-col w-full justify-center items-center`}>
                <div class={`flex flex-row w-full justify-start items-center`}>
                    <p class={`font-sans font-[400] text-layer-0-glyph`}>hi</p>
                </div>
            </div>
        </div>
    {/if}
</LayoutView>
