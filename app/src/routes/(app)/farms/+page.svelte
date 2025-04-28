<script lang="ts">
    import { locale, ls } from "$lib/locale/i18n";
    import { db, route } from "$lib/util";
    import { lc_geocode } from "$lib/util/callback";
    import {
        Farms,
        handle_err,
        type FarmBasis,
        type IViewFarmsData,
    } from "@radroots/lib-app";
    import { location_gcs_to_location_basis } from "@radroots/util";
    import { onMount } from "svelte";

    type LoadData = IViewFarmsData | undefined;
    let data: LoadData = $state(undefined);

    onMount(async () => {
        data = await load_data();
    });

    const load_data = async (): Promise<LoadData> => {
        try {
            const tb_farms = await db.farm_read_list();
            return {
                list:
                    `results` in tb_farms
                        ? (await Promise.all(
                              tb_farms.results.map(async (i) => {
                                  const tb_loc_gcs =
                                      await db.location_gcs_read_list({
                                          table: [`on_farm`, { id: i.id }],
                                      });
                                  return {
                                      farm: {
                                          id: i.id,
                                          name: i.name,
                                          area: i.area,
                                          area_unit: i.area_unit,
                                      },
                                      location:
                                          `results` in tb_loc_gcs &&
                                          tb_loc_gcs.results[0]
                                              ? location_gcs_to_location_basis(
                                                    tb_loc_gcs.results[0],
                                                )
                                              : undefined,
                                  } satisfies FarmBasis;
                              }),
                          )) || []
                        : [],
            } satisfies LoadData;
        } catch (e) {
            await handle_err(e, `load`);
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
                await route(`/farms/details`, [[`id`, farm_id]]);
            } catch (e) {
                await handle_err(e, `lc_handle_farm_view`);
            }
        },
    }}
/>
