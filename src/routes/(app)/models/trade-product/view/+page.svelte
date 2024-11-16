<script lang="ts">
    import { db } from "$lib/client";
    import type { TradeProductBundle } from "$lib/types";
    import {
        app_notify,
        LayoutTrellis,
        LayoutView,
        Nav,
        qp_id,
        t,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = TradeProductBundle;
    let ld: LoadData | undefined = undefined;

    onMount(async () => {
        try {
            if (!$qp_id) app_notify.set(`${$t(`error.client.page.load`)}`);
            ld = await load_data();
        } catch (e) {
        } finally {
        }
    });

    const load_data = async (): Promise<LoadData | undefined> => {
        try {
            const _trade_product = await db.trade_product_get_one({
                id: $qp_id,
            });
            if (`err` in _trade_product) {
                app_notify.set(`${$t(`error.client.page.load`)}`); //@todo
                return;
            }
            const { result: trade_product } = _trade_product;

            const location_gcs_res = await db.location_gcs_get({
                list: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in location_gcs_res) {
                //@todo
                return;
            }
            const location_gcs = location_gcs_res.results[0];

            const data: LoadData = {
                trade_product,
                location_gcs,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };
</script>

{#if ld}
    <LayoutView>
        <LayoutTrellis>
            <p class={`font-sans font-[400] text-layer-0-glyph break-all`}>
                {JSON.stringify(ld)}
            </p>
        </LayoutTrellis>
    </LayoutView>
{/if}
<Nav
    basis={{
        prev: {
            label: `${$t(`common.back`)}`,
            route: `/models/trade-product`,
        },
        title: {
            label: {
                value: `${$t(`common.product`)}`,
            },
        },
    }}
/>
