<script lang="ts">
    import { datastore, db, gui, http, keys } from "$lib/util";
    import {
        app_notify,
        app_splash,
        handle_err,
        idb_init,
        ndk,
        ndk_user,
        nostr_ndk_configured,
    } from "@radroots/lib-app";
    import { ndk_init } from "@radroots/nostr-util";
    import { throw_err } from "@radroots/util";

    import { nostr_sync_metadata } from "$lib/util/nostr/sync";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { children, data }: LayoutProps = $props();

    onMount(async () => {
        try {
            await init();
        } catch (e) {
            handle_err(e, `on_mount`);
        } finally {
            app_splash.set(false);
        }
    });

    const init = async (): Promise<void> => {
        try {
            if (`paintWorklet` in CSS)
                (CSS as any).paintWorklet.addModule(`/assets/squircle.min.js`);
            await http.init();
            await datastore.init();
            await idb_init();
            await nostr_init();
        } catch (e) {
            await handle_err(e, `init`);
        }
    };

    const nostr_init = async (): Promise<void> => {
        try {
            if (!data.public_key) throw_err(`*-key_nostr`);
            const keys_nostr_read = await keys.nostr_read(data.public_key);
            if (`err` in keys_nostr_read) throw_err(keys_nostr_read.err);
            const tb_nostr_relays = await db.nostr_relay_read_list({
                table: [`on_profile`, { public_key: data.public_key }],
            });
            if (`err` in tb_nostr_relays) throw_err(tb_nostr_relays.err);
            for (const { url } of tb_nostr_relays.results)
                $ndk.addExplicitRelay(url);
            await $ndk.connect();
            const ndk_user_init = await ndk_init({
                $ndk,
                secret_key: keys_nostr_read.secret_key,
            });
            nostr_ndk_configured.set(!!ndk_user_init);
            if (!ndk_user_init) throw_err(`error.nostr.ndk_user_undefined`);
            $ndk_user = ndk_user_init;
            $ndk_user.ndk = $ndk;
            await nostr_sync_metadata();
        } catch (e) {
            await handle_err(e, `nostr_init`);
        }
    };

    app_notify.subscribe(async (_app_notify) => {
        console.log(`_app_notify `, _app_notify);
        if (!_app_notify) return;
        await gui.notify_send($app_notify);
        app_notify.set(``);
    });
</script>

{@render children()}
