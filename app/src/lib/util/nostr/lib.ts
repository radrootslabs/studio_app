import { ls } from "$lib/locale/i18n";
import { nostr_poll_relays_handler, nostr_sync_handler } from "@radroots/lib-app";
import { type NostrRelayFormFields, nostr_relay_parse_form_keys } from "@radroots/models";
import { throw_err } from "@radroots/util";
import { db, gui, http } from "..";

export const nostr_poll_relays = async (): Promise<void> => {
    await nostr_poll_relays_handler({
        ls,
        callback_alert: async (msg: string) => void await gui.alert(msg),
        callback_relay_urls: async (public_key: string) => {
            const list_relays = await db.nostr_relay_read_list({
                table: [`on_profile`, { public_key }],
            });
            if (`err` in list_relays) return throw_err(list_relays.err);
            return list_relays.results.map(({ id, url }) => ({ id, url }));
        },
        callback_fetch_document: async (relay_url: string) => {
            return await http.fetch({
                url: relay_url.replace(`ws://`, `http://`).replace(`wss://`, `https://`),
                headers: {
                    Accept: "application/nostr+json",
                },
            });
        },
        callback_set_relay_document: async ({ url, doc }) => {
            const fields: Partial<NostrRelayFormFields> = {};
            for (const [k, v] of Object.entries(doc)) {
                const field_k = nostr_relay_parse_form_keys(k);
                if (field_k) fields[field_k] = v;
            }
            if (Object.keys(fields).length < 1) return;
            await db.nostr_relay_update({
                filter: { url },
                fields,
            });
        },
    });
};

export const nostr_sync = async (): Promise<void> => {
    await nostr_sync_handler({
        ls,
        callback_alert: async (msg: string) => void await gui.alert(msg),
        callback_confirm: async (message: string) => {
            return await gui.confirm({ message });
        },
        callback_metadata: async (public_key: string) => {
            const tb_nostr_profile = await db.nostr_profile_read({ public_key });
            if (`err` in tb_nostr_profile) throw_err(tb_nostr_profile);
            return tb_nostr_profile.result;
        },
        callback_relay_urls: async (public_key: string) => {
            const list_relays = await db.nostr_relay_read_list({
                table: [`on_profile`, { public_key }],
            });
            if (`err` in list_relays) return throw_err(list_relays.err);
            return list_relays.results.map(({ id, url }) => ({ id, url }));
        },
    });
};
