import { PUBLIC_RADROOTS_URL } from "$env/static/public";
import { http } from "$lib/client";
import type { UploadFilePresignedUrl } from "@radroots/svelte-lib";
import { err_msg, type ErrorMessage, type FilePath, type ResultPass } from "@radroots/utils";

export const fetch_uploads_presigned_url = async (opts: FilePath): Promise<
    | UploadFilePresignedUrl
    | ErrorMessage<string>
> => {
    try {
        const { file_name, mime_type } = opts;
        const res = await http.fetch({
            url: `${PUBLIC_RADROOTS_URL}/public/image/upload`,
            method: `post`,
            data: {
                file_name,
                mime_type,
            },
        });
        if (`err` in res) return res;
        else if (
            res.data &&
            `url` in res.data &&
            typeof res.data.url === `string` &&
            `storage_key` in res.data &&
            typeof res.data.storage_key === `string` &&
            `file_name` in res.data &&
            typeof res.data.file_name === `string`
        ) {
            return {
                url: res.data.url,
                storage_key: res.data.storage_key,
                file_name: res.data.file_name,
            };
        }
        return err_msg(`request_failure`);
    } catch (e) {
        console.log(`(error) fetch_uploads_presigned_url `, e);
        return err_msg(`network_failure`);
    }
};

export const fetch_put_upload = async (opts: {
    url: string;
    file_data: Uint8Array;
    mime_type: string;
}): Promise<ResultPass | ErrorMessage<string>> => {
    try {
        const { url, file_data, mime_type } = opts;
        const res = await http.fetch({
            url,
            method: `put`,
            headers: {
                "Content-Type": mime_type,
            },
            data_bin: file_data,
        });
        if (`err` in res) return res;
        else if (res && res.status === 200) {
            return { pass: true };
        }
        return err_msg(`request_failure`);
    } catch (e) {
        console.log(`(error) fetch_put_upload `, e);
        return err_msg(`network_failure`);
    }
};
