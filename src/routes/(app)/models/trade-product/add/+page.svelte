<script lang="ts">
    import { dialog } from "$lib/client";
    import ImageUploadDisplay from "$lib/components/image_upload_display.svelte";
    import ImageUploadRow from "$lib/components/image_upload_row.svelte";
    import {
        kv_init_trade_product_fields,
        validate_trade_product_fields,
    } from "$lib/utils/kv";
    import { mass_units, trade_product_form_fields } from "@radroots/models";
    import {
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        carousel_num,
        el_id,
        EntryLine,
        EntryMultiline,
        EntrySelect,
        EntryWrap,
        fmt_id,
        InputElement,
        kv,
        LayoutTrellis,
        LayoutTrellisLine,
        LayoutView,
        Nav,
        price_locale_fmt,
        SelectElement,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import {
        fiat_currencies,
        fmt_trade_quantity_sel_val,
        parse_trade_key,
        trade,
        trade_keys,
        type TradeKey,
    } from "@radroots/utils";
    import { onMount } from "svelte";
    import { writable } from "svelte/store";
    import { fade } from "svelte/transition";

    type CarouselParam = {
        label_prev?: string;
        label_next: string;
    };
    const page_param: {
        trade_product: {
            key_default: TradeKey;
        };
        carousel: Map<number, CarouselParam>;
    } = {
        carousel: new Map<number, CarouselParam>([
            [
                0,
                {
                    label_next: `${$t(`common.add`)}`,
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
        ]),
        trade_product: {
            key_default: `coffee`,
        },
    };

    type View = `main` | `finish`;
    let view: View = `main`;
    $: {
        view_effect<View>(view);
    }

    let loading_submit = false;

    let photo_add_loading = false;
    let photo_add_list: { file_path: string }[] = [];

    let tradeproduct_key_sel_toggle = false;
    const tradeproduct_key_sel = writable<string>(``);
    $: tradeproduct_key_parsed = parse_trade_key($tradeproduct_key_sel);
    $: ls_trade_product_quantities = tradeproduct_key_parsed
        ? trade.key[tradeproduct_key_parsed].quantity
        : trade.default.quantity;

    const tradeproduct_price_curr_sel = writable<string>(``);
    const tradeproduct_price_qty_unit_sel = writable<string>(``);

    const tradeproduct_qty_unit_tup_sel = writable<string>(``);
    let tradeproduct_qty_unit_tup_sel_toggle = false;

    const tradeproduct_process_sel = writable<string>(``);
    let tradeproduct_process_sel_toggle = false;
    $: ls_trade_product_processes = tradeproduct_key_parsed
        ? trade.key[tradeproduct_key_parsed].process
        : [];

    onMount(async () => {
        try {
            await kv_init();
            await handle_view(`main`);
            carousel_index.set(0);
            carousel_index_max.set(page_param.carousel.size - 1);
            tradeproduct_price_curr_sel.set(`eur`);
            tradeproduct_price_qty_unit_sel.set(`kg`);
            await setup_test();
        } catch (e) {
        } finally {
            await handle_view(`main`);
        }
    });

    tradeproduct_key_sel.subscribe(async (_tradeproduct_key_sel) => {
        await kv.set(fmt_id(`key`), _tradeproduct_key_sel);
    });

    $: if (tradeproduct_key_parsed) {
        tradeproduct_qty_unit_tup_sel.set(
            fmt_trade_quantity_sel_val(
                trade.key[tradeproduct_key_parsed].quantity[0],
            ),
        );
        tradeproduct_process_sel.set(
            trade.key[tradeproduct_key_parsed].process[0],
        );
    }

    tradeproduct_price_curr_sel.subscribe(
        async (_tradeproduct_price_curr_sel) => {
            await kv.set(
                fmt_id(`price_currency`),
                _tradeproduct_price_curr_sel,
            );
        },
    );

    tradeproduct_price_qty_unit_sel.subscribe(
        async (_tradeproduct_price_qty_unit_sel) => {
            await kv.set(
                fmt_id(`price_qty_unit`),
                _tradeproduct_price_qty_unit_sel,
            );
        },
    );

    tradeproduct_qty_unit_tup_sel.subscribe(
        async (_tradeproduct_qty_unit_tup_sel) => {
            await kv.set(fmt_id(`qty_unit`), _tradeproduct_qty_unit_tup_sel);
        },
    );

    const setup_test = async (): Promise<void> => {
        try {
            tradeproduct_key_sel.set(page_param.trade_product.key_default);

            await kv.set(
                fmt_id(`summary`),
                [`This is the first line`, `This is the another line`].join(
                    `.\n`,
                ),
            );
        } catch (e) {
            console.log(`(error) setup_test `, e);
        }
    };

    const kv_init = async (): Promise<void> => {
        try {
            const kv_pref = fmt_id();
            const range = Keyva.prefix(kv_pref);
            const kv_list = await kv.each({ range }, `keys`);
            await Promise.all(kv_list.map((k) => kv.set(k, ``)));
        } catch (e) {
            console.log(`(error) kv_init `, e);
        }
    };

    const handle_view = async (new_view: View): Promise<void> => {
        try {
            view = new_view;
        } catch (e) {
            console.log(`(error) handle_view `, e);
        }
    };

    const toggle_tradeproduct_key = async (
        visible_input: boolean,
    ): Promise<void> => {
        try {
            tradeproduct_key_sel_toggle = visible_input;
            if (visible_input) {
                tradeproduct_key_sel.set(``);
            } else {
                //@todo tradeproduct_key_sel = trade_keys[0];
            }
        } catch (e) {
            console.log(`(error) toggle_tradeproduct_key `, e);
        }
    };

    const toggle_tradeproduct_qty_amt = async (
        visible_input: boolean,
    ): Promise<void> => {
        try {
            tradeproduct_qty_unit_tup_sel_toggle = visible_input;
            if (visible_input) {
                tradeproduct_qty_unit_tup_sel.set(mass_units[0]);
            } else {
                $tradeproduct_qty_unit_tup_sel = tradeproduct_key_parsed
                    ? fmt_trade_quantity_sel_val(
                          trade.key[tradeproduct_key_parsed].quantity[0],
                      )
                    : ``;
            }
        } catch (e) {
            console.log(`(error) toggle_tradeproduct_qty_amt `, e);
        }
    };

    const toggle_tradeproduct_process = async (
        visible_input: boolean,
    ): Promise<void> => {
        try {
            tradeproduct_process_sel_toggle = visible_input;
            if (visible_input) {
                tradeproduct_process_sel.set(``);
            } else {
                tradeproduct_process_sel.set(
                    ls_trade_product_processes.length
                        ? ls_trade_product_processes[0]
                        : ``,
                );
            }
        } catch (e) {
            console.log(`(error) toggle_tradeproduct_process `, e);
        }
    };

    const handle_photo_add = async (): Promise<void> => {
        try {
            photo_add_loading = true;
            const photo_paths = await dialog.open_photos();
            if (!photo_paths) {
                return; //@todo
            }
            const file_path = photo_paths.results[0];
            photo_add_list = [
                ...photo_add_list,
                {
                    file_path,
                },
            ];
        } catch (e) {
            console.log(`(error) handle_photo_add `, e);
        } finally {
            photo_add_loading = false;
        }
    };

    const handle_back = async (carousel_offset?: number): Promise<void> => {
        try {
            switch ($carousel_index) {
                default:
                    {
                        await carousel_dec(view);
                    }
                    break;
            }
            if (carousel_offset) {
                carousel_num.set(carousel_offset);
                carousel_index.set($carousel_index - (carousel_offset - 1));
            }
        } catch (e) {
            console.log(`(error) handle_back `, e);
        }
    };

    const handle_continue = async (): Promise<void> => {
        try {
            switch ($carousel_index) {
                case 0:
                    {
                        const validate_fields =
                            await validate_trade_product_fields({
                                kv_pref: fmt_id(),
                                fields: [`key`, `summary`],
                            });
                        if (`err` in validate_fields) {
                            await dialog.alert(
                                `${$t(`icu.enter_a_*`, { value: `${$t(`model.trade_product.${validate_fields.err}`)}`.toLowerCase() })}`,
                            );
                            return;
                        }

                        if (photo_add_list.length < 1) {
                            await dialog.alert(`A primary photo is required`);
                            return; //@todo
                        }

                        await carousel_inc(view);
                    }
                    break;
                case 1:
                    {
                        const validate_fields =
                            await validate_trade_product_fields({
                                kv_pref: fmt_id(),
                                fields: [
                                    `price_amt`,
                                    `price_currency`,
                                    `price_qty_unit`,
                                ],
                            });
                        if (`err` in validate_fields) {
                            await dialog.alert(
                                `${$t(`icu.enter_the_*`, { value: `${$t(`model.trade_product.${validate_fields.err}`)}`.toLowerCase() })}`,
                            );
                            return;
                        }
                        console.log(
                            JSON.stringify(validate_fields, null, 4),
                            `validate_fields`,
                        );
                    }
                    break;
            }
        } catch (e) {
            console.log(`(error) handle_continue `, e);
        }
    };

    const submit = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) submit `, e);
        }
    };
</script>

<LayoutView>
    <div
        in:fade={{ delay: 100, duration: 200 }}
        out:fade={{ delay: 0, duration: 200 }}
        data-carousel-container={`main`}
        class={`carousel-container flex h-full w-full`}
    >
        <div
            data-carousel-item={`main`}
            class={`carousel-item flex flex-col w-full pt-4 justify-start items-center`}
        >
            <LayoutTrellis>
                <ImageUploadRow
                    basis={{
                        loading: photo_add_loading,
                        list: photo_add_list,
                        callback_add: handle_photo_add,
                    }}
                />
                <LayoutTrellisLine
                    basis={{
                        label: {
                            value: `${$t(`common.product`)}`,
                        },
                        notify: tradeproduct_key_sel_toggle
                            ? {
                                  label: {
                                      value: `${$t(`common.back`)}`,
                                  },
                                  glyph: {
                                      key: `selection-foreground`,
                                      weight: `bold`,
                                      dim: `xs`,
                                  },
                                  callback: async () => {
                                      const kv_other = await kv.get(
                                          fmt_id(`key`),
                                      );
                                      if (kv_other) {
                                          const confirm = await dialog.confirm({
                                              message: `${$t(`icu.the_current_entry_*_will_be_deleted`, { value: kv_other })}. ${$t(`common.do_you_want_to_continue_q`)}`,
                                          });
                                          if (confirm === false) return;
                                      }
                                      await toggle_tradeproduct_key(false);
                                  },
                              }
                            : undefined,
                    }}
                >
                    {#if !tradeproduct_key_sel_toggle}
                        <EntrySelect
                            bind:value={$tradeproduct_key_sel}
                            basis={{
                                wrap: {
                                    id: fmt_id(`key_wrap`),
                                },
                                el: {
                                    id: fmt_id(`key`),
                                    layer: 1,
                                    options: [
                                        {
                                            entries: [
                                                {
                                                    value: ``,
                                                    label: `Choose product`,
                                                    disabled: true,
                                                },
                                                ...trade_keys.map((i) => ({
                                                    value: i,
                                                    label: `${$t(`trade.product.key.${i}.name`, { default: i })}`,
                                                })),
                                                {
                                                    value: `other`,
                                                    label: `${$t(`common.other`)}`,
                                                },
                                            ],
                                        },
                                    ],
                                    callback: async (opt) => {
                                        const el = el_id(fmt_id(`key_wrap`));
                                        el?.classList.remove(
                                            `entry-layer-1-highlight`,
                                        );
                                        if (opt.value === `other`) {
                                            await toggle_tradeproduct_key(true);
                                        }
                                    },
                                },
                            }}
                        />
                    {:else}
                        <EntryLine
                            basis={{
                                wrap: {
                                    id: fmt_id(`key_wrap`),
                                },
                                el: {
                                    classes: `fade-in-long`,
                                    id: fmt_id(`key`),
                                    sync: true,
                                    placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_name`, { value: `${$t(`common.product`)}` })}`.toLowerCase() })}`,
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
                    {/if}
                </LayoutTrellisLine>
                <LayoutTrellisLine
                    basis={{
                        label: {
                            value: `${$t(`common.description`)}`,
                        },
                    }}
                >
                    <EntryMultiline
                        basis={{
                            wrap: {
                                id: fmt_id(`summary_wrap`),
                            },
                            el: {
                                classes: `h-[14rem]`,
                                id: fmt_id(`summary`),
                                sync: true,
                                placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_description`, { value: `${$t(`common.listing`)}` })}`.toLowerCase() })}`,
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
            data-carousel-item={`main`}
            class={`carousel-item flex flex-col w-full pt-4 justify-start items-center`}
        >
            <ImageUploadDisplay
                basis={{
                    loading: photo_add_loading,
                    list: photo_add_list,
                    callback_add: handle_photo_add,
                }}
            />
            <LayoutTrellis>
                <LayoutTrellisLine
                    basis={{
                        label: {
                            value: `${$t(`icu.*_price`, { value: `${$t(`common.product`)}` })}`,
                        },
                    }}
                >
                    <EntryWrap
                        basis={{
                            classes: `pl-3`,
                            id: fmt_id(`price_wrap`),
                        }}
                    >
                        <div
                            class={`flex flex-row justify-start items-center pr-3`}
                        >
                            <SelectElement
                                bind:value={$tradeproduct_price_curr_sel}
                                basis={{
                                    id: fmt_id(`price_currency`),
                                    classes: `w-fit font-circ font-[500] text-[1.2rem] -translate-y-[1px]`,
                                    layer: false,
                                    options: [
                                        {
                                            entries: fiat_currencies.map(
                                                (i) => ({
                                                    value: `${i}`,
                                                    label: `${$t(`currency.${i}.symbol`, { default: i })}`,
                                                }),
                                            ),
                                        },
                                    ],
                                }}
                            />
                        </div>
                        <InputElement
                            basis={{
                                id: fmt_id(`price_amt`),
                                layer: 1,
                                sync: true,
                                sync_init: true,
                                placeholder: `${$t(`common.price`)}`,
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
                                    const lastchar = value[value.length - 1];
                                    const period_count =
                                        value.split(".").length - 1;
                                    if (
                                        (pass &&
                                            lastchar !== `.` &&
                                            period_count < 2) ||
                                        value.length < 1
                                    ) {
                                        const el = el_id(fmt_id(`price_wrap`));
                                        el?.classList.remove(
                                            `entry-layer-1-highlight`,
                                        );
                                    } else {
                                        const el = el_id(fmt_id(`price_wrap`));
                                        el?.classList.add(
                                            `entry-layer-1-highlight`,
                                        );
                                    }
                                },
                                callback_blur: async ({ el }) => {
                                    if (!el.value) return;
                                    el.value = price_locale_fmt(
                                        $tradeproduct_price_curr_sel,
                                        el.value,
                                    ).slice(1);
                                },
                            }}
                        />
                        <div
                            class={`flex flex-row gap-2 justify-end items-center text-layer-1-glyph/70`}
                        >
                            <p class={`font-circ font-[500] text-[1.05rem]`}>
                                {`${$t(`common.per`)}`}
                            </p>
                            <SelectElement
                                bind:value={$tradeproduct_price_qty_unit_sel}
                                basis={{
                                    id: fmt_id(`price_qty_unit`),
                                    classes: `w-fit font-circ font-[500] text-[1.05rem]`,
                                    layer: false,
                                    options: [
                                        {
                                            entries: mass_units.map((i) => ({
                                                value: i,
                                                label: `${$t(`measurement.mass.unit.${i}_ab`, { default: i })}`.toLowerCase(),
                                            })),
                                        },
                                    ],
                                }}
                            />
                        </div>
                    </EntryWrap>
                </LayoutTrellisLine>
                <LayoutTrellisLine
                    basis={{
                        label: {
                            value: `${$t(`common.quantity`)}`,
                        },
                        notify: tradeproduct_qty_unit_tup_sel_toggle
                            ? {
                                  label: {
                                      value: `${$t(`common.back`)}`,
                                  },
                                  glyph: {
                                      key: `selection-foreground`,
                                      weight: `bold`,
                                      dim: `xs`,
                                  },
                                  callback: async () => {
                                      await toggle_tradeproduct_qty_amt(false);
                                  },
                              }
                            : undefined,
                    }}
                >
                    {#if !tradeproduct_qty_unit_tup_sel_toggle}
                        <EntrySelect
                            bind:value={$tradeproduct_qty_unit_tup_sel}
                            basis={{
                                wrap: {
                                    id: fmt_id(`qty_wrap`),
                                },
                                el: {
                                    layer: 1,
                                    options: [
                                        {
                                            entries: [
                                                ...ls_trade_product_quantities.map(
                                                    (i) => ({
                                                        value: fmt_trade_quantity_sel_val(
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
                                        },
                                    ],
                                    callback: async ({ value }) => {
                                        el_id(
                                            fmt_id(`qty_wrap`),
                                        )?.classList.remove(
                                            `entry-layer-1-highlight`,
                                        );
                                        if (value === `other`) {
                                            await toggle_tradeproduct_qty_amt(
                                                true,
                                            );
                                        } else {
                                            await kv.set(
                                                fmt_id(`qty_avail`),
                                                `1`,
                                            );
                                        }
                                    },
                                },
                            }}
                        />
                    {:else}
                        <EntryWrap
                            basis={{
                                id: fmt_id(`qty_wrap`),
                            }}
                        >
                            <InputElement
                                basis={{
                                    id: fmt_id(`qty_amt`),
                                    layer: 1,
                                    sync: true,
                                    placeholder: `${$t(`icu.enter_*_per_order`, { value: `${$t(`common.quantity`)}`.toLowerCase() })}`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.qty_amt
                                                .charset,
                                        validate:
                                            trade_product_form_fields.qty_amt
                                                .validation,
                                        validate_keypress: true,
                                    },
                                }}
                            />
                            <div
                                class={`absolute top-0 right-0 flex flex-row h-full pr-4 justify-end items-center`}
                            >
                                <SelectElement
                                    bind:value={$tradeproduct_price_qty_unit_sel}
                                    basis={{
                                        id: fmt_id(`qty_unit`),
                                        classes: `w-fit font-circ font-[500] text-[1.1rem] text-layer-1-glyph/70`,
                                        layer: false,
                                        options: [
                                            {
                                                entries: mass_units.map(
                                                    (i) => ({
                                                        value: i,
                                                        label: `${$t(`measurement.mass.unit.${i}_ab`, { default: i })}`,
                                                    }),
                                                ),
                                            },
                                        ],
                                    }}
                                />
                            </div>
                        </EntryWrap>
                    {/if}
                </LayoutTrellisLine>
                <LayoutTrellisLine
                    basis={{
                        label: {
                            value: `${$t(`common.process`)}`,
                        },
                        notify: tradeproduct_process_sel_toggle
                            ? {
                                  label: {
                                      value: `${$t(`common.back`)}`,
                                  },
                                  glyph: {
                                      key: `selection-foreground`,
                                      weight: `bold`,
                                      dim: `xs`,
                                  },
                                  glyph_last: true,
                                  callback: async () => {
                                      await toggle_tradeproduct_process(false);
                                  },
                              }
                            : undefined,
                    }}
                >
                    {#if !tradeproduct_process_sel_toggle}
                        <EntrySelect
                            bind:value={$tradeproduct_process_sel}
                            basis={{
                                wrap: {
                                    id: fmt_id(`process_wrap`),
                                },
                                el: {
                                    layer: 1,
                                    options: [
                                        {
                                            entries: [
                                                ...ls_trade_product_processes.map(
                                                    (i) => ({
                                                        value: i,
                                                        label: `${$t(`trade.product.key.${tradeproduct_key_parsed}.process.${i}`)}`,
                                                    }),
                                                ),
                                                {
                                                    value: `other`,
                                                    label: `${$t(`common.other`)}`,
                                                },
                                            ],
                                        },
                                    ],
                                    callback: async ({ value }) => {
                                        el_id(
                                            fmt_id(`process_wrap`),
                                        )?.classList.remove(
                                            `entry-layer-1-highlight`,
                                        );
                                        if (value === `other`) {
                                            await toggle_tradeproduct_process(
                                                true,
                                            );
                                        }
                                    },
                                },
                            }}
                        />
                    {:else}
                        <EntryWrap
                            basis={{
                                id: fmt_id(`process_wrap`),
                            }}
                        >
                            <InputElement
                                basis={{
                                    id: fmt_id(`process`),
                                    layer: 1,
                                    sync: true,
                                    placeholder: `Enter the process`,
                                    field: {
                                        charset:
                                            trade_product_form_fields.process
                                                .charset,
                                        validate:
                                            trade_product_form_fields.process
                                                .validation,
                                        validate_keypress: true,
                                    },
                                }}
                            />
                        </EntryWrap>
                    {/if}
                </LayoutTrellisLine>
            </LayoutTrellis>
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
                value: `${$t(`icu.new_*`, { value: `${$t(`common.product`)}` })}`,
            },
            callback: async () => {},
        },
        option: {
            loading: loading_submit,
            label: {
                value:
                    $carousel_num > 1
                        ? `${$t(`common.return`)}`
                        : page_param.carousel.get($carousel_index)
                              ?.label_next || ``,
            },
            callback: async () => {
                if ($carousel_index === $carousel_index_max) await submit();
                else await handle_continue();
            },
        },
    }}
/>
