<script lang="ts">
    import { cfg } from "$lib/conf";
    import { app_thc } from "@radroots/svelte-lib";
    import { MapLibre, Marker } from "@radroots/svelte-maplibre";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";
    import MapMarkerDot from "./map_marker_dot.svelte";

    export let basis: {
        point: GeolocationCoordinatesPoint;
        zoom?: number;
    };
</script>

<MapLibre
    center={basis.point}
    zoom={basis.zoom || 4}
    class={`h-full w-full`}
    style={cfg.map.styles.base[$app_thc]}
    attributionControl={false}
>
    <Marker bind:lngLat={basis.point} draggable={false}>
        <MapMarkerDot />
    </Marker>
</MapLibre>
