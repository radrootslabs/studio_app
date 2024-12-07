<script lang="ts">
    import { db, geoc, keystore, notification } from "$lib/client";
    import { cfg, ks } from "$lib/conf";
    import { fetch_relay_documents } from "$lib/util/fetch";
    import { nostr_sync } from "$lib/util/nostr-sync";
    import {
        app_cfg_type,
        app_geoc,
        app_init,
        app_nostr_key,
        app_splash,
        ndk,
        ndk_user,
        nostr_ndk_configured,
        nostr_relays_poll_documents,
        sleep,
    } from "@radroots/svelte-lib";
    import { ndk_init } from "@radroots/utils";
    import { onMount } from "svelte";

    onMount(async () => {
        try {
            const geoc_connected = await geoc.connect();
            app_geoc.set(!!geoc_connected);
        } catch (e) {
            console.log(`e (app) onMount`, e);
        } finally {
            app_splash.set(false);
            app_init.set(true);
        }
    });

    app_cfg_type.subscribe(async (_app_cfg_type) => {
        //@todo
    });

    app_splash.subscribe(async (_app_splash) => {
        //@todo
    });

    app_init.subscribe(async (_app_init) => {
        try {
            if (!app_init) return;
            await sleep(cfg.delay.load_notify);
            await notification.init();
        } catch (e) {
            console.log(`(app_init) error `, e);
        }
    });

    app_nostr_key.subscribe(async (_app_nostr_key) => {
        try {
            console.log(`_app_nostr_key `, _app_nostr_key);
            if (!_app_nostr_key) return;
            const ks_nostr_secretkey = await keystore.get(
                ks.keys.nostr_secretkey($app_nostr_key),
            );
            if (`err` in ks_nostr_secretkey) {
                return; //@todo;
            }
            const nostr_relays = await db.nostr_relay_get({
                list: [`on_profile`, { public_key: $app_nostr_key }],
            });
            if (`err` in nostr_relays) throw new Error(nostr_relays.err);
            for (const { url } of nostr_relays.results)
                $ndk.addExplicitRelay(url);
            await $ndk.connect();
            const ndk_user = await ndk_init({
                $ndk,
                secret_key: ks_nostr_secretkey.result,
            });
            if (!ndk_user) {
                nostr_ndk_configured.set(false);
                return;
            }
            $ndk_user = ndk_user;
            $ndk_user.ndk = $ndk;
            nostr_ndk_configured.set(true);
        } catch (e) {
            console.log(`(error) app_nostr_key`, e);
        }
    });

    nostr_ndk_configured.subscribe(async (_nostr_ndk_configured) => {
        try {
            if (!_nostr_ndk_configured) return;
            console.log(`(nostr_ndk_configured) success`);
            nostr_relays_poll_documents.set(true);
            await nostr_sync();
        } catch (e) {
            console.log(`(nostr_ndk_configured) error `, e);
        }
    });

    nostr_relays_poll_documents.subscribe(
        async (_nostr_relays_poll_documents) => {
            try {
                if (!_nostr_relays_poll_documents) return;
                await fetch_relay_documents();
            } catch (e) {
                console.log(`(error) nostr_relays_poll_documents`, e);
            }
        },
    );
</script>

<slot />
