import { _env } from "$lib/_env";
import type { AppConfigRole } from "@radroots/apps-lib-pwa/types/app";
import { root_symbol } from "@radroots/utils";
import type { NostrEventTagClient } from "@radroots/nostr";

export const cfg_data = {
    sql_cipher: {
        database: "radroots-pwa-v1",
        store: "radroots.security.cipher.sql",
    }
} as const;

export const _cfg = {
    role_default: `public`
} as const;

export const radroots_nostr_pubkey = `1f5a37bd7050cffe4dd9c35e029fa1da5c84f429de0818c9e22ecf122815e184`

export const err = {
    nostr: {
        no_relays: `error.nostr.no_relays_connected`
    }
};

export const cfg_delay = {
    load: 321,
    notify: 123,
    mount_el: 500,
    nostr_relay_poll_document: 3000,
    entry_focus: 2000,
    load_notify: 3000,
};

export const cfg_nostr = {
    relay_url: _env.RADROOTS_RELAY,
    relay_pubkey: radroots_nostr_pubkey,
    relay_polling_count_max: 10,
};

export const cfg_nostr_client: NostrEventTagClient = {
    name: root_symbol,
    pubkey: cfg_nostr.relay_pubkey,
    relay: cfg_nostr.relay_url
};

export const cfg_datastore_key_map = {
    //cfg_nostr_key: "cfg:nostr:key",
    //cfg_profile: "cfg:profile",
    nostr_key: "nostr:key",
    eula_date: "app:eula:date",
} as const;

export const cfg_datastore_key_param_map = {
    nostr_profile: (public_key: string) => `nostr:${public_key}:profile`,
    radroots_profile: (public_key: string) => `radroots:${public_key}:profile`,
} as const;


export type ConfigData = {
    nostr_public_key?: string;
    nostr_profile?: string;
    role?: AppConfigRole;
    nip05_request?: boolean;
    nip05_key?: string;
};

export type AppData = {
    active_key: string;
    role: AppConfigRole;
    eula_date: string;
    nip05_key?: string;
};

export type cfg_datastore_key_obj_map_types = {
    cfg_data: ConfigData;
    app_data: AppData;
}

export const cfg_datastore_key_obj_map: Record<keyof cfg_datastore_key_obj_map_types, string> = {
    cfg_data: "cfg:data",
    app_data: "app:data",
};
