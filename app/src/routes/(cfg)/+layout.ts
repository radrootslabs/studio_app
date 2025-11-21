
import { datastore, nostr_keys, route } from '$lib/utils/app';
import type { AppData } from '$lib/utils/config';
import { handle_err } from '@radroots/apps-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const app_data = await datastore.get_obj<AppData>("app_data");
        if ("result" in app_data) {
            const nostr_key = await nostr_keys.read(app_data.result.active_key);
            if ("result" in nostr_key) return void await route(`/`);
            // @todo
        }
    } catch (e) {
        handle_err(e, `(cfg)load`)
    } finally {
        return {};
    };
};
