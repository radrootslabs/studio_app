
import { datastore, keys, route } from '$lib/util';
import { handle_err } from '@radroots/lib-app';
import type { LayoutLoad, LayoutLoadEvent } from '../$types';

export type LayoutAppLoad = {
    public_key: string;
} | undefined;

export const load: LayoutLoad<LayoutAppLoad> = async (_: LayoutLoadEvent) => {
    try {
        await datastore.init();
        const ks_keynostr = await datastore.get(`key_nostr`);
        if (`err` in ks_keynostr) return void await route(`/init`);
        const nostrkey = await keys.nostr_read(ks_keynostr.result);
        if (`err` in nostrkey) return void await route(`/init`);
        return {
            public_key: ks_keynostr.result
        };
    } catch (e) {
        await handle_err(e, `(app)load`);
    };
};
