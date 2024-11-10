<script lang="ts">
    import { geoc } from "$lib/client";
    import { cfg } from "$lib/conf";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        app_thc,
        envelope_visible,
        EnvelopeLower,
        fmt_geol_latitude,
        fmt_geol_longitude,
        t,
        type CallbackPromise,
    } from "@radroots/svelte-lib";
    import { MapLibre, Marker } from "@radroots/svelte-maplibre";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";
    import MapMarkerDot from "./map_marker_dot.svelte";
    import MapPopupPointGeolocation from "./map_popup_point_geolocation.svelte";

    export let map_point_select: GeolocationCoordinatesPoint | undefined;
    export let map_point_select_geoc: GeocoderReverseResult | undefined =
        undefined;

    export let basis: {
        visible: Boolean;
        close: CallbackPromise;
    };

    let map_point_center: GeolocationCoordinatesPoint = cfg.map.coords.default;

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
            class={`absolute top-6 left-4 flex flex-col min-w-[180px] px-4 py-1 justify-start items-start bg-layer-1-surface rounded-xl shadow-md`}
        >
            <p class={`font-sans font-[400] text-layer-0-glyph capitalize`}>
                {`${`${$t(`common.current_location`)}`}:`}
            </p>
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`${fmt_geol_latitude(map_point_select.lat, `dms`)}`}
            </p>
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`${fmt_geol_longitude(map_point_select.lng, `dms`)}`}
            </p>
        </div>
    {/if}
</EnvelopeLower>
