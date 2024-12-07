<script lang="ts">
    import { dialog, geol } from "$lib/client";
    import MapPointSelect from "$lib/components/map_point_select.svelte";
    import type { IClientGeolocationPosition } from "@radroots/client";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        ButtonGlyphPrimary,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_init,
        Fade,
        fmt_geol_latitude,
        fmt_geol_longitude,
        fmt_id,
        Glyph,
        GlyphTitleSelect,
        InputElement,
        LayoutView,
        ls,
        NavToolbar,
        PageHeader,
        SelectMenu,
    } from "@radroots/svelte-lib";
    import { num_str, regex } from "@radroots/utils";
    import { onMount } from "svelte";

    let view_init: View = `c_1`;
    type View = `c_1`;
    let view: View = view_init;

    let geol_pos: IClientGeolocationPosition | undefined = undefined;
    let geol_c: GeocoderReverseResult | undefined = undefined;

    let lgcs_label = ``;
    let lgcs_area = ``;
    let lgcs_area_unit = `ac`;
    let lgcs_elevation = ``;
    let lgcs_elevation_unit = `m`;
    let lgcs_climate = ``;

    $: lgcs_elevation =
        typeof geol_pos?.altitude === `number`
            ? num_str(geol_pos.altitude)
            : lgcs_elevation;

    onMount(async () => {
        try {
            await init_page();
        } catch (e) {
        } finally {
        }
    });

    const init_page = async (): Promise<void> => {
        try {
            carousel_init(1);
            const geolc = await geol.current();
            if (`err` in geolc) {
                await dialog.alert(
                    `${$ls(`icu.failure_*`, { value: `${$ls(`icu.reading_*`, { value: `${$ls(`common.geocode`)}`.toLowerCase() })}` })}`,
                );
                return;
            }
            geol_pos = geolc;
        } catch (e) {
            console.log(`(error) init_page `, e);
        }
    };

    const handle_inc = async (): Promise<void> => {
        try {
            await carousel_inc(view);
        } catch (e) {
            console.log(`(error) handle_inc `, e);
        }
    };

    const handle_dec = async (): Promise<void> => {
        try {
            await carousel_dec(view);
        } catch (e) {
            console.log(`(error) handle_dec `, e);
        }
    };
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader
        basis={{
            label: [
                `${$ls(`icu.add_*`, { value: `${$ls(`common.farm_land`)}` })}`,
                {
                    route: `/farm/land`,
                },
            ],
        }}
    >
        <div slot="option" class={`flex flex-row justify-start items-center`}>
            {#if $carousel_index > 0}
                <Fade>
                    <ButtonGlyphPrimary
                        basis={{
                            label: `${$ls(`common.back`)}`,
                            callback: async () => {
                                await handle_dec();
                            },
                        }}
                    />
                </Fade>
            {/if}
        </div>
    </PageHeader>
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
                <div
                    class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
                >
                    {#if geol_pos}
                        <div
                            class={`flex flex-row h-[24rem] w-full justify-center items-center bg-layer-2-surface round-44 overflow-hidden`}
                        >
                            <MapPointSelect
                                bind:map_point={geol_pos}
                                bind:map_geoc={geol_c}
                            />
                        </div>
                        <div
                            class={`flex flex-col w-full pt-2 justify-center items-center`}
                        >
                            <ButtonGlyphPrimary
                                basis={{
                                    label: `${$ls(`icu.add_*`, { value: `${$ls(`common.location`)}` })}`,
                                    callback: async () => {
                                        if (geol_c) await handle_inc();
                                    },
                                }}
                            />
                        </div>
                    {:else}
                        <div
                            class={`flex flex-row h-[24rem] w-full justify-center items-center bg-layer-2-surface round-44`}
                        >
                            <Glyph
                                basis={{
                                    classes: `text-layer-0-glyph`,
                                    dim: `md`,
                                    weight: `bold`,
                                    key: `compass`,
                                }}
                            />
                        </div>
                    {/if}
                </div>
            </div>
            <div
                data-carousel-item={`c_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <div
                    class={`flex flex-col w-full px-4 gap-4 justify-start items-center`}
                >
                    <div
                        class={`flex flex-col h-[24rem] w-full px-2 gap-4 justify-start items-center`}
                    >
                        {#if geol_c && geol_pos}
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <div
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.location`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.1rem] text-layer-0-glyph`}
                                    >
                                        {`${geol_c.name}, ${geol_c.admin1_id}, ${geol_c.country_name}`}
                                    </p>
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <p
                                    class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                >
                                    {`${$ls(`common.coordinates`)}`}
                                </p>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <p
                                        class={`font-sans font-[400] text-[1.1rem] text-layer-0-glyph`}
                                    >
                                        {`${fmt_geol_latitude(
                                            geol_pos.lat,
                                            `d`,
                                            4,
                                        )}, ${fmt_geol_longitude(
                                            geol_pos.lng,
                                            `d`,
                                            4,
                                        )}`}
                                    </p>
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <div
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.farm`)}/${`${$ls(`common.estate`)}`}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
                                        bind:value={lgcs_label}
                                        basis={{
                                            id: fmt_id(`label`),
                                            sync: true,
                                            layer: 0,
                                            classes: `h-10 placeholder:text-[1.1rem]`,
                                            placeholder: `Name of farm or estate`,
                                            field: {
                                                charset: regex.description,
                                                validate: regex.description_ch,
                                                validate_keypress: true,
                                            },
                                        }}
                                    />
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <div
                                    class={`flex flex-row w-full gap-1 justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.area`)}`}
                                    </p>
                                    <SelectMenu
                                        bind:value={lgcs_area_unit}
                                        basis={{
                                            layer: 0,
                                            options: [
                                                {
                                                    entries: [
                                                        {
                                                            label: `${$ls(`measurement.area.ac`)}`,
                                                            value: `ac`,
                                                        },
                                                        {
                                                            label: `${$ls(`measurement.area.ha`)}`,
                                                            value: `ha`,
                                                        },
                                                        {
                                                            label: `${$ls(`measurement.area.m2`)}`,
                                                            value: `m2`,
                                                        },
                                                    ],
                                                },
                                            ],
                                        }}
                                    >
                                        <svelte:fragment slot="element">
                                            <GlyphTitleSelect
                                                basis={{
                                                    label: `${$ls(`measurement.area.${lgcs_area_unit}_ab`)}`,
                                                }}
                                            />
                                        </svelte:fragment>
                                    </SelectMenu>
                                </div>
                                <div
                                    class={`relative flex flex-row h-12 w-full justify-between items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
                                        bind:value={lgcs_area}
                                        basis={{
                                            id: fmt_id(`area`),
                                            sync: true,
                                            layer: 0,
                                            classes: `h-10 placeholder:text-[1.1rem]`,
                                            placeholder: `Land area`,
                                            field: {
                                                charset: regex.num,
                                                validate: regex.num,
                                                validate_keypress: true,
                                            },
                                        }}
                                    />
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <div
                                    class={`flex flex-row w-full gap-1 justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.elevation`)}`}
                                    </p>
                                    <SelectMenu
                                        bind:value={lgcs_elevation_unit}
                                        basis={{
                                            layer: 0,
                                            options: [
                                                {
                                                    entries: [
                                                        {
                                                            label: `${$ls(`measurement.length.m`)}`,
                                                            value: `m`,
                                                        },
                                                        {
                                                            label: `${$ls(`measurement.length.ft`)}`,
                                                            value: `ft`,
                                                        },
                                                    ],
                                                },
                                            ],
                                        }}
                                    >
                                        <svelte:fragment slot="element">
                                            <GlyphTitleSelect
                                                basis={{
                                                    label: `${$ls(`measurement.length.${lgcs_elevation_unit}_ab`)}`,
                                                }}
                                            />
                                        </svelte:fragment>
                                    </SelectMenu>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
                                        bind:value={lgcs_elevation}
                                        basis={{
                                            id: fmt_id(`elevation`),
                                            sync: true,
                                            layer: 0,
                                            classes: `h-10 placeholder:text-[1.1rem]`,
                                            placeholder: `Approx. elevation`,
                                            field: {
                                                charset: regex.num,
                                                validate: regex.num,
                                                validate_keypress: true,
                                            },
                                        }}
                                    />
                                </div>
                            </div>
                            <div
                                class={`flex flex-col w-full gap-1 justify-start items-start`}
                            >
                                <div
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.climate`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
                                        bind:value={lgcs_climate}
                                        basis={{
                                            id: fmt_id(`climate`),
                                            sync: true,
                                            layer: 0,
                                            classes: `h-10 placeholder:text-[1.1rem]`,
                                            placeholder: `Local climate`,
                                            field: {
                                                charset: regex.description,
                                                validate: regex.description_ch,
                                                validate_keypress: true,
                                            },
                                        }}
                                    />
                                </div>
                            </div>
                            <div
                                class={`flex flex-row w-full pt-2 justify-center items-center`}
                            >
                                <ButtonGlyphPrimary
                                    basis={{
                                        label: `${$ls(`icu.add_*`, { value: `${$ls(`common.location`)}` })}`,
                                        callback: async () => {
                                            if (geol_c) await handle_inc();
                                        },
                                    }}
                                />
                            </div>
                        {/if}
                    </div>
                </div>
            </div>
        </div>
    </div>
</LayoutView>
