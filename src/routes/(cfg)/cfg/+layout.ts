import { keystore } from '$lib/client';
import { ks } from '$lib/conf';
import { app_splash, route } from '@radroots/svelte-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async ({ url }: LayoutLoadEvent) => {
    try {
        await keystore.init();

        console.log(`(cfg) `, url.pathname)
        const ks_nostr_publickey = await keystore.get(
            ks.keys.nostr_publickey,
        );
        if (`result` in ks_nostr_publickey) {
            const ks_nostr_secretkey = await keystore.get(
                ks.keys.nostr_secretkey(ks_nostr_publickey.result),
            );
            if (`result` in ks_nostr_secretkey) {
                await route(`/`);
                return;
            }
        }
        app_splash.set(false);
    } catch (e) {
        console.log(`(cfg) ERROR`, e)
    } finally {
        //await win.splash_hide();
        return {};
    };
};
