import { PUBLIC_RADROOTS_NOSTR_PUBKEY, PUBLIC_RADROOTS_RELAY_URL } from "$env/static/public";
import type { NostrEventTagClient } from "@radroots/nostr-util";
import { root_symbol } from "@radroots/util";

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
    relay_pubkey: PUBLIC_RADROOTS_NOSTR_PUBKEY,
    relay_polling_count_max: 10,
};

export const cfg_nostr_client: NostrEventTagClient = {
    name: root_symbol,
    pubkey: cfg_nostr.relay_pubkey,
    relay: cfg_nostr.relay_url
};
