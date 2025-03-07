<script lang="ts">
    import { datastore, db, http, keys } from "$lib/util";
    import {
        app_splash,
        handle_err,
        idb_init,
        ndk,
        ndk_user,
        nostr_ndk_configured,
    } from "@radroots/lib-app";
    import { ndk_init } from "@radroots/nostr-util";
    import { throw_err } from "@radroots/util";

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
            if (!data.public_key) return void throw_err(`*-key_nostr`);
            const keys_nostr_read = await keys.nostr_read(data.public_key);
            if (`err` in keys_nostr_read)
                return void throw_err(keys_nostr_read.err);
            const tb_nostr_relays = await db.nostr_relay_read_list({
                table: [`on_profile`, { public_key: data.public_key }],
            });
            if (`err` in tb_nostr_relays)
                return void throw_err(tb_nostr_relays.err);
            for (const { url } of tb_nostr_relays.results)
                $ndk.addExplicitRelay(url);
            await $ndk.connect();
            const ndk_user_init = await ndk_init({
                $ndk,
                secret_key: keys_nostr_read.secret_key,
            });
            nostr_ndk_configured.set(!!ndk_user_init);
            if (!ndk_user_init) return; //@todo
            $ndk_user = ndk_user_init;
            $ndk_user.ndk = $ndk;
        } catch (e) {
            await handle_err(e, `nostr_init`);
        }
    };
</script>

{@render children()}
