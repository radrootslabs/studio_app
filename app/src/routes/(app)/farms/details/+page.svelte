<script lang="ts">
    import { locale, ls } from "$lib/locale/i18n";
    import { db, route } from "$lib/util";
    import { lc_geocode } from "$lib/util/callback";
    import {
        FarmsDetails,
        handle_err,
        qp_id,
        type IViewFarmsDetailsData,
    } from "@radroots/lib-app";
    import { location_gcs_to_location_basis } from "@radroots/util";
    import { onMount } from "svelte";

    type LoadData = IViewFarmsDetailsData | undefined;
    let data: LoadData = $state(undefined);

    onMount(async () => {
        data = await load_data();
    });

    const load_data = async (): Promise<LoadData> => {
        try {
            const tb_farm = await db.farm_read({ id: $qp_id || `` });
            if (`err` in tb_farm) return void route(`/farms`);
            const tb_farm_location = await db.location_gcs_read_list({
                table: [`on_farm`, { id: tb_farm.result.id }],
            });
            return {
                farm: {
                    ...tb_farm.result,
                },
                location:
                    `results` in tb_farm_location && tb_farm_location.results[0]
                        ? location_gcs_to_location_basis(
                              tb_farm_location.results[0],
                          )
                        : undefined,
            } satisfies LoadData;
        } catch (e) {
            await handle_err(e, `load_data`);
        }
    };
</script>

{#if data}
    <FarmsDetails
        {ls}
        {locale}
        basis={{
            data,
            callback_route: { route: `/farms` },
            lc_geocode,
            lc_handle_farm_lot_add: async (farm_id) => {
                try {
                    // await route(`/farms/lots/add`, { farm_id });
                } catch (e) {
                    await handle_err(e, `lc_handle_farm_lot_add`);
                }
            },
            lc_handle_farm_products_view: async (farm_id) => {
                try {
                    /*
                if (data?.farm_lots?.every((i) => !i.farm_lot_products?.length))
                    await route(`/farms/products/add`, { farm_id });
                else
                */
                } catch (e) {
                    await handle_err(e, `lc_handle_farm_products_view`);
                }
            },
            lc_handle_farm_orders_view: async (farm_id) => {
                try {
                } catch (e) {
                    await handle_err(e, `lc_handle_farm_orders_view`);
                }
            },
        }}
    />
{/if}
