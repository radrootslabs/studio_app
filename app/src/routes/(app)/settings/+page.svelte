<script lang="ts">
    import {
        __APP_INFO__,
        app_init,
        app_init_reset,
        datastore,
        db,
        nostr_keys,
        notif,
    } from "$lib/utils/app";
    import type { cfg_datastore_key_obj_map_types } from "$lib/utils/config";
    import { ls } from "$lib/utils/i18n";
    import { get_store, handle_err } from "@radroots/apps-lib";
    import { Settings } from "@radroots/apps-lib-pwa";
    import type {
        NostrEventEnvelope,
        TangleDatabaseExportSigner,
        TangleNostrSyncSigner
    } from "@radroots/client/tangle";
    import { nostr_event_sign } from "@radroots/nostr";

    const ls_val = get_store(ls);

    const logout = async (): Promise<void> => {
        app_init_reset();
    };

    const load_sync_signers = async (): Promise<TangleNostrSyncSigner[]> => {
        const keys_res = await nostr_keys.keys();
        if ("err" in keys_res) throw new Error(keys_res.err);
        const signers: TangleNostrSyncSigner[] = [];
        for (const public_key of keys_res.results) {
            const secret_res = await nostr_keys.read(public_key);
            if ("err" in secret_res) throw new Error(secret_res.err);
            const secret_key = secret_res.secret_key;
            if (!secret_key || typeof secret_key !== "string") continue;
            signers.push({ secret_key });
        }
        if (!signers.length) throw new Error("nostr sync requires signer keys");
        return signers;
    };

    const load_sync_relays = async (public_key: string): Promise<string[]> => {
        const relays_res = await db.nostr_relay_find_many({
            rel: {
                on_profile: {
                    public_key,
                },
            },
        });
        if ("err" in relays_res) throw new Error(relays_res.err);
        const relays = Array.from(
            new Set(
                relays_res.results
                    .map((relay) => relay.url)
                    .filter((url) => typeof url === "string" && url.trim().length)
                    .map((url) => url.trim())
            )
        );
        if (!relays.length) throw new Error("nostr sync requires relays");
        return relays;
    };

    const sync_nostr_events = async (public_key: string): Promise<void> => {
        const signers = await load_sync_signers();
        const relays = await load_sync_relays(public_key);
        const sync_res = await db.nostr_sync_all({
            relays,
            signers,
            publish_timeout_ms: 10000,
        });
        if (sync_res && "err" in sync_res) throw new Error(sync_res.err);
    };

    const export_database = async (): Promise<void> => {
        try {
            await app_init();
            console.log(`done app_init()`);
            const app_data =
                await datastore.get_obj<
                    cfg_datastore_key_obj_map_types["app_data"]
                >("app_data");
            console.log(JSON.stringify(app_data, null, 4), `app_data`);
            if ("err" in app_data) throw new Error(app_data.err);
            await sync_nostr_events(app_data.result.active_key);
            let signer: TangleDatabaseExportSigner | undefined;
            const active_key = app_data.result.active_key;
            const secret_key = await nostr_keys.read(active_key);
            if (!("err" in secret_key)) {
                const key = secret_key.secret_key;
                signer = async ({
                    db_sha256,
                    manifest,
                }): Promise<NostrEventEnvelope | null> => {
                    const payload = JSON.stringify({
                        db_sha256,
                        export_version: manifest.rust.export_version,
                        schema_hash: manifest.rust.schema_hash,
                    });
                    const event = nostr_event_sign({
                        secret_key: key,
                        event: {
                            kind: 1,
                            created_at: Math.floor(Date.now() / 1000),
                            tags: [["t", "radroots:tangle-db-export"]],
                            content: payload,
                        },
                    });
                    return event;
                };
            }

            const res = await db.export_database({
                app_name: __APP_INFO__.name,
                app_version: __APP_INFO__.version,
                signer,
            });
            console.log(`res `, res);
            if (res && "err" in res) throw new Error(res.err);
        } catch (e) {
            handle_err(e, "settings.export_database");
            await notif.alert(`${ls_val(`error.backup.export_failure`)}`);
        }
    };
</script>

<Settings
    basis={{
        trellis_ext: [
            {
                list: [
                    {
                        hide_active: true,
                        touch: {
                            label: {
                                left: [
                                    {
                                        value: "export database",
                                        classes: `capitalize`,
                                    },
                                ],
                            },
                            end: {
                                glyph: {
                                    key: `caret-right`,
                                },
                            },
                            callback: export_database,
                        },
                    },
                    {
                        hide_active: true,
                        touch: {
                            label: {
                                left: [
                                    {
                                        value: `${$ls(`common.logout`)}`,
                                        classes: `capitalize`,
                                    },
                                ],
                            },
                            end: {
                                glyph: {
                                    key: `caret-right`,
                                },
                            },
                            callback: logout,
                        },
                    },
                ],
            },
        ],
    }}
/>
