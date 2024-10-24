import { keystore } from "$lib/client";
import { app_notify, route, type NavigationRoute } from "@radroots/svelte-lib";

export const keystore_reset = async (): Promise<void> => {
    try {
        const ks_keys = await keystore.keys();
        if (`err` in ks_keys) return; //@todo
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
