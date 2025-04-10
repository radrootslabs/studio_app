import { ls } from "$lib/locale/i18n";
import { get_store, handle_err, ndk_user, nostr_poll_relays_stop, nostr_relays_connected } from "@radroots/lib-app";
import { nostr_relay_parse_form_keys, type NostrRelayFormFields } from "@radroots/models";
import { throw_err } from "@radroots/util";
import { db, gui, http, nostrl } from "..";

export const nostr_poll_relays = async (): Promise<void> => {
    try {
        console.log(`[nostr_poll_relays] start`);
        const $ls = get_store(ls);
        const $ndk_user = get_store(ndk_user);
        const public_key = $ndk_user?.pubkey;
        if (!public_key) return void await gui.alert(
            `${$ls(`error.client.nostr_poll_relays_failure`)}`
        );

        const $nostr_relays_connected = get_store(nostr_relays_connected);

        const nostr_relays = await db.nostr_relay_read_list({
            table: [`on_profile`, { public_key }],
        });
        console.log(JSON.stringify(nostr_relays, null, 4), `nostr_relays`)
        if (`err` in nostr_relays) return throw_err(nostr_relays.err);
        const unconnected_relays = nostr_relays.results.filter(
            (i) => !$nostr_relays_connected.includes(i.id),
        );
        console.log(JSON.stringify(unconnected_relays, null, 4), `unconnected_relays`)
        if (unconnected_relays.length === 0) return void nostr_poll_relays_stop.set(true);

        for (const nostr_relay of unconnected_relays) {
            const res = await http.fetch({
                url: nostr_relay.url.replace(`ws://`, `http://`).replace(`wss://`, `https://`),
                headers: {
                    Accept: "application/nostr+json",
                },
            });
            if (`err` in res) continue;
            else if (res.status === 200 && res.data) {
                const relaydoc = nostrl.parse_information_document(res.data);
                if (!relaydoc) continue;
                const fields: Partial<NostrRelayFormFields> = {};
                for (const [k, v] of Object.entries(relaydoc)) {
                    const field_k = nostr_relay_parse_form_keys(k);
                    if (field_k) fields[field_k] = v;
                }
                if (Object.keys(fields).length < 1) continue;
                await db.nostr_relay_update({
                    filter: {
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
        console.log(`[nostr_poll_relays] done`);
    } catch (e) {
        await handle_err(e, `nostr_poll_relays`);
    }
};


/*
import { get_store, handle_err, ndk_user, nostr_poll_relays, nostr_poll_relays_attempts, nostr_poll_relays_attempts_max, nostr_relays_connected } from "$lib";
import type { db } from "@nostr-dev-kit/ndk-cache-dexie";
import { throw_err } from "@radroots/util";

export const nostr_fetch_relay_documents = async (
    callback: () => Promise<void>
): Promise<void> => {
    try {
    
        if (
            $nostr_poll_relays_attempts >=
            $nostr_poll_relays_attempts_max
        ) {
            nostr_poll_relays.set(false);
            return;
        }
        nostr_poll_relays_attempts.set(
            $nostr_poll_relays_attempts + 1,
        );
     
        -
        if (unconnected_relays.length === 0) return void nostr_poll_relays.set(false);
       
        setTimeout(
            async () => nostr_fetch_relay_documents(callback),
            3000
        );
    } catch (e) {
        await handle_err(e, `nostr_fetch_relay_documents`);
    }
};


*/