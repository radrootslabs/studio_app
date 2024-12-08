<script lang="ts">
    import { db } from "$lib/client";
    import TradeProductListCard from "$lib/components/trade_product_list_card.svelte";
    import type { TradeProductBundle } from "$lib/types";
    import {
        app_notify,
        catch_err,
        LayoutTrellis,
        LayoutView,
        ls,
        Nav,
        route,
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
                    `${$ls(`icu.error_loading_*`, { value: `${$ls(`common.page`)}` })}`,
                );
                return;
            }
            const results: TradeProductBundle[] = [];
            for (const trade_product of trade_products.results) {
                const location_gcs_res = await db.location_gcs_get({
                    list: [`on_trade_product`, { id: trade_product.id }],
                });
                if (`err` in location_gcs_res) {
                    //@todo
                    return;
                }
                const location_gcs = location_gcs_res.results[0];
                const media_uploads_res = await db.media_upload_get({
                    list: [`on_trade_product`, { id: trade_product.id }],
                });
                results.push({
                    trade_product,
                    location_gcs,
                    media_uploads:
                        `results` in media_uploads_res
                            ? media_uploads_res.results
                            : undefined,
                });
            }
            return {
                results,
            } satisfies LoadData;
        } catch (e) {
            await catch_err(e, `load_data`);
        }
    };
</script>

{#if ld && ld.results.length > 0}
    <LayoutView>
        <LayoutTrellis basis={{ classes: `pt-8` }}>
            {#each ld.results as li, li_i}
                <TradeProductListCard
                    basis={{
                        index: li_i,
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
            label: `${$ls(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `${$ls(`common.products`)}`,
            },
        },
        option:
            ld && ld?.results?.length > 0
                ? {
                      label: {
                          value: `${$ls(`common.add_new`)}`,
                          classes: `tap-color capitalize`,
                      },
                      callback: async () => {
                          await route(`/models/trade-product/add`);
                      },
                  }
                : undefined,
    }}
/>
