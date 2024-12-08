import { PUBLIC_RADROOTS_URL } from "$env/static/public";
import { db } from "$lib/client";
import type { IDialogAlert, IDialogConfirm } from "$lib/types";
import type { IClientHttpResponseError } from "@radroots/client";
import { catch_err, ls } from "@radroots/svelte-lib";
import { parse_file_path, type FilePath, type ResultsList } from "@radroots/utils";
import { get } from "svelte/store";
import { fetch_put_upload } from "./fetch";

export const model_media_upload_add_list = async (opts: {
    photo_paths: string[];
}): Promise<IDialogAlert | IDialogConfirm | ResultsList<string>> => {
    try {
        const $ls = get(ls);
        if (!opts.photo_paths.length) {
            return {
                alert: `No photos provided`
            }
        }
        const photo_path_uploads: {
            file_path: FilePath;
            res_base: string;
            res_path: string;
        }[] = [];
        const photo_path_uploads_err: {
            file_path: FilePath;
            err_msg: string;
        }[] = [];
        const photo_path_uploads_error: IClientHttpResponseError[] = [];

        for (const photo_path of opts.photo_paths) {
            const file_path = parse_file_path(photo_path);
            console.log(JSON.stringify(file_path, null, 4), `file_path`)
            if (!file_path) continue;
            const url = `${PUBLIC_RADROOTS_URL}/public/upload/image`; //@todo
            const put_upload = await fetch_put_upload({
                url,
                file_path,
            });
            console.log(JSON.stringify(put_upload, null, 4), `put_upload`)
            if (`err` in put_upload) {
                photo_path_uploads_err.push({
                    file_path,
                    err_msg: put_upload.err,
                });
                continue;
            } else if (`error` in put_upload) {
                photo_path_uploads_error.push(put_upload.error);
                continue;
            }
            photo_path_uploads.push({
                file_path,
                res_base: put_upload.res_base,
                res_path: put_upload.res_path,
            });
        }

        if (photo_path_uploads_error.length) {
            return {
                confirm: {
                    message: `${$ls(photo_path_uploads_error[0].message)}`, //@todo
                    ok_label: photo_path_uploads_error[0].label_ok
                        ? `${$ls(photo_path_uploads_error[0].label_ok)}` ||
                        undefined
                        : undefined,
                    cancel_label: photo_path_uploads_error[0].label_cancel
                        ? `${$ls(photo_path_uploads_error[0].label_cancel)}` ||
                        undefined
                        : undefined,
                }
            };

        }
        if (photo_path_uploads_err.length) {
            return {
                alert: `${$ls(`icu.there_was_a_failure_while_*`, {
                    value: `${$ls(`icu.uploading_*_photos`, {
                        value:
                            photo_path_uploads_err.length ===
                                opts.photo_paths.length
                                ? `${$ls(`common.all`)}`
                                : `${photo_path_uploads_err.length}`,
                    })}`.toLowerCase(),
                })}`
            };
        }

        const results: string[] = [];
        if (photo_path_uploads.length) {
            for (const photo_path_upload of photo_path_uploads) {
                const media_upload_add = await db.media_upload_add({
                    file_path: photo_path_upload.file_path.file_path,
                    mime_type: photo_path_upload.file_path.mime_type,
                    res_base: photo_path_upload.res_base,
                    res_path: photo_path_upload.res_path,
                });
                if (
                    `err` in media_upload_add ||
                    `err_s` in media_upload_add
                )
                    continue; //@todo
                results.push(media_upload_add.id);
            }
        }

        return {
            results
        };
    } catch (e) {
        await catch_err(e, `model_media_upload_add_list`);
        return {
            alert: `Failed to upload photos` //@todo
        }
    }
};

