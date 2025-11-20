import { PUBLIC_RADROOTS_URL } from "$env/static/public";
import { cfg_data, cfg_datastore_key_map, cfg_datastore_key_obj_map, cfg_datastore_key_param_map } from "$lib/utils/config";
import { WebDatastore } from "@radroots/client/datastore";
import { WebFs } from "@radroots/client/fs";
import { WebGeolocation } from "@radroots/client/geolocation";
import { WebHttp } from "@radroots/client/http";
import { WebKeystoreNostr } from "@radroots/client/keystore";
import { WebNotifications } from "@radroots/client/notifications";
import { WebClientRadroots } from "@radroots/client/radroots";
import { WebTangleDatabase } from "@radroots/client/tangle";
import { Geocoder } from "@radroots/geocoder";

export const datastore = new WebDatastore(
    cfg_datastore_key_map,
    cfg_datastore_key_param_map,
    cfg_datastore_key_obj_map,
    {
        database: "radroots-pwa-v1-datastore"
    }
);
export const fs = new WebFs();
export const geol = new WebGeolocation();
export const geoc = new Geocoder();
export const http = new WebHttp();
export const notif = new WebNotifications();
export const radroots = new WebClientRadroots(PUBLIC_RADROOTS_URL);
export const nostr_keys = new WebKeystoreNostr({
    database: "radroots-pwa-v1-keystore-nostr"
});

export const db = new WebTangleDatabase({
    cipher: cfg_data.sql_cipher
});

let db_i: Promise<WebTangleDatabase> | null = null;

export const create_db = async (): Promise<WebTangleDatabase> => {
    if (!db_i) {
        const db_client = new WebTangleDatabase();
        db_i = (async () => {
            await db_client.init();
            return db_client;
        })();
    }
    return db_i;
};
