<script lang="ts">
    import { scroll_args } from "$lib/conf";
    import type { TradeProductBundle } from "$lib/types";
    import { fmt_location_gcs } from "@radroots/models";
    import {
        app_layout,
        el_id,
        els_id_pref_index,
        fmt_geol_latitude,
        fmt_geol_longitude,
        Glyph,
        ImagePath,
        locale,
        ls,
        route,
    } from "@radroots/svelte-lib";
    import {
        fmt_currency_price,
        fmt_plural_agreement,
        mass_tf_str,
        num_min,
        parse_currency_price,
    } from "@radroots/utils";
    import LineEntriesBetween from "./line_entries_between.svelte";
    import LineEntryData from "./line_entry_data.svelte";
    import LineEntryLabel from "./line_entry_label.svelte";

    const id_pref = `trade-product-list-card`;

    let el_c: HTMLDivElement;

    export let basis: {
        index: number;
        result: TradeProductBundle;
    };
    $: ({
        result: { trade_product, location_gcs, media_uploads },
    } = basis);

    $: tradeproduct_qty_sold = 0;
    $: tradeproduct_qty_avail =
        num_min(trade_product.qty_avail, 1) - tradeproduct_qty_sold;

    const handle_display_focus = async (
        el: EventTarget & HTMLButtonElement,
    ): Promise<void> => {
        try {
            //   if (media_uploads?.length) return;
            el.focus();
            el_c.scrollTo();
            el_id(
                `${id_pref}-control-${basis.index}-${trade_product.id}`,
            )?.classList.remove(`hidden`);
            els_id_pref_index(
                `${id_pref}-control`,
                basis.index,
                `not`,
            )?.forEach((el) => el.classList.add(`hidden`));
            els_id_pref_index(`${id_pref}-display`, basis.index)?.forEach(
                (el) => el.classList.add(`translate-y-12`),
            );
        } catch (e) {
            console.log(`(error) handle_display_focus `, e);
        }
    };

    const handle_display_blur = async (): Promise<void> => {
        try {
            els_id_pref_index(
                `${id_pref}-control`,
                basis.index,
                `not`,
            )?.forEach((el) => el.classList.add(`hidden`));
            els_id_pref_index(`${id_pref}-display`, basis.index)?.forEach(
                (el) => el.classList.remove(`translate-y-12`),
            );
        } catch (e) {
            console.log(`(error) handle_display_blur `, e);
        }
    };
</script>

<div
    bind:this={el_c}
    class={`relative flex flex-col w-full justify-center items-center`}
>
    <div
        id={`${id_pref}-control-${basis.index}-${trade_product.id}`}
        class={`hidden absolute top-0 left-0 flex flex-row h-12 w-full justify-center items-start el-re`}
    >
        <button
            class={`flex flex-row px-5 py-1 justify-center items-center bg-layer-1-surface layer-1-active-surface rounded-full`}
            on:click|stopPropagation={async () => {
                await route(`/models/trade-product/view`, [
                    [`id`, trade_product.id],
                ]);
            }}
        >
            <p class={`font-sans font-[500] text-layer-0-glyph`}>
                {`${$ls(`common.view`)}`}
            </p>
        </button>
    </div>
    <button
        id={`${id_pref}-display-${basis.index}-${trade_product.id}`}
        class={`flex flex-col min-h-[22rem] w-${$app_layout} justify-start items-start bg-layer-1-surface layer-1-focus-surface layer-1-focus-raise-less round-44 focus:translate-y-12`}
        on:click|stopPropagation={async ({ currentTarget: el }) => {
            handle_display_focus(el);
        }}
        on:blur={async () => {
            await handle_display_blur();
        }}
    >
        <div
            class={`flex flex-row min-h-[10rem] w-full justify-center items-center border-b-line border-b-layer-1-surface-edge`}
        >
            {#if media_uploads && media_uploads.length}
                <div
                    class={`relative flex flex-row h-full w-full justify-center items-center`}
                >
                    <div
                        class={`flex flex-row justify-start items-center overflow-x-auto scroll-hide`}
                    >
                        {#each media_uploads as li, li_i (li.id)}
                            <ImagePath
                                basis={{
                                    id: `${li.id}-index${li_i}`,
                                    path: `${li.res_base}/${li.res_path}.${li.mime_type}`,
                                    callback: async ({ currentTarget: el }) => {
                                        const el_id_next = el.id.replace(
                                            `-index${li_i}`,
                                            `-index${li_i + 1}`,
                                        );
                                        el_id(el_id_next)?.scrollIntoView(
                                            scroll_args.into_view,
                                        );
                                    },
                                }}
                            />
                        {/each}
                    </div>
                </div>
            {:else}
                <button
                    class={`group flex flex-row w-20 justify-center items-center`}
                    on:click|stopPropagation={async () => {}}
                >
                    <div
                        class={`relative flex flex-col w-full justify-start items-center`}
                    >
                        <div
                            class={`relative flex flex-row py-2 px-[0.8rem] justify-center items-center`}
                        >
                            <Glyph
                                basis={{
                                    classes: `text-layer-0-glyph group-active:text-layer-0-glyph_a el-re`,
                                    dim: `xl`,
                                    weight: `bold`,
                                    key: `camera`,
                                }}
                            />
                            <div
                                class={`absolute top-0 right-0 flex flex-row justify-center items-center`}
                            >
                                <Glyph
                                    basis={{
                                        classes: `text-layer-0-glyph group-active:text-layer-0-glyph_a el-re`,
                                        dim: `xs`,
                                        weight: `bold`,
                                        key: `plus`,
                                    }}
                                />
                            </div>
                        </div>
                        <div
                            class={`absolute -bottom-4 left-0 flex flex-row w-full justify-center items-center`}
                        >
                            <p
                                class={`font-sans font-[500] text-[1rem] text-layer-0-glyph group-active:text-layer-0-glyph_a el-re`}
                            >
                                {`${$ls(`icu.no_*`, { value: `${$ls(`common.photos`)}`.toLowerCase() })}`}
                            </p>
                        </div>
                    </div>
                </button>
            {/if}
        </div>
        {#if location_gcs}
            <div
                class={`flex flex-col min-h-[11rem] w-full pt-8 pb-12 justify-start items-start`}
            >
                <div
                    class={`flex flex-col w-full px-5 gap-6 justify-center items-center`}
                >
                    <div class={`grid grid-cols-12 w-full`}>
                        <LineEntryLabel
                            basis={{
                                classes_wrap: `col-span-7`,
                                value: `"${trade_product.title}"`,
                            }}
                        />
                        <LineEntryData
                            basis={{
                                classes_wrap: `col-span-5 justify-end capitalize`,
                                value: `${trade_product.key}`,
                            }}
                        />
                    </div>
                    <div class={`grid grid-cols-12 w-full gap-y-[2px]`}>
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`common.origin`)}`,
                                },
                                data: {
                                    value: `${fmt_geol_latitude(
                                        location_gcs.lat,
                                        `d`,
                                        4,
                                    )}, ${fmt_geol_longitude(
                                        location_gcs.lng,
                                        `d`,
                                        4,
                                    )}`,
                                },
                            }}
                        />
                        <LineEntryData
                            basis={{
                                classes_wrap: `col-span-12 justify-end capitalize`,
                                value: `${fmt_location_gcs(location_gcs, `city`)}`,
                            }}
                        />
                    </div>
                    <div class={`grid grid-cols-12 w-full gap-y-[2px]`}>
                        {#await parse_currency_price($locale, trade_product.price_currency, trade_product.price_amt) then price}
                            <LineEntriesBetween
                                basis={{
                                    classes: `col-span-12`,
                                    label: {
                                        value: `${$ls(`common.price`)}`,
                                    },
                                    data: {
                                        value: `${price ? fmt_currency_price(price) : ``} / ${`${$ls(`measurement.mass.unit.${trade_product.price_qty_unit}_ab`)}`}`,
                                    },
                                }}
                            />
                        {/await}
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`icu.*_order`, { value: `${$ls(`common.quantity`)}` })}`,
                                },
                                data: {
                                    value: `${trade_product.qty_amt} / ${`${$ls(`measurement.mass.unit.${trade_product.qty_unit}_ab`)}`}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`icu.*_available`, { value: `${$ls(`common.quantity`)}` })}`,
                                },
                                data: {
                                    value: `${tradeproduct_qty_avail} ${trade_product.qty_label || fmt_plural_agreement(tradeproduct_qty_avail, `${$ls(`common.bag`)}`, `${$ls(`common.bags`)}`)}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`icu.*_sold`, { value: `${$ls(`common.quantity`)}` })}`,
                                },
                                data: {
                                    value: `${tradeproduct_qty_sold} ${trade_product.qty_label || fmt_plural_agreement(tradeproduct_qty_sold, `${$ls(`common.bag`)}`, `${$ls(`common.bags`)}`)}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`common.lot`)}`,
                                },
                                data: {
                                    classes: `capitalize`,
                                    value: `${trade_product.lot}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`common.process`)}`,
                                },
                                data: {
                                    classes: `capitalize`,
                                    value: `${trade_product.process.replaceAll(`_`, ` `)}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`common.profile`)}`,
                                },
                                data: {
                                    classes: `capitalize`,
                                    value: `${trade_product.profile.replaceAll(`_`, ` `)}`,
                                },
                            }}
                        />
                        <LineEntriesBetween
                            basis={{
                                classes: `col-span-12`,
                                label: {
                                    value: `${$ls(`common.year`)}`,
                                },
                                data: {
                                    value: `${trade_product.year}`,
                                },
                            }}
                        />
                    </div>
                    <div class={`grid grid-cols-12 w-full gap-y-[2px]`}>
                        {#await parse_currency_price($locale, trade_product.price_currency, trade_product.price_amt * Math.floor(Math.max(trade_product.price_qty_amt, 1)) * mass_tf_str(trade_product.price_qty_unit, trade_product.qty_unit, trade_product.qty_amt)) then price}
                            <LineEntriesBetween
                                basis={{
                                    classes: `col-span-12`,
                                    label: {
                                        value: `${$ls(`icu.total_*`, { value: `${$ls(`common.value`)}` })}`,
                                    },
                                    data: {
                                        value: `${price ? fmt_currency_price(price) : ``} / ${`${$ls(`measurement.mass.unit.${trade_product.price_qty_unit}_ab`)}`}`,
                                    },
                                }}
                            />
                        {/await}
                    </div>
                </div>
            </div>
        {/if}
    </button>
</div>
