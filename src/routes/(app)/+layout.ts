import { keystore } from '$lib/client';
import { ks } from '$lib/conf';
import { app_cfg_type, app_nostr_key, app_splash, parse_cfg_type, route } from '@radroots/svelte-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const load: LayoutLoad = async ({ url }: LayoutLoadEvent) => {
    try {
        console.log(`(load) (app) `, url.pathname)
        const ks_nostr_publickey = await keystore.get(
            ks.keys.nostr_publickey,
        );
        if (`err` in ks_nostr_publickey) {
            await route(`/cfg/init`);
            return;
        }
        const ks_nostr_secretkey = await keystore.get(
            ks.keys.nostr_secretkey(ks_nostr_publickey.result),
        );
        if (`err` in ks_nostr_secretkey) {
            await route(`/cfg/error`);
            return;
        }
        const ks_pref_cfg_type = await keystore.get(
            ks.pref.cfg_type
        );
        //@todo handle err
        if (`result` in ks_pref_cfg_type) {
            app_cfg_type.set(parse_cfg_type(ks_pref_cfg_type.result))
        }
        app_splash.set(false);
        app_nostr_key.set(ks_nostr_publickey.result);
    } catch (e) {
        console.log(`(load) (conf) app ERROR`, e)
    } finally {
        //await win.splash_hide();
        return {};
    };
};
