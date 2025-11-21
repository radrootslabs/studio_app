<script lang="ts">
    import { db, nostr_keys } from "$lib/utils/app";
    import { ndk, ndk_init, ndk_user } from "@radroots/apps-lib";
    import { handle_err, throw_err } from "@radroots/utils";
    import { onMount } from "svelte";
    import type { LayoutProps } from "./$types";

    let { data, children }: LayoutProps = $props();

    onMount(async () => {
        try {
            await init();
            await nostr_init();
        } catch (e) {
            handle_err(e, `on_mount`);
        } finally {
            //app_splash.set(false);
        }
    });

    const init = async (): Promise<void> => {
        await db.init();
    };

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
        $ndk.explicitRelayUrls = [];
        for (const { url } of nostr_relays.results) $ndk.addExplicitRelay(url);
        await $ndk.connect().then(() => {
            console.log(`[tangle] ndk connected`);
        });
        const ndk_user_init = await ndk_init($ndk, nostr_key.secret_key);
        $ndk_user = ndk_user_init;
        $ndk_user.ndk = $ndk;
        //nostr_ndk_configured.set(true);
    };
</script>

{@render children()}
