<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
<script lang="ts">
    import { db, dialog, geol } from "$lib/client";
    import MapChooseLocation from "$lib/components/map_choose_location.svelte";
    import { cfg } from "$lib/conf";
    import {
        kv_init_trade_product_fields,
        kv_validate_trade_product_fields,
    } from "$lib/utils/kv";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        mass_units,
        trade_product_form_fields,
        type LocationGcs,
        type TradeProductFormFields,
    } from "@radroots/models";
    import {
        ButtonGlyph,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        carousel_num,
        el_id,
        EntryLine,
        EntryMultiline,
        EntryOption,
        Fill,
        fmt_geol_latitude,
        fmt_geol_longitude,
        fmt_id,
        Glyph,
        InputElement,
        int_step,
        kv,
        LayoutTrellis,
        LayoutTrellisLine,
        LayoutView,
        locale,
        Nav,
        sleep,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import {
        fiat_currencies,
        fmt_trade_quantity_val,
        parse_currency_price,
        parse_trade_key,
        parse_trade_mass_tuple,
        trade_keys,
        trade_quantities,
        type CurrencyPrice,
        type GeolocationCoordinatesPoint,
        type TradeKey,
    } from "@radroots/utils";
    import { onMount } from "svelte";

    const trade_key_default: TradeKey = `coffee`;

    type CarouselParam = {
        label_prev?: string;
        label_next: string;
    };
    const carousel_param = new Map<number, CarouselParam>([
        [
            0,
            {
                label_next: `${$t(`icu.add_*`, { value: `${$t(`common.product`)}` })}`,
            },
        ],
        [
            1,
            {
                label_next: `${$t(`icu.add_*`, { value: `${$t(`common.location`)}` })}`,
            },
        ],
        [
            2,
            {
                label_next: `${$t(`icu.add_*`, { value: `${$t(`common.listing`)}` })}`,
            },
        ],
        [
            3,
            {
                label_next: `${$t(`common.preview`)}`,
            },
        ],
        [
            4,
            {
                label_next: `${$t(`common.publish`)}`,
            },
        ],
    ]);

    type View = `load` | `form_1`;
    let view: View = `load`;
    $: {
        view_effect<View>(view);
    }

    onMount(async () => {
        try {
            await kv.set(fmt_id(`price_amt`), `2`);
            await handle_view(`load`);
            carousel_index.set(0);
            carousel_index_max.set(carousel_param.size - 1);
            sel_trade_product_price_currency = "eur";
            sel_trade_product_price_qty_unit = "kg";
            sel_trade_product_qty_unit = "kg";
            await fetch_models_models_location_gcs();
        } catch (e) {
        } finally {
            await handle_view(`form_1`);
        }
    });

    $: {
        console.log(`$carousel_index `, $carousel_index);
        console.log(`$carousel_index_max `, $carousel_index_max);
    }

    let el_trellis_wrap_price: HTMLElement | null;

    let loading_submit = false;

    let show_sel_trade_product_key_other = false;
    let sel_trade_product_key: string = trade_key_default;
    $: sel_trade_product_key_parsed = parse_trade_key(sel_trade_product_key);
    $: ls_trade_product_quantities = sel_trade_product_key_parsed
        ? trade_quantities[sel_trade_product_key_parsed]
        : trade_quantities[trade_key_default];

    let sel_location_gcs_id = ``;
    let ls_models_location_gcs: LocationGcs[] = [];

    let map_choose_loc_loading = true;
    let map_choose_loc_geoc_point: GeolocationCoordinatesPoint =
        cfg.map.coords.default;
    let map_choose_loc_geoc_point_select: GeolocationCoordinatesPoint =
        cfg.map.coords.default;
    let map_choose_loc_geoc_point_select_geoc:
        | GeocoderReverseResult
        | undefined = undefined;

    let show_sel_trade_product_qty_tup_other = false;
    let sel_trade_product_qty_tup = ``;

    let sel_trade_product_price_currency = ``;
    let sel_trade_product_price_qty_unit = ``;

    let sel_trade_product_qty_unit = ``;

    let preview_trade_product: TradeProductFormFields | undefined = undefined;
    let preview_trade_product_qty_avail = 0;

    //let review_trade_product: TradeProductFormFields | undefined = undefined;
    //let review_location_gcs: LocationGcs | undefined = undefined;

    $: num_trade_product_qty_amt = preview_trade_product
        ? Number(preview_trade_product.qty_amt)
        : 0;

    $: num_trade_product_price_amt = preview_trade_product
        ? Number(preview_trade_product.price_amt)
        : 0;

    $: num_trade_product_price_total =
        num_trade_product_price_amt *
        num_trade_product_qty_amt *
        preview_trade_product_qty_avail;

    let preview_trade_product_price_currency: CurrencyPrice | undefined =
        undefined;
    $: preview_trade_product_price_currency =
        preview_trade_product && num_trade_product_price_amt > 0
            ? parse_currency_price(
                  $locale,
                  preview_trade_product.price_currency,
                  num_trade_product_price_amt,
              )
            : undefined;

    $: sel_trade_product_qty_tup = sel_trade_product_key_parsed
        ? fmt_trade_quantity_val(
              trade_quantities[sel_trade_product_key_parsed][0],
          )
        : fmt_trade_quantity_val(trade_quantities[trade_key_default][0]);

    $: if (ls_models_location_gcs.length && !sel_location_gcs_id) {
        sel_location_gcs_id = ls_models_location_gcs[0].id;
    }

    $: if (sel_location_gcs_id) {
        (async () => {
            try {
                await kv.set(fmt_id(`location_gcs_id`), sel_location_gcs_id);
            } catch (e) {}
        })();
    }

    $: if (sel_trade_product_qty_tup) {
        (async () => {
            try {
                const mass_tup = parse_trade_mass_tuple(
                    sel_trade_product_qty_tup,
                );
                if (mass_tup) {
                    await kv.set(fmt_id(`qty_amt`), mass_tup[0].toString());
                    await kv.set(fmt_id(`qty_unit`), mass_tup[1]);
                    await kv.set(fmt_id(`qty_label`), mass_tup[2]);
                    //@note
                    await kv.set(fmt_id(`qty_avail`), `1`);
                }
            } catch (e) {}
        })();
    }

    $: if (sel_trade_product_price_currency) {
        (async () => {
            try {
                await kv.set(
                    fmt_id(`price_currency`),
                    sel_trade_product_price_currency,
                );
            } catch (e) {}
        })();
    }

    carousel_index.subscribe(async (_carousel_index) => {
        switch (view) {
            case `form_1`: {
                switch (_carousel_index) {
                    case 1:
                        {
                            await handle_render_map_initial();
                        }
                        break;
                    case 2:
                        {
                            const kv_qty_avail = await kv.get(
                                fmt_id(`qty_avail`),
                            );
                            if (isNaN(kv_qty_avail))
                                preview_trade_product_qty_avail = 1;
                            else
                                preview_trade_product_qty_avail =
                                    Number(kv_qty_avail);
                        }
                        break;
                }
            }
        }
    });

    const handle_view = async (new_view: View): Promise<void> => {
        try {
            view = new_view;
        } catch (e) {
            console.log(`(error) handle_view `, e);
        }
    };

    const handle_back = async (num?: number): Promise<void> => {
        try {
            switch ($carousel_index) {
                default:
                    {
                        await carousel_dec(view);
                    }
                    break;
            }
            if (num) {
                carousel_num.set(num);
                carousel_index.set($carousel_index - (num - 1));
            }
        } catch (e) {
            console.log(`(error) handle_back `, e);
        }
    };

    const handle_continue = async (num?: number): Promise<void> => {
        try {
            switch ($carousel_index) {
                case 0:
                    {
                        const vals_1 = await kv_validate_trade_product_fields({
                            kv_pref: fmt_id(),
                            no_validation: [
                                `year`,
                                `price_qty_amt`,
                                `qty_avail`,
                                `summary`,
                                `title`,
                            ],
                        });
                        if (`err` in vals_1) {
                            await dialog.alert(
                                `${$t(`trade.product.fields.${vals_1.err}.err_invalid`, { default: `${$t(`icu.invalid_*`, { value: vals_1.err })}` })}`,
                            );
                            return;
                        }
                        if (!vals_1.year)
                            await kv.set(
                                fmt_id(`year`),
                                new Date().getFullYear().toString(),
                            );
                        if (!vals_1.price_qty_amt)
                            await kv.set(fmt_id(`price_qty_amt`), `1`);
                        if (!vals_1.qty_avail)
                            await kv.set(fmt_id(`qty_avail`), `1`);

                        const vals_2 = await kv_validate_trade_product_fields({
                            kv_pref: fmt_id(),
                            no_validation: [`summary`, `title`],
                        });
                        if (`err` in vals_2) {
                            await dialog.alert(
                                `${$t(`trade.product.fields.${vals_2.err}.err_invalid`, { default: `${$t(`icu.invalid_*`, { value: vals_2.err })}` })}`,
                            );
                            return;
                        }
                        preview_trade_product = vals_2;
                        await carousel_inc(view);
                    }
                    break;
                case 1:
                    {
                        await carousel_inc(view);
                    }
                    break;
                case 2:
                    {
                        await carousel_inc(view);
                    }
                    break;
                case 3:
                    {
                        const vals = await kv_validate_trade_product_fields({
                            kv_pref: fmt_id(),
                        });
                        if (`err` in vals) {
                            const el = el_id(fmt_id(`${vals.err}_wrap`));
                            await dialog.alert(
                                `${$t(`icu.enter_the_*`, { value: `${$t(`common.${vals.err}`)}`.toLowerCase() })}`,
                            );
                            el?.classList.add(`entry-layer-1-highlight`);
                            await sleep(cfg.delay.entry_focus);
                            el?.classList.remove(`entry-layer-1-highlight`);
                            return;
                        }
                        await carousel_inc(view);
                    }
                    break;
                case 4:
                    {
                        await submit();
                    }
                    break;
            }
            if (num) {
                carousel_num.set(num);
            }
        } catch (e) {
            console.log(`(error) handle_continue `, e);
        }
    };

    const toggle_show_key_other = async (visible: boolean): Promise<void> => {
        try {
            show_sel_trade_product_key_other = visible;
            if (visible) {
                await kv.set(fmt_id(`key`), ``);
            } else {
                sel_trade_product_key = trade_keys[0];
            }
        } catch (e) {
            console.log(`(error) toggle_show_key_other `, e);
        }
    };

    const toggle_show_qty_amt_other = async (
        visible: boolean,
    ): Promise<void> => {
        try {
            show_sel_trade_product_qty_tup_other = visible;
            if (visible) {
                sel_trade_product_price_qty_unit = mass_units[0];
                await kv.set(fmt_id(`qty_amt`), ``);
            } else {
                sel_trade_product_qty_tup = sel_trade_product_key_parsed
                    ? fmt_trade_quantity_val(
                          trade_quantities[sel_trade_product_key_parsed][0],
                      )
                    : ``;
            }
        } catch (e) {
            console.log(`(error) toggle_show_qty_amt_other `, e);
        }
    };

    const fetch_models_models_location_gcs = async (): Promise<void> => {
        try {
            const res = await db.location_gcs_get({
                list: [`all`],
            });
            if (`results` in res) ls_models_location_gcs = res.results;
        } catch (e) {
            console.log(`(error) fetch_models_models_location_gcs `, e);
        }
    };

    const handle_render_map_initial = async (): Promise<void> => {
        try {
            const geolc = await geol.current();
            if (!(`err` in geolc)) {
                map_choose_loc_geoc_point = {
                    ...geolc,
                };
                map_choose_loc_geoc_point_select = map_choose_loc_geoc_point;
            }
            await sleep(600);
        } catch (e) {
            console.log(`(error) handle_render_map_initial `, e);
        } finally {
            map_choose_loc_loading = false;
        }
    };

    const handle_map_reset = (): void => {
        map_choose_loc_geoc_point_select = map_choose_loc_geoc_point;
    };

    const submit = async (): Promise<void> => {
        try {
            loading_submit = true;
            console.log(`@todo submit`);
        } catch (e) {
            console.log(`(error) submit `, e);
        } finally {
            loading_submit = false;
        }
    };
</script>

<LayoutView>
    <div
        data-view={`load`}
        class={`flex flex-col h-full w-full justify-start items-start`}
    >
        <Fill />
    </div>
    <div
        data-view={`form_1`}
        class={`hidden flex flex-col h-full w-full justify-start items-center`}
    >
        <div
            data-carousel-container={`form_1`}
            class={`carousel-container flex h-full w-full ${view === `form_1` ? `fade-in` : ``}`}
        >
            <div
                data-carousel-item={`form_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis basis={{ classes: `gap-5` }}>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.product`)}`,
                            },
                            notify: show_sel_trade_product_key_other
                                ? {
                                      classes: `w-full justify-end`,
                                      label: {
                                          value: `${$t(`icu.show_*`, { value: `${$t(`common.options`)}` })}`,
                                      },
                                      callback: async () => {
                                          await toggle_show_key_other(false);
                                      },
                                  }
                                : undefined,
                        }}
                    >
                        {#if show_sel_trade_product_key_other}
                            <div
                                class={`relative flex flex-row w-full justify-center items-center rounded-2xl`}
                            >
                                <EntryLine
                                    basis={{
                                        el: {
                                            id: fmt_id(`title`),
                                            sync: true,
                                            sync_init: true,
                                            placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_title`, { value: `${$t(`common.listing`)}` })}`.toLowerCase() })}`,
                                            field: {
                                                charset:
                                                    trade_product_form_fields
                                                        .title.charset,
                                                validate:
                                                    trade_product_form_fields
                                                        .title.validation,
                                                validate_keypress: true,
                                            },
                                        },
                                    }}
                                />
                            </div>
                        {:else}
                            <EntryOption
                                bind:value={sel_trade_product_key}
                                basis={{
                                    id_wrap: fmt_id(`key_wrap`),
                                    id: fmt_id(`key`),
                                    classes: `font-mono-display`,
                                    sync: true,
                                    options: [
                                        ...trade_keys.map((i) => ({
                                            value: i,
                                            label: `${$t(`trade.product.key.${i}`, { default: i })}`,
                                        })),
                                        {
                                            value: `other`,
                                            label: `${$t(`common.other`)}`,
                                        },
                                    ],
                                    callback: async (val) => {
                                        el_id(
                                            fmt_id(`key_wrap`),
                                        )?.classList.remove(
                                            `entry-layer-1-highlight`,
                                        );
                                        if (val === `other`) {
                                            await toggle_show_key_other(true);
                                        }
                                    },
                                }}
                            />
                        {/if}
                    </LayoutTrellisLine>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.price`)}`,
                            },
                        }}
                    >
                        <div
                            bind:this={el_trellis_wrap_price}
                            id={fmt_id(`price_wrap`)}
                            tabindex={-1}
                            class={`relative el-re flex flex-row w-full pl-2 justify-between items-center h-form_line bg-layer-1-surface rounded-2xl`}
                        >
                            <InputElement
                                basis={{
                                    id: fmt_id(`price_amt`),
                                    layer: 1,
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_*`, { value: `${$t(`common.price`)}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.price_amt
                                                .charset,
                                        validate:
                                            trade_product_form_fields.price_amt
                                                .validation,
                                        validate_keypress: true,
                                    },
                                    callback: async ({ value, pass }) => {
                                        const lastchar =
                                            value[value.length - 1];
                                        const period_count =
                                            value.split(".").length - 1;
                                        if (
                                            (pass &&
                                                lastchar !== `.` &&
                                                period_count < 2) ||
                                            value.length < 1
                                        ) {
                                            el_trellis_wrap_price?.classList.remove(
                                                `entry-layer-1-highlight`,
                                            );
                                        } else {
                                            el_trellis_wrap_price?.classList.add(
                                                `entry-layer-1-highlight`,
                                            );
                                        }
                                    },
                                }}
                            />
                            <div
                                class={`flex flex-row gap-2 pl-3 pr-4 justify-end items-center bg-layer-1-surface rounded-r-2xl text-layer-1-glyph/70`}
                            >
                                <EntryOption
                                    bind:value={sel_trade_product_price_currency}
                                    basis={{
                                        id: fmt_id(`price_currency`),
                                        classes: `w-fit font-mono-display font-[500] text-lg`,
                                        layer: false,
                                        hide_arrows: true,
                                        sync: true,
                                        options: fiat_currencies.map((i) => ({
                                            value: i,
                                            label: `${$t(`currency.${i}.symbol`, { default: i })}`,
                                        })),
                                    }}
                                />
                                <p
                                    class={`font-sans font-[500] text-[1.1rem] -translate-y-[1px] scale-y-[110%]`}
                                >
                                    {`/`}
                                </p>
                                <EntryOption
                                    bind:value={sel_trade_product_price_qty_unit}
                                    basis={{
                                        id: fmt_id(`price_qty_unit`),
                                        classes: `w-fit font-mono-display font-[600] text-[0.95rem] leading-[1rem]`,
                                        layer: false,
                                        hide_arrows: true,
                                        sync: true,
                                        options: mass_units.map((i) => ({
                                            value: i,
                                            label: `${$t(`measurement.mass.unit.${i}_ab`, { default: i })}`,
                                        })),
                                    }}
                                />
                            </div>
                        </div>
                    </LayoutTrellisLine>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.quantity`)}`,
                            },
                            notify: show_sel_trade_product_qty_tup_other
                                ? {
                                      classes: `w-full justify-end`,
                                      label: {
                                          value: `${$t(`icu.show_*`, { value: `${$t(`common.options`)}` })}`,
                                      },
                                      callback: async () => {
                                          await toggle_show_qty_amt_other(
                                              false,
                                          );
                                      },
                                  }
                                : undefined,
                        }}
                    >
                        <div
                            id={fmt_id(`qty_wrap`)}
                            tabindex={-1}
                            class={`relative el-re flex flex-row w-full gap-3 justify-between items-center h-form_line bg-layer-1-surface text-layer-1-glyph rounded-2xl`}
                        >
                            {#if show_sel_trade_product_qty_tup_other}
                                <div
                                    class={`relative flex flex-row w-full justify-center items-center`}
                                >
                                    <EntryLine
                                        basis={{
                                            el: {
                                                id: fmt_id(`qty_amt`),
                                                layer: false,
                                                sync: true,
                                                placeholder: `${$t(`icu.enter_number_of_*_per_order`, { value: `${$t(`measurement.mass.unit.${sel_trade_product_price_qty_unit}_ab`, { default: sel_trade_product_price_qty_unit })}` })}`,
                                                field: {
                                                    charset:
                                                        trade_product_form_fields
                                                            .qty_amt.charset,
                                                    validate:
                                                        trade_product_form_fields
                                                            .qty_amt.validation,
                                                    validate_keypress: true,
                                                },
                                            },
                                        }}
                                    />
                                    <div
                                        class={`absolute top-0 right-0 flex flex-row h-full gap-2 justify-center items-center`}
                                    >
                                        <EntryOption
                                            bind:value={sel_trade_product_qty_unit}
                                            basis={{
                                                id: fmt_id(`qty_unit`),
                                                classes: `w-[3.5rem] text-layer-1-glyph font-[500]`,
                                                layer: false,
                                                sync: true,
                                                options: mass_units.map(
                                                    (i) => ({
                                                        value: i,
                                                        label: `${$t(`measurement.mass.unit.${i}_ab`, { default: i })}`,
                                                    }),
                                                ),
                                            }}
                                        />
                                    </div>
                                </div>
                            {:else}
                                <EntryOption
                                    bind:value={sel_trade_product_qty_tup}
                                    basis={{
                                        classes: `font-mono-display`,
                                        options: [
                                            ...ls_trade_product_quantities.map(
                                                (i) => ({
                                                    value: fmt_trade_quantity_val(
                                                        i,
                                                    ),
                                                    label: `${i.mass} ${$t(`measurement.mass.unit.${i.mass_unit}_ab`, { default: i.mass_unit })} ${i.label}`,
                                                }),
                                            ),
                                            {
                                                value: `other`,
                                                label: `${$t(`common.other`)}`,
                                            },
                                        ],
                                        callback: async (val) => {
                                            el_id(
                                                fmt_id(`qty_wrap`),
                                            )?.classList.remove(
                                                `entry-layer-1-highlight`,
                                            );
                                            if (val === `other`) {
                                                await toggle_show_qty_amt_other(
                                                    true,
                                                );
                                            } else {
                                                await kv.set(
                                                    fmt_id(`qty_avail`),
                                                    `1`,
                                                );
                                            }
                                        },
                                    }}
                                />
                            {/if}
                        </div>
                    </LayoutTrellisLine>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`model.trade_product.lot`)}`,
                            },
                        }}
                    >
                        <EntryLine
                            basis={{
                                el: {
                                    id: fmt_id(`lot`),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_*`, { value: `${$t(`common.lot_name`)}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.lot
                                                .charset,
                                        validate:
                                            trade_product_form_fields.lot
                                                .validation,
                                        validate_keypress: true,
                                    },
                                },
                            }}
                        />
                    </LayoutTrellisLine>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`model.trade_product.process`)}`,
                            },
                        }}
                    >
                        <EntryLine
                            basis={{
                                el: {
                                    id: fmt_id(`process`),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_*`, { value: `${$t(`icu.*_description`, { value: `${$t(`common.process`)}` })}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.process
                                                .charset,
                                        validate:
                                            trade_product_form_fields.process
                                                .validation,
                                        validate_keypress: true,
                                    },
                                },
                            }}
                        />
                    </LayoutTrellisLine>
                </LayoutTrellis>
            </div>
            <div
                data-carousel-item={`form_1`}
                class={`carousel-item flex flex-col w-full pt-4 gap-4 justify-start items-center`}
            >
                <MapChooseLocation
                    basis={{
                        classes_map: `h-[280px] w-full`,
                        loading: map_choose_loc_loading,
                    }}
                    bind:map_point_center={map_choose_loc_geoc_point}
                    bind:map_point_select={map_choose_loc_geoc_point_select}
                    bind:map_point_select_geoc={map_choose_loc_geoc_point_select_geoc}
                />
                <div
                    class={`grid grid-cols-12 flex flex-col w-full px-4 gap-1 justify-start items-center`}
                >
                    <div
                        class={`col-span-9 flex flex-row w-full justify-start items-center`}
                    >
                        <p
                            class={`w-[5.3rem] font-circ font-[500] text-layer-1-glyph-shade tracking-tight`}
                        >
                            {`${$t(`common.latitude`)}:`}
                        </p>
                        <p class={`font-circ font-[400] text-layer-1-glyph`}>
                            {fmt_geol_latitude(
                                map_choose_loc_geoc_point_select.lat,
                                `dms`,
                            )}
                        </p>
                    </div>
                    <div
                        class={`col-span-3 flex flex-row w-full justify-end items-center`}
                    >
                        <button
                            class={`group flex flex-row gap-[6px] justify-center items-center`}
                            on:click={async () => {
                                handle_map_reset();
                            }}
                        >
                            <p
                                class={`font-sg font-[500] text-layer-1-glyph tracking-tight group-active:opacity-80 el-re`}
                            >
                                {`${$t(`common.reset`)}`}
                            </p>
                            <Glyph
                                basis={{
                                    classes: `text-layer-1-glyph/80 group-active:opacity-60 el-re`,
                                    dim: `sm`,
                                    weight: `bold`,
                                    key: `arrow-counter-clockwise`,
                                }}
                            />
                        </button>
                    </div>
                    <div
                        class={`col-span-9 flex flex-row w-full justify-start items-center`}
                    >
                        <p
                            class={`w-[5.3rem] font-circ font-[500] text-layer-1-glyph-shade tracking-tight`}
                        >
                            {`${$t(`common.longitude`)}:`}
                        </p>
                        <p class={`font-circ font-[400] text-layer-1-glyph`}>
                            {fmt_geol_longitude(
                                map_choose_loc_geoc_point_select.lng,
                                `dms`,
                            )}
                        </p>
                    </div>
                    {#if map_choose_loc_geoc_point_select_geoc}
                        <div
                            class={`col-span-9 flex flex-row w-full justify-start items-center`}
                        >
                            <p
                                class={`w-[5.3rem] font-circ font-[500] text-layer-1-glyph-shade tracking-tight`}
                            >
                                {`${$t(`common.location`)}:`}
                            </p>
                            <p
                                class={`font-circ font-[400] text-layer-1-glyph`}
                            >
                                {`${map_choose_loc_geoc_point_select_geoc.admin1_name}, ${map_choose_loc_geoc_point_select_geoc.country_name}`}
                            </p>
                        </div>
                    {/if}
                </div>
            </div>
            <div
                data-carousel-item={`form_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis>
                    {#if preview_trade_product && preview_trade_product_price_currency}
                        <LayoutTrellisLine>
                            <div
                                class={`relative flex flex-col h-[36rem] w-full px-6 pb-4 gap-8 justify-around items-start bg-layer-0-surface rounded-touch`}
                            >
                                <div
                                    class={`flex flex-col flex-grow w-full justify-around items-center`}
                                >
                                    <div
                                        class={`flex flex-col w-full gap-2 justify-start items-start`}
                                    >
                                        <p
                                            class={`font-mono font-[400] text-[1.25rem] text-layer-1-glyph`}
                                        >
                                            {`${$t(`common.product`)}:`}
                                        </p>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row flex-grow gap-1 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-layer-2-glyph/40 pr-2`}
                                                >
                                                    {`-`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade capitalize`}
                                                >
                                                    {`${preview_trade_product.key}`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row justify-end items-center`}
                                            >
                                                <button
                                                    class={`flex flex-row justify-start items-center active:opacity-60 transition-all`}
                                                    on:click={async () => {
                                                        const el = el_id(
                                                            fmt_id(`key_wrap`),
                                                        );
                                                        el?.classList.add(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                        el?.focus();
                                                        await handle_back(2);
                                                        await sleep(1000);
                                                        el?.classList.remove(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                    }}
                                                >
                                                    <p
                                                        class={`label-sg font-[600] text-[1rem] text-layer-2-glyph`}
                                                    >
                                                        {`${$t(`common.edit`)}`}
                                                    </p>
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                    <div
                                        class={`flex flex-col w-full gap-2 justify-start items-start`}
                                    >
                                        <p
                                            class={`font-mono font-[400] text-[1.25rem] text-layer-1-glyph`}
                                        >
                                            {`${$t(`common.origin`)}:`}
                                        </p>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row flex-grow gap-1 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-layer-2-glyph/40 pr-2`}
                                                >
                                                    {`-`}
                                                </p>
                                                {#if map_choose_loc_geoc_point_select_geoc}
                                                    <p
                                                        class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade capitalize`}
                                                    >
                                                        {`${map_choose_loc_geoc_point_select_geoc.admin1_name}, ${map_choose_loc_geoc_point_select_geoc.country_id}`}
                                                    </p>
                                                {/if}
                                            </div>
                                            <div
                                                class={`flex flex-row justify-end items-center`}
                                            >
                                                <button
                                                    class={`flex flex-row justify-start items-center active:opacity-60 transition-all`}
                                                    on:click={async () => {
                                                        await handle_back();
                                                    }}
                                                >
                                                    <p
                                                        class={`label-sg font-[600] text-[1rem] text-layer-2-glyph`}
                                                    >
                                                        {`${$t(`common.edit`)}`}
                                                    </p>
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                    <div
                                        class={`flex flex-col w-full gap-2 justify-start items-start`}
                                    >
                                        <p
                                            class={`font-mono font-[400] text-[1.25rem] text-layer-1-glyph`}
                                        >
                                            {`${$t(`common.price`)}:`}
                                        </p>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row flex-grow gap-1 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-layer-2-glyph/40 pr-2`}
                                                >
                                                    {`-`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {preview_trade_product_price_currency.symbol}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {`${(preview_trade_product_price_currency.val_i + preview_trade_product_price_currency.val_f).toFixed(2)}`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade lowercase`}
                                                >
                                                    {`${$t(`common.per`)}`}
                                                </p>
                                                {#if preview_trade_product.price_qty_amt === `1`}
                                                    <p
                                                        class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                    >
                                                        {`${$t(`measurement.mass.unit.${preview_trade_product.price_qty_unit}_ab`, { default: preview_trade_product.price_qty_unit })}`}
                                                    </p>
                                                {/if}
                                            </div>
                                            <div
                                                class={`flex flex-row justify-end items-center`}
                                            >
                                                <button
                                                    class={`flex flex-row justify-start items-center active:opacity-60 transition-all`}
                                                    on:click={async () => {
                                                        const el = el_id(
                                                            fmt_id(
                                                                `price_wrap`,
                                                            ),
                                                        );
                                                        el?.classList.add(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                        el?.focus();
                                                        await handle_back(2);
                                                        await sleep(1000);
                                                        el?.classList.remove(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                    }}
                                                >
                                                    <p
                                                        class={`label-sg font-[600] text-[1rem] text-layer-2-glyph`}
                                                    >
                                                        {`${$t(`common.edit`)}`}
                                                    </p>
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                    <div
                                        class={`flex flex-col w-full gap-2 justify-start items-start`}
                                    >
                                        <p
                                            class={`font-mono font-[400] text-[1.25rem] text-layer-1-glyph`}
                                        >
                                            {`${$t(`common.quantity`)}:`}
                                        </p>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row flex-grow gap-1 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-layer-2-glyph/40 pr-2`}
                                                >
                                                    {`-`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {preview_trade_product.qty_amt}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {`${$t(`measurement.mass.unit.${preview_trade_product.qty_unit}_ab`, { default: preview_trade_product.qty_unit })}`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {preview_trade_product.qty_label}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row justify-end items-center`}
                                            >
                                                <button
                                                    class={`flex flex-row justify-start items-center active:opacity-60 transition-all`}
                                                    on:click={async () => {
                                                        const el = el_id(
                                                            fmt_id(`qty_wrap`),
                                                        );
                                                        el?.classList.add(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                        el?.focus();
                                                        await handle_back(2);
                                                        await sleep(1000);
                                                        el?.classList.remove(
                                                            `entry-layer-1-highlight`,
                                                        );
                                                    }}
                                                >
                                                    <p
                                                        class={`label-sg font-[600] text-[1rem] text-layer-2-glyph`}
                                                    >
                                                        {`${$t(`common.edit`)}`}
                                                    </p>
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                    <div
                                        class={`flex flex-col w-full gap-2 justify-start items-start`}
                                    >
                                        <p
                                            class={`font-mono font-[400] text-[1.25rem] text-layer-1-glyph`}
                                        >
                                            {`${$t(`icu.*_available`, { value: `${$t(`common.quantity`)}` })}:`}
                                        </p>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row flex-grow gap-1 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[600] text-layer-2-glyph/40 pr-2`}
                                                >
                                                    {`-`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {`${preview_trade_product_qty_avail} ${preview_trade_product.qty_label}`}
                                                </p>
                                                <p
                                                    class={`font-circ font-[400] text-[1.3rem] text-layer-1-glyph-shade`}
                                                >
                                                    {`(${(preview_trade_product_qty_avail * num_trade_product_qty_amt).toFixed(2)} ${$t(`measurement.mass.unit.${preview_trade_product.qty_unit}_ab`, { default: preview_trade_product.qty_unit })})`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row gap-[10px] justify-center items-center`}
                                            >
                                                <ButtonGlyph
                                                    basis={{
                                                        key: `arrow-down`,
                                                        dim: `xs+`,
                                                        weight: `bold`,
                                                        classes: `h-8 w-8 text-layer-2-glyph bg-layer-2-surface active:opacity-60 rounded-full transition-all`,
                                                        callback: async () => {
                                                            preview_trade_product_qty_avail =
                                                                int_step(
                                                                    preview_trade_product_qty_avail,
                                                                    `-`,
                                                                    1,
                                                                );
                                                            await kv.set(
                                                                fmt_id(
                                                                    `qty_avail`,
                                                                ),
                                                                preview_trade_product_qty_avail.toString(),
                                                            );
                                                        },
                                                    }}
                                                />
                                                <ButtonGlyph
                                                    basis={{
                                                        key: `arrow-up`,
                                                        dim: `xs+`,
                                                        weight: `bold`,
                                                        classes: `h-8 w-8 text-layer-2-glyph bg-layer-2-surface active:opacity-60 rounded-full transition-all`,
                                                        callback: async () => {
                                                            preview_trade_product_qty_avail =
                                                                int_step(
                                                                    preview_trade_product_qty_avail,
                                                                    `+`,
                                                                );
                                                            await kv.set(
                                                                fmt_id(
                                                                    `qty_avail`,
                                                                ),
                                                                preview_trade_product_qty_avail.toString(),
                                                            );
                                                        },
                                                    }}
                                                />
                                            </div>
                                        </div>
                                    </div>
                                </div>
                                <div
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <div
                                        class={` flex flex-row h-[1px] w-full justify-start items-center bg-layer-2-surface`}
                                    >
                                        <Fill />
                                    </div>
                                </div>
                                <div
                                    class={`flex flex-col w-full gap-4 justify-start items-start`}
                                >
                                    <div
                                        class={`flex flex-col w-full gap-4 justify-start items-start`}
                                    >
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[400] text-layer-1-glyph text-lg`}
                                                >
                                                    {`${`${$t(`common.quantity`)}`}:`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row gap-2 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[500] text-layer-1-glyph/80 text-lg`}
                                                >
                                                    {preview_trade_product_qty_avail}
                                                </p>
                                                <p
                                                    class={`font-mono font-[400] text-layer-2-glyph`}
                                                >
                                                    {`x`}
                                                </p>
                                                <p
                                                    class={`font-mono font-[500] text-layer-1-glyph/80 text-lg`}
                                                >
                                                    {`${num_trade_product_qty_amt.toFixed(2)} ${$t(`measurement.mass.unit.${preview_trade_product.qty_unit}_ab`, { default: preview_trade_product.qty_unit })}`}
                                                </p>
                                                <p
                                                    class={`font-mono font-[500] text-layer-1-glyph/80 text-lg`}
                                                >
                                                    {preview_trade_product.qty_label}
                                                </p>
                                            </div>
                                        </div>
                                        <div
                                            class={`flex flex-row w-full justify-between items-center`}
                                        >
                                            <div
                                                class={`flex flex-row justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[400] text-layer-1-glyph text-lg`}
                                                >
                                                    {`${$t(`icu.total_*`, { value: `${$t(`common.price`)}` })}:`}
                                                </p>
                                            </div>
                                            <div
                                                class={`flex flex-row gap-2 justify-start items-center`}
                                            >
                                                <p
                                                    class={`font-mono font-[400] text-layer-2-glyph text-lg`}
                                                >
                                                    {preview_trade_product_price_currency.symbol}
                                                </p>
                                                <p
                                                    class={`font-mono font-[600] text-layer-1-glyph/80 text-lg`}
                                                >
                                                    {num_trade_product_price_total.toFixed(
                                                        2,
                                                    )}
                                                </p>
                                                <p
                                                    class={`font-mono font-[600] text-layer-1-glyph/80 text-lg uppercase`}
                                                >
                                                    {preview_trade_product_price_currency.currency}
                                                </p>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </LayoutTrellisLine>
                    {/if}
                </LayoutTrellis>
            </div>
            <div
                data-carousel-item={`form_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.title`)}`,
                            },
                        }}
                    >
                        <EntryLine
                            basis={{
                                id_wrap: fmt_id(`title_wrap`),
                                el: {
                                    id: fmt_id(`title`),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_title`, { value: `${$t(`common.listing`)}` })}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.title
                                                .charset,
                                        validate:
                                            trade_product_form_fields.title
                                                .validation,
                                        validate_keypress: true,
                                    },
                                },
                            }}
                        />
                    </LayoutTrellisLine>
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.summary`)}`,
                            },
                        }}
                    >
                        <EntryMultiline
                            basis={{
                                id_wrap: fmt_id(`summary_wrap`),
                                el: {
                                    classes: `h-[20rem]`,
                                    id: fmt_id(`summary`),
                                    sync: true,
                                    sync_init: true,
                                    placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_summary`, { value: `${$t(`common.listing`)}` })}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.summary
                                                .charset,
                                        validate:
                                            trade_product_form_fields.summary
                                                .validation,
                                        validate_keypress: true,
                                    },
                                },
                            }}
                        />
                    </LayoutTrellisLine>
                </LayoutTrellis>
            </div>
            <div
                data-carousel-item={`form_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis>
                    <p class={`font-sans font-[400] text-layer-0-glyph`}>
                        {`@todo add preview`}
                    </p>
                </LayoutTrellis>
            </div>
        </div>
    </div>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.back`)}`,
            route: `/models/trade-product`,
            prevent_route:
                $carousel_index > 0
                    ? {
                          callback: async () => {
                              await handle_back();
                          },
                      }
                    : undefined,
            callback: async () => {
                await kv_init_trade_product_fields(fmt_id());
            },
        },
        title: {
            label: {
                value: `${$t(`common.product`)}`,
            },
            callback: async () => {},
        },
        option: {
            loading: loading_submit,
            label: {
                value:
                    $carousel_num > 1
                        ? `${$t(`common.return`)}`
                        : carousel_param.get($carousel_index)?.label_next || ``,
                classes: `text-layer-1-glyph-hl`,
                glyph:
                    $carousel_num > 1
                        ? {
                              key: `caret-right`,
                              classes: `text-layer-1-glyph-hl`,
                          }
                        : undefined,
            },
            callback: async () => {
                if ($carousel_index === $carousel_index_max) await submit();
                else await handle_continue();
            },
        },
    }}
/>
