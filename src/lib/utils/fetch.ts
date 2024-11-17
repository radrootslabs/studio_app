import { db, fs, http, keystore } from "$lib/client";
import { cfg, ks } from "$lib/conf";
import type { IClientHttpResponseError } from "@radroots/client";
import { parse_nostr_relay_form_keys, type NostrRelayFormFields } from "@radroots/models";
import { app_nostr_key, nostr_relays_connected, nostr_relays_poll_documents, nostr_relays_poll_documents_count } from "@radroots/svelte-lib";
import { err_msg, err_res, nostr_event_sign_attest, parse_nostr_relay_information_document_fields, type ErrorMessage, type ErrorResponse, type FilePath } from "@radroots/utils";
import { get as get_store } from "svelte/store";

export const fetch_put_upload = async (opts: {
    url: string;
    file_path: FilePath;
}): Promise<{
    res_base: string;
    res_path: string;
} | ErrorResponse<IClientHttpResponseError> | ErrorMessage<string>> => {
    try {
        const nostr_public_key = get_store(app_nostr_key);
        const secret_key = await keystore.get(
            ks.keys.nostr_secretkey(nostr_public_key),
        );
        if (`err` in secret_key) return err_msg(`error.client.keystore_nostr_secretkey`);
        const { url, file_path } = opts;
        const file_data = await fs.read_bin(file_path.file_path);
        if (!file_data) return err_msg(`error.client.file_path_read_bin_undefined`);;
        const res = await http.fetch({
            url,
            method: `put`,
            headers: {
                "Content-Type": file_path.mime_type,
                "X-Nostr-Event": JSON.stringify(nostr_event_sign_attest(secret_key.result)),
            },
            authorization: nostr_public_key,
            data_bin: file_data,
        });
        if (`err` in res) err_msg(`error.client.request_failure`);
        else if (res.error) {
            return err_res(res.error);
        }
        else if (
            res.status === 200 &&
            res.data &&
            `pass` in res.data &&
            `res_base` in res.data &&
            typeof res.data.res_base === `string` &&
            `res_path` in res.data &&
            typeof res.data.res_path === `string`
        ) {
            return {
                res_base: res.data.res_base,
                res_path: res.data.res_path,
            };
        }
        return err_msg(`error.client.request_unhandled`);
    } catch (e) {
        console.log(`(error) fetch_put_upload `, e);
        return err_msg(`error.client.network_failure`);
    }
};

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
        if (`err` in nostr_relays) throw new Error(nostr_relays.err);

        const unconnected_relays = nostr_relays.results.filter(
            (i) => !$nostr_relays_connected.includes(i.id),
        );
        if (unconnected_relays.length === 0) {
            nostr_relays_poll_documents.set(false);
            return;
        }

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
        console.log(`(error) fetch_relay_documents `, e);
    }
};