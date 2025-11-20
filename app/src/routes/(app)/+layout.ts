
import { datastore, nostr_keys, route } from '$lib/utils/app';
import { type AppData } from '$lib/utils/config';
import { handle_err } from '@radroots/apps-lib';
import type { LayoutLoad, LayoutLoadEvent } from '../$types';

export type LayoutAppLoad = {
    public_key: string;
} | undefined;

export const load: LayoutLoad<LayoutAppLoad> = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const ds_app_data = await datastore.get_obj<AppData>("app_data");
        if ("err" in ds_app_data) return void await route(`/setup`);
        const ks_active_key = await nostr_keys.read(ds_app_data.result.active_key);
        if ("err" in ks_active_key) return void await route(`/setup`);
        return {
            public_key: ds_app_data.result.active_key
        };
    } catch (e) {
        handle_err(e, `(app)load`);
    };
};
