import { PUBLIC_RADROOTS_RELAY_URL } from "$env/static/public";
import type { NostrEventTagClient } from "@radroots/nostr-util";
import { root_symbol } from "@radroots/util";

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
    relay_url: PUBLIC_RADROOTS_RELAY_URL,
    relay_pubkey: radroots_nostr_pubkey,
    relay_polling_count_max: 10,
};

export const cfg_nostr_client: NostrEventTagClient = {
    name: root_symbol,
    pubkey: cfg_nostr.relay_pubkey,
    relay: cfg_nostr.relay_url
};
