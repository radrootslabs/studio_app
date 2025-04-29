<script lang="ts">
    import { db, route } from "$lib/util";
    import {
        FarmsProductsAdd,
        handle_err,
        qp_id,
        type IViewFarmsProductsAddData,
    } from "@radroots/lib-app";
    import { location_gcs_to_location_basis } from "@radroots/util";
    import { onMount } from "svelte";

    type LoadData = IViewFarmsProductsAddData | undefined;
    let data: LoadData = $state(undefined);

    onMount(async () => {
        data = await load_data();
    });

    const load_data = async (): Promise<LoadData> => {
        try {
            const tb_farm = await db.farm_read({ id: $qp_id || `` });
            if (`err` in tb_farm) return void route(`/farms`);
            console.log(JSON.stringify(tb_farm, null, 4), `tb_farm`);
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
    <FarmsProductsAdd
        basis={{
            data,
            on_handle_farm_lot_add: async () => {
                try {
                } catch (e) {
                    await handle_err(e, `on_handle_farm_lot_add`);
                }
            },
            on_handle_photo_envelope_edit: async () => {
                try {
                } catch (e) {
                    await handle_err(e, `on_handle_photo_envelope_edit`);
                }
            },
            on_handle_tradepr_key_toggle: async () => {
                try {
                    return ``;
                } catch (e) {
                    await handle_err(e, `on_handle_tradepr_key_toggle`);
                    return ``;
                }
            },
            on_submit: async ({ payload, farm_id, geolocation_id }) => {
                try {
                    console.log(JSON.stringify(payload, null, 4), `payload`);
                } catch (e) {
                    await handle_err(e, `on_submit`);
                }
            },
        }}
    />
{/if}
