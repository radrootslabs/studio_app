import { goto } from "$app/navigation";
import { _env } from "$lib/_env";
import { cfg_data, cfg_datastore_key_map, cfg_datastore_key_obj_map, cfg_datastore_key_param_map } from "$lib/utils/config";
import { ls } from "$lib/utils/i18n";
import { create_router, get_store, handle_err } from "@radroots/apps-lib";
import { app_notify } from "@radroots/apps-lib-pwa/stores/app";
import { WebDatastore } from "@radroots/client/datastore";
import { WebFs } from "@radroots/client/fs";
import { WebGeolocation } from "@radroots/client/geolocation";
import { IDB_CONFIG_DATASTORE, IDB_CONFIG_KEYSTORE_NOSTR, RADROOTS_IDB_DATABASE, idb_store_bootstrap } from "@radroots/client/idb";
import { WebKeystoreNostr } from "@radroots/client/keystore";
import { WebNotifications } from "@radroots/client/notifications";
import { WebClientRadroots } from "@radroots/client/radroots";
import { WebTangleDatabase } from "@radroots/client/tangle";
import { Geocoder } from "@radroots/geocoder";
import { WebHttp } from "@radroots/http";
import type { CallbackPromise } from "@radroots/utils";
import { reset_sql_cipher } from "./cipher";
import type { NavigationRoute } from "./routes";

const { GEOCODER_DB_URL, RADROOTS_API, SQL_WASM_URL } = _env;

const ls_val = get_store(ls);

declare const __APP_GIT_HASH__: string;
declare const __APP_NAME__: string;
declare const __APP_VERSION__: string;

export const datastore = new WebDatastore(
    cfg_datastore_key_map,
    cfg_datastore_key_param_map,
    cfg_datastore_key_obj_map,
    IDB_CONFIG_DATASTORE
);
export const fs = new WebFs();
export const geol = new WebGeolocation();
export const geoc = new Geocoder();
export const http = new WebHttp({
    app_name: __APP_NAME__,
    app_version: __APP_VERSION__,
    app_hash: __APP_GIT_HASH__
});
export const notif = new WebNotifications();
export const radroots = new WebClientRadroots(RADROOTS_API);
export const nostr_keys = new WebKeystoreNostr(IDB_CONFIG_KEYSTORE_NOSTR);

export const db = new WebTangleDatabase({
    cipher_config: cfg_data.sql_cipher,
    sql_wasm_path: SQL_WASM_URL,
});

let db_i: Promise<WebTangleDatabase> | null = null;
let db_init_promise: Promise<void> | null = null;
let geoc_init_promise: Promise<void> | null = null;
let app_init_promise: Promise<void> | null = null;

export const db_init = async (): Promise<void> => {
    if (!db_init_promise) {
        db_init_promise = (async () => {
            await idb_store_bootstrap(RADROOTS_IDB_DATABASE);
            await db.init();
        })();
    }
    try {
        await db_init_promise;
    } catch (e) {
        db_init_promise = null;
        throw e;
    }
};

export const geoc_init = async (): Promise<void> => {
    if (!geoc_init_promise) {
        geoc_init_promise = (async () => {
            const geoc_ready = await geoc.connect({
                wasm_path: SQL_WASM_URL,
                database_path: GEOCODER_DB_URL || "/assets/geonames.db"
            });
            if (geoc_ready !== true) throw new Error(geoc_ready.err);
        })();
    }
    try {
        await geoc_init_promise;
    } catch (e) {
        geoc_init_promise = null;
        throw e;
    }
};

export const app_init = async (): Promise<void> => {
    if (!app_init_promise) {
        app_init_promise = (async () => {
            await idb_store_bootstrap(RADROOTS_IDB_DATABASE);
            await db_init();
            await geoc_init();
        })();
    }
    try {
        await app_init_promise;
    } catch (e) {
        app_init_promise = null;
        throw e;
    }
};

export const create_db = async (): Promise<WebTangleDatabase> => {
    if (!db_i) {
        const db_client = new WebTangleDatabase({
            sql_wasm_path: SQL_WASM_URL
        });
        db_i = (async () => {
            await db_client.init();
            return db_client;
        })();
    }
    return db_i;
};

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
        const confirm = await notif.confirm({
            message: `${ls_val(`notification.device.reset`)}. ${ls_val(`common.this_action_is_irreversible`)}. ${ls_val(`common.do_you_want_to_continue_q`)}`
        });
        if (!confirm) return;
        await nostr_keys.reset();
        await datastore.reset();
        await reset_sql_cipher(db.get_store_key());
        await db.reinit();
        goto(`/`);
        app_notify.set(`${ls_val(`notification.device.reset_complete`)}`);
    } catch (e) {
        handle_err(e, `reset`);
    }
};

export const message_callback = async (message: string, callback: CallbackPromise): Promise<void> => {
    notif.alert(message);
    await callback();
};
