
import { datastore, nostr_keys } from '$lib/utils/app';
import { route } from '$lib/utils/app/app';
import { handle_err } from '@radroots/apps-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const ds_key_nostr = await datastore.get("nostr_key");
        if (`result` in ds_key_nostr) {
            const nostrkey = await nostr_keys.read(ds_key_nostr.result);
            if (`result` in nostrkey) return void await route(`/`);
            await datastore.del("nostr_key");
        }
    } catch (e) {
        handle_err(e, `(cfg)load`)
    } finally {
        return {};
    };
};
