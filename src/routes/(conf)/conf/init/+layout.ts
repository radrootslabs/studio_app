import { keystore } from '$lib/client';
import { ks } from '$lib/conf';
import { app_nostr_key, route } from '@radroots/svelte-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async (_: LayoutLoadEvent) => {
    try {
        const nostr_publickey = await keystore.get(
            ks.nostr.nostr_key_active,
        );
        if (`result` in nostr_publickey) {
            const nostr_secretkey = await keystore.get(
                ks.nostr.nostr_key(nostr_publickey.result),
            );
            if (`result` in nostr_secretkey) {
                app_nostr_key.set(nostr_publickey.result);
                await route(`/`);
                return;
            }
        }
    } catch (e) {
        console.log(`(load) (conf) init`, e)
    } finally {
        //await win.splash_hide();
        return {};
    };
};
