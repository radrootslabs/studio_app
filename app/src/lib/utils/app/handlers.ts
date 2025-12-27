import { theme_mode, type LocalCallbackColorMode, type LocalCallbackGeocode, type LocalCallbackGeocodeCurrent, type LocalCallbackGuiAlert, type LocalCallbackGuiConfirm, type LocalCallbackImgBin, type LocalCallbackPhotosAddMultiple, type LocalCallbackPhotosUpload } from "@radroots/apps-lib";
import { parse_theme_mode } from "@radroots/themes";
import { throw_err } from "@radroots/utils";
import { fs, geoc, geoc_init, geol, http, notif } from ".";

type PhotoUploadResponse = {
    res_base: string;
    res_path: string;
    file_ext: string;
};

const is_photo_upload_response = (value: unknown): value is PhotoUploadResponse => {
    if (!value || typeof value !== "object") return false;
    const record = value as Record<string, unknown>;
    return typeof record.res_base === "string"
        && typeof record.res_path === "string"
        && typeof record.file_ext === "string";
};

export const lc_gui_alert: LocalCallbackGuiAlert = async (message) => {
    return await notif.alert(message);
};

export const lc_gui_confirm: LocalCallbackGuiConfirm = async (opts) => {
    return await notif.confirm({
        message: typeof opts === `string` ? opts : opts.message,
        ok: typeof opts === `string` ? undefined : opts.ok,
        cancel: typeof opts === `string` ? undefined : opts.cancel,
    });
};

export const lc_geocode: LocalCallbackGeocode = async (geoc_p) => {
    await geoc_init();
    const geoc_res = await geoc.reverse(geoc_p);
    if ("err" in geoc_res) throw_err(geoc_res);
    return geoc_res.results[0] || undefined;
};

export const lc_geop_current: LocalCallbackGeocodeCurrent = async () => {
    const resolve = await geol.current();
    if ("err" in resolve) throw_err(resolve);
    return resolve;
};

export const lc_photos_add: LocalCallbackPhotosAddMultiple = async () => {
    const photo_paths_open = await notif.open_photos();
    if (!photo_paths_open || "err" in photo_paths_open) return;
    if (photo_paths_open.results) return photo_paths_open.results;
};

export const lc_img_bin: LocalCallbackImgBin = async (file_path) => {
    const resolve = await fs.read_bin(file_path);
    if ("err" in resolve) throw_err(resolve);
    return resolve;
};

export const lc_color_mode: LocalCallbackColorMode = async ({ value }) => {
    if (value) theme_mode.set(parse_theme_mode(value));
};

export const lc_photos_upload: LocalCallbackPhotosUpload = async ({ url, path }) => {
    const data_bin = await lc_img_bin(path);
    if (!data_bin) return;
    const res = await http.fetch({
        url,
        method: `put`,
        headers: {
            "Content-Type": `image/jpeg`,
        },
        data_bin,
    });
    if ("err" in res) throw_err(res);
    else if (is_photo_upload_response(res.data)) {
        return {
            base_url: res.data.res_base,
            file_hash: res.data.res_path,
            file_ext: res.data.file_ext
        }
    }
};
