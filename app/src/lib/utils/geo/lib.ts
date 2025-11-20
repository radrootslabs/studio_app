import { handle_err, } from "@radroots/apps-lib";
import type { ILocationGcsFields } from "@radroots/tangle-schema-bindings";
import type { IError } from "@radroots/types-bindings";
import { err_msg, location_geohash, type GeocoderReverseResult, type GeolocationPoint } from "@radroots/utils";
import { geoc } from "../app";

export const geolocation_fields_from_point = async (opts: {
    label?: string;
    tag_0?: string;
    geol_p: GeolocationPoint;
}): Promise<ILocationGcsFields | IError<string>> => {
    const { label, geol_p } = opts;
    try {
        const fields: ILocationGcsFields = {
            lat: geol_p.lat,
            lng: geol_p.lng,
            geohash: location_geohash(geol_p),
            tag_0: opts.tag_0,
        }
        if (label) fields.label = label;
        const geoc_rev = await geoc.reverse({ lat: geol_p.lat, lng: geol_p.lng });
        if ("err" in geoc_rev) return err_msg(geoc_rev);
        else if ("results" in geoc_rev && geoc_rev.results.length > 0) {
            const geoc_res = geoc_rev.results[0];
            fields.gc_id = geoc_res.id.toString();
            fields.gc_name = geoc_res.name;
            fields.gc_admin1_id = geoc_res.admin1_id.toString();
            fields.gc_admin1_name = geoc_res.admin1_name;
            fields.gc_country_id = geoc_res.country_id;
            fields.gc_country_name = geoc_res.country_name;
        };
        return fields;
    } catch (e) {
        handle_err(e, `geolocation_fields_from_point`);
        return err_msg(`*`)
    }
};

export const geolocation_fields_from_point_with_geocode = async (opts: {
    label?: string;
    tag_0?: string;
    geoc_r: GeocoderReverseResult;
    geol_p: GeolocationPoint;
}): Promise<ILocationGcsFields | IError<string>> => {
    const { label, geoc_r, geol_p } = opts;
    try {
        const fields: ILocationGcsFields = {
            lat: geol_p.lat,
            lng: geol_p.lng,
            geohash: location_geohash(geol_p),
            tag_0: opts.tag_0 || undefined,
            gc_id: geoc_r.id.toString(),
            gc_name: geoc_r.name,
            gc_admin1_id: geoc_r.admin1_id.toString(),
            gc_admin1_name: geoc_r.admin1_name,
            gc_country_id: geoc_r.country_id,
            gc_country_name: geoc_r.country_name,
        };
        if (label) fields.label = label;
        return fields;
    } catch (e) {
        handle_err(e, `geolocation_fields_from_point_with_geocode`);
        return err_msg(`*`)
    }
};
