import { keystore } from "$lib/client";
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
        const ks_keys = await keystore.keys();
        console.log(JSON.stringify(ks_keys, null, 4), `ks_keys`)
        if (`err` in ks_keys) return ks_keys;
        for (const ks_key of ks_keys.results) await keystore.remove(ks_key);
        // @todo add database reset
        await route(`/`);
    } catch (e) {
        console.log(`(error) reset_device `, e);
    }
};
