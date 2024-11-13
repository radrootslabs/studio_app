<script lang="ts">
    import { geoc } from "$lib/client";
    import { cfg } from "$lib/conf";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        app_thc,
        envelope_tilt,
        envelope_visible,
        EnvelopeLower,
        Glyph,
        t,
        type CallbackPromise,
    } from "@radroots/svelte-lib";
    import { MapLibre, Marker } from "@radroots/svelte-maplibre";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";
    import { onDestroy, onMount } from "svelte";
    import MapMarkerDot from "./map_marker_dot.svelte";
    import MapPopupPointGeolocation from "./map_popup_point_geolocation.svelte";

    export let map_point_select: GeolocationCoordinatesPoint | undefined;
    export let map_point_select_geoc: GeocoderReverseResult | undefined =
        undefined;

    export let basis: {
        visible: boolean;
        close: CallbackPromise;
    };
    $: basis = basis;

    let map_point_center: GeolocationCoordinatesPoint = cfg.map.coords.default;

    onMount(async () => {
        try {
            envelope_tilt.set(false);
        } catch (e) {
        } finally {
        }
    });

    onDestroy(async () => {
        try {
            envelope_tilt.set(true);
        } catch (e) {
        } finally {
        }
    });

    $: envelope_visible.set(!!basis.visible);

    $: if (
        map_point_select &&
        map_point_center.lat === 0 &&
        map_point_center.lng === 0
    ) {
        map_point_center = {
            lat: map_point_select.lat,
            lng: map_point_select.lng - 0.065,
        };
    }

    $: {
        if (
            map_point_select &&
            map_point_center &&
            map_point_center.lat !== 0 &&
            map_point_center.lng !== 0
        ) {
            (async () => {
                try {
                    const geoc_res = await geoc.reverse({
                        point: map_point_select,
                    });
                    if (`results` in geoc_res && geoc_res.results.length > 0)
                        map_point_select_geoc = geoc_res.results[0];
                    else map_point_select_geoc = undefined;
                } catch (e) {
                    console.log(`(error) map choose location`, e);
                }
            })();
        }
    }
</script>

<EnvelopeLower
    basis={{
        full_cover: true,
        close: async () => {
            await basis.close();
        },
    }}
>
    {#if basis.visible && map_point_select}
        <MapLibre
            center={map_point_center}
            zoom={10}
            class={`h-full w-full`}
            style={cfg.map.styles.base[$app_thc]}
            attributionControl={false}
        >
            <Marker
                bind:lngLat={map_point_select}
                draggable
                on:dragend={async () => {
                    if (!map_point_select) return;
                    const geoc_res = await geoc.reverse({
                        point: map_point_select,
                        limit: 1,
                    });
                    if (`results` in geoc_res && geoc_res.results.length > 0)
                        map_point_select_geoc = geoc_res.results[0];
                }}
            >
                <button
                    class={`flex flex-row justify-center items-center transform -translate-x-[42%]`}
                    on:click={async () => {}}
                >
                    <MapPopupPointGeolocation
                        basis={{
                            point: map_point_select,
                            geoc: map_point_select_geoc,
                        }}
                    />
                </button>
                <MapMarkerDot />
            </Marker>
        </MapLibre>
        <div
            class={`absolute top-16 left-0 flex flex-col w-full pt-1 justify-center items-center`}
        >
            <button
                class={`group flex flex-row h-8 px-4 gap-2 justify-center items-center bg-layer-1-surface active:bg-layer-1-surface_a rounded-2xl shadow-md el-re`}
                on:click={async () => {
                    await basis.close();
                }}
            >
                <Glyph
                    basis={{
                        classes: `text-layer-0-glyph`,
                        dim: `xs`,
                        weight: `bold`,
                        key: `arrow-up`,
                    }}
                />
                <p
                    class={`font-sans font-[400] text-layer-0-glyph text-[1rem] capitalize`}
                >
                    {`${$t(`common.back`)}`}
                </p>
            </button>
        </div>
    {/if}
</EnvelopeLower>
