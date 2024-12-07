<script lang="ts">
    import { geoc } from "$lib/client";
    import { cfg } from "$lib/conf";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import { app_thc } from "@radroots/svelte-lib";
    import { MapLibre, Marker } from "@radroots/svelte-maplibre";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";
    import MapMarkerDot from "./map_marker_dot.svelte";
    import MapPopupPointGeolocation from "./map_popup_point_geolocation.svelte";

    export let map_point: GeolocationCoordinatesPoint;
    export let map_geoc: GeocoderReverseResult | undefined = undefined;
    export let map_center: GeolocationCoordinatesPoint = cfg.map.coords.default;

    $: if (map_point && map_center.lat === 0 && map_center.lng === 0) {
        map_center = {
            lat: map_point.lat,
            lng: map_point.lng - 0.065,
        };
    }

    $: {
        if (
            map_point &&
            map_center &&
            map_center.lat !== 0 &&
            map_center.lng !== 0
        ) {
            (async () => {
                try {
                    const geoc_res = await geoc.reverse({
                        point: map_point,
                    });
                    if (`results` in geoc_res && geoc_res.results.length > 0)
                        map_geoc = geoc_res.results[0];
                    else map_geoc = undefined;
                } catch (e) {
                    console.log(`(error) map choose location`, e);
                }
            })();
        }
    }
</script>

{#if map_point}
    <MapLibre
        center={map_center}
        zoom={10}
        class={`h-full w-full`}
        style={cfg.map.styles.base[$app_thc]}
        attributionControl={false}
    >
        <Marker
            bind:lngLat={map_point}
            draggable
            on:dragend={async () => {
                if (!map_point) return;
                const geoc_res = await geoc.reverse({
                    point: map_point,
                    limit: 1,
                });
                if (`results` in geoc_res && geoc_res.results.length > 0)
                    map_geoc = geoc_res.results[0];
            }}
        >
            <button
                class={`flex flex-row justify-center items-center transform -translate-x-[42%]`}
                on:click={async () => {}}
            >
                <MapPopupPointGeolocation
                    basis={{
                        point: map_point,
                        geoc: map_geoc,
                    }}
                />
            </button>
            <MapMarkerDot />
        </Marker>
    </MapLibre>
{/if}
