import { db, geoc } from "$lib/client";
import type { IClientGeolocationPosition } from "@radroots/client";
import type { GeocoderReverseResult } from "@radroots/geocoder";
import type { ILocationGcsAddResolve, LocationGcsFormFields } from "@radroots/models";
import { err_msg, location_geohash } from "@radroots/utils";

export const model_location_gcs_add_position = async (opts: {
    label?: string;
    geo_loc: IClientGeolocationPosition;
}): Promise<ILocationGcsAddResolve<string>> => {
    try {
        const { label, geo_loc } = opts;
        const fields: LocationGcsFormFields = {
            lat: geo_loc.lat.toString(),
            lng: geo_loc.lng.toString(),
            geohash: location_geohash(geo_loc),
        }
        if (label) fields.label = label;
        const geoc_rev = await geoc.reverse({
            point: {
                lat: geo_loc.lat,
                lng: geo_loc.lng
            }
        });
        if (`results` in geoc_rev && geoc_rev.results.length > 0) {
            const geoc_res = geoc_rev.results[0];
            fields.gc_id = geoc_res.id.toString();
            fields.gc_name = geoc_res.name;
            fields.gc_admin1_id = geoc_res.admin1_id.toString();
            fields.gc_admin1_name = geoc_res.admin1_name;
            fields.gc_country_id = geoc_res.country_id;
            fields.gc_country_name = geoc_res.country_name;
        };
        const res = await db.location_gcs_add(fields);
        return res;
    } catch (e) {
        console.log(`(error) model_location_gcs_add_position `, e);
        return err_msg(`*`)
    }
};

export const model_location_gcs_add_geocode = async (opts: {
    label?: string;
    geo_code: GeocoderReverseResult;
}): Promise<ILocationGcsAddResolve<string>> => {
    try {
        const { label, geo_code } = opts;
        const fields: LocationGcsFormFields = {
            lat: geo_code.latitude.toString(),
            lng: geo_code.longitude.toString(),
            geohash: location_geohash({ lat: geo_code.latitude, lng: geo_code.longitude }),
            gc_id: geo_code.id.toString(),
            gc_name: geo_code.name,
            gc_admin1_id: geo_code.admin1_id.toString(),
            gc_admin1_name: geo_code.admin1_name,
            gc_country_id: geo_code.country_id,
            gc_country_name: geo_code.country_name,
        };
        if (label) fields.label = label;
        const res = await db.location_gcs_add(fields);
        return res;
    } catch (e) {
        console.log(`(error) model_location_gcs_add_geocode `, e);
        return err_msg(`*`)
    }
};
