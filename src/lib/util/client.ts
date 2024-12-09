import { db, dialog, keystore } from "$lib/client";
import { app_notify, catch_err, el_id, route, sleep, type CallbackPromise, type NavigationRoute } from "@radroots/svelte-lib";
import type { ThemeLayer } from "@radroots/theme";
import type { ErrorMessage } from "@radroots/utils";

export const keystore_reset = async (): Promise<ErrorMessage<string> | undefined> => {
    try {
        const ks_keys = await keystore.keys();
        if (`err` in ks_keys) return ks_keys;
        for (const ks_key of ks_keys.results) await keystore.remove(ks_key);
    } catch (e) {
        await catch_err(e, `keystore_reset`);
    }
};

export const callback_alert = async (message: string, callback: CallbackPromise): Promise<void> => {
    try {
        dialog.alert(message);
        await callback();
    } catch (e) {
        await catch_err(e, `cb_alert`);
    }
};

export const page_reload = async (message?: string): Promise<void> => {
    try {
        if (message) dialog.alert(message);
        location.reload();
    } catch (e) {
        await catch_err(e, `page_reload`);
    }
};

export const restart = async (opts?: {
    notify_message?: string;
    route?: NavigationRoute;
}): Promise<void> => {
    try {
        if (opts?.notify_message) app_notify.set(opts.notify_message);
        if (opts?.route) await route(opts.route);
        else location.reload();
    } catch (e) {
        await catch_err(e, `restart`);
    }
};

export const reset_device = async (): Promise<ErrorMessage<string> | undefined> => {
    try {
        // delete keystore keys
        const ks_keys = await keystore.keys();
        if (`err` in ks_keys) return ks_keys;
        for (const ks_key of ks_keys.results) await keystore.remove(ks_key);
        // delete database tables
        const location_gcss = await db.location_gcs_get({ list: [`all`] });
        if (`err` in location_gcss) return location_gcss;
        for (const { id } of location_gcss.results) await db.location_gcs_delete({ id });
        const trade_products = await db.trade_product_get({ list: [`all`] });
        if (`err` in trade_products) return trade_products;
        for (const { id } of trade_products.results) await db.trade_product_delete({ id });
        const nostr_profiles = await db.nostr_profile_get({ list: [`all`] });
        if (`err` in nostr_profiles) return nostr_profiles;
        for (const { id } of nostr_profiles.results) await db.nostr_profile_delete({ id });
        const nostr_relays = await db.nostr_relay_get({ list: [`all`] });
        if (`err` in nostr_relays) return nostr_relays;
        for (const { id } of nostr_relays.results) await db.nostr_relay_delete({ id });
        await route(`/`);
    } catch (e) {
        await catch_err(e, `reset_device`);
    }
};

export const el_focus = async (id: string, callback: () => Promise<void>, layer: ThemeLayer = 1): Promise<void> => {
    try {
        const el = el_id(id);
        el?.classList.add(`entry-layer-${layer}-highlight`);
        el?.focus();
        await sleep(1200);
        await callback();
        el?.classList.remove(`entry-layer-${layer}-highlight`);
    } catch (e) {
        await catch_err(e, `el_focus`);
    }
};
