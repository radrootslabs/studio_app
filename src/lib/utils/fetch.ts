import { fs, http, keystore } from "$lib/client";
import { ks } from "$lib/conf";
import type { IClientHttpResponseError } from "@radroots/client";
import { app_nostr_key } from "@radroots/svelte-lib";
import { err_msg, err_res, nostr_event_sign_attest, type ErrorMessage, type ErrorResponse, type FilePath } from "@radroots/utils";
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
        console.log(JSON.stringify(res, null, 4), `res`)
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
