<script lang="ts">
    import { geoc } from "$lib/client";
    import { cfg } from "$lib/conf";
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import { app_thc, fmt_cl, sleep } from "@radroots/svelte-lib";
    import { MapLibre, Marker } from "@radroots/svelte-maplibre";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";
    import { onMount } from "svelte";
    import MapMarkerDot from "./map_marker_dot.svelte";
    import MapPopupLocationInfo from "./map_popup_location_info.svelte";

    export let basis: {
        classes_wrap?: string;
        classes?: string;
    };
    $: basis = basis;

    let map_visible = false;

    //export let map_point_center: GeolocationCoordinatesPoint;
    export let map_point_select: GeolocationCoordinatesPoint;
    export let map_point_select_geoc: GeocoderReverseResult | undefined =
        undefined;

    let map_point_center: GeolocationCoordinatesPoint | undefined = undefined;

    onMount(async () => {
        try {
            map_point_center = {
                ...map_point_select,
            };
            await sleep(300);
        } catch (e) {
        } finally {
            map_visible = true;
        }
    });

    $: {
        if (
            map_point_center &&
            map_point_center.lat !== 0 &&
            map_point_center.lng !== 0
        ) {
            (async () => {
                try {
                    const geoc_res = await geoc.reverse({
                        point: map_point_center,
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

<div
    class={`${fmt_cl(basis.classes_wrap)} flex flex-col justify-start items-center`}
>
    <div
        class={`relative flex flex-col w-full justify-center items-center bg-layer-1-surface overflow-hidden`}
    >
        <MapLibre
            center={map_point_center}
            zoom={10}
            class={`${fmt_cl(basis.classes || `h-full w-full`)} ${map_visible ? `fade-in` : `hidden`}`}
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
                    <MapPopupLocationInfo
                        basis={{
                            point: map_point_select,
                            geoc: map_point_select_geoc,
                        }}
                    />
                </button>
                <MapMarkerDot />
            </Marker>
        </MapLibre>
    </div>
</div>
