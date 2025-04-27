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
        nostr_poll_relays_retry_handler,
    } from "@radroots/lib-app";
    import { ndk_init } from "@radroots/nostr-util";
    import { throw_err } from "@radroots/util";

    import { _cfg } from "$lib/config";
    import { cfg_role, cfg_setup } from "$lib/store";
    import { nostr_poll_relays } from "$lib/util/nostr/lib";
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
            //
            // is setup
            const ds_setup = await datastore.get(`is_setup`);
            if (`err` in ds_setup) cfg_setup.set(false);
            else cfg_setup.set(true);
            //
            // role
            const ds_role = await datastore.get(`role`);
            if (`err` in ds_role) {
                await datastore.set(`role`, _cfg.role_default);
                cfg_role.set(_cfg.role_default);
            } else cfg_role.set(ds_role.result);
        } catch (e) {
            await handle_err(e, `init`);
        }
    };

    const nostr_init = async (): Promise<void> => {
        if (!data.public_key) throw_err(`*-key_nostr`);
        const keys_nostr_read = await keys.nostr_read(data.public_key);
        if (`err` in keys_nostr_read) throw_err(keys_nostr_read);
        const tb_nostr_relays = await db.nostr_relay_read_list({
            table: [`on_profile`, { public_key: data.public_key }],
        });
        if (`err` in tb_nostr_relays) throw_err(tb_nostr_relays);
        $ndk.explicitRelayUrls = [];
        for (const { url } of tb_nostr_relays.results)
            $ndk.addExplicitRelay(url);
        await $ndk.connect().then(() => {
            console.log(`[tangle] ndk connected`);
        });
        const ndk_user_init = await ndk_init({
            $ndk,
            secret_key: keys_nostr_read.secret_key,
        });
        if (!ndk_user_init) throw_err(`error.nostr.ndk_user_undefined`);
        $ndk_user = ndk_user_init;
        $ndk_user.ndk = $ndk;
        nostr_ndk_configured.set(true);
    };

    nostr_ndk_configured.subscribe(async (_sub) => {
        if (!_sub) return;
        // @todo await nostr_sync_retry_handler(nostr_sync);
        await nostr_poll_relays_retry_handler(nostr_poll_relays);
    });

    app_notify.subscribe(async (_sub) => {
        if (!_sub) return;
        await gui.notify_send(_sub);
        app_notify.set(``);
    });

    cfg_role.subscribe(async (_sub) => {
        if (!_sub) return;
    });

    cfg_setup.subscribe(async (_sub) => {
        if (!_sub) return;
    });
</script>

{#if typeof $cfg_setup !== `undefined`}
    {#if !$cfg_setup}
        {@render children()}
    {:else}
        <div class={`flex flex-col w-full pt-20 justify-start items-center`}>
            <button
                class={`flex flex-row justify-center items-center`}
                onclick={async () => {
                    await datastore.remove(`is_setup`);
                    location.reload();
                }}
            >
                {`setup device`}
            </button>
        </div>
    {/if}
{/if}
