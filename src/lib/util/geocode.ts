import { geoc } from "$lib/client";
import type { GeocoderReverseResult } from "@radroots/geocoder";
import type { LocationGcs } from "@radroots/models";
import { catch_err } from "@radroots/svelte-lib";
import type { GeolocationCoordinatesPoint } from "@radroots/utils";

export const geoc_rev = async (point: GeolocationCoordinatesPoint): Promise<GeocoderReverseResult | undefined> => {
    try {
        const geoc_res = await geoc.reverse({ point });
        if (`results` in geoc_res && geoc_res.results.length > 0)
            return geoc_res.results[0];
    } catch (e) {
        await catch_err(e, `geoc_rev`);
    }
};


export const location_gcs_to_geoc = (opts: LocationGcs): GeocoderReverseResult | undefined => {
    const {
        gc_id: id,
        gc_name: name,
        gc_admin1_id: admin1_id,
        gc_admin1_name: admin1_name,
        gc_country_id: country_id,
        gc_country_name: country_name,
        lat: latitude,
        lng: longitude,
    } = opts;
    if (
        (typeof id === `string` && id) &&
        (typeof name === `string` && name) &&
        (typeof admin1_id === `string` && admin1_id) &&
        (typeof admin1_name === `string` && admin1_name) &&
        (typeof country_id === `string` && country_id) &&
        (typeof country_name === `string` && country_name) &&
        (typeof latitude === `number`) &&
        (typeof longitude === `number`)
    ) {
        return {
            id: Number(id),
            name,
            admin1_id,
            admin1_name,
            country_id,
            country_name,
            latitude,
            longitude
        }
    }
    return undefined;
};