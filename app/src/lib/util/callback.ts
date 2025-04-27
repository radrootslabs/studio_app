import { handle_err } from "@radroots/lib-app";
import type { GeocoderReverseResult, GeolocationPoint } from "@radroots/util";
import { geoc } from ".";

export const lc_geocode = async (geoc_p: GeolocationPoint): Promise<GeocoderReverseResult | undefined> => {
    try {
        await geoc.connect();
        const geoc_res = await geoc.reverse(geoc_p);
        if (
            `results` in geoc_res &&
            geoc_res.results.length > 0
        )
            return geoc_res.results[0];
    } catch (e) {
        await handle_err(e, `lc_geocode`);
    }
};
