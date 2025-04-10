import { ls } from "$lib/locale/i18n";
import { get_store, handle_err, ndk_user, nostr_sync_prevent } from "@radroots/lib-app";
import { throw_err } from "@radroots/util";
import { db, gui, nostrsync } from "..";
import { err } from "../err";

export const nostr_sync = async (): Promise<void> => {
    try {
        console.log(`[nostr_sync] start`);
        const $ls = get_store(ls);
        const $ndk_user = get_store(ndk_user);
        const public_key = $ndk_user.pubkey;
        if (!public_key) return void await gui.alert(
            `${$ls(`error.client.nostr_sync_failure`)}`
        );
        const $nostr_sync_prevent = get_store(nostr_sync_prevent);
        if ($nostr_sync_prevent) {
            const confirm = await gui.confirm({
                message: `${$ls(`error.client.nostr_sync_disabled`)}`,
            });
            if (confirm) {
                nostr_sync_prevent.set(false);
                await nostr_sync();
            }
            return;
        }
        const nostr_relays = await db.nostr_relay_read_list({
            table: [`on_profile`, { public_key }]
        }); //@todo
        console.log(JSON.stringify(nostr_relays, null, 4), `nostr_relays`)
        if (`err` in nostr_relays) return throw_err(nostr_relays);
        if (!nostr_relays.results.length) return throw_err(err.nostr.no_relays);

        await nostr_sync_metadata();
        console.log(`[nostr_sync] done`);
    } catch (e) {
        await handle_err(e, `nostr_sync`);
    }
};

export const nostr_sync_metadata = async (): Promise<void> => {
    const $ndk_user = get_store(ndk_user);
    const { pubkey: public_key } = $ndk_user;
    if (!public_key) throw_err(`todo-public-key-undefined`);
    const tb_nostr_profile = await db.nostr_profile_read({ public_key });
    if (`err` in tb_nostr_profile) throw_err(tb_nostr_profile);
    const ev = await nostrsync.metadata({ metadata: tb_nostr_profile.result });
    if (`err` in ev) throw_err(ev);
    await ev.publish();
};


