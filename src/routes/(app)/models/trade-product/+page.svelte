<script lang="ts">
    import { db } from "$lib/client";
    import { type LocationGcs, type TradeProduct } from "@radroots/models";
    import {
        app_layout,
        app_notify,
        LayoutTrellis,
        LayoutView,
        Nav,
        route,
        t,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadDataResult = {
        trade_product: TradeProduct;
        location_gcs?: LocationGcs;
    };
    type LoadData = {
        results: LoadDataResult[];
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

            const results: LoadDataResult[] = [];
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
                <div
                    class={`flex flex-col h-[22rem] w-${$app_layout} justify-start items-start bg-layer-1-surface round-44`}
                >
                    <div
                        class={`flex flex-row h-[11rem] w-${$app_layout} justify-center items-center border-b-line border-b-layer-1-surface-edge`}
                    >
                        <p class={`font-sans font-[400] text-layer-0-glyph`}>
                            photos
                        </p>
                    </div>
                    <div
                        class={`flex flex-row h-[11rem] w-full justify-center items-center`}
                    >
                        <p class={`font-sans font-[400] text-layer-0-glyph`}>
                            body
                        </p>
                    </div>
                </div>
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
