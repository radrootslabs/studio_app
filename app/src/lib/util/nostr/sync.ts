import { get_store, handle_err, ndk_user } from "@radroots/lib-app";
import { throw_err } from "@radroots/util";
import { db, nostrsync } from "..";

export const nostr_sync_metadata = async (): Promise<void> => {
    try {
        const $ndk_user = get_store(ndk_user);
        const tb_nostr_profile = await db.nostr_profile_read({
            public_key: $ndk_user?.pubkey,
        });
        if (`err` in tb_nostr_profile) throw_err(tb_nostr_profile.err);
        const ev_metadata = await nostrsync.metadata({
            metadata: tb_nostr_profile.result,
        });
        if (`err` in ev_metadata) throw_err(ev_metadata.err);
        await ev_metadata.publish();
    } catch (e) {
        await handle_err(e, `nostr_sync_metadata`);
        //location.reload(); @todo 
    }
};