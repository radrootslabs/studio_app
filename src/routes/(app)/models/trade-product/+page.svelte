<script lang="ts">
    import { db } from "$lib/client";
    import { type TradeProduct } from "@radroots/models";
    import {
        app_notify,
        LayoutTrellis,
        LayoutView,
        Nav,
        route,
        t,
        time_iso,
        Trellis,
    } from "@radroots/svelte-lib";
    import { onMount } from "svelte";

    type LoadData = {
        trade_products: TradeProduct[];
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
            const data: LoadData = {
                trade_products: trade_products.results,
            };
            return data;
        } catch (e) {
            console.log(`(error) load_data `, e);
        }
    };
</script>

{#if ld && ld.trade_products.length > 0}
    <LayoutView>
        <LayoutTrellis>
            {#each ld.trade_products as li, li_i}
                <Trellis
                    basis={{
                        args: {
                            layer: 1,
                            title:
                                li_i === 0
                                    ? {
                                          value: `Trade Products`,
                                      }
                                    : undefined,
                            list: [
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Product:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value: li.key,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Date Created:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value: time_iso(
                                                        li.created_at,
                                                    ),
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Lot:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value: li.lot || `@todo`,
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                                {
                                    hide_active: true,
                                    touch: {
                                        label: {
                                            left: [
                                                {
                                                    value: `Process:`,
                                                    classes: `capitalize`,
                                                },
                                            ],
                                            right: [
                                                {
                                                    value:
                                                        li.process || `(@todo)`,
                                                },
                                            ],
                                        },
                                        callback: async () => {},
                                    },
                                },
                            ],
                        },
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
            ld && ld?.trade_products?.length > 0
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
