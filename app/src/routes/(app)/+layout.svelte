<script lang="ts">
    import { app_init, db, nostr_keys } from "$lib/utils/app";
    import { nostr_login_nip01 } from "@radroots/apps-nostr";
    import {
        nostr_context_default,
        nostr_relays_clear,
        nostr_relays_open,
    } from "@radroots/nostr";
    import { handle_err, throw_err } from "@radroots/utils";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { data, children }: LayoutProps = $props();
    const nostr_context = nostr_context_default();

    onMount(async () => {
        try {
            await app_init();
            await nostr_init();
        } catch (e) {
            handle_err(e, `on_mount`);
        } finally {
            //app_splash.set(false);
        }
    });

    const nostr_init = async (): Promise<void> => {
        if (!data.public_key) throw_err(`*-key_nostr`);
        const nostr_key = await nostr_keys.read(data.public_key);
        if ("err" in nostr_key) throw_err(nostr_key);
        const nostr_relays = await db.nostr_relay_find_many({
            rel: {
                on_profile: {
                    public_key: data.public_key,
                },
            },
        });
        if ("err" in nostr_relays) throw_err(nostr_relays);
        const relay_urls = nostr_relays.results.map(({ url }) => url);
        nostr_relays_clear(nostr_context);
        if (relay_urls.length) nostr_relays_open(nostr_context, relay_urls);
        if (relay_urls.length) console.log(`[tangle] nostr relays opened`);
        nostr_login_nip01(nostr_key.secret_key);
        //nostr_ndk_configured.set(true);
    };
</script>

{@render children()}
