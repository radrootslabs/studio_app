import { PUBLIC_RADROOTS_NOSTR_PUBKEY } from "$env/static/public";
import type { NostrTagClient, NumberTuple } from "@radroots/utils";

export const ks = {
    cfg_init: {
        nostr_secretkey: `cfg:init:nostr:secretkey`,
        nostr_profilename: `cfg:init:nostr:profilename`,
        radroots_tok: `cfg:init:radroots:tok`
    },
    pref: {
        cfg_type: `pref:cfg_type`,
    },
    keys: {
        nostr_secretkey: (public_key: string) => `nostr:key:${public_key}`,
        nostr_publickey: `keys:nostr:`,
    }
};

export const root_symbol = "»--`--,---";

export const err = {
    nostr: {
        no_relays: `error.nostr.no_relays_connected`
    }
}

export const cfg = {
    app: {
        title: `Radroots`,
        description: `Creating networks between farmers, communities and small businesses that give customers greater access to natural foods and grow circular economies where profits are more fairly distributed. Radroots is built on the Nostr protocol and released under a copyleft open source license to provide transparency and give users the option to offer feedback and add or request new features.`,
        version: `0.0.1`
    },
    nostr: {
        relay_url: `wss://radroots.org`,
        relay_pubkey: PUBLIC_RADROOTS_NOSTR_PUBKEY,
        relay_polling_count_max: 10,
    },
    delay: {
        load: 321,
        notify: 123,
        mount_el: 500,
        nostr_relay_poll_document: 3000,
        entry_focus: 2000,
        load_notify: 3000,
    },
    debounce: {
        search: 200
    },
    cmd: {
        layout_route: `*-route`
    },
    map: {
        styles: {
            base: {
                light: `https://basemaps.cartocdn.com/gl/voyager-gl-style/style.json`,
                dark: `https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json`
            }
        },
        popup: {
            dot: {
                offset: [0, -10] as NumberTuple
            }
        },
        coords: {
            default: {
                lat: 0,
                lng: 0,
            }
        }
    }
};

export const nostr_client: NostrTagClient = {
    name: root_symbol,
    pubkey: cfg.nostr.relay_pubkey,
    relay: cfg.nostr.relay_url
};

export const scroll_args: {
    into_view: ScrollIntoViewOptions
} = {
    into_view: {
        behavior: `smooth`,
        block: `nearest`,
        inline: `start`,
    }
}