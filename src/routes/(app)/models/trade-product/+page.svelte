<script lang="ts">
    import { db } from "$lib/client";
    import { type TradeProduct } from "@radroots/models";
    import {
        app_notify,
        LayoutArea,
        LayoutTrellis,
        LayoutView,
        locale,
        Nav,
        route,
        t,
        time_fmt_iso,
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
            console.log(`trade_products `, trade_products);
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
                                                    value: time_fmt_iso(
                                                        $locale,
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
{:else}
    <LayoutArea>
        <div
            class={`flex flex-col h-full w-full gap-2 justify-center items-center`}
        >
            <p
                class={`font-sans font-[400] text-layer-1-glyph text-line_display`}
            >
                {`${$t(`icu.no_*_to_display`, { value: `${$t(`common.items`)}`.toLowerCase() })}`}
            </p>
            <a href={`/models/trade-product/add`}>
                <button
                    class={`flex flex-row justify-center items-center active:opacity-80`}
                    on:click={async () => {}}
                >
                    <p class={`font-sans font-[400] text-layer-2-glyph-hl`}>
                        {`${$t(`icu.click_to_add_a_*`, { value: `${$t(`icu.new_*`, { value: `${$t(`common.product`)}` })}`.toLowerCase() })}`}
                    </p>
                </button>
            </a>
        </div>
    </LayoutArea>
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
