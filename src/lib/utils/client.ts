import { db, keystore } from "$lib/client";
import { app_notify, route, type NavigationRoute } from "@radroots/svelte-lib";
import type { ErrorMessage } from "@radroots/utils";

export const keystore_reset = async (): Promise<ErrorMessage<string> | undefined> => {
    try {
        const ks_keys = await keystore.keys();
        if (`err` in ks_keys) return ks_keys;
        for (const ks_key of ks_keys.results) await keystore.remove(ks_key);
    } catch (e) {
        console.log(`(error) keystore_reset `, e);
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
        console.log(`(error) restart `, e);
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
        console.log(`(error) reset_device `, e);
    }
};
