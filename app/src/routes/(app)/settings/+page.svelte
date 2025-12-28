<script lang="ts">
    import { ls } from "$lib/utils/i18n";
    import { Settings } from "@radroots/apps-lib-pwa";
    import { get_store, handle_err } from "@radroots/apps-lib";
    import { nostr_event_sign } from "@radroots/nostr";
    import { app_init, datastore, db, nostr_keys, notif } from "$lib/utils/app";
    import type { cfg_datastore_key_obj_map_types } from "$lib/utils/config";
    import type { NostrEventEnvelope, TangleDatabaseExportSigner } from "@radroots/client/tangle";

    declare const __APP_NAME__: string;
    declare const __APP_VERSION__: string;

    const ls_val = get_store(ls);

    const logout = async (): Promise<void> => {
        alert("not implemented");
    };

    const export_database = async (): Promise<void> => {
        try {
            await app_init();
            const app_data = await datastore.get_obj<cfg_datastore_key_obj_map_types["app_data"]>("app_data");
            let signer: TangleDatabaseExportSigner | undefined;
            if (!("err" in app_data)) {
                const active_key = app_data.result.active_key;
                const secret_key = await nostr_keys.read(active_key);
                if (!("err" in secret_key)) {
                    const key = secret_key.secret_key;
                    signer = async ({ db_sha256, manifest }): Promise<NostrEventEnvelope | null> => {
                        const payload = JSON.stringify({
                            db_sha256,
                            export_version: manifest.rs.export_version,
                            schema_hash: manifest.rs.schema_hash
                        });
                        const event = nostr_event_sign({
                            secret_key: key,
                            event: {
                                kind: 1,
                                created_at: Math.floor(Date.now() / 1000),
                                tags: [["t", "radroots:tangle-db-export"]],
                                content: payload
                            }
                        });
                        return event;
                    };
                }
            }

            const res = await db.export_database({
                app_name: __APP_NAME__,
                app_version: __APP_VERSION__,
                signer
            });
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
