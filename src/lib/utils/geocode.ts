import { geoc } from "$lib/client";
import type { GeocoderReverseResult } from "@radroots/geocoder";
import type { GeolocationCoordinatesPoint } from "@radroots/utils";

export const geoc_rev = async (point: GeolocationCoordinatesPoint): Promise<GeocoderReverseResult | undefined> => {
    try {
        const geoc_res = await geoc.reverse({ point });
        if (`results` in geoc_res && geoc_res.results.length > 0)
            return geoc_res.results[0];
    } catch (e) {
        console.log(`(error) geoc_rev `, e);
    }
};