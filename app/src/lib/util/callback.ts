import { app_thc, type LocalCallbackColorMode, type LocalCallbackGeocode, type LocalCallbackGeocodeCurrent, type LocalCallbackGuiAlert, type LocalCallbackGuiConfirm, type LocalCallbackImgBin, type LocalCallbackPhotosAddMultiple, type LocalCallbackPhotosUpload } from "@radroots/lib-app";
import { parse_color_mode } from "@radroots/theme";
import { throw_err } from "@radroots/util";
import { fs, geoc, geol, gui, http } from ".";

export const lc_gui_alert: LocalCallbackGuiAlert = async (message) => {
    return await gui.alert(message);
};

export const lc_gui_confirm: LocalCallbackGuiConfirm = async (opts) => {
    return await gui.confirm({
        message: typeof opts === `string` ? opts : opts.message,
        ok: typeof opts === `string` ? undefined : opts.ok,
        cancel: typeof opts === `string` ? undefined : opts.cancel,
    });
};

export const lc_geocode: LocalCallbackGeocode = async (geoc_p) => {
    await geoc.connect();
    const geoc_res = await geoc.reverse(geoc_p);
    if (`err` in geoc_res) throw_err(geoc_res);
    return geoc_res.results[0] || undefined;
};

export const lc_geop_current: LocalCallbackGeocodeCurrent = async () => {
    const geop = await geol.current();
    if (`err` in geop) throw_err(geop);
    return geop;
};

export const lc_photos_add: LocalCallbackPhotosAddMultiple = async () => {
    const photo_paths_open = await gui.open_photos();
    if (!photo_paths_open) return;
    if (photo_paths_open.results) return photo_paths_open.results;
};

export const lc_img_bin: LocalCallbackImgBin = async (file_path) => {
    const data = await fs.read_bin(file_path);
    return data;
};

export const lc_color_mode: LocalCallbackColorMode = async ({ value }) => {
    if (value) app_thc.set(parse_color_mode(value));
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
    if (`err` in res) throw_err(res);
    else if (res.data && res.data.res_base && res.data.res_path && res.data.file_ext) {
        return {
            base_url: res.data.res_base,
            file_hash: res.data.res_path,
            file_ext: res.data.file_ext
        }
    }
};