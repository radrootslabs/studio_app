<script lang="ts">
    import { dialog, geol } from "$lib/client";
    import MapPointSelect from "$lib/components/map_point_select.svelte";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        ButtonGlyphPrimary,
        Fade,
        InputElement,
        LayoutView,
        NavToolbar,
        PageHeader,
        carousel_dec,
        carousel_inc,
        carousel_index,
        carousel_init,
        fmt_geol_latitude,
        fmt_geol_longitude,
        fmt_id,
        ls,
    } from "@radroots/svelte-lib";
    import { type GeolocationCoordinatesPoint, regex } from "@radroots/utils";
    import { onMount } from "svelte";

    let view_init: View = `c_1`;
    type View = `c_1`;
    let view: View = view_init;

    let _map_point: GeolocationCoordinatesPoint | undefined = undefined;
    let _map_geoc: GeocoderReverseResult | undefined = undefined;

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
            _map_point = {
                lat: geolc.lat,
                lng: geolc.lng,
            };
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
            label: `${$ls(`icu.add_*`, { value: `${$ls(`common.farm_land`)}` })}`,
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
                    {#if _map_point}
                        <div
                            class={`flex flex-row h-[24rem] w-full justify-center items-center round-44 overflow-hidden`}
                        >
                            <MapPointSelect
                                bind:map_point={_map_point}
                                bind:map_geoc={_map_geoc}
                            />
                        </div>
                        <div
                            class={`flex flex-col w-full pt-2 justify-center items-center`}
                        >
                            <ButtonGlyphPrimary
                                basis={{
                                    label: `${$ls(`icu.add_*`, { value: `${$ls(`common.location`)}` })}`,
                                    callback: async () => {
                                        if (_map_geoc) await handle_inc();
                                    },
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
                        {#if _map_geoc && _map_point}
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
                                        {`${_map_geoc.name}, ${_map_geoc.admin1_id}, ${_map_geoc.country_name}`}
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
                                            _map_point.lat,
                                            `d`,
                                            4,
                                        )}, ${fmt_geol_longitude(
                                            _map_point.lng,
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
                                        basis={{
                                            id: fmt_id(`farm_estate`),
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
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.area`)} (ac.)`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
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
                                    class={`flex flex-row w-full justify-start items-center`}
                                >
                                    <p
                                        class={`font-sansd text-trellisTitle text-layer-0-glyph-label uppercase`}
                                    >
                                        {`${$ls(`common.elevation`)}`}
                                    </p>
                                </div>
                                <div
                                    class={`flex flex-row h-12 w-full justify-start items-center border-y-line border-layer-0-surface-edge`}
                                >
                                    <InputElement
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
                                            if (_map_geoc) await handle_inc();
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
