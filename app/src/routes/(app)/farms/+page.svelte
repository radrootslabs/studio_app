<script lang="ts">
    import { db, route } from "$lib/utils/app";
    import { handle_err } from "@radroots/apps-lib";
    import { Farms } from "@radroots/apps-lib-pwa";
    import type {
        FarmExtended,
        IViewFarmsData,
    } from "@radroots/apps-lib-pwa/types/views/farms";
    import { gcs_to_location_basis } from "@radroots/utils";
    import { onMount } from "svelte";

    type LoadData = IViewFarmsData | undefined;
    let data: LoadData = $state(undefined);

    onMount(async () => {
        await db.init();
        data = await load_data();
    });

    const load_data = async (): Promise<LoadData> => {
        try {
            const farms = await db.farm_find_many();
            if ("err" in farms) return undefined;

            const list: FarmExtended[] = [];
            for (const farm of farms.results) {
                const farm_location = await db.location_gcs_find_many({
                    rel: {
                        on_farm: {
                            id: farm.id,
                        },
                    },
                });
                if ("err" in farm_location) continue;
                list.push({
                    farm,
                    location: gcs_to_location_basis(farm_location.results[0]),
                });
            }

            const data: LoadData = {
                list,
            };
            return data;
        } catch (e) {
            handle_err(e, `load`);
        }
    };
</script>

<Farms
    basis={{
        data,
        callback_route: { route: `/` },
        on_handle_farm_add: async () => {
            try {
                await route("/farms/add");
            } catch (e) {
                handle_err(e, `on_handle_farm_add`);
            }
        },
        on_handle_farm_view: async (farm_id) => {
            try {
                await route("/farms/info", [[`id`, farm_id]]);
            } catch (e) {
                handle_err(e, `on_handle_farm_view`);
            }
        },
    }}
/>
