<script lang="ts">
    import { db } from "$lib/client";
    import TradeProductListCard from "$lib/components/trade_product_list_card.svelte";
    import type { TradeProductBundle } from "$lib/types";
    import {
        app_notify,
        LayoutTrellis,
        LayoutView,
        Nav,
        route,
        t,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        results: TradeProductBundle[];
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
            const trade_products = await db.trade_product_get({
                list: [`all`],
            });
            if (`err` in trade_products) {
                app_notify.set(
                    `${$t(`icu.error_loading_*`, { value: `${$t(`common.page`)}` })}`,
                );
                return;
            }

            const results: TradeProductBundle[] = [];
            for (const trade_product of trade_products.results) {
                const location_gcs = await db.location_gcs_get({
                    list: [`on_trade_product`, { id: trade_product.id }],
                });
                results.push({
                    trade_product,
                    location_gcs:
                        `results` in location_gcs
                            ? location_gcs.results[0]
                            : undefined,
                });
            }

            const data: LoadData = {
                results,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };

    $: {
        console.log(JSON.stringify(ld, null, 4), `ld`);
    }
</script>

{#if ld && ld.results.length > 0}
    <LayoutView>
        <LayoutTrellis>
            {#each ld.results as li, li_i}
                <TradeProductListCard
                    basis={{
                        result: li,
                    }}
                />
            {/each}
        </LayoutTrellis>
    </LayoutView>
{/if}
<Nav
    basis={{
        prev: {
            label: `Home`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$t(`common.products`)}`,
            },
        },
        option:
            ld && ld?.results?.length > 0
                ? {
                      label: {
                          value: `${$t(`common.add`)}`,
                          classes: `tap-color`,
                      },
                      callback: async () => {
                          await route(`/models/trade-product/add`);
                      },
                  }
                : undefined,
    }}
/>
