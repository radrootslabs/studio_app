
import { datastore, keys, route } from '$lib/util';
import { handle_err, key_nostr } from '@radroots/lib-app';
import type { LayoutLoad, LayoutLoadEvent } from '../$types';

export const load: LayoutLoad = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const ks_keynostr = await datastore.get(`key_nostr`);
        if (`err` in ks_keynostr) return void await route(`/init`);
        const nostrkey = await keys.nostr_read(ks_keynostr.result);
        if (`err` in nostrkey) return void await route(`/init`);
        key_nostr.set(ks_keynostr.result);
    } catch (e) {
        await handle_err(e, `(app)load`)
    } finally {
        return {};
    };
};
