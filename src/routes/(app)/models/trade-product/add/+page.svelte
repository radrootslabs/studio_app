<script lang="ts">
    import { db, dialog, geol } from "$lib/client";
    import ImageUploadControl from "$lib/components/image_upload_control.svelte";
    import ImageUploadEditEnvelope from "$lib/components/image_upload_edit_envelope.svelte";
    import MapPointSelectEnvelope from "$lib/components/map_point_select_envelope.svelte";
    import TradeFieldDisplayEl from "$lib/components/trade_field_display_el.svelte";
    import TradeFieldDisplayKv from "$lib/components/trade_field_display_kv.svelte";
    import { el_focus } from "$lib/util/client";
    import { location_gcs_to_geoc } from "$lib/util/geocode";
    import { kv_init_page, kv_sync } from "$lib/util/kv";
    import { model_location_gcs_add_geocode } from "$lib/util/models-location-gcs";
    import { model_media_upload_add_list } from "$lib/util/models-media-upload";
    import {
        trade_product_fields_validate,
        tradeproduct_init_kv,
        tradeproduct_validate_fields,
    } from "$lib/util/models-trade-product";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        trade_product_form_fields,
        type LocationGcs,
    } from "@radroots/models";
    import {
        app_layout,
        ascii,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_index_max,
        carousel_num,
        catch_err,
        el_id,
        EntryLine,
        EntryMultiline,
        EntrySelect,
        EntryWrap,
        fmt_id,
        fmt_price,
        Glyph,
        InputElement,
        int_step,
        kv,
        layout_view_cover,
        LayoutTrellis,
        LayoutTrellisLine,
        LayoutView,
        Loading,
        locale,
        ls,
        Nav,
        route,
        SelectElement,
        sleep,
        view_effect,
    } from "@radroots/svelte-lib";
    import {
        err_system,
        fiat_currencies,
        fmt_currency_price,
        fmt_trade_quantity_tup,
        mass_tf_u,
        mass_units,
        num_str,
        parse_currency_marker,
        parse_currency_price,
        parse_trade_key,
        parse_trade_quantity_tup,
        sum_currency_price,
        trade,
        trade_keys,
        year_curr,
        type CurrencyPrice,
        type GeolocationCoordinatesPoint,
        type TradeKey,
        type TradeQuantity,
    } from "@radroots/utils";
    import { onMount } from "svelte";
    import { writable } from "svelte/store";

    type CarouselParam = {
        label_prev?: string;
        label_next: string;
    };
    const page_param: {
        carousel: Record<View, Map<number, CarouselParam>>;
        default: {
            tradepr_key: TradeKey;
        };
    } = {
        carousel: {
            c_1: new Map<number, CarouselParam>([
                [
                    0,
                    {
                        label_next: `${$ls(`common.add`)}`,
                    },
                ],
                [
                    1,
                    {
                        label_next: `${$ls(`common.add`)}`,
                    },
                ],
                [
                    2,
                    {
                        label_next: ``,
                    },
                ],
            ]),
            success: new Map<number, CarouselParam>([
                [
                    0,
                    {
                        label_next: ``,
                    },
                ],
            ]),
        },
        default: {
            tradepr_key: `coffee`,
        },
    };

    let view_init: View = `c_1`;
    type View = `c_1` | `success`;
    let view: View = view_init;
    $: {
        view_effect<View>(view);
    }

    let load_page = false;
    let load_submit = false;

    let tradepr_success_id = ``;
    let tradepr_photo_paths: string[] = [];
    let tradepr_photo_edit: { index: number; file_path: string } | undefined =
        undefined;
    let tradepr_key_sel_toggle = false;
    let tradepr_key_sel = ``;
    $: tradepr_key_parsed = parse_trade_key(tradepr_key_sel);
    $: tradepr_key_quantities_list = tradepr_key_parsed
        ? trade.key[tradepr_key_parsed].quantity
        : trade.default.quantity;

    let tradepr_process_sel = ``;
    let tradepr_process_sel_toggle = false;
    $: tradepr_process_list = tradepr_key_parsed
        ? trade.key[tradepr_key_parsed].process
        : [];

    const tradepr_lgc_sel = writable<string>(``);
    let tradepr_lgc_sel_geoc: GeocoderReverseResult | undefined = undefined;
    let tradepr_lgc_map_vis = false;
    let tradepr_lgc_map_point: GeolocationCoordinatesPoint | undefined =
        undefined;
    let tradepr_lgc_map_geoc: GeocoderReverseResult | undefined = undefined;
    let tradepr_lgc_list: LocationGcs[] = [];

    let tradepr_price_amt_val = ``;
    let tradepr_price_curr_sel = ``;
    let tradepr_price_qty_unit_sel = ``;

    const tradepr_qty_tup_sel = writable<string>(``);
    let tradepr_qty_tup_sel_toggle = false;
    let tradepr_qty_unit_sel = ``;
    const tradepr_qty_avail = writable<number>(1);

    let tradepr_parsed_quantity: TradeQuantity | undefined = undefined;

    onMount(async () => {
        try {
            await init_page();
            await setup_tests();
        } catch (e) {
        } finally {
            load_page = true;
        }
    });

    const init_page = async (): Promise<void> => {
        try {
            await kv_init_page();
            await handle_view(view_init);
            layout_view_cover.set(false);
            carousel_index.set(0);
            carousel_index_max.set(page_param.carousel[view].size - 1);
            carousel_num.set(0);
            tradepr_price_curr_sel = `eur`;
            tradepr_price_qty_unit_sel = `kg`;
            tradepr_qty_unit_sel = `kg`;
            const location_gcs_get_ls = await db.location_gcs_get({
                list: [`all`],
            });
            if (`results` in location_gcs_get_ls)
                tradepr_lgc_list = location_gcs_get_ls.results;
        } catch (e) {
            await catch_err(e, `init_page`);
        }
    };

    const setup_tests = async (): Promise<void> => {
        try {
            tradepr_key_sel = page_param.default.tradepr_key;
            tradepr_process_sel = `washed`;
            tradepr_price_amt_val = `4.30`;
            tradepr_qty_tup_sel.set(`60-kg-bag`);
            await kv_sync([
                [fmt_id(`title`), `Green Coffee Beans`],
                [fmt_id(`lot`), `Ancestor slope`],
                [
                    fmt_id(`summary`),
                    `Good coffee, an amazing year, wonderful flavour, we love it!`,
                ],
            ]);
        } catch (e) {
            await catch_err(e, `setup_tests`);
        }
    };

    let tradepr_cprice_amt: CurrencyPrice | undefined = undefined;
    $: tradepr_cprice_amt =
        tradepr_price_amt_val && tradepr_price_curr_sel
            ? parse_currency_price(
                  $locale,
                  tradepr_price_curr_sel,
                  tradepr_price_amt_val,
              )
            : undefined;

    let tradepr_cprice_total: CurrencyPrice | undefined = undefined;
    $: tradepr_cprice_total =
        tradepr_cprice_amt && tradepr_parsed_quantity
            ? parse_currency_price(
                  $locale,
                  tradepr_cprice_amt.cur,
                  sum_currency_price(tradepr_cprice_amt) *
                      mass_tf_u(
                          tradepr_parsed_quantity?.mass_unit,
                          tradepr_price_qty_unit_sel,
                          tradepr_parsed_quantity?.mass,
                      ) *
                      $tradepr_qty_avail,
              )
            : undefined;

    tradepr_qty_avail.subscribe(async (_tradepr_qty_avail) => {
        if (_tradepr_qty_avail < 1) {
            tradepr_qty_avail.set(Math.max(_tradepr_qty_avail, 1));
            return;
        }
    });

    tradepr_qty_tup_sel.subscribe(async (_tradepr_qty_tup_sel) => {
        tradepr_parsed_quantity =
            parse_trade_quantity_tup(_tradepr_qty_tup_sel);
        if (tradepr_parsed_quantity) {
            await kv.set(
                fmt_id(`qty_amt`),
                num_str(tradepr_parsed_quantity.mass),
            );
            await kv.set(fmt_id(`qty_unit`), tradepr_parsed_quantity.mass_unit);
        }
    });

    tradepr_lgc_sel.subscribe(async (_tradepr_lgc_sel) => {
        if (!_tradepr_lgc_sel) return;
        else if (_tradepr_lgc_sel.startsWith(`*`) && tradepr_lgc_map_geoc) {
            tradepr_lgc_sel_geoc = tradepr_lgc_map_geoc;
        } else {
            const location_gcs_sel = await db.location_gcs_get_one({
                id: _tradepr_lgc_sel,
            });
            if (`result` in location_gcs_sel)
                tradepr_lgc_sel_geoc = location_gcs_to_geoc(
                    location_gcs_sel.result,
                );
        }
    });

    const handle_view = async (view_new: View): Promise<void> => {
        try {
            carousel_index.set(0);
            view = view_new;
        } catch (e) {
            await catch_err(e, `handle_view`);
        }
    };

    const handle_tradepr_key_toggle = async (
        vis_input: boolean,
    ): Promise<void> => {
        try {
            const kv_curr = await kv.get(fmt_id(`key`));
            if (kv_curr) {
                const confirm = await dialog.confirm({
                    message: `${$ls(`icu.the_current_entry_*_will_be_deleted`, { value: kv_curr })}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
            }
            tradepr_key_sel_toggle = vis_input;
            if (vis_input) tradepr_key_sel = ``;
        } catch (e) {
            await catch_err(e, `handle_tradepr_key_toggle`);
        }
    };

    const handle_tradepr_process_toggle = async (
        vis_input: boolean,
    ): Promise<void> => {
        try {
            const kv_curr = await kv.get(fmt_id(`process`));
            if (kv_curr) {
                const confirm = await dialog.confirm({
                    message: `${$ls(`icu.the_current_entry_*_will_be_deleted`, { value: kv_curr })}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                });
                if (!confirm) return;
            }
            tradepr_process_sel_toggle = vis_input;
            if (vis_input) tradepr_process_sel = ``;
        } catch (e) {
            await catch_err(e, `handle_tradepr_process_toggle`);
        }
    };

    const handle_tradepr_qty_amt_toggle = async (
        vis_input: boolean,
    ): Promise<void> => {
        try {
            if (vis_input) {
                tradepr_qty_tup_sel.set(``);
            } else {
                const kv_curr = await kv.get(fmt_id(`qty_amt`));
                if (kv_curr) {
                    const kv_curr_unit = await kv.get(fmt_id(`qty_unit`));
                    const confirm = await dialog.confirm({
                        message: `${$ls(`icu.the_current_entry_*_will_be_deleted`, { value: `${kv_curr}${kv_curr_unit ? ` ${kv_curr_unit}` : `${$ls(`measurement.mass.unit.${kv_curr_unit}_ab`)}`}` })}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                    });
                    if (!confirm) return;
                    await kv.delete(fmt_id(`qty_amt`));
                }
            }
            tradepr_qty_tup_sel_toggle = vis_input;
        } catch (e) {
            await catch_err(e, `handle_tradepr_qty_amt_toggle`);
        }
    };

    const handle_tradepr_lgc_sel_map = async (): Promise<void> => {
        try {
            const geolc = await geol.current();
            if (`err` in geolc) {
                await dialog.alert(
                    `${$ls(`icu.failure_*`, { value: `${$ls(`icu.reading_*`, { value: `${$ls(`common.geocode`)}`.toLowerCase() })}` })}`,
                );
                return;
            }
            tradepr_lgc_map_vis = true;
            tradepr_lgc_map_point = {
                lat: geolc.lat,
                lng: geolc.lng,
            };
        } catch (e) {
            await catch_err(e, `handle_tradepr_lgc_sel_map`);
        }
    };

    const handle_back = async (num?: number): Promise<void> => {
        try {
            switch (view) {
                case `c_1`:
                    {
                        switch ($carousel_index) {
                            default:
                                {
                                    await carousel_dec(view);
                                }
                                break;
                        }
                    }
                    break;
            }
            if (num) {
                carousel_num.set(Math.max(num, 0));
                carousel_index.set(Math.max($carousel_index - (num - 1), 0));
            }
        } catch (e) {
            await catch_err(e, `handle_back`);
        }
    };

    const handle_continue = async (): Promise<void> => {
        try {
            switch (view) {
                case `c_1`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    const validate_fields =
                                        await tradeproduct_validate_fields({
                                            kv_pref: fmt_id(),
                                            fields: [`title`, `key`, `process`],
                                        });
                                    if (`err` in validate_fields) {
                                        await dialog.alert(
                                            `${$ls(`icu.enter_the_*`, { value: `${$ls(`models.trade_product.fields.${validate_fields.err}.label`)}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }
                                    await carousel_inc(view);
                                }
                                break;
                            case 1:
                                {
                                    const validate_fields =
                                        await tradeproduct_validate_fields({
                                            kv_pref: fmt_id(),
                                            fields: [
                                                `price_amt`,
                                                `key`,
                                                `process`,
                                            ],
                                        });
                                    if (`err` in validate_fields) {
                                        await dialog.alert(
                                            `${$ls(`icu.enter_the_*`, { value: `${$ls(`models.trade_product.fields.${validate_fields.err}.label`)}`.toLowerCase() })}`,
                                        );
                                        return;
                                    }
                                    await carousel_inc(view);
                                }
                                break;
                        }
                    }
                    break;
            }
        } catch (e) {
            await catch_err(e, `handle_continue`);
        }
    };

    const submit = async (): Promise<void> => {
        try {
            if (load_submit) return;
            load_submit = true;
            if (!tradepr_photo_paths.length) {
                const confirm = await dialog.confirm({
                    message: `${`${$ls(`icu.the_listing_will_be_created_without_a_*`, { value: `${$ls(`common.photo`)}`.toLowerCase() })}`}. ${$ls(`common.do_you_want_to_continue_q`)}`,
                    cancel_label: `${$ls(`icu.add_*`, { value: `${$ls(`common.photo`)}` })}`,
                    ok_label: `${$ls(`common.continue`)}`,
                });
                if (!confirm) {
                    await el_focus(
                        fmt_id(`image-upload-control`),
                        async () => await handle_back(2),
                    );
                    return;
                }
            }
            let location_gcs_id = ``;
            const location_gcs_get_i = await db.location_gcs_get_one({
                id: $tradepr_lgc_sel,
            });
            if (`result` in location_gcs_get_i) {
                location_gcs_id = location_gcs_get_i.result.id;
            } else {
                //@todo add check for existing geohash
                if (tradepr_lgc_map_point && tradepr_lgc_map_geoc) {
                    const location_gcs_add_geocode =
                        await model_location_gcs_add_geocode({
                            geo_code: tradepr_lgc_map_geoc,
                            point: tradepr_lgc_map_point,
                            kind: `trade_product`, //@todo
                        });
                    if (
                        `err` in location_gcs_add_geocode ||
                        `err_s` in location_gcs_add_geocode
                    ) {
                        await dialog.alert(
                            `err` in location_gcs_add_geocode
                                ? err_system(location_gcs_add_geocode.err)
                                    ? `${$ls(`error.client.database_read_failure`)}`
                                    : `${$ls(`icu.the_*_is_incomplete`, { value: `${$ls(`models.location_gcs.fields.${location_gcs_add_geocode.err}.label`)}`.toLowerCase() })}`
                                : `${$ls(`${location_gcs_add_geocode.err_s[0]}`)}`,
                        );
                        return;
                    }
                    location_gcs_id = location_gcs_add_geocode.id;
                }
            }
            const location_gcs_get_c = await db.location_gcs_get_one({
                id: location_gcs_id,
            });
            if (`err` in location_gcs_get_c) {
                const confirm = await dialog.confirm({
                    message: `${$ls(`icu.a_*_is_required`, { value: `${$ls(`common.location`)}`.toLowerCase() })}`,
                    ok_label: `${$ls(`icu.add_*`, { value: `${$ls(`common.location`)}` })}`,
                });
                if (confirm) {
                    await el_focus(
                        fmt_id(`tradepr_location_wrap`),
                        async () => await handle_back(2),
                    );
                }
                return;
            }
            //@todo edit here
            // photos
            /*const photo_path_uploads: {
                file_path: FilePath;
                res_base: string;
                res_path: string;
            }[] = [];
            const photo_path_uploads_err: {
                file_path: FilePath;
                err_msg: string;
            }[] = [];
            const photo_path_uploads_error: IClientHttpResponseError[] = [];
            if (tradepr_photo_paths.length) {
                for (const photo_path of tradepr_photo_paths) {
                    const file_path = parse_file_path(photo_path);
                    if (!file_path) continue;
                    const url = `${PUBLIC_RADROOTS_URL}/public/upload/image`; //@todo
                    const put_upload = await fetch_put_upload({
                        url,
                        file_path,
                    });
                    if (`err` in put_upload) {
                        photo_path_uploads_err.push({
                            file_path,
                            err_msg: put_upload.err,
                        });
                        continue;
                    } else if (`error` in put_upload) {
                        photo_path_uploads_error.push(put_upload.error);
                        continue;
                    }
                    photo_path_uploads.push({
                        file_path,
                        res_base: put_upload.res_base,
                        res_path: put_upload.res_path,
                    });
                }
            }
            if (photo_path_uploads_error.length) {
                const confirm = await dialog.confirm({
                    message: `${$ls(photo_path_uploads_error[0].message)}`, //@todo
                    ok_label: photo_path_uploads_error[0].label_ok
                        ? `${$ls(photo_path_uploads_error[0].label_ok)}` ||
                          undefined
                        : undefined,
                    cancel_label: photo_path_uploads_error[0].label_cancel
                        ? `${$ls(photo_path_uploads_error[0].label_cancel)}` ||
                          undefined
                        : undefined,
                });
                if (confirm) {
                    console.log(`@todo add profile name`);
                    return;
                }
            }
            if (photo_path_uploads_err.length) {
                await dialog.alert(
                    `${$ls(`icu.there_was_a_failure_while_*`, {
                        value: `${$ls(`icu.uploading_*_photos`, {
                            value:
                                photo_path_uploads_err.length ===
                                tradepr_photo_paths.length
                                    ? `${$ls(`common.all`)}`
                                    : `${photo_path_uploads_err.length}`,
                        })}`.toLowerCase(),
                    })}`,
                );
                return;
            }*/

            const media_uploads = await model_media_upload_add_list({
                photo_paths: tradepr_photo_paths,
            });

            if (`alert` in media_uploads) {
                await dialog.alert(media_uploads.alert);
                return;
            } else if (`confirm` in media_uploads) {
                await dialog.confirm(media_uploads.confirm);
                return;
            }

            // trade product
            const trade_product_fields = await trade_product_fields_validate({
                field_defaults: [
                    [`price_qty_amt`, num_str(1)],
                    [`profile`, `natural`],
                    [`qty_avail`, num_str(Math.max($tradepr_qty_avail, 1))],
                    [`year`, year_curr()],
                ],
            });
            if (`err` in trade_product_fields) {
                await dialog.alert(
                    `${$ls(`icu.the_*_is_incomplete`, { value: `${$ls(`models.trade_product.fields.${trade_product_fields.err}.label`)}`.toLowerCase() })}`,
                );
                return;
            }
            const trade_product_add =
                await db.trade_product_add(trade_product_fields);
            if (`err` in trade_product_add || `err_s` in trade_product_add) {
                await dialog.alert(
                    `err` in trade_product_add
                        ? `${$ls(`icu.the_*_is_incomplete`, { value: `${$ls(`models.trade_product.fields.${trade_product_add.err}.label`)}`.toLowerCase() })}`
                        : `${$ls(`${trade_product_add.err_s[0]}`)}`,
                );
                return;
            }
            let trade_product_location_set_err: string = ``;
            const trade_product_location_set =
                await db.trade_product_location_set({
                    trade_product: {
                        id: trade_product_add.id,
                    },
                    location_gcs: {
                        id: location_gcs_get_c.result.id,
                    },
                });
            if (`err` in trade_product_location_set) {
                trade_product_location_set_err = trade_product_location_set.err;
            }
            const trade_product_media_set_err: string[] = [];
            for (const media_upload of media_uploads.results) {
                const trade_product_media_set =
                    await db.trade_product_media_set({
                        trade_product: {
                            id: trade_product_add.id,
                        },
                        media_upload: {
                            id: media_upload,
                        },
                    });
                if (`err` in trade_product_media_set) {
                    trade_product_media_set_err.push(
                        trade_product_media_set.err,
                    );
                }
            }

            if (trade_product_location_set_err) {
                //@todo
            }

            if (trade_product_media_set_err.length) {
                //@todo
            }

            handle_view(`success`);
            await sleep(2000);
            await route(`/`);
        } catch (e) {
            await catch_err(e, `submit`);
        } finally {
            load_submit = false;
        }
    };
</script>

{#if load_page}
    <LayoutView>
        <div
            data-view={`c_1`}
            class={`flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                data-carousel-container={`c_1`}
                class={`carousel-container flex h-full w-full`}
            >
                <div
                    data-carousel-item={`c_1`}
                    class={`carousel-item flex flex-col w-full justify-start items-center`}
                >
                    <LayoutTrellis>
                        <ImageUploadControl
                            bind:photo_paths={tradepr_photo_paths}
                            bind:photo_edit={tradepr_photo_edit}
                            basis={{
                                id: fmt_id(`image-upload-control`),
                            }}
                        />
                        <LayoutTrellisLine
                            basis={{
                                label: {
                                    value: `${$ls(`icu.listing_*`, { value: `${$ls(`common.title`)}` })}`,
                                },
                            }}
                        >
                            <EntryLine
                                basis={{
                                    wrap: {
                                        id: fmt_id(`title_wrap`),
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`title`),
                                        layer: 1,
                                        sync: true,
                                        classes: `fade-in-long`,
                                        placeholder: `${$ls(`icu.enter_*`, { value: `${$ls(`common.title`)}`.toLowerCase() })}`,
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
                                    value: `${$ls(`common.product`)}`,
                                },
                                notify: tradepr_key_sel_toggle
                                    ? {
                                          label: {
                                              value: `${$ls(`common.close`)}`,
                                          },
                                          callback: async () => {
                                              await handle_tradepr_key_toggle(
                                                  false,
                                              );
                                          },
                                      }
                                    : undefined,
                            }}
                        >
                            {#if !tradepr_key_sel_toggle}
                                <EntrySelect
                                    bind:value={tradepr_key_sel}
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
                                                            label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.product`)}`.toLowerCase() })}`,
                                                            disabled: true,
                                                        },
                                                        ...trade_keys.map(
                                                            (i) => ({
                                                                value: i,
                                                                label: `${$ls(`trade.product.key.${i}.name`)}`,
                                                            }),
                                                        ),
                                                        {
                                                            value: ``,
                                                            label: `${$ls(`common.other`)}`,
                                                        },
                                                    ],
                                                },
                                            ],
                                            callback: async (opt) => {
                                                el_id(
                                                    fmt_id(`key_wrap`),
                                                )?.classList.remove(
                                                    `layer-1-ring-apply`,
                                                );
                                                if (!opt.value) {
                                                    await handle_tradepr_key_toggle(
                                                        true,
                                                    );
                                                    tradepr_key_sel = ``;
                                                    tradepr_process_sel = ``;
                                                } else {
                                                    tradepr_process_sel = ``;
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
                                            placeholder: `${$ls(`icu.enter_the_*`, { value: `${$ls(`icu.*_name`, { value: `${$ls(`common.product`)}` })}`.toLowerCase() })}`,
                                            field: {
                                                charset:
                                                    trade_product_form_fields
                                                        .key.charset,
                                                validate:
                                                    trade_product_form_fields
                                                        .key.validation,
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
                                    value: `${$ls(`common.process`)}`,
                                },
                                notify: tradepr_process_sel_toggle
                                    ? {
                                          label: {
                                              value: `${$ls(`common.close`)}`,
                                          },
                                          callback: async () => {
                                              await handle_tradepr_process_toggle(
                                                  false,
                                              );
                                          },
                                      }
                                    : undefined,
                            }}
                        >
                            {#if !tradepr_process_sel_toggle}
                                <EntrySelect
                                    bind:value={tradepr_process_sel}
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
                                                    entries:
                                                        tradepr_process_list.length
                                                            ? [
                                                                  {
                                                                      value: ``,
                                                                      label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.process`)}`.toLowerCase() })}`,
                                                                      disabled: true,
                                                                  },
                                                                  ...tradepr_process_list.map(
                                                                      (i) => ({
                                                                          value: i,
                                                                          label: `${$ls(`trade.product.key.${tradepr_key_parsed}.process.${i}`)}`,
                                                                      }),
                                                                  ),
                                                                  {
                                                                      value: ``,
                                                                      label: `${$ls(`common.other`)}`,
                                                                  },
                                                              ]
                                                            : [
                                                                  {
                                                                      value: ``,
                                                                      label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.process`)}`.toLowerCase() })}`,
                                                                      disabled: true,
                                                                  },

                                                                  {
                                                                      value: `*choose-product`,
                                                                      label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.product`)}`.toLowerCase() })}`,
                                                                  },
                                                              ],
                                                },
                                            ],
                                            callback: async ({ value }) => {
                                                el_id(
                                                    fmt_id(`process_wrap`),
                                                )?.classList.remove(
                                                    `layer-1-ring-apply`,
                                                );
                                                if (!value) {
                                                    await handle_tradepr_process_toggle(
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
                                            placeholder: `${$ls(`icu.enter_the_*`, { value: `${$ls(`common.process`)}`.toLowerCase() })}`,
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
                                    value: `${$ls(`common.location`)}`,
                                },
                            }}
                        >
                            <EntrySelect
                                bind:value={$tradepr_lgc_sel}
                                basis={{
                                    wrap: {
                                        id: fmt_id(`tradepr_location_wrap`),
                                        layer: 1,
                                    },
                                    el: {
                                        id: fmt_id(`tradepr_location`),
                                        sync: true,
                                        layer: 1,
                                        options: [
                                            {
                                                entries: tradepr_lgc_map_geoc
                                                    ? [
                                                          {
                                                              value: ``,
                                                              label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.location`)}`.toLowerCase() })}`,
                                                              disabled: true,
                                                          },
                                                          {
                                                              value: `*map`,
                                                              label: `${$ls(`icu.choose_on_*`, { value: `${$ls(`common.map`)}`.toLowerCase() })}`,
                                                          },
                                                          {
                                                              value: `*geoc`,
                                                              label: `${tradepr_lgc_map_geoc.name}, ${tradepr_lgc_map_geoc.admin1_name}, ${tradepr_lgc_map_geoc.country_id}`,
                                                          },
                                                          ...tradepr_lgc_list.map(
                                                              (i) => ({
                                                                  value: i.id,
                                                                  label: `${i.gc_name}, ${i.gc_admin1_name}, ${i.gc_country_id}`,
                                                              }),
                                                          ),
                                                      ]
                                                    : [
                                                          {
                                                              value: ``,
                                                              label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.location`)}`.toLowerCase() })}`,
                                                              disabled: true,
                                                          },
                                                          {
                                                              value: `*map`,
                                                              label: `${$ls(`icu.choose_on_*`, { value: `${$ls(`common.map`)}`.toLowerCase() })}`,
                                                          },
                                                          ...tradepr_lgc_list.map(
                                                              (i) => ({
                                                                  value: i.id,
                                                                  label: `${i.gc_name}, ${i.gc_admin1_name}, ${i.gc_country_id}`,
                                                              }),
                                                          ),
                                                      ],
                                            },
                                        ],
                                        callback: async ({ value }) => {
                                            el_id(
                                                fmt_id(`process_wrap`),
                                            )?.classList.remove(
                                                `layer-1-ring-apply`,
                                            );
                                            if (value === `*map`) {
                                                await handle_tradepr_lgc_sel_map();
                                            }
                                        },
                                    },
                                }}
                            />
                        </LayoutTrellisLine>
                    </LayoutTrellis>
                </div>
                <div
                    data-carousel-item={`c_1`}
                    class={`carousel-item flex flex-col w-full justify-start items-center`}
                >
                    <LayoutTrellis>
                        <div
                            class={`flex flex-col w-full justify-center items-center`}
                        >
                            <div
                                class={`flex flex-col h-[11rem] w-${$app_layout} justify-start items-start bg-layer-1-surface rounded-[2rem] overflow-hidden`}
                            >
                                <div
                                    class={`flex flex-row h-[2.5rem] w-full justify-center items-center`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.05rem] text-layer-1-glyph_d`}
                                    >
                                        {`${$ls(`common.listing`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-col h-[8.5rem] w-full px-4 justify-start items-start`}
                                >
                                    <div
                                        class={`flex flex-col h-full w-full gap-[5px] justify-center items-center border-t-line border-layer-0-glyph_d`}
                                    >
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 1,
                                                label: `${$ls(`common.title`)}`,
                                                display: {
                                                    kv: `title`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.title`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 1,
                                                label: `${$ls(`common.product`)}`,
                                                display: {
                                                    kv: `key`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.product`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 1,
                                                label: `${$ls(`common.process`)}`,
                                                display: {
                                                    kv: `process`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.process`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 1,
                                                kv_wrap: `tradepr_location_wrap`,
                                                label: `${$ls(`common.location`)}`,
                                                display: {
                                                    value: tradepr_lgc_sel_geoc
                                                        ? `${tradepr_lgc_sel_geoc.name}, ${tradepr_lgc_sel_geoc.admin1_name}, ${tradepr_lgc_sel_geoc.country_id}`
                                                        : ``,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.location`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                    </div>
                                </div>
                            </div>
                        </div>
                        <LayoutTrellisLine
                            basis={{
                                label: {
                                    value: `${$ls(`common.lot`)}`,
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
                                        placeholder: `${$ls(`icu.enter_the_*`, { value: `${$ls(`icu.*_name`, { value: `${$ls(`common.lot`)}` })}`.toLowerCase() })}`,
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
                                    value: `${$ls(`icu.*_price`, { value: `${$ls(`common.product`)}` })} (${tradepr_price_curr_sel}/${`${$ls(`measurement.mass.unit.${tradepr_price_qty_unit_sel}_ab`)}`})`,
                                },
                            }}
                        >
                            <EntryWrap
                                basis={{
                                    id: fmt_id(`price_wrap`),
                                    layer: 1,
                                }}
                            >
                                <div
                                    class={`flex flex-row justify-start pr-1 items-center`}
                                >
                                    <SelectElement
                                        bind:value={tradepr_price_curr_sel}
                                        basis={{
                                            id: fmt_id(`price_currency`),
                                            layer: 1,
                                            sync: true,
                                            classes: `w-fit font-sans font-[400] text-[1.1rem] ${tradepr_price_amt_val ? `text-layer-1-glyph_d` : `text-layer-1-glyph_pl`} el-re`,
                                            options: [
                                                {
                                                    entries:
                                                        fiat_currencies.map(
                                                            (i) => ({
                                                                value: `${i}`,
                                                                label: parse_currency_marker(
                                                                    $locale,
                                                                    i,
                                                                ),
                                                            }),
                                                        ),
                                                },
                                            ],
                                        }}
                                    />
                                </div>
                                <InputElement
                                    bind:value={tradepr_price_amt_val}
                                    basis={{
                                        id: fmt_id(`price_amt`),
                                        layer: 1,
                                        sync: true,
                                        placeholder: `${$ls(`icu.enter_the_*`, { value: `${$ls(`common.price`)}`.toLowerCase() })}`,
                                        field: {
                                            charset:
                                                trade_product_form_fields
                                                    .price_amt.charset,
                                            validate:
                                                trade_product_form_fields
                                                    .price_amt.validation,
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
                                                el_id(
                                                    fmt_id(`price_wrap`),
                                                )?.classList.remove(
                                                    `layer-1-ring-apply`,
                                                );
                                            } else {
                                                el_id(
                                                    fmt_id(`price_wrap`),
                                                )?.classList.add(
                                                    `layer-1-ring-apply`,
                                                );
                                            }
                                        },
                                        callback_blur: async ({ el }) => {
                                            if (!el.value) return;
                                            el.value = fmt_price(
                                                tradepr_price_curr_sel,
                                                el.value,
                                            ).slice(1); //@todo fmt handles 'en' only
                                        },
                                    }}
                                />
                                <div
                                    class={`flex flex-row gap-2 justify-end items-center`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.05rem] text-layer-1-glyph_d lowercase`}
                                    >
                                        {num_str(1)}
                                    </p>
                                    <SelectElement
                                        bind:value={tradepr_price_qty_unit_sel}
                                        basis={{
                                            id: fmt_id(`price_qty_unit`),
                                            sync: true,
                                            layer: 1,
                                            classes: `w-fit font-sans font-[400] text-[1.05rem]`,
                                            show_arrows: `r`,
                                            options: [
                                                {
                                                    entries: mass_units.map(
                                                        (i) => ({
                                                            value: i,
                                                            label: `${$ls(`measurement.mass.unit.${i}_ab`)}`.toLowerCase(),
                                                        }),
                                                    ),
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
                                    value: `${$ls(`icu.*_quantity`, { value: `${$ls(`common.order`)}` })}`,
                                },
                                notify: tradepr_qty_tup_sel_toggle
                                    ? {
                                          label: {
                                              value: `${$ls(`common.close`)}`,
                                          },
                                          callback: async () => {
                                              await handle_tradepr_qty_amt_toggle(
                                                  false,
                                              );
                                          },
                                      }
                                    : undefined,
                            }}
                        >
                            {#if !tradepr_qty_tup_sel_toggle}
                                <EntrySelect
                                    bind:value={$tradepr_qty_tup_sel}
                                    basis={{
                                        wrap: {
                                            id: fmt_id(`qty_wrap`),
                                            layer: 1,
                                        },
                                        el: {
                                            layer: 1,
                                            options: [
                                                {
                                                    entries: [
                                                        {
                                                            value: ``,
                                                            label: `${$ls(`icu.choose_*`, { value: `${$ls(`common.quantity`)}`.toLowerCase() })}`,
                                                            disabled: true,
                                                        },
                                                        ...tradepr_key_quantities_list.map(
                                                            (i) => ({
                                                                value: fmt_trade_quantity_tup(
                                                                    i,
                                                                ),
                                                                label: `${i.mass} ${$ls(`measurement.mass.unit.${i.mass_unit}_ab`)} ${i.label}`,
                                                            }),
                                                        ),
                                                        {
                                                            value: ``,
                                                            label: `${$ls(`common.other`)}`,
                                                        },
                                                    ],
                                                },
                                            ],
                                            callback: async ({ value }) => {
                                                el_id(
                                                    fmt_id(`qty_wrap`),
                                                )?.classList.remove(
                                                    `layer-1-ring-apply`,
                                                );
                                                if (value === ``) {
                                                    await handle_tradepr_qty_amt_toggle(
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
                                        id: fmt_id(`qty_wrap`),
                                        layer: 1,
                                    }}
                                >
                                    <InputElement
                                        basis={{
                                            id: fmt_id(`qty_amt`),
                                            sync: true,
                                            layer: 1,
                                            placeholder: `${$ls(`icu.enter_*_per_order`, { value: `${$ls(`measurement.mass.unit.${tradepr_qty_unit_sel}_ab`)}`.toLowerCase() })}`,
                                            field: {
                                                charset:
                                                    trade_product_form_fields
                                                        .qty_amt.charset,
                                                validate:
                                                    trade_product_form_fields
                                                        .qty_amt.validation,
                                                validate_keypress: true,
                                            },
                                        }}
                                    />
                                    <div
                                        class={`flex flex-row gap-2 justify-end items-center`}
                                    >
                                        <SelectElement
                                            bind:value={tradepr_qty_unit_sel}
                                            basis={{
                                                id: fmt_id(`qty_unit`),
                                                sync: true,
                                                layer: 1,
                                                classes: `w-fit font-sans font-[400] text-[1.05rem] text-layer-1-glyph_d`,
                                                show_arrows: `r`,
                                                options: [
                                                    {
                                                        entries: mass_units.map(
                                                            (i) => ({
                                                                value: i,
                                                                label: `${$ls(`measurement.mass.unit.${i}_ab`)}`,
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
                                    value: `${$ls(`common.description`)}`,
                                },
                            }}
                        >
                            <EntryMultiline
                                basis={{
                                    wrap: {
                                        id: fmt_id(`summary_wrap`),
                                    },
                                    el: {
                                        classes: `h-[7rem]`,
                                        id: fmt_id(`summary`),
                                        sync: true,
                                        placeholder: `${$ls(`icu.enter_the_*`, { value: `${$ls(`icu.*_description`, { value: `${$ls(`common.listing`)}` })}`.toLowerCase() })}`,
                                        field: {
                                            charset:
                                                trade_product_form_fields
                                                    .summary.charset,
                                            validate:
                                                trade_product_form_fields
                                                    .summary.validation,
                                            validate_keypress: true,
                                        },
                                    },
                                }}
                            />
                        </LayoutTrellisLine>
                    </LayoutTrellis>
                </div>
                <div
                    data-carousel-item={`c_1`}
                    class={`carousel-item flex flex-col w-full justify-start items-center`}
                >
                    <LayoutTrellis>
                        <div
                            class={`flex flex-col w-full justify-center items-center`}
                        >
                            <div
                                class={`flex flex-col h-auto w-${$app_layout} justify-start items-start bg-layer-1-surface rounded-[2rem] overflow-hidden`}
                            >
                                <div
                                    class={`flex flex-row h-[3rem] w-full justify-center items-center`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.05rem] text-layer-1-glyph_d`}
                                    >
                                        {`${$ls(`common.listing`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-col w-full px-4 justify-start items-center`}
                                >
                                    <div
                                        class={`flex flex-col h-auto w-full py-6 gap-[5px] justify-center items-start border-t-line border-layer-0-glyph_d`}
                                    >
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.title`)}`,
                                                display: {
                                                    kv: `title`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.title`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(2);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.product`)}`,
                                                display: {
                                                    kv: `key`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.product`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(2);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.process`)}`,
                                                display: {
                                                    kv: `process`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.process`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(2);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                kv_wrap: `tradepr_location_wrap`,
                                                label: `${$ls(`common.location`)}`,
                                                display: {
                                                    value: tradepr_lgc_sel_geoc
                                                        ? `${tradepr_lgc_sel_geoc.name}, ${tradepr_lgc_sel_geoc.admin1_name}, ${tradepr_lgc_sel_geoc.country_id}`
                                                        : ``,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.location`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(2);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.lot`)}`,
                                                display: {
                                                    kv: `lot`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.lot`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.description`)}`,
                                                display: {
                                                    kv: `summary`,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.description`)}`.toLowerCase() })}`,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                    </div>
                                    <div
                                        class={`flex flex-col h-auto w-full py-6 gap-[5px] justify-center items-start border-t-line border-layer-0-glyph_d`}
                                    >
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`common.price`)}`,
                                                kv_wrap: `price_wrap`,
                                                display: {
                                                    value:
                                                        tradepr_cprice_amt &&
                                                        tradepr_price_qty_unit_sel
                                                            ? `${fmt_currency_price(tradepr_cprice_amt)} / ${`${$ls(`measurement.mass.unit.${tradepr_price_qty_unit_sel}_ab`)}`.toLowerCase()}`
                                                            : ``,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`common.price`)}`.toLowerCase() })}`,
                                                    nostyle: true,
                                                },
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayKv
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`icu.*_quantity`, { value: `${$ls(`common.order`)}` })}`,
                                                display: {
                                                    value: tradepr_parsed_quantity
                                                        ? `${tradepr_parsed_quantity.mass} ${`${$ls(`measurement.mass.unit.${tradepr_parsed_quantity.mass_unit}_ab`)}`.toLowerCase()} ${tradepr_parsed_quantity.label || `${$ls(`common.bag`)}`}`
                                                        : ``,
                                                    undef: `${$ls(`icu.no_*`, { value: `${$ls(`icu.*_quantity`, { value: `${$ls(`common.order`)}` })}` })}`,
                                                    nostyle:
                                                        !!tradepr_parsed_quantity,
                                                },
                                                kv_wrap: `qty_wrap`,
                                                handle_back: async () => {
                                                    await handle_back(1);
                                                },
                                            }}
                                        />
                                        <TradeFieldDisplayEl
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`icu.*_available`, { value: `${$ls(`common.quantity`)}` })}${tradepr_parsed_quantity ? ` (${$tradepr_qty_avail})` : ``}`,
                                                display: {
                                                    classes:
                                                        tradepr_parsed_quantity
                                                            ? ``
                                                            : `pr-4`,
                                                    value: tradepr_parsed_quantity
                                                        ? ascii.bullet
                                                        : ``,
                                                    hide: !!tradepr_parsed_quantity,
                                                    undef: ascii.dash,
                                                },
                                            }}
                                        >
                                            {#if tradepr_parsed_quantity}
                                                <div
                                                    class={`flex flex-row gap-2 pl-2 pr-1 justify-center items-center`}
                                                >
                                                    <button
                                                        class={`group flex flex-row justify-center items-center`}
                                                        on:click={async () => {
                                                            tradepr_qty_avail.set(
                                                                int_step(
                                                                    $tradepr_qty_avail,
                                                                    `-`,
                                                                    1,
                                                                ),
                                                            );
                                                        }}
                                                    >
                                                        <Glyph
                                                            basis={{
                                                                key: `arrow-down`,
                                                                dim: `xs`,
                                                                classes: `h-[1.3rem] w-[1.3rem] text-layer-1-glyph_d bg-layer-2-surface/60 rounded-full el-re`,
                                                            }}
                                                        />
                                                    </button>
                                                    <button
                                                        class={`group flex flex-row justify-center items-center`}
                                                        on:click={async () => {
                                                            tradepr_qty_avail.set(
                                                                int_step(
                                                                    $tradepr_qty_avail,
                                                                    `+`,
                                                                ),
                                                            );
                                                        }}
                                                    >
                                                        <Glyph
                                                            basis={{
                                                                key: `arrow-up`,
                                                                dim: `xs`,
                                                                classes: `h-[1.3rem] w-[1.3rem] text-layer-1-glyph_d bg-layer-2-surface/60 rounded-full el-re`,
                                                            }}
                                                        />
                                                    </button>
                                                </div>
                                            {/if}
                                        </TradeFieldDisplayEl>
                                        <TradeFieldDisplayEl
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`icu.*_total`, { value: `${$ls(`common.quantity`)}` })}`,
                                                display: {
                                                    classes:
                                                        tradepr_parsed_quantity
                                                            ? `pr-2`
                                                            : `pr-4`,
                                                    value: tradepr_parsed_quantity
                                                        ? `${Number(
                                                              $tradepr_qty_avail *
                                                                  tradepr_parsed_quantity.mass,
                                                          ).toFixed(
                                                              2,
                                                          )} ${`${$ls(`measurement.mass.unit.${tradepr_parsed_quantity.mass_unit}_ab`)}`.toLowerCase()}`
                                                        : ``,
                                                    undef: ascii.dash,
                                                    nostyle: true,
                                                },
                                            }}
                                        />
                                    </div>
                                    <div
                                        class={`flex flex-col h-auto w-full pb-6 gap-[5px] justify-center items-start`}
                                    >
                                        <TradeFieldDisplayEl
                                            basis={{
                                                visible: $carousel_index === 2,
                                                label: `${$ls(`icu.*_total`, { value: `${$ls(`common.order`)}` })}`,
                                                display: {
                                                    classes:
                                                        tradepr_parsed_quantity &&
                                                        tradepr_cprice_amt
                                                            ? `pr-2`
                                                            : `pr-4`,
                                                    value: tradepr_cprice_total
                                                        ? `${fmt_currency_price(tradepr_cprice_total)}`
                                                        : ``,
                                                    undef: ascii.dash,
                                                },
                                            }}
                                        />
                                    </div>
                                </div>
                            </div>
                        </div>
                        <button
                            class={`flex flex-row h-12 w-${$app_layout} justify-center items-center bg-layer-1-glyph-hl active:bg-layer-1-glyph-hl_a round-40 font-sans font-[600] text-[1.1rem] text-white capitalize el-re`}
                            on:click={async () => {
                                await submit();
                            }}
                        >
                            {#if load_submit}
                                <Loading />
                            {:else}
                                {`${$ls(`common.post`)}`}
                            {/if}
                        </button>
                    </LayoutTrellis>
                </div>
            </div>
        </div>
        <div
            data-view={`success`}
            class={`hidden flex flex-col h-full w-full justify-start items-center`}
        >
            <div
                data-carousel-container={`success`}
                class={`carousel-container flex h-full w-full`}
            >
                <div
                    data-carousel-item={`success`}
                    class={`carousel-item flex flex-col w-full justify-start items-center ${view === `success` ? `fade-in-long` : ``}`}
                >
                    <LayoutTrellis>
                        <div
                            class={`flex flex-col h-trellis_centered_${$app_layout} w-full justify-center items-center`}
                        >
                            <Glyph
                                basis={{
                                    classes: `text-success text-[72px]`,
                                    weight: `bold`,
                                    key: `seal-check`,
                                }}
                            />
                            <div
                                class={`flex flex-col pt-1 justify-start items-center`}
                            >
                                <div
                                    class={`flex flex-row w-full justify-center items-center`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.3rem] text-layer-0-glyph`}
                                    >
                                        {`${$ls(`common.complete`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row w-full justify-center items-center`}
                                >
                                    <button
                                        class={`flex flex-row justify-center items-center`}
                                        on:click={async () => {
                                            if (tradepr_success_id)
                                                await route(
                                                    `/models/trade-product/view`,
                                                    [
                                                        [
                                                            `id`,
                                                            tradepr_success_id,
                                                        ],
                                                    ],
                                                );
                                        }}
                                    >
                                        <p
                                            class={`font-sans font-[400] text-[1.1rem] text-layer-0-glyph`}
                                        >
                                            {`${$ls(`icu.click_to_*`, { value: `${$ls(`icu.view_the_*`, { value: `${$ls(`common.product`)}` })}`.toLowerCase() })}`}
                                        </p>
                                    </button>
                                </div>
                            </div>
                        </div>
                    </LayoutTrellis>
                </div>
            </div>
        </div>
    </LayoutView>
{/if}
<Nav
    basis={{
        prev: {
            label:
                view === `success`
                    ? `${$ls(`common.home`)}`
                    : `${$ls(`common.back`)}`,
            route: view === `success` ? `/` : `/models/trade-product`,
            prevent_route:
                (view === `c_1` && $carousel_index === 0) || view === `success`
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
                value:
                    view === `success`
                        ? ``
                        : `${$ls(`icu.new_*`, { value: `${$ls(`common.product`)}` })}`,
            },
            callback: async () => {},
        },
        option: {
            label: {
                value:
                    $carousel_num > 1
                        ? `${$ls(`common.back`)}`
                        : page_param.carousel[view].get($carousel_index)
                              ?.label_next || ``,
                glyph:
                    $carousel_index === $carousel_index_max ||
                    view === `success`
                        ? undefined
                        : {
                              key: `caret-right`,
                              classes: `text-layer-1-glyph-hl`,
                          },
            },
            callback: async () => {
                await handle_continue();
            },
        },
    }}
/>
<MapPointSelectEnvelope
    bind:map_point_select={tradepr_lgc_map_point}
    bind:map_point_select_geoc={tradepr_lgc_map_geoc}
    basis={{
        visible: tradepr_lgc_map_vis,
        close: async () => {
            tradepr_lgc_sel.set(`*geoc`);
            tradepr_lgc_map_vis = false;
        },
    }}
/>
<ImageUploadEditEnvelope
    bind:photo_paths={tradepr_photo_paths}
    bind:photo_edit={tradepr_photo_edit}
/>
