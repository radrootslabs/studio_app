import { datastore, db, nostr_keys, notif } from "$lib/utils/app";
import { download_json, handle_err } from "@radroots/apps-lib";
import type { ExportedAppState } from "@radroots/apps-lib-pwa/types/app";
import { throw_err } from "@radroots/utils";
import { createStore, get as idb_get, keys as idb_keys, type UseStore } from "idb-keyval";
import { app_cfg } from "../app/config";

const parse_idb_entry = async (key: IDBValidKey, store: UseStore): Promise<[string, string] | undefined> => {
    if (typeof key !== "string") return undefined;
    const value = await idb_get(key, store);
    if (typeof value === "undefined") return undefined;
    return [key, value];
};

const export_datastore_state = async (): Promise<ExportedAppState["datastore"]> => {
    await datastore.init();
    const ds_cfg = datastore.get_config();
    if (!ds_cfg) throw_err("error.app.export.missing_datastore_config");
    const ds_store = createStore(ds_cfg.database, ds_cfg.store);
    const ds_keys = await idb_keys(ds_store);
    const entries: Record<string, unknown> = {};
    for (const key of ds_keys) {
        const entry = await parse_idb_entry(key, ds_store);
        if (entry) entries[entry[0]] = entry[1];
    }
    return { config: ds_cfg, entries };
};

const export_nostr_keystore_state = async (): Promise<ExportedAppState["nostr_keystore"]> => {
    const nostr_cfg = nostr_keys.get_config();
    if (!nostr_cfg) throw_err("error.app.export.missing_nostr_keystore_config");

    const keys: ExportedAppState["nostr_keystore"]["keys"] = [];
    try {
        const nostr_keys_all = await nostr_keys.keys();
        if ("err" in nostr_keys_all) throw_err(nostr_keys_all.err);
        for (const public_key of nostr_keys_all.results) {
            const secret_key = await nostr_keys.read(public_key);
            if ("err" in secret_key) throw_err(secret_key.err);
            keys.push({
                public_key,
                secret_key: secret_key.secret_key
            });
        }
    } catch { }
    return {
        config: nostr_cfg,
        keys
    };
};

const export_tangle_db_state = async (): Promise<ExportedAppState["database"]> => {
    await db.init();
    const store_key = db.get_store_key();
    const backup = await db.export_backup();
    return { store_key, backup };
};

export const export_app_state = async (): Promise<void> => {
    try {
        if (typeof window === "undefined") return;
        const [
            datastore_state,
            nostr_keystore_state,
            tangle_db_state
        ] = await Promise.all([
            export_datastore_state(),
            export_nostr_keystore_state(),
            export_tangle_db_state()
        ]);
        const payload: ExportedAppState = {
            backup_version: app_cfg.backup.version,
            exported_at: new Date().toISOString(),
            versions: {
                app: app_cfg.version,
                tangle_sql: tangle_db_state.backup.tangle_sql_version,
                backup_format: tangle_db_state.backup.format_version
            },
            datastore: datastore_state,
            nostr_keystore: nostr_keystore_state,
            database: tangle_db_state
        };
        const ts = payload.exported_at.replace(/[:.]/g, "-");
        download_json(payload, `radroots-app-state-${ts}.json`);
    } catch (e) {
        handle_err(e, `export_app_state`);
        await notif.alert(`Unable to export app state. Please try again.`);
    }
};
