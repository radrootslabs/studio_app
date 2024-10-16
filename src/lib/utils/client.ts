import { keystore } from "$lib/client";
import { app_notify, type NavigationRoute } from "@radroots/svelte-lib";

export const keystore_reset = async (): Promise<void> => {
    try {
        const ks_keys = await keystore.keys();
        if (`err` in ks_keys) return; //@todo
        for (const ks_key of ks_keys.results) await keystore.remove(ks_key);
    } catch (e) {
        console.log(`(error) keystore_reset `, e);
    }
};

export const restart = async (route_to: true | NavigationRoute, notify_message?: string): Promise<void> => {
    try {
        //await window.splash_show();
        if (notify_message) {
            app_notify.set(notify_message);
        }
        //await route(typeof route_to === `string` ? route_to : `/`)
        //location.reload();
    } catch (e) {
        console.log(`(error) restart `, e);
    }
};
