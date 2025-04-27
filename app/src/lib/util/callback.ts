import { ls } from "$lib/locale/i18n";
import { get_store, handle_err } from "@radroots/lib-app";
import type { GeocoderReverseResult, GeolocationPoint, IClientGeolocationPosition } from "@radroots/util";
import { geoc, geol, gui } from ".";

export const lc_gui_alert = async (message: string): Promise<boolean> => {
    try {
        return await gui.alert(message);
    } catch (e) {
        await handle_err(e, `lc_gui_alert`);
        return false;
    }
};

export const lc_gui_confirm = async (opts: string | { message: string; ok?: string; cancel?: string }): Promise<boolean> => {
    try {
        return await gui.confirm({
            message: typeof opts === `string` ? opts : opts.message,
            ok: typeof opts === `string` ? undefined : opts.ok,
            cancel: typeof opts === `string` ? undefined : opts.cancel,
        });
    } catch (e) {
        await handle_err(e, `lc_gui_alert`);
        return false;
    }
};

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

export const lc_geop_current = async (show_alert: boolean | string = false): Promise<IClientGeolocationPosition | undefined> => {
    const $ls = get_store(ls);
    const geop = await geol.current();
    if (`err` in geop) {
        if (show_alert) {
            await gui.alert(
                typeof show_alert === `string` ? show_alert :
                    `${$ls(`icu.failure_*`, { value: `${$ls(`icu.reading_*`, { value: `${$ls(`common.geocode`)}`.toLowerCase() })}` })}`,
            )
        }
        return undefined
    }
    return geop;
};