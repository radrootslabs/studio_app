
import { datastore, keys, route } from '$lib/util';
import { handle_err, key_nostr } from '@radroots/lib-app';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const ks_keynostr = await datastore.get(`key_nostr`);
        if (`result` in ks_keynostr) {
            const nostrkey = await keys.nostr_read(ks_keynostr.result);
            if (`result` in nostrkey) return void await route(`/`);
            await datastore.remove(`key_nostr`);
        }
        key_nostr.set(``);
    } catch (e) {
        await handle_err(e, `(cfg)load`)
    } finally {
        return {};
    };
};
