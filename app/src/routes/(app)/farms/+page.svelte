<script lang="ts">
    import { locale, ls } from "$lib/locale/i18n";
    import { db, route } from "$lib/util";
    import { lc_geocode } from "$lib/util/callback";
    import { Farms, handle_err, type IViewFarmsData } from "@radroots/lib-app";
    import { location_gcs_to_geolocation_basis } from "@radroots/util";
    import { onMount } from "svelte";

    type LoadData = IViewFarmsData | undefined;
    let data: LoadData = $state(undefined);

    onMount(async () => {
        data = await load_data();
    });

    const load_data = async (): Promise<LoadData> => {
        try {
            const tb_farms = await db.farm_read_list();
            if (`err` in tb_farms) return;
            return {
                list: await Promise.all(
                    tb_farms.results.map(async (i) => {
                        const tb_loc_gcs = await db.location_gcs_read_list({
                            table: [`on_farm`, { id: i.id }],
                        });
                        console.log(
                            JSON.stringify(tb_loc_gcs, null, 4),
                            `tb_loc_gcs`,
                        );
                        return {
                            farm: {
                                id: i.id,
                                name: i.name,
                            },
                            geolocation:
                                `results` in tb_loc_gcs && tb_loc_gcs.results[0]
                                    ? location_gcs_to_geolocation_basis(
                                          tb_loc_gcs.results[0],
                                      )
                                    : undefined,
                        };
                    }),
                ),
            };
        } catch (e) {
            await handle_err(e, `load`);
        } finally {
        }
    };
</script>

<Farms
    {ls}
    {locale}
    basis={{
        data,
        callback_route: { route: `/` },
        lc_geocode,
        lc_handle_farm_add: async () => {
            try {
                await route(`/farms/add`);
            } catch (e) {
                await handle_err(e, `lc_handle_farm_add`);
            }
        },
        lc_handle_farm_view: async (farm_id) => {
            try {
            } catch (e) {
                await handle_err(e, `lc_handle_farm_view`);
            }
        },
    }}
/>
