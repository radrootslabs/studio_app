import { db, http } from "$lib/client";
import { cfg } from "$lib/conf";
import { parse_nostr_relay_form_keys, type NostrRelayFormFields } from "@radroots/models";
import { app_nostr_key, catch_err, get_store, nostr_relays_connected, nostr_relays_poll_documents, nostr_relays_poll_documents_count } from "@radroots/svelte-lib";
import { parse_nostr_relay_information_document_fields } from "@radroots/utils";
import { throw_err } from "./client";

export const fetch_relay_documents = async (): Promise<void> => {
    try {
        const $nostr_relays_poll_documents_count = get_store(nostr_relays_poll_documents_count);
        const $app_nostr_key = get_store(app_nostr_key);
        const $nostr_relays_connected = get_store(nostr_relays_connected);
        if (
            $nostr_relays_poll_documents_count >=
            cfg.nostr.relay_polling_count_max
        ) {
            nostr_relays_poll_documents.set(false);
            return;
        }
        nostr_relays_poll_documents_count.set(
            $nostr_relays_poll_documents_count + 1,
        );
        const nostr_relays = await db.nostr_relay_get({
            list: [`on_profile`, { public_key: $app_nostr_key }],
        });
        if (`err` in nostr_relays) return throw_err(nostr_relays.err);
        const unconnected_relays = nostr_relays.results.filter(
            (i) => !$nostr_relays_connected.includes(i.id),
        );
        if (unconnected_relays.length === 0) return void nostr_relays_poll_documents.set(false);
        for (const nostr_relay of unconnected_relays) {
            const res = await http.fetch({
                url: nostr_relay.url.replace(`ws://`, `http://`),
                headers: {
                    Accept: "application/nostr+json",
                },
            });
            if (`err` in res) continue;
            else if (res.status === 200 && res.data) {
                const doc = parse_nostr_relay_information_document_fields(
                    res.data,
                );
                if (!doc) continue;
                const fields: Partial<NostrRelayFormFields> = {};
                for (const [k, v] of Object.entries(doc)) {
                    const field_k = parse_nostr_relay_form_keys(k);
                    if (field_k) fields[field_k] = v;
                }
                if (Object.keys(fields).length < 1) continue;
                await db.nostr_relay_update({
                    on: {
                        url: nostr_relay.url,
                    },
                    fields,
                });
                nostr_relays_connected.set(
                    Array.from(
                        new Set([
                            ...$nostr_relays_connected,
                            nostr_relay.id,
                        ]),
                    ),
                );
            }
        }
        setTimeout(
            fetch_relay_documents,
            cfg.delay.nostr_relay_poll_document,
        );
    } catch (e) {
        await catch_err(e, `fetch_relay_documents`);
    }
};
