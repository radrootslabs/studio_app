import { handle_err, } from "@radroots/lib-app";
import type { ILocationGcsCreateResolve, LocationGcsFormFields } from "@radroots/models";
import { err_msg, location_geohash, type GeocoderReverseResult, type GeolocationCoordinatesPoint, type GeolocationPoint } from "@radroots/util";
import { db, geoc } from "..";

export const model_location_gcs_create_geol_p = async (opts: {
    label?: string;
    kind: string;
    geol_p: GeolocationPoint;
}): Promise<ILocationGcsCreateResolve<string>> => {
    try {
        const { label, geol_p } = opts;
        const fields: LocationGcsFormFields = {
            lat: geol_p.lat.toString(),
            lng: geol_p.lng.toString(),
            geohash: location_geohash(geol_p),
            kind: opts.kind,
        }
        if (label) fields.label = label;
        const geoc_rev = await geoc.reverse({
            lat: geol_p.lat,
            lng: geol_p.lng
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
        const res = await db.location_gcs_create(fields);
        return res;
    } catch (e) {
        await handle_err(e, `model_location_gcs_add_position`);
        return err_msg(`*`)
    }
};

export const model_location_gcs_create_geoc_r = async (opts: {
    label?: string;
    kind: string;
    geoc_r: GeocoderReverseResult;
    geol_p: GeolocationCoordinatesPoint;
}): Promise<ILocationGcsCreateResolve<string>> => {
    try {
        const { label, geoc_r, geol_p } = opts;
        const fields: LocationGcsFormFields = {
            lat: geol_p.lat.toString(),
            lng: geol_p.lng.toString(),
            geohash: location_geohash(geol_p),
            kind: opts.kind,
            gc_id: geoc_r.id.toString(),
            gc_name: geoc_r.name,
            gc_admin1_id: geoc_r.admin1_id.toString(),
            gc_admin1_name: geoc_r.admin1_name,
            gc_country_id: geoc_r.country_id,
            gc_country_name: geoc_r.country_name,
        };
        if (label) fields.label = label;
        const res = await db.location_gcs_create(fields);
        return res;
    } catch (e) {
        await handle_err(e, `model_location_gcs_add_geocode`);
        return err_msg(`*`)
    }
};
