<script lang="ts">
    import type { GeocoderReverseResult } from "@radroots/geocoder";
    import {
        fmt_geol_latitude,
        fmt_geol_longitude,
        Glyph,
    } from "@radroots/svelte-lib";
    import type { GeolocationCoordinatesPoint } from "@radroots/utils";

    export let basis: {
        point: GeolocationCoordinatesPoint;
        geoc?: GeocoderReverseResult;
    };
    $: basis = basis;
</script>

<button
    class={`flex flex-row justify-center items-center el-re`}
    on:click={async () => {}}
>
    <div
        class={`flex flex-col w-fit px-5 py-[0.7rem] gap-2 justify-start items-start bg-layer-1-surface round-20 shadow-lg`}
    >
        {#if basis.geoc}
            <div class={`flex flex-col w-full gap-2 justify-start items-start`}>
                <div
                    class={`flex flex-col w-full gap-1 justify-start items-start`}
                >
                    <div
                        class={`flex flex-row gap-1 justify-start items-center`}
                    >
                        <p
                            class={`font-sans font-[500] text-[1.05rem] text-layer-2-glyph`}
                        >
                            {basis.geoc.name}
                        </p>
                        <Glyph
                            basis={{
                                classes: `text-layer-2-glyph -translate-y-[2px]`,
                                dim: `xs`,
                                weight: `bold`,
                                key: `map-pin-simple`,
                            }}
                        />
                    </div>
                    <div
                        class={`flex flex-row w-full justify-start items-center`}
                    >
                        <p
                            class={`font-sans font-[500] text-[1rem] tracking-tight text-layer-2-glyph`}
                        >
                            {`${basis.geoc.admin1_name}, ${basis.geoc.country_name}`}
                        </p>
                    </div>
                </div>
                <div class={`flex flex-col w-full justify-start items-start`}>
                    <p
                        class={`font-sans font-[400] text-[0.9rem] text-layer-0-glyph`}
                    >
                        {`${fmt_geol_latitude(basis.point.lat, `dms`)}`}
                    </p>
                    <p
                        class={`font-sans font-[400] text-[0.9rem] text-layer-0-glyph`}
                    >
                        {`${fmt_geol_longitude(basis.point.lng, `dms`)}`}
                    </p>
                </div>
            </div>
        {:else}
            <div class={`flex flex-row w-full justify-start items-center`}>
                <p class={`font-mono font-[400] text-layer-2-glyph`}>
                    {`Marker location:`}
                </p>
            </div>
            <div
                class={`flex flex-row w-full gap-2 justify-start items-center`}
            >
                <p class={`font-mono font-[400] text-layer-2-glyph`}>
                    {`${basis.point.lat.toFixed(4)}`}
                </p>
                <p class={`font-mono font-[400] text-layer-2-glyph`}>
                    {`${basis.point.lng.toFixed(4)}`}
                </p>
            </div>
        {/if}
    </div>
</button>
