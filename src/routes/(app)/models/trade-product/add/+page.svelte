<script lang="ts">
    import { dialog, geol } from "$lib/client";
    import ImageUploadMulti from "$lib/components/image_upload_multi.svelte";
    import MapPointSelectEnvelope from "$lib/components/map_point_select_envelope.svelte";
    import { tradeproduct_init_kv } from "$lib/utils/trade_product";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import { trade_product_form_fields } from "@radroots/models";
    import {
        carousel_index,
        carousel_index_max,
        carousel_num,
        el_id,
        EntryLine,
        EntrySelect,
        fmt_id,
        kv,
        layout_view_cover,
        LayoutTrellis,
        LayoutTrellisLine,
        LayoutView,
        Nav,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import {
        parse_trade_key,
        trade,
        trade_keys,
        type GeolocationCoordinatesPoint,
        type TradeKey,
    } from "@radroots/utils";
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";

    type CarouselParam = {
        label_prev?: string;
        label_next: string;
    };
    const page_param: {
        carousel: Record<View, Map<number, CarouselParam>>;
        trade_product: {
            key_default: TradeKey;
        };
    } = {
        carousel: {
            fl_1: new Map<number, CarouselParam>([
                [
                    0,
                    {
                        label_next: `${$t(`common.add`)}`,
                    },
                ],
                [
                    1,
                    {
                        label_next: `${$t(`common.location`)}`,
                    },
                ],
                [
                    2,
                    {
                        label_next: `*`,
                    },
                ],
                [
                    3,
                    {
                        label_next: `Post`,
                    },
                ],
            ]),
        },
        trade_product: {
            key_default: `coffee`,
        },
    };

    let view_init: View = `fl_1`;
    type View = `fl_1`;
    let view: View = view_init;
    $: {
        view_effect<View>(view);
    }

    let load_submit = false;

    let tradeproduct_photo_paths: string[] = [];

    let tradeproduct_key_sel_toggle = false;
    let tradeproduct_key_sel = ``;
    $: tradeproduct_key_parsed = parse_trade_key(tradeproduct_key_sel);
    $: tradeproduct_key_quantities_list = tradeproduct_key_parsed
        ? trade.key[tradeproduct_key_parsed].quantity
        : trade.default.quantity;

    let tradeproduct_process_sel = ``;
    let tradeproduct_process_sel_toggle = false;
    $: tradeproduct_process_list = tradeproduct_key_parsed
        ? trade.key[tradeproduct_key_parsed].process
        : [];

    let tradeproduct_location_sel = ``;
    let tradeproduct_location_select_vis = false;
    let tradeproduct_location_select_point:
        | GeolocationCoordinatesPoint
        | undefined = undefined;
    let tradeproduct_location_select_geoc: GeocoderReverseResult | undefined =
        undefined;

    onMount(async () => {
        try {
            await handle_view(view_init);
            layout_view_cover.set(false);
            carousel_index.set(0);
            carousel_index_max.set(page_param.carousel[view].size - 1);
            await setup_tests();
        } catch (e) {
        } finally {
        }
    });

    const setup_tests = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) setup_tests `, e);
        }
    };

    const handle_view = async (view_new: View): Promise<void> => {
        try {
            carousel_index.set(0);
            view = view_new;
        } catch (e) {
            console.log(`(error) handle_view `, e);
        }
    };

    const handle_tradeproduct_key_toggle = async (
        vis_input: boolean,
    ): Promise<void> => {
        try {
            const kv_curr = await kv.get(fmt_id(`key`));
            if (kv_curr) {
                const confirm = await dialog.confirm({
                    message: `${$t(`icu.the_current_entry_*_will_be_deleted`, { value: kv_curr })}. ${$t(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
            }
            tradeproduct_key_sel_toggle = vis_input;
            if (vis_input) tradeproduct_key_sel = ``;
        } catch (e) {
            console.log(`(error) handle_tradeproduct_key_toggle `, e);
        }
    };

    const handle_tradeproduct_process_toggle = async (
        vis_input: boolean,
    ): Promise<void> => {
        try {
            const kv_curr = await kv.get(fmt_id(`process`));
            if (kv_curr) {
                const confirm = await dialog.confirm({
                    message: `${$t(`icu.the_current_entry_*_will_be_deleted`, { value: kv_curr })}. ${$t(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
            }
            tradeproduct_process_sel_toggle = vis_input;
            if (vis_input) tradeproduct_process_sel = ``;
        } catch (e) {
            console.log(`(error) handle_tradeproduct_process_toggle `, e);
        }
    };

    const handle_tradeproduct_location_sel_map_select =
        async (): Promise<void> => {
            try {
                const geolc = await geol.current();
                if (`err` in geolc) return; //@todo
                tradeproduct_location_select_vis = true;
                tradeproduct_location_select_point = {
                    lat: geolc.lat,
                    lng: geolc.lng,
                };
            } catch (e) {
                console.log(
                    `(error) handle_tradeproduct_location_sel_map_select `,
                    e,
                );
            }
        };

    const handle_back = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) handle_back `, e);
        }
    };

    const handle_continue = async (): Promise<void> => {
        try {
            switch (view) {
                case `fl_1`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    console.log(
                                        JSON.stringify(
                                            tradeproduct_photo_paths,
                                            null,
                                            4,
                                        ),
                                        `tradeproduct_photo_paths`,
                                    );
                                }
                                break;
                        }
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
        data-view={`fl_1`}
        class={`flex flex-col h-full w-full justify-start items-center`}
    >
        <div
            data-carousel-container={`fl_1`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`fl_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis>
                    <ImageUploadMulti
                        bind:photo_paths={tradeproduct_photo_paths}
                    />
                    <LayoutTrellisLine
                        basis={{
                            label: {
                                value: `${$t(`common.product`)}`,
                            },
                            notify: tradeproduct_key_sel_toggle
                                ? {
                                      label: {
                                          value: `${$t(`common.close`)}`,
                                      },
                                      callback: async () => {
                                          await handle_tradeproduct_key_toggle(
                                              false,
                                          );
                                      },
                                  }
                                : undefined,
                        }}
                    >
                        {#if !tradeproduct_key_sel_toggle}
                            <EntrySelect
                                bind:value={tradeproduct_key_sel}
                                basis={{
                                    wrap: {
                                        id: fmt_id(`key_wrap`),
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`key`),
                                        sync: true,
                                        layer: 1,
                                        options: [
                                            {
                                                entries: [
                                                    {
                                                        value: ``,
                                                        label: `${$t(`icu.choose_*`, { value: `${$t(`common.product`)}`.toLowerCase() })}`,
                                                        disabled: true,
                                                    },
                                                    ...trade_keys.map((i) => ({
                                                        value: i,
                                                        label: `${$t(`trade.product.key.${i}.name`, { default: i })}`,
                                                    })),
                                                    {
                                                        value: ``,
                                                        label: `${$t(`common.other`)}`,
                                                    },
                                                ],
                                            },
                                        ],
                                        callback: async (opt) => {
                                            el_id(
                                                fmt_id(`key_wrap`),
                                            )?.classList.remove(
                                                `entry-layer-1-highlight`,
                                            );
                                            if (!opt.value) {
                                                await handle_tradeproduct_key_toggle(
                                                    true,
                                                );
                                                tradeproduct_key_sel = ``;
                                                tradeproduct_process_sel = ``;
                                            } else {
                                                tradeproduct_process_sel = ``;
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
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`key`),
                                        layer: 1,
                                        sync: true,
                                        classes: `fade-in-long`,
                                        placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_name`, { value: `${$t(`common.product`)}` })}`.toLowerCase() })}`,
                                        field: {
                                            charset:
                                                trade_product_form_fields.key
                                                    .charset,
                                            validate:
                                                trade_product_form_fields.key
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
                                value: `${$t(`common.lot`)}`,
                            },
                        }}
                    >
                        <EntryLine
                            basis={{
                                wrap: {
                                    id: fmt_id(`lot_wrap`),
                                    layer: 1,
                                },
                                el: {
                                    id: fmt_id(`lot`),
                                    layer: 1,
                                    sync: true,
                                    classes: `fade-in-long`,
                                    placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`icu.*_name`, { value: `${$t(`common.lot`)}` })}`.toLowerCase() })}`,
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
                                value: `${$t(`common.process`)}`,
                            },
                            notify: tradeproduct_process_sel_toggle
                                ? {
                                      label: {
                                          value: `${$t(`common.close`)}`,
                                      },
                                      callback: async () => {
                                          await handle_tradeproduct_process_toggle(
                                              false,
                                          );
                                      },
                                  }
                                : undefined,
                        }}
                    >
                        {#if !tradeproduct_process_sel_toggle}
                            <EntrySelect
                                bind:value={tradeproduct_process_sel}
                                basis={{
                                    wrap: {
                                        id: fmt_id(`process_wrap`),
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`process`),
                                        sync: true,
                                        layer: 1,
                                        options: [
                                            {
                                                entries: [
                                                    {
                                                        value: ``,
                                                        label: `${$t(`icu.choose_*`, { value: `${$t(`common.process`)}`.toLowerCase() })}`,
                                                        disabled: true,
                                                    },
                                                    ...tradeproduct_process_list.map(
                                                        (i) => ({
                                                            value: i,
                                                            label: `${$t(`trade.product.key.${tradeproduct_key_parsed}.process.${i}`)}`,
                                                        }),
                                                    ),
                                                    {
                                                        value: ``,
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
                                            if (!value) {
                                                await handle_tradeproduct_process_toggle(
                                                    true,
                                                );
                                            }
                                        },
                                    },
                                }}
                            />
                        {:else}
                            <EntryLine
                                basis={{
                                    wrap: {
                                        id: fmt_id(`process_wrap`),
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`process`),
                                        layer: 1,
                                        sync: true,
                                        classes: `fade-in-long`,
                                        placeholder: `${$t(`icu.enter_the_*`, { value: `${$t(`common.process`)}`.toLowerCase() })}`,
                                        field: {
                                            charset:
                                                trade_product_form_fields
                                                    .process.charset,
                                            validate:
                                                trade_product_form_fields
                                                    .process.validation,
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
                                value: `${$t(`common.location`)}`,
                            },
                        }}
                    >
                        <EntrySelect
                            bind:value={tradeproduct_location_sel}
                            basis={{
                                wrap: {
                                    id: fmt_id(`tradeproduct_location_wrap`),
                                    layer: 1,
                                },
                                el: {
                                    id: fmt_id(`tradeproduct_location`),
                                    sync: true,
                                    layer: 1,
                                    options: [
                                        {
                                            entries:
                                                tradeproduct_location_select_geoc
                                                    ? [
                                                          {
                                                              value: `map-select`,
                                                              label: `${tradeproduct_location_select_geoc.name}, ${tradeproduct_location_select_geoc.admin1_name}, ${tradeproduct_location_select_geoc.country_id}`,
                                                          },
                                                          {
                                                              value: `map-select`,
                                                              label: `Choose new location`,
                                                          },
                                                      ]
                                                    : [
                                                          {
                                                              value: ``,
                                                              label: `${$t(`icu.choose_*`, { value: `${$t(`common.location`)}`.toLowerCase() })}`,
                                                              disabled: true,
                                                          },
                                                          {
                                                              value: `map-select`,
                                                              label: `Choose on map`,
                                                          },
                                                          {
                                                              value: ``,
                                                              label: `Clear`,
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
                                        if (!value) {
                                            //@todo
                                        } else if (value === `map-select`) {
                                            await handle_tradeproduct_location_sel_map_select();
                                        }
                                    },
                                },
                            }}
                        />
                    </LayoutTrellisLine>
                </LayoutTrellis>
            </div>
        </div>
    </div>
</LayoutView>
<div
    in:fade={{ delay: 0, duration: 50 }}
    out:fade={{ delay: 50, duration: 200 }}
    class={`flex flex-col w-full justify-start items-center fade-in`}
>
    <Nav
        basis={{
            prev: {
                label: `${$t(`common.back`)}`,
                route: `/models/trade-product`,
                prevent_route:
                    view === `fl_1` && $carousel_index === 0
                        ? undefined
                        : {
                              callback: async () => {
                                  await handle_back();
                              },
                          },
                callback: async () => {
                    await tradeproduct_init_kv(fmt_id());
                },
            },
            title: {
                label: {
                    value: `${$t(`icu.new_*`, { value: `${$t(`common.product`)}` })}`,
                },
                callback: async () => {},
            },
            option: {
                loading: load_submit,
                label: {
                    value:
                        $carousel_num > 1
                            ? `${$t(`common.return`)}`
                            : page_param.carousel[view].get($carousel_index)
                                  ?.label_next || ``,
                    glyph:
                        $carousel_index > 0
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
</div>
<MapPointSelectEnvelope
    bind:map_point_select={tradeproduct_location_select_point}
    bind:map_point_select_geoc={tradeproduct_location_select_geoc}
    basis={{
        visible: tradeproduct_location_select_vis,
        close: async () => {
            tradeproduct_location_select_vis = false;
        },
    }}
/>
