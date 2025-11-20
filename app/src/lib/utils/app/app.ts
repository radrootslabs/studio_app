import { goto } from "$app/navigation";
import { datastore, db, nostr_keys, notif } from "$lib/utils/app";
import { ls } from "$lib/utils/i18n";
import { create_router, get_store, handle_err } from "@radroots/apps-lib";
import { app_notify } from "@radroots/apps-lib-pwa/stores/app";
import type { CallbackPromise } from "@radroots/utils";
import { reset_sql_cipher } from "./cipher";
import type { NavigationRoute } from "./routes";

export const route = create_router<NavigationRoute>();

export const restart = async (opts?: {
    notify_message?: string;
    route?: NavigationRoute;
}): Promise<void> => {
    try {
        goto(opts?.route || `/`);
        if (opts?.notify_message) app_notify.set(opts.notify_message);
        location.reload();
    } catch (e) {
        handle_err(e, `restart`);
    }
};

export const reset = async (): Promise<void> => {
    try {
        const $ls = get_store(ls);
        const confirm = await notif.confirm({
            message: `${$ls(`notification.device.reset`)}. ${$ls(`common.this_action_is_irreversible`)}. ${$ls(`common.do_you_want_to_continue_q`)}`
        });
        if (!confirm) return;
        await nostr_keys.reset();
        await datastore.reset();
        await reset_sql_cipher(db.get_store_key());
        await db.reinit();
        goto(`/`);
        app_notify.set(`${$ls(`notification.device.reset_complete`)}`);
    } catch (e) {
        handle_err(e, `reset`);
    }
};

export const message_callback = async (message: string, callback: CallbackPromise): Promise<void> => {
    notif.alert(message);
    await callback();
};
