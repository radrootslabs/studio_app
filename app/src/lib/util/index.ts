import { browser } from "$app/environment";
import { goto } from "$app/navigation";
import { PUBLIC_RADROOTS_URL } from "$env/static/public";
import { ls } from "$lib/locale/i18n";
import { TauriClientDatabase, TauriClientDatastore, TauriClientFs, TauriClientGeolocation, TauriClientGui, TauriClientHttp, TauriClientKeys, TauriClientRadroots } from "@radroots/client";
import { Geocoder } from "@radroots/geocoder";
import { app_notify, get_store, handle_err, NostrSyncService, type NavigationRouteParamKey, type NavigationRouteParamTuple } from "@radroots/lib-app";
import { NostrEventService, NostrKeyService } from "@radroots/nostr-util";
import { encode_route, type CallbackPromise } from "@radroots/util";
import type { NavigationRoute } from "./routes";

export const db = new TauriClientDatabase();
export const datastore = new TauriClientDatastore();
export const fs = new TauriClientFs();
export const geol = new TauriClientGeolocation();
export const gui = new TauriClientGui();
export const http = new TauriClientHttp();
export const keys = new TauriClientKeys();
export const radroots = new TauriClientRadroots(PUBLIC_RADROOTS_URL);

export const geoc = new Geocoder();
export const nostrkey = new NostrKeyService();
export const nostre = new NostrEventService();
export let nostrsync: NostrSyncService
if (browser) nostrsync = new NostrSyncService();

export const route = async (nav_route: NavigationRoute, params: NavigationRouteParamTuple[] = []): Promise<void> => {
    try {
        if (params.length) await goto(encode_route<NavigationRoute, NavigationRouteParamKey>(nav_route, params));
        else await goto(nav_route);
    } catch (e) {
        await handle_err(e, `route`);
    };
};

export const restart = async (opts?: {
    notify_message?: string;
    route?: NavigationRoute;
}): Promise<void> => {
    try {
        goto(opts?.route || `/`);
        if (opts?.notify_message) app_notify.set(opts.notify_message);
        location.reload();
    } catch (e) {
        await handle_err(e, `restart`);
    }
};

export const reset = async (): Promise<void> => {
    try {
        const $ls = get_store(ls);
        const confirm = await gui.confirm({
            message: `${$ls(`notification.device.reset`)}. ${$ls(`common.this_action_is_irreversible`)}. ${$ls(`common.do_you_want_to_continue_q`)}`
        });
        if (!confirm) return;
        await keys.nostr_keystore_reset();
        await db.reset();
        goto(`/`)
        app_notify.set(`${$ls(`notification.device.reset_complete`)}`);
    } catch (e) {
        await handle_err(e, `reset`);
    }
};

export const message_callback = async (message: string, callback: CallbackPromise): Promise<void> => {
    gui.alert(message);
    await callback();
};